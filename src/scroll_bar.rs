//! App-owned copy of the dissolved cce-ui `ScrollBar` (Phase 6av): settings is the last
//! consumer — the page scrollbar of the dissolved Page subtree, rendered into the window
//! assembly (`collect_window_child`) and evented directly (`dispatch_page_event` feeds it
//! through `propagate_event`). Phase 6az: on the narrow traits, wrapped in
//! `Adapted<ScrollBar>` — the constructor returns the wrapper so every call site keeps its
//! shape (WidgetHost methods on the wrapper, fields/`update` through Deref).

use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::PaintCtx;
use cce_ui::widget::{Adapted, Event, EventCtx, MouseButton, ElementState, ScrollbarActivity};

#[derive(Debug, Clone)]
pub struct ScrollBar {
    pub scroll_y: f32,
    pub content_h: f32,
    pub viewport_h: f32,
    pub dragging: bool,
    hovered: bool,
    /// The shared raise/sink hysteresis (the designer parameter-pane treatment):
    /// idle the bar sinks behind the translucent window plate and takes no
    /// input; a scroll raises it, hover sustains it, the hold decays in `tick`.
    activity: ScrollbarActivity,
}

impl ScrollBar {
    pub fn new() -> Adapted<ScrollBar> {
        Adapted::new(Self {
            scroll_y: 0.0,
            content_h: 0.0,
            viewport_h: 0.0,
            dragging: false,
            hovered: false,
            activity: ScrollbarActivity::new(),
        })
    }

    pub fn update(&mut self, scroll_y: f32, content_h: f32, viewport_h: f32) {
        self.scroll_y = scroll_y;
        self.content_h = content_h;
        self.viewport_h = viewport_h;
    }

    fn overflowing(&self) -> bool {
        self.content_h > self.viewport_h
    }

    /// Whether the bar currently rides in front of the content (and takes
    /// input) rather than idling behind the window plate.
    pub fn raised(&self) -> bool {
        self.activity.raised()
    }

    /// A scroll landed (wheel fast path, keyboard): refresh the hold and raise
    /// the bar in the same frame.
    pub fn on_scroll(&mut self) {
        self.activity.bump();
        let visible = self.overflowing();
        self.activity.recompute(visible, self.dragging);
    }

    /// Per-frame raise/sink upkeep; true = keep redrawing (hold running or the
    /// bar just flipped depth). Named apart from the `Input`/`WidgetHost` tick
    /// so the call through `Adapted`'s Deref can't collide.
    pub fn tick_activity(&mut self, dt: f32) -> bool {
        let holding = self.activity.holding();
        let visible = self.overflowing();
        self.activity.tick(dt, visible, self.dragging) || holding
    }

    /// The track + thumb quads for the host's two-layer emission: drawn under
    /// the window plate while sunk, over the page content while raised. Colors
    /// keep the widget's hover/drag tint.
    pub fn layer_quads(&self, rect: Rect) -> Vec<(Rect, [f32; 4])> {
        let mut out = Vec::new();
        if self.content_h > self.viewport_h && rect.height > 0.0 {
            out.push((rect, [0.15, 0.15, 0.20, 0.3]));
            if let Some((tx, ty, tw, th)) = self.thumb_rect(rect) {
                let thumb_color = if self.dragging {
                    [0.70, 0.70, 0.75, 0.6]
                } else if self.hovered && self.activity.raised() {
                    [0.65, 0.65, 0.70, 0.5]
                } else {
                    [0.60, 0.60, 0.65, 0.4]
                };
                out.push((Rect { x: tx, y: ty, width: tw, height: th }, thumb_color));
            }
        }
        out
    }

    fn thumb_rect(&self, rect: Rect) -> Option<(f32, f32, f32, f32)> {
        if self.content_h <= self.viewport_h || self.viewport_h <= 0.0 || rect.height <= 0.0 {
            return None;
        }
        let visible_ratio = self.viewport_h / self.content_h;
        let thumb_h = if rect.height <= 20.0 {
            rect.height
        } else {
            (rect.height * visible_ratio).clamp(20.0, rect.height)
        };
        let max_scroll = (self.content_h - self.viewport_h).max(0.0);
        let scroll_ratio = if max_scroll > 0.0 { self.scroll_y / max_scroll } else { 0.0 };
        let thumb_y = rect.y + scroll_ratio * (rect.height - thumb_h);

        Some((rect.x, thumb_y, rect.width, thumb_h))
    }

    /// Thumb-center-tracking drag scroll (legacy `on_cursor_moved`'s dragging branch).
    /// Returns whether the scroll position changed.
    fn drag_track(&mut self, py: f32, rect: Rect) -> bool {
        if let Some((_, _, _, thumb_h)) = self.thumb_rect(rect) {
            let track_scroll_range = rect.height - thumb_h;
            if track_scroll_range > 0.0 {
                let mouse_y_in_track = (py - rect.y).clamp(0.0, rect.height);
                let scroll_ratio = (mouse_y_in_track - thumb_h / 2.0) / track_scroll_range;
                let max_scroll = (self.content_h - self.viewport_h).max(0.0);
                let new_scroll_y = (scroll_ratio.clamp(0.0, 1.0) * max_scroll).clamp(0.0, max_scroll);
                if (self.scroll_y - new_scroll_y).abs() > 0.01 {
                    self.scroll_y = new_scroll_y;
                    return true;
                }
            }
        }
        false
    }
}

impl cce_ui::widget::Layout for ScrollBar {}

impl cce_ui::widget::Paint for ScrollBar {
    fn color(&self) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn paint(&self, _rect: Rect, _ctx: &mut PaintCtx) {
        // Deliberately empty: the bar straddles the window plate (sunk under it
        // idle, over the page content while raised), so the host emits it as two
        // possible layers in `display_list` via [`ScrollBar::layer_quads`] — a
        // single in-tree paint could only ever sit at one depth.
    }
}

impl cce_ui::widget::Input for ScrollBar {
    /// The legacy hit shape: ±6px horizontal grab margin, exact vertical span.
    fn hit(&self, rect: Rect, x: f32, y: f32) -> bool {
        let hit_margin = 6.0;
        x >= rect.x - hit_margin
            && x <= rect.x + rect.width + hit_margin
            && y >= rect.y
            && y <= rect.y + rect.height
    }

    fn on_event(&mut self, event: &Event, ectx: &mut EventCtx) -> bool {
        match event {
            // Presses arrive hit-gated (margin hit): grab the thumb and jump-scroll to the
            // press point, like the legacy `mouse_input` → `on_cursor_moved` pair.
            // Only a raised bar can be grabbed — sunk it sits behind the window
            // plate, so the press falls through to whatever the plate carries.
            Event::MouseButton { button: MouseButton::Left, state: ElementState::Pressed, y, .. } => {
                if !self.activity.raised() {
                    return false;
                }
                self.dragging = true;
                self.drag_track(*y, ectx.rect);
                true
            }
            // Releases arrive ungated: end the drag wherever the cursor is.
            Event::MouseButton { button: MouseButton::Left, state: ElementState::Released, .. } => {
                if self.dragging {
                    self.dragging = false;
                    // The release starts the hold window: the bar lingers
                    // briefly, then sinks back behind the plate.
                    self.activity.bump();
                    return true;
                }
                false
            }
            // Mid-drag moves track the thumb; non-drag moves fall through to the adapter's
            // hover bookkeeping (which synthesizes the Enter/Leave handled below).
            Event::PointerMove { y, .. } => {
                if self.dragging {
                    return self.drag_track(*y, ectx.rect);
                }
                false
            }
            Event::MouseEnter => {
                self.hovered = true;
                // Hover only SUSTAINS a raised bar (recomputed in tick); it can
                // never raise a sunk one — the plate is what the pointer is on.
                self.activity.set_hover(true);
                true
            }
            Event::MouseLeave => {
                self.hovered = false;
                self.activity.set_hover(false);
                true
            }
            _ => false,
        }
    }
}
