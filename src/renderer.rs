use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{DisplayList, PaintCtx, Prim, RenderTarget};
use cce_ui::widget::WidgetHostExt;
use crate::{SystemInterface, PageButton, make_text_buffer_with_font};
use cce_settings::app::PageContent;
use cce_settings::pages::Page;
use cce_ui::widget::WidgetHost;

/// What a rebuild leaves for `display_list` to emit, every frame, until the
/// next one. The page's lists are in PAGE coordinates and go out under the
/// scroll offset (`PaintCtx::append_translated`), so the wheel's fast path
/// moves the page by changing `scroll_y` alone; the popovers' are in window
/// coordinates, placed at the rebuild.
#[derive(Default)]
pub(crate) struct CachedFrame {
    /// The page's fills and carves in the order its view drew them, then the
    /// search's highlights, then each page button's face and plate.
    pub geo: DisplayList,
    /// The section wells, carved into the window plate after `geo`.
    pub wells: DisplayList,
    /// The page buttons' icon faces, then the page's own glyphs.
    pub icons: DisplayList,
    /// The page's text runs (dimmed or lit by a search), then the page
    /// buttons' labels.
    pub text: DisplayList,
    /// The open popovers — the page dropdown's, then the page widgets' — as
    /// geometry, glyphs and text: the text goes out after the page's.
    pub popover_geo: DisplayList,
    pub popover_icons: DisplayList,
    pub popover_text: DisplayList,
}

/// A text colour as the page's layers carry it (`PageContent`'s truncation).
fn text_rgb(c: [f32; 4]) -> [u8; 3] {
    [(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8]
}

/// Run `f` under `clip` (`[x, y, w, h]`) when there is one.
fn under_clip(pc: &mut PaintCtx, clip: Option<[f32; 4]>, f: impl FnOnce(&mut PaintCtx)) {
    match clip {
        Some([x, y, w, h]) => pc.clip(Rect { x, y, width: w, height: h }, f),
        None => f(pc),
    }
}

impl SystemInterface {

    /// Lay the page out and record it into the cached frame, keeping what
    /// its widgets claimed for text input (`text_claim`).
    pub(crate) fn rebuild_layout(&mut self, sw: f32, sh: f32) {
        let ((), claim) = cce_ui::text_input::capture(|| self.rebuild_layout_inner(sw, sh));
        self.text_claim = claim;
    }

    fn rebuild_layout_inner(&mut self, sw: f32, sh: f32) {
        if std::env::var("CCE_HOVER_DEBUG").is_ok() {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 100000;
            eprintln!("[hover] t={} rebuild", t);
        }
        // Keyboard focus on a per-rebuild button clone dies with the registry
        // wipe below — on EVERY rebuild, not just the one a Tab step triggers
        // (the page polls and re-lays-out on its own). Remember where the
        // focused widget sat: this pass lights the clone at that rect as it
        // collects its plate, and the pass's tail hands it the focus. A
        // persistent widget resolves to itself.
        self.refocus_rect = self
            .ui_context
            .focused_widget
            .and_then(|id| self.ui_context.get_widget(id))
            .map(|w| w.rect());
        // SectionContainer dissolved (Phase 6w): no per-rebuild section clones to
        // relink — the page's widgets dispatch directly (registration happens in
        // render_widget during the view pass below).
        self.ui_context.clear_hierarchy();

        self.sidebar_width = 0.0;
        self.header_height = 0.0; // No CSD Titlebar

        cce_ui::widget::hover_animation::reset_frame_registration();
        self.ui_context.clear_popovers();
        // The page scroll, where the toolkit's popover and context-menu hit
        // tests (and a TreeList's) still read it.
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        cce_ui::widget::hover_animation::set_cursor_pos(self.cursor_x, self.cursor_y);

        let cursor = (self.cursor_x, self.cursor_y);
        let over = |wx: f32, wy: f32, ww: f32, wh: f32| -> bool {
            cursor.0 >= wx && cursor.0 <= wx + ww && cursor.1 >= wy && cursor.1 <= wy + wh
        };

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw - self.sidebar_width;
        let mut lch = sh - self.header_height - self.status_height;
        if self.search_open {
            lch -= 42.0;
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.ui_context[self.page_dropdown].selected = page_idx;

        // root plate container DISSOLVED (Phase 6s): top-level widgets stay parentless
        // (`place` registers their popovers); the window plate, the root aggregate's
        // emission order, and the StatusBar's root plate container-coupled theming are
        // all replicated by hand in display_list.

        // The page switcher, at the status bar's right end.
        let dropdown_h = cce_ui::layout::dropdown_height();
        let size = self.ui_context[self.page_dropdown].measure(
            cce_ui::widget::LayoutConstraints::new(0.0, 500.0, dropdown_h, dropdown_h),
            &self.ui_context,
        );
        let dropdown_w = size.width;
        let dropdown_gap = (self.status_height - dropdown_h) / 2.0;
        let dropdown_x = sw - dropdown_w - dropdown_gap;
        let dropdown_y = sh - self.status_height + dropdown_gap;
        // The dropdown sits flush against the window's rounded bottom-right corner; with the
        // root plate container dissolved, hand it the plate frame for its concentric-corner cut.
        self.ui_context[self.page_dropdown].set_corner_frame(Some(((0.0, 0.0, sw, sh), 12.0, (true, true, true, true))));
        // Laid out here (and its open menu registered); painted live in
        // display_list (`paint_root_into`), chevron and all.
        cce_ui::compose::place_h(self.page_dropdown, dropdown_x, dropdown_y, dropdown_w, dropdown_h, &mut self.ui_context);
        let switcher_h = if self.search_open {
            sh - self.header_height - 42.0 - self.status_height
        } else {
            sh - self.header_height - self.status_height
        };
        // The page scrollbar, the DE's one design: it rides the page's
        // CENTRE line, over the sections with no lane of its own, idling
        // behind the root plate until a scroll raises it. Updated with LAST
        // frame's content height — the legacy window pass also ran before
        // this frame's content was measured.
        let sb_w = crate::scroll_bar::ScrollBar::width();
        // TODO(style): the bar's 4px vertical stand-off is the toolkit
        // track's own end inset, not a rung of the ladder.
        self.ui_context[self.page_scroll_bar].set_rect(
            lcx + (lcw - sb_w) * 0.5,
            self.header_height + 4.0,
            sb_w,
            switcher_h - 8.0,
        );
        if self.ui_context[self.page_scroll_bar].dragging {
            self.scroll_y = self.ui_context[self.page_scroll_bar].scroll_y;
        }
        self.ui_context[self.page_scroll_bar].update(self.scroll_y, self.content_h, switcher_h);

        // The search box, while open: laid out here in its band at the
        // window's foot, painted live in display_list. The search box stands
        // on the root plate: the window-edge inset, the toolkit's textbox
        // height centred in the 42px band.
        if self.search_open {
            let inset = cce_ui::layout::root_plate_inset();
            let box_h = cce_ui::layout::textbox_height();
            cce_ui::compose::place_h(
                self.search_box,
                self.sidebar_width + inset,
                sh - 42.0 + (42.0 - box_h) / 2.0,
                sw - self.sidebar_width - 2.0 * inset,
                box_h,
                &mut self.ui_context,
            );
        }

        // The page, in page coordinates (before the scroll).
        let page = self.render_page_content(lcx, lcy, lcw, lch).finish();
        let max_y = page.bottom();

        let searching = self.search_open && !self.search_query.is_empty();
        let query = self.search_query.to_lowercase();
        let matches = |t: &str| searching && t.to_lowercase().contains(&query);

        // A search jumps the page to its first match.
        if searching && !self.ui_context[self.page_scroll_bar].dragging {
            let first_match_y = page.text.items.iter().find_map(|item| match &item.prim {
                Prim::Text { text, y, .. } if matches(text) => Some(*y),
                _ => None,
            });
            if let Some(y) = first_match_y {
                let local_max_scroll_y = (max_y - lch).max(0.0);
                self.scroll_y = (y - 100.0).clamp(0.0, local_max_scroll_y);
            }
        }

        self.max_scroll_y = (max_y - lch).max(0.0);
        static mut FRAME_COUNT: usize = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT > 5 {
                self.scroll_y = self.scroll_y.min(self.max_scroll_y);
            }
        }

        if self.ui_context[self.page_scroll_bar].dragging {
            self.scroll_y = self.ui_context[self.page_scroll_bar].scroll_y;
        } else {
            self.ui_context[self.page_scroll_bar].scroll_y = self.scroll_y;
        }
        self.content_h = max_y;
        self.ui_context[self.page_scroll_bar].update(self.scroll_y, max_y, lch);

        let scroll_y = self.scroll_y;
        // The page's band in the window: under the header, over the status
        // bar. A page button's hit rect is cut to it.
        let viewport_bottom = sh - self.status_height;

        // The search: a matched run is lit and gets a highlight under it, the
        // rest dims to a quarter.
        let mut highlights = PaintCtx::new();
        let mut text = page.text.items;
        if searching {
            for item in &mut text {
                let Prim::Text { text, x, y, font_size, color, font, .. } = &mut item.prim else { continue };
                if matches(text) {
                    let buf = make_text_buffer_with_font(
                        &mut self.font_system,
                        text,
                        *font_size,
                        font.as_deref(),
                        &self.sans_serif_family,
                        &self.serif_family,
                        &self.monospace_family,
                    );
                    let scale = cce_ui::scale::scale_factor();
                    let text_w = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0) / scale;
                    let (pad_x, pad_y) = (4.0, 2.0);
                    highlights.rounded_rect(
                        Rect { x: *x - pad_x, y: *y - pad_y, width: text_w + 2.0 * pad_x, height: *font_size + 2.0 * pad_y },
                        3.0,
                        (true, true, true, true),
                        [0.65, 0.45, 0.05, 0.4],
                    );
                    *color = text_rgb([1.0, 0.95, 0.80, 1.0]);
                } else {
                    *color = color.map(|c| (c as f32 * 0.25) as u8);
                }
            }
        }

        // The page buttons: the page drew none of them, so each is drawn here
        // from what it is — its face (idle or hovered), its plate, its icon or
        // label — and a dispatch clone of it goes into the context at its rect
        // in the window, cut to its clip and the page's band. A button off
        // that band at this rebuild is left out altogether, until the next.
        let mut faces = PaintCtx::new();
        let mut button_icons = PaintCtx::new();
        let mut labels = PaintCtx::new();
        let mut page_buttons = Vec::new();
        for (btn, action, clip) in &page.buttons {
            let base = btn.base();
            // The button's own idle and hover faces: the page's colours when
            // it passed them, else the toolkit's for the button's kind — a
            // list row's transparent-until-hover wash (`PageContent::list_row`).
            let mut probe = (**btn).clone();
            probe.set_hovered(false);
            let bg = cce_ui::widget::Paint::color(&probe);
            probe.set_hovered(true);
            let hover_bg = cce_ui::widget::Paint::color(&probe);
            // Cut to the emission-time clip (page coords), so a partially
            // scrolled list row's button draws cut at the list edge, not bleeding.
            let (mut px0, mut py0, mut px1, mut py1) =
                (base.x, base.y, base.x + base.w, base.y + base.h);
            if let Some(c) = clip {
                px0 = px0.max(c[0]);
                py0 = py0.max(c[1]);
                px1 = px1.min(c[0] + c[2]);
                py1 = py1.min(c[1] + c[3]);
                if px0 >= px1 || py0 >= py1 {
                    continue;
                }
            }
            // Where that lands in the window now, cut to the page's band.
            let wx = px0;
            let mut wy = py0 - scroll_y;
            let ww = px1 - px0;
            let mut wh = py1 - py0;
            if wy >= viewport_bottom || wy + wh <= 0.0 {
                continue;
            }
            if wy < 0.0 {
                wh = (wh + wy).max(0.0);
                wy = 0.0;
            }
            if wy + wh > viewport_bottom {
                wh = (viewport_bottom - wy).max(0.0);
            }
            let hovered = over(wx, wy, ww, wh);

            // The face: shrunk to the clip, its corners rounded at the cut.
            let face = Rect { x: px0, y: py0, width: px1 - px0, height: py1 - py0 };
            let radius = cce_ui::layout::button_corner_radius();
            let color = if hovered { hover_bg } else { bg };
            if radius > 0.1 {
                faces.rounded_rect(face, radius, (true, true, true, true), color);
            } else {
                faces.quad(face, color);
            }
            // Its plate, right after the face, as `Button::paint` draws it —
            // under the button's own clip, as its face is cut: drawn whole, a
            // row half-scrolled out of a list kept its plate standing past the
            // list's edge.
            if cce_ui::layout::control_relief() {
                let rect = Rect { x: base.x, y: base.y, width: base.w, height: base.h };
                if let Some(mut plate) = btn.plate(rect) {
                    // This pass's clone of the button a Tab step focused (its
                    // registered rect, window coords, matches `refocus_rect`;
                    // the pass's tail hands it the focus): light its rim now.
                    let near = |a: f32, b: f32| (a - b).abs() < 0.5;
                    let refocused = self
                        .refocus_rect
                        .is_some_and(|(fx, fy, fw, fh)| near(wx, fx) && near(wy, fy) && near(ww, fw) && near(wh, fh));
                    if refocused {
                        plate.tint = Some(cce_ui::widget::ControlPlate::focus_tint());
                    }
                    let r = plate.rect;
                    let (fill, radius, depth, tint) = (plate.face_fill(), plate.radii.0, plate.depth, plate.tint);
                    under_clip(&mut faces, *clip, |pc| match tint {
                        Some(t) => RenderTarget::inset_plate_tinted(pc, fill, r.x, r.y, r.width, r.height, radius, depth, t),
                        None => RenderTarget::inset_plate(pc, fill, r.x, r.y, r.width, r.height, radius, depth),
                    });
                }
            }
            // An icon face replaces the label entirely (as it does in
            // `Button::paint`), placed by the button's own `icon_rect` and
            // under its clip: the page clip alone let a row half-scrolled out
            // of a list show its whole glyph past the list's edge.
            let icon = btn.icon_rect(Rect { x: base.x, y: base.y, width: base.w, height: base.h });
            if let Some((image, irect, alpha)) = icon {
                under_clip(&mut button_icons, *clip, |pc| pc.image(image, irect, alpha));
            } else {
                // The label: 12px, shaped as the renderer shapes it, centred —
                // or left-aligned in a list.
                let label = base.label.as_deref().unwrap_or("");
                let label_size = 12.0;
                let buf = make_text_buffer_with_font(
                    &mut self.font_system,
                    label,
                    label_size,
                    btn.widget_font().as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                );
                let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
                let lh = buf.metrics().line_height;
                let mut left_align = btn.justify == cce_ui::widget::Justification::Left;

                // Auto-detect if inside a list frame to apply left alignment by default
                if !left_align && base.w >= 60.0 {
                    if self.app.current_page == Page::Processes {
                        let sb1 = &self.app.processes.cpu_list;
                        if base.x >= sb1.x - 1.0 && base.x + base.w <= sb1.x + sb1.w + 1.0
                           && base.y >= sb1.y - 1.0 && base.y + base.h <= sb1.y + sb1.h + 1.0 {
                            left_align = true;
                        }
                    }
                    if self.app.current_page == Page::Services {
                        let sb2 = &self.app.services.list;
                        if base.x >= sb2.x - 1.0 && base.x + base.w <= sb2.x + sb2.w + 1.0
                           && base.y >= sb2.y - 1.0 && base.y + base.h <= sb2.y + sb2.h + 1.0 {
                            left_align = true;
                        }
                    }
                }

                let scale_factor = cce_ui::scale::scale_factor();
                let logical_tw = tw / scale_factor;
                let logical_lh = lh / scale_factor;
                let text_x = if left_align {
                    base.x + cce_ui::layout::CONTROL_TEXT_INSET
                } else {
                    base.x + (base.w - logical_tw) / 2.0
                };
                let label_color = btn.label_color.unwrap_or([0.83, 0.83, 0.83, 1.0]);
                // Bounded by the button's own (clip-cut) box — a label longer
                // than its button truncates instead of spilling over the neighbour.
                labels.text_with(
                    label,
                    text_x,
                    base.y + (base.h - logical_lh) / 2.0,
                    label_size,
                    text_rgb(label_color),
                    btn.widget_font(),
                    Some([px0, py0, px1, py1]),
                );
            }
            // The dispatch clone hit-tests at the CUT rect, so clicks in a
            // row's clipped-away region fall through to what's visible there.
            // The label is skipped for an icon face, but not the clone: an
            // icon button still has to be clickable.
            let mut clone = btn.clone();
            {
                let b = clone.base_mut();
                (b.x, b.y, b.w, b.h) = (wx, wy, ww, wh);
            }
            page_buttons.push((clone, action.clone(), hovered));
        }

        // The page's own glyphs follow the buttons' and dim with the text a
        // search leaves unmatched.
        let mut icons = button_icons.finish().items;
        icons.extend(page.icons.items.into_iter().map(|mut item| {
            if let (true, Prim::Image { alpha, .. }) = (searching, &mut item.prim) {
                *alpha *= 0.25;
            }
            item
        }));

        // The section wells: the designer's union carve, radius 20, as deep
        // as the bevel (never more than a fifth of the well).
        let mut wells = PaintCtx::new();
        for &((x, y, w, h), tab) in &page.reliefs {
            wells.section_well(
                Rect { x, y, width: w, height: h },
                tab.map(|(x, y, w, h)| Rect { x, y, width: w, height: h }),
                (20.0, 20.0, 20.0, 20.0),
                cce_ui::layout::bevel_width().min(h * 0.2),
            );
        }

        // Popovers draw INTO the frame (Phase 6t; there is no engine xdg
        // popup), over the whole window, so the dropdown's open-upward menu
        // renders where it hit-tests. The page dropdown's is in window coords;
        // a page widget's is in page coords, lifted by this rebuild's scroll —
        // the same subtraction the old popup positioner applied at creation.
        // dl-text occlusion comes from the context's popover registration.
        let chrome_id = self.page_dropdown.id();
        let mut chrome_pop = PageContent::new();
        let mut page_pop = PageContent::new();
        for &pop_id in &self.ui_context.active_popovers {
            let Some(popover) = self.ui_context.get_widget(pop_id) else { continue };
            if pop_id == chrome_id {
                popover.render_popover(&mut chrome_pop);
            } else {
                popover.render_popover(&mut page_pop);
            }
        }
        let (chrome_pop, page_pop) = (chrome_pop.finish(), page_pop.finish());
        let lifted = |chrome: DisplayList, page: DisplayList| DisplayList {
            items: chrome.items.into_iter().chain(page.items.into_iter().map(|i| i.translated(0.0, -scroll_y))).collect(),
        };
        // The context menu is not collected here: display_list paints it
        // straight into the frame with `context_menu::paint_with_labels`.

        let mut geo = page.geo.items;
        geo.extend(highlights.finish().items);
        geo.extend(faces.finish().items);
        text.extend(labels.finish().items);
        self.frame = CachedFrame {
            geo: DisplayList { items: geo },
            wells: wells.finish(),
            icons: DisplayList { items: icons },
            text: DisplayList { items: text },
            popover_geo: lifted(chrome_pop.geo, page_pop.geo),
            popover_icons: lifted(chrome_pop.icons, page_pop.icons),
            popover_text: lifted(chrome_pop.text, page_pop.text),
        };

        // The dispatch copies are the context's: last frame's go back, this frame's go in.
        for b in std::mem::take(&mut self.page_buttons) {
            self.ui_context.remove(b.handle);
        }
        let ctx = &mut self.ui_context;
        self.page_buttons = page_buttons
            .into_iter()
            .map(|(b, action, hovered)| PageButton { handle: ctx.insert(b), action, hovered })
            .collect();

        // A Tab step's focus, handed to the fresh clone at the same rect (the
        // page buttons above are per-rebuild allocations; see `focus_stepped`).
        if let Some((fx, fy, fw, fh)) = self.refocus_rect.take() {
            let near = |a: f32, b: f32| (a - b).abs() < 0.5;
            let heir = self.ui_context.widgets().find_map(|(id, w)| {
                let (x, y, ww, hh) = w.rect();
                (w.focus_role() != cce_ui::widget::FocusRole::None && near(x, fx) && near(y, fy) && near(ww, fw) && near(hh, fh))
                    .then_some(id)
            });
            if let Some(id) = heir {
                self.ui_context.set_focused_id(id);
            }
        }
        self.needs_rebuild = false;
        self.laid_out_page = Some(self.app.current_page);
        self.ui_context.clear_dirty();
    }

    pub(crate) fn render_page_content(&mut self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use cce_ui::compose::PageFlow;
        // The sections are wells carved straight into the root plate (no
        // sidebar, no pane between), so the page's edge is the window's edge:
        // the root rung's inset, and the grid's gap the root rung's gap (set
        // once in main; `PageFlow::init` reads the toolkit's grid getters).
        let margin = cce_ui::layout::root_plate_inset();
        let cx = cx + margin;
        let cy = cy + margin;
        // No lane for the page scrollbar: it rides the page's centre line,
        // over the sections, behind the root plate until a scroll raises it.
        let cw = (cw - 2.0 * margin).max(1.0);
        let ch = (ch - 2.0 * margin).max(1.0);
        let mut layout = PageFlow::new();
        // Page root dissolved (6u): the ctrl-nav entry focuses section 0, so root focus is
        // permanently false; views that highlighted on it OR in their first section's bool.
        // Section focus is the app-side index now (Phase 6w).
        let n_sections = self.app.get_current_page_mut().section_widgets().len();
        let sec_focused: Vec<bool> = (0..n_sections)
            .map(|i| self.focused_section == Some(i))
            .collect();
        self.app.get_current_page_mut().view(cx, cy, cw, ch, false, &sec_focused, &mut layout, &mut self.ui_context)
    }

}
