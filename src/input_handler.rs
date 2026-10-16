use crate::SystemInterface;
use cce_settings::app::AppAction;
use cce_settings::pages::Page;

/// App shortcuts, resolved once per process from input.kdl
/// (`cce-system-interface` domain → `cce-ui` domain).
struct SettingsKeys {
    open_search: String,
    focus_next: String,
    focus_prev: String,
    focus_ascend: String,
    focus_descend: String,
    page_next: String,
    page_prev: String,
}

fn settings_keys() -> &'static SettingsKeys {
    static KEYS: std::sync::OnceLock<SettingsKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        // Defaults from the Keyboard page's table, so the page lists what is used.
        let get = |name: &str| cce_ui::input::app_chord(name, cce_settings::pages::keyboard::default_chord(name));
        SettingsKeys {
            open_search: get("open_search"),
            focus_next: get("focus_next"),
            focus_prev: get("focus_prev"),
            focus_ascend: get("focus_ascend"),
            focus_descend: get("focus_descend"),
            page_next: get("page_next"),
            page_prev: get("page_prev"),
        }
    })
}

/// `CCE_HOVER_DEBUG`, resolved once. The move handler runs ~60/s, so the old
/// per-event `std::env::var` (which allocates and scans the environment) had no
/// business being on this path.
fn hover_debug() -> bool {
    static FLAG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FLAG.get_or_init(|| std::env::var_os("CCE_HOVER_DEBUG").is_some())
}

impl SystemInterface {

    pub(crate) fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        // Runs once per pointer motion event — ~60/s while the mouse moves — and
        // the loop trace puts a single dispatch of ~60 of them at ~839ms, i.e.
        // ~14ms each. Time the stages to find which scales with the 1359-row list.
        let hov_t0 = if hover_debug() {
            Some(std::time::Instant::now())
        } else {
            None
        };
        self.cursor_x = x;
        self.cursor_y = y;
        let s = 1.0f32;
        let lx_no_scroll = x / s;
        let ly_no_scroll = y / s;

        let sh_logical = self.height as f32 / s;
        if self.search_open && ly_no_scroll >= (sh_logical - 42.0) {
            // Routed (6bd): chrome coords, no scroll offset — same as the direct call.
            let ev = cce_ui::widget::Event::PointerMove { x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
            let sb = self.search_box.id();
            if self.ui_context.propagate_event(&ev, sb) {
                self.needs_rebuild = true;
            }
            return true;
        }
        
        if cce_ui::widget::context_menu::is_visible() {
            if cce_ui::widget::context_menu::cursor_moved(lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
            return false;
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let mut changed = false;
        {
            let ev = cce_ui::widget::Event::PointerMove { x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
            let dd = self.page_dropdown.id();
            if self.ui_context.propagate_event(&ev, dd) {
                changed = true;
            }
        }
        let t_dropdown = hov_t0.map(|s| s.elapsed().as_micros());

        // The open page switcher's menu covers the page: the page gets the
        // pointer as gone, far off every widget, so whatever it lit un-lights
        // and nothing under the menu lights.
        let (lx, ly) = if self.under_switcher_menu(lx_no_scroll, ly_no_scroll) { (-1.0e6, -1.0e6) } else { (lx, ly) };

        // Drag updates are high-priority overrides
        let mut drag_handled = false;
        let mut drag_actions = Vec::new();
        if self.app.get_current_page_mut().handle_pointer_move(lx, ly, &mut drag_actions, &mut self.ui_context) {
            drag_handled = true;
            changed = true;
            for action in drag_actions {
                self.handle_action(&action);
            }
        }

        let t_pagemove = hov_t0.map(|s| s.elapsed().as_micros());

        if !drag_handled {
            let event = cce_ui::widget::Event::PointerMove { x: lx, y: ly, local_x: lx, local_y: ly };
            if self.dispatch_page_event(&event) {
                changed = true;
            }
            // Routed drags (6bd): a DragUpdate delivered inside the dispatch surfaces as
            // the widget's Changed — swept and acted on per move, as the old page drag
            // hooks did.
            let mut move_actions = Vec::new();
            self.propagate_widget_changes(&mut move_actions);
            if !move_actions.is_empty() {
                changed = true;
                for action in move_actions {
                    self.handle_action(&action);
                }
            }
        }

        let t_dispatch = hov_t0.map(|s| s.elapsed().as_micros());

        // A page button's face is painted lit or not at the rebuild (the
        // page is cached until the next one): rebuild when the pointer
        // crossed one's edge since — or a scroll carried one under it.
        let stale = self.page_buttons.iter().any(|b| self.page_button_over(b, x, y) != b.hovered);
        if stale {
            changed = true;
        }

        if changed {
            self.needs_rebuild = true;
        }
        if let (Some(s), Some(dd), Some(pm), Some(dp)) = (hov_t0, t_dropdown, t_pagemove, t_dispatch) {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() % 100000;
            eprintln!(
                "[hover] t={} move ({:.0},{:.0}) changed={} total={}us dropdown={}us page={}us dispatch={}us buttons={}us n_buttons={}",
                t, self.cursor_x, self.cursor_y, changed,
                s.elapsed().as_micros(), dd, pm - dd, dp - pm, s.elapsed().as_micros() - dp,
                self.page_buttons.len()
            );
        }
        changed
    }

    pub(crate) fn handle_mouse_input_internal(&mut self, button: cce_ui::widget::MouseButton, state: cce_ui::widget::ElementState) -> bool {
        let s = 1.0f32;
        let lx_no_scroll = self.cursor_x / s;
        let ly_no_scroll = self.cursor_y / s;

        let sh_logical = self.height as f32 / s;
        if self.search_open && ly_no_scroll >= (sh_logical - 42.0) {
            let ev = cce_ui::widget::Event::MouseButton { button, state, x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
            let sb = self.search_box.id();
            if self.ui_context.propagate_event(&ev, sb) {
                self.needs_rebuild = true;
            }
            return true;
        }

        if self.search_open && state == cce_ui::widget::ElementState::Pressed && ly_no_scroll < (sh_logical - 42.0) {
            self.search_open = false;
            self.search_query.clear();
            self.ui_context[self.search_box].set_value_string("");
            self.ui_context.clear_focus();
            self.needs_rebuild = true;
        }

        // CSD Close Button Interaction removed

        if cce_ui::widget::context_menu::is_visible()
            && cce_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll, Some(&mut self.ui_context))
        {
            let mut actions = Vec::new();
            self.propagate_widget_changes(&mut actions);
            for action in actions {
                self.handle_action(&action);
            }
            self.needs_rebuild = true;
            return true;
        }

        let dd_ev = cce_ui::widget::Event::MouseButton { button, state, x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
        let dd_root = self.page_dropdown.id();
        if self.ui_context.propagate_event(&dd_ev, dd_root) {
            // A page picked from its menu turns the page in the sweep.
            let mut actions = Vec::new();
            self.propagate_widget_changes(&mut actions);
            for action in actions {
                self.handle_action(&action);
            }
            self.needs_rebuild = true;
            return true;
        }

        if button != cce_ui::widget::MouseButton::Left && button != cce_ui::widget::MouseButton::Right { return false; }

        // The page buttons, in page coordinates. Every one sees the event —
        // a release has to reach the button it was pressed on wherever it
        // lands — but where a button cannot be reached (under the status bar,
        // outside the clip it was painted under) the point is somewhere no
        // button is, so a press there misses it and a release cancels it.
        let mut button_handled = false;
        let btn_events: Vec<_> = self
            .page_buttons
            .iter()
            .map(|b| {
                let (px, py) = self.page_button_point(b, self.cursor_x, self.cursor_y).unwrap_or((-1.0e6, -1.0e6));
                (b.handle.id(), cce_ui::widget::Event::MouseButton { button, state, x: px, y: py, local_x: px, local_y: py })
            })
            .collect();
        for (root, ev) in btn_events {
            if self.ui_context.propagate_event(&ev, root) {
                button_handled = true;
            }
        }
        // A release that completed a click: the sweep puts the button's
        // action first.
        let mut actions = Vec::new();
        if self.propagate_widget_changes(&mut actions) {
            button_handled = true;
        }
        for action in &actions {
            self.handle_action(action);
        }
        if button_handled {
            return true;
        }
        let mut actions = Vec::new();
        if button == cce_ui::widget::MouseButton::Left
            && state == cce_ui::widget::ElementState::Released
            && self.app.get_current_page_mut().handle_pointer_up(&mut self.ui_context)
        {
            self.needs_rebuild = true;
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let event = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
        self.dispatch_page_event(&event);

        if state == cce_ui::widget::ElementState::Pressed {
            self.app.get_current_page_mut().handle_pointer_down(lx, ly, &mut self.ui_context);
        }

        self.propagate_widget_changes(&mut actions);

        // Single-slot focus (Phase 6w): if a widget click took the window's focus, the
        // section-level highlight yields.
        if state == cce_ui::widget::ElementState::Pressed && self.ui_context.has_focus() {
            self.focused_section = None;
        }

        for a in &actions {
            self.handle_action(a);
        }
        if !actions.is_empty() {
            self.needs_rebuild = true;
            return true;
        }
        self.needs_rebuild = true;
        true
    }

    /// One sweep of what the widgets reported since the last
    /// (`UiContext::take_events`), acted on. The window chrome's own are
    /// taken here: a page picked in the switcher turns the page, the search
    /// box's text becomes the query, and a page button's click puts its
    /// action first in `actions`. The rest go to the current page, which
    /// turns its widgets' into `actions` (another page's widgets have nothing
    /// to report: none of them is laid out, focused or routed to). Run after
    /// every pointer, wheel and key dispatch and on each tick, so a click or
    /// change from the keyboard or an assistive tool is acted on at once,
    /// not with the next pointer event. True when a page button was clicked.
    pub(crate) fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) -> bool {
        use cce_ui::widget::WidgetEvent;
        let events = self.ui_context.take_events();
        if events.is_empty() {
            return false;
        }
        let (switcher, search) = (self.page_dropdown.id(), self.search_box.id());
        let (mut picked_page, mut search_changed) = (false, false);
        let mut clicked = Vec::new();
        let mut page_events = Vec::with_capacity(events.len());
        for (id, ev) in events {
            match ev {
                WidgetEvent::Changed if id == switcher => picked_page = true,
                WidgetEvent::Changed if id == search => search_changed = true,
                WidgetEvent::Clicked if self.page_buttons.iter().any(|b| b.handle.id() == id) => clicked.push(id),
                ev => page_events.push((id, ev)),
            }
        }

        // Overlapping page buttons all see a press; the LAST in page order
        // that took the click wins (a process row's Kill over the row).
        let button = self.page_buttons.iter().rev().find(|b| clicked.contains(&b.handle.id()));
        let button = button.map(|b| b.action.clone());
        let button_clicked = button.is_some();
        if let Some(action) = button {
            actions.push(action);
            self.needs_rebuild = true;
        }

        if !page_events.is_empty() {
            self.app.get_current_page_mut().propagate_widget_changes(&page_events, actions, &mut self.ui_context);
        }

        // Typed into while open; a closed box's change is the echo of the
        // clear that closed it.
        if search_changed && self.search_open {
            self.search_query = self.ui_context[self.search_box].text.clone();
            self.needs_rebuild = true;
        }

        if picked_page {
            let idx = self.ui_context[self.page_dropdown].selected;
            if idx < Page::ALL.len() {
                self.ui_context.clear_focus();
                self.focused_section = None;
                self.app.current_page = Page::ALL[idx];
                self.current_page_shared.send_replace(idx as u8);
                self.scroll_y = 0.0;
            }
            self.needs_rebuild = true;
        }
        button_clicked
    }

    pub(crate) fn handle_mouse_wheel_internal(&mut self, delta: &cce_ui::widget::MouseScrollDelta, px: f32, py: f32) -> bool {
        log::trace!(
            "handle_mouse_wheel_internal: px={}, py={}, delta={:?}, sidebar_w={}",
            px, py, delta, self.sidebar_width
        );
        let s = 1.0f32;
        // Over the open page switcher's menu the wheel is the menu's, never
        // the page's behind it.
        if self.under_switcher_menu(px / s, py / s) {
            let ev = cce_ui::widget::Event::MouseWheel { delta: *delta, x: px / s, y: py / s, local_x: px / s, local_y: py / s };
            let dd = self.page_dropdown.id();
            if self.ui_context.propagate_event(&ev, dd) {
                self.needs_rebuild = true;
            }
            return true;
        }
        if px >= self.sidebar_width * s {
            let lx = px / s;
            let ly = py / s + self.scroll_y;
            


            // The gesture's bookkeeping, before anything asks who owns it
            // (a page with no controls and no lists routes nothing that would).
            self.ui_context.note_scroll_event();
            let event = cce_ui::widget::Event::MouseWheel { delta: *delta, x: lx, y: ly, local_x: lx, local_y: ly };
            // Page dissolved (6u): one dispatch path for every page — scrollbar, then
            // sections, then the dissolved inner lists (the app-owned ScrollRegions,
            // hit-scoped like the old inner ScrollBoxes), then the manual page scroll
            // below as the fallback, exactly as the non-System pages worked.
            let mut handled = self.dispatch_page_event(&event);
            if !handled {
                handled = self.app.get_current_page_mut().handle_mouse_wheel(delta, lx, ly, &mut self.ui_context);
            }

            let mut actions = Vec::new();
            self.propagate_widget_changes(&mut actions);
            for a in &actions {
                self.handle_action(a);
            }
            if !actions.is_empty() {
                self.needs_rebuild = true;
                return true;
            }
            if handled {
                self.needs_rebuild = true;
                return true;
            }

            // The whole-page scroll: a wheel notch moves the motion's target
            // and `tick_page_scroll` glides the page there; a trackpad finger
            // moves it now. Either way the bar raises in the same frame.
            // It takes part in the scroll-gesture rule as the lists do: a
            // gesture a control or a list took leaves the page put, and one
            // the page moves is the page's, so the controls its scroll
            // carries under the pointer decline the rest of it.
            use cce_ui::widget::{Bounds, LINE_PX};
            if !self.ui_context.may_take_wheel(self.page_gesture_id) {
                return false;
            }
            self.page_scroll_motion.reconcile(0.0, self.scroll_y);
            let moved = self.page_scroll_motion.apply(delta, (LINE_PX, LINE_PX), Bounds::max(0.0), Bounds::max(self.max_scroll_y));
            if moved {
                self.ui_context.claim_scroll_gesture(self.page_gesture_id);
                self.shift_page_to(self.page_scroll_motion.y.pos());
                self.ui_context[self.page_scroll_bar].on_scroll();
                return true;
            }
        }
        false
    }

    /// Move the page to `new_scroll_y` WITHOUT a rebuild — the wheel fast
    /// path. The page is cached in page coordinates and drawn under
    /// `scroll_y`, and its widgets (the page buttons too) live in page
    /// coordinates, so the offset is all that moves, with the scrollbar thumb
    /// (display_list emits the bar fresh each frame from this state). False
    /// when the offset did not actually change.
    pub(crate) fn shift_page_to(&mut self, new_scroll_y: f32) -> bool {
        if (new_scroll_y - self.scroll_y).abs() <= 0.01 {
            return false;
        }
        self.scroll_y = new_scroll_y;
        self.ui_context[self.page_scroll_bar].scroll_y = self.scroll_y;
        true
    }

    /// The current page's event-dispatch roots (Phase 6w — SectionContainer dissolved):
    /// the pages' widgets themselves, flattened in the legacy propagate order (sections
    /// last-to-first, and within a section the container children were visited in
    /// reverse link order).
    pub(crate) fn page_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        self.app
            .get_current_page_mut()
            .section_widgets()
            .into_iter()
            .rev()
            .flat_map(|group| group.into_iter().rev())
            .collect()
    }

    /// Replicates the dissolved Page's event routing: the out-of-bounds gate (events whose
    /// screen position is outside the page viewport never reach page widgets, unless the
    /// scrollbar is mid-drag), then the legacy child order reversed — the scrollbar first
    /// (with Page's y-unshift, its coords are screen-space while the event carries the
    /// scroll offset), then the sections last-to-first. PointerMove visits everything
    /// (hover bookkeeping); other events stop at the first handler.
    pub(crate) fn dispatch_page_event(&mut self, event: &cce_ui::widget::Event) -> bool {
        use cce_ui::widget::Event;
        // A page switch takes effect immediately, but widget registration is a side effect
        // of the view pass and `clear_hierarchy` wipes the registry every rebuild — so
        // until the new page has been laid out once, none of its `section_widgets()` ids
        // resolve. Dispatching anyway can't reach a widget; it only emits one
        // "unregistered/stale root" warning per root (the pages that implement
        // `register_extra_dispatch_roots` were incidentally immune, which is why only 9 of
        // the 14 spammed). Suppress across the gap instead — the rebuild is one frame away.
        if self.laid_out_page != Some(self.app.current_page) {
            return false;
        }
        let is_pointer_event = matches!(
            event,
            Event::PointerMove { .. } | Event::MouseButton { .. } | Event::MouseWheel { .. }
        );
        if is_pointer_event && !self.ui_context[self.page_scroll_bar].dragging && !self.ui_context.is_dragging {
            if let Event::PointerMove { x, y, .. }
            | Event::MouseButton { x, y, .. }
            | Event::MouseWheel { x, y, .. } = event
            {
                let rx = self.sidebar_width;
                let ry = self.header_height;
                let rw = self.width as f32 - self.sidebar_width;
                let mut rh = self.height as f32 - self.header_height - self.status_height;
                if self.search_open {
                    rh -= 42.0;
                }
                let screen_y = *y - self.scroll_y;
                if *x < rx || *x > rx + rw || screen_y < ry || screen_y > ry + rh {
                    return false;
                }
            }
        }

        let is_pointer_move = matches!(event, Event::PointerMove { .. });
        let mut handled = false;

        // Open-popover priority: a press inside an open menu goes to its owner
        // BEFORE positional dispatch — the roots iterate in z-ignorant order,
        // so a closed sibling whose trigger band sits under the overlaying
        // popover would otherwise claim the point first (the Default Apps
        // Terminal dropdown's menu covers the Images row's trigger).
        if let Event::MouseButton { x, y, .. } = event {
            if let Some(owner) = self.ui_context.popover_owner_at(*x, *y) {
                if self.ui_context.propagate_event(event, owner) {
                    return true;
                }
            }
        }

        if self.ui_context[self.page_scroll_bar].content_h > self.ui_context[self.page_scroll_bar].viewport_h {
            let mut sb_event = event.clone();
            if let Event::PointerMove { y, local_y, .. }
            | Event::MouseButton { y, local_y, .. }
            | Event::MouseWheel { y, local_y, .. } = &mut sb_event
            {
                *y -= self.scroll_y;
                *local_y -= self.scroll_y;
            }
            let sb_root = self.page_scroll_bar.id();
            if self.ui_context.propagate_event(&sb_event, sb_root) {
                if !is_pointer_move {
                    return true;
                }
                handled = true;
            }
        }

        // Row widgets of the dissolved lists (Phase 6v): they used to receive events as
        // ScrollBox children under the sections; now they dispatch directly. Adapted's
        // hit-gate keeps missed presses falling through, so order vs the sections only
        // matters for overlap — and the rows sit inside list frames the sections never
        // claim. Collected fresh per event: the item Vecs get rebuilt across frames.
        let dbg_t0 = if hover_debug() && is_pointer_move {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let extra_roots = {
            let page = self.app.get_current_page_mut();
            page.register_extra_dispatch_roots(&mut self.ui_context);
            page.extra_dispatch_roots()
        };
        let t_register = dbg_t0.map(|s| s.elapsed().as_micros());
        let n_roots = extra_roots.len();
        for root in extra_roots {
            if self.ui_context.propagate_event(event, root) {
                if !is_pointer_move {
                    return true;
                }
                handled = true;
            }
        }
        if let (Some(s), Some(reg)) = (dbg_t0, t_register) {
            let total = s.elapsed().as_micros();
            eprintln!(
                "[hover]   extra_roots n={} register+collect={}us propagate={}us",
                n_roots, reg, total - reg
            );
        }

        let roots = self.page_dispatch_roots();
        for root in roots {
            if self.ui_context.propagate_event(event, root) {
                if !is_pointer_move {
                    return true;
                }
                handled = true;
            }
        }
        handled
    }

    pub(crate) fn handle_key_input_internal(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        if cce_ui::widget::context_menu::is_visible()
            && event.state == cce_ui::widget::ElementState::Pressed
            && event.logical_key == cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape)
        {
            cce_ui::widget::context_menu::hide();
            self.needs_rebuild = true;
            return true;
        }

        if self.search_open
            && event.state == cce_ui::widget::ElementState::Pressed
            && event.logical_key == cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape)
        {
            self.search_open = false;
            self.search_query.clear();
            self.ui_context[self.search_box].set_value_string("");
            self.ui_context.clear_focus();
            self.needs_rebuild = true;
            return true;
        }

        let is_text_box_focused = self
            .ui_context
            .focused_widget
            .and_then(|id| self.ui_context.get_widget(id))
            .is_some_and(|focused| focused.as_any().is::<cce_ui::widget::input::TextBox>());

        if !self.search_open
            && !is_text_box_focused
            && event.state == cce_ui::widget::ElementState::Pressed
            && !event.repeat
        {
            if cce_ui::widget::match_key_shortcut(event, &settings_keys().open_search) {
                self.search_open = true;
                self.ui_context[self.search_box].set_value_string("");
                self.search_query.clear();
                cce_ui::widget::WidgetHost::focus(&mut self.ui_context[self.search_box]);
                self.ui_context.set_focused_id(self.search_box.id());
                self.needs_rebuild = true;
                return true;
            }

            let next = cce_ui::widget::match_key_shortcut(event, &settings_keys().page_next);
            let prev = cce_ui::widget::match_key_shortcut(event, &settings_keys().page_prev);
            if next || prev {
                let n = Page::ALL.len();
                let cur = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
                let idx = if next { (cur + 1) % n } else { (cur + n - 1) % n };
                self.ui_context.clear_focus();
                self.focused_section = None;
                self.app.current_page = Page::ALL[idx];
                self.current_page_shared.send_replace(idx as u8);
                self.scroll_y = 0.0;
                self.needs_rebuild = true;
                return true;
            }
        }

        if event.state == cce_ui::widget::ElementState::Pressed && !event.repeat {
            let keys = settings_keys();
            let m = |chord: &str| cce_ui::widget::match_key_shortcut(event, chord);
            let (forward, backward, ascend, descend) = (
                m(&keys.focus_next),
                m(&keys.focus_prev),
                m(&keys.focus_ascend),
                m(&keys.focus_descend),
            );
            if forward || backward || ascend || descend {
                // SectionContainer dissolved (Phase 6w): section-level focus is the
                // app-side index, widget-level focus is the window's (`UiContext`),
                // and the two are single-slot (as when sections and widgets shared the
                // one FOCUSED_WIDGET). Nav within a section walks the page's widget
                // group where the container's child list used to be walked.
                if self.ui_context.has_focus() {
                    // (`focus::navigate_focus` is gone — it walked an empty dummy context
                    // and always returned false here; the section machinery below is the
                    // real ctrl-nav.)
                    let groups = self.app.get_current_page_mut().section_widgets();
                    let focused_pos = groups.iter().enumerate().find_map(|(si, g)| {
                        g.iter()
                            .position(|&id| self.ui_context.is_focused_id(id))
                            .map(|wi| (si, wi))
                    });
                    if let Some((si, wi)) = focused_pos {
                        if forward || backward {
                            let group = &groups[si];
                            let next = if forward {
                                (wi + 1) % group.len()
                            } else if wi == 0 {
                                group.len() - 1
                            } else {
                                wi - 1
                            };
                            let next_id = group[next];
                            // The context tells the widget (FocusIn), as the Tab walk does.
                            self.ui_context.set_focused_id(next_id);
                            self.needs_rebuild = true;
                            return true;
                        }
                        if ascend {
                            self.ui_context.clear_focus();
                            self.focused_section = Some(si);
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                } else if let Some(idx) = self.focused_section {
                    let groups = self.app.get_current_page_mut().section_widgets();
                    if !groups.is_empty() {
                        let idx = idx.min(groups.len() - 1);
                        if forward || backward {
                            self.focused_section = Some(if forward {
                                (idx + 1) % groups.len()
                            } else if idx == 0 {
                                groups.len() - 1
                            } else {
                                idx - 1
                            });
                            self.needs_rebuild = true;
                            return true;
                        }
                        if descend {
                            if let Some(&first) = groups[idx].first() {
                                self.ui_context.set_focused_id(first);
                                self.focused_section = None;
                                self.needs_rebuild = true;
                                return true;
                            }
                        }
                    }
                } else {
                    // Entry point: focus the first section.
                    if !self.app.get_current_page_mut().section_widgets().is_empty() {
                        self.focused_section = Some(0);
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }
        }

        let event_wrapper = cce_ui::widget::Event::KeyInput(event.clone());
        let mut key_handled = false;
        if self.dispatch_page_event(&event_wrapper) {
            let mut actions = Vec::new();
            self.propagate_widget_changes(&mut actions);
            for a in actions {
                self.handle_action(&a);
            }
            self.needs_rebuild = true;
            key_handled = true;
        }

        // The dissolved inner lists' keyboard scrolling (hover/focus-scoped, like the old
        // ScrollBox::keyboard_input) — before the whole-page fallback so a hovered list
        // takes the scroll keys first.
        if !key_handled && self.app.get_current_page_mut().handle_key_input(event) {
            self.needs_rebuild = true;
            key_handled = true;
        }

        // The dissolved Page's keyboard scrolling: when nothing in the page tree took the
        // key and the cursor is over the page viewport, scroll keys move the page.
        if !key_handled && event.state == cce_ui::widget::ElementState::Pressed && self.max_scroll_y > 0.0 {
            let over_page = {
                let ry = self.header_height;
                let mut rh = self.height as f32 - self.header_height - self.status_height;
                if self.search_open {
                    rh -= 42.0;
                }
                self.cursor_x >= self.sidebar_width
                    && self.cursor_y >= ry
                    && self.cursor_y <= ry + rh
            };
            if over_page {
                use cce_ui::widget::{Bounds, Key, NamedKey, LINE_PX};
                // Arrow steps ride the same glide as wheel notches (a held
                // key accumulates into one motion); Home/End glide to the
                // absolute target. `tick_page_scroll` carries the page there.
                let s = cce_ui::widget::scroll_motion::scroll_settings();
                let b = Bounds::max(self.max_scroll_y);
                self.page_scroll_motion.reconcile(0.0, self.scroll_y);
                let moved = match &event.logical_key {
                    Key::Named(NamedKey::ArrowDown) => self.page_scroll_motion.y.wheel(LINE_PX, b, &s),
                    Key::Named(NamedKey::ArrowUp) => self.page_scroll_motion.y.wheel(-LINE_PX, b, &s),
                    Key::Named(NamedKey::Home) => self.page_scroll_motion.y.scroll_to(0.0, b, &s),
                    Key::Named(NamedKey::End) => self.page_scroll_motion.y.scroll_to(self.max_scroll_y, b, &s),
                    _ => false,
                };
                if moved {
                    // With smoothing off the axis jumped: land the page now.
                    self.shift_page_to(self.page_scroll_motion.y.pos());
                    // Keyboard scrolling raises the bar like the wheel does.
                    self.ui_context[self.page_scroll_bar].on_scroll();
                    self.needs_rebuild = true;
                    key_handled = true;
                }
            }
        }

        // What the key changed that nothing above acted on: the search box's
        // text (the sweep makes it the query) and a control's own keys (Enter
        // or Space on a focused button, a focused dropdown's pick, the page
        // switcher's too).
        let mut actions = Vec::new();
        self.propagate_widget_changes(&mut actions);
        for a in &actions {
            self.handle_action(a);
        }
        if !actions.is_empty() {
            self.needs_rebuild = true;
            key_handled = true;
        }

        // The search box lost the focus: the search closes.
        if self.search_open && !self.ui_context.is_focused(&self.ui_context[self.search_box]) {
            self.search_open = false;
            self.search_query.clear();
            self.ui_context[self.search_box].set_value_string("");
            self.needs_rebuild = true;
        }

        key_handled
    }
}

