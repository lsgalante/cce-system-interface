use crate::SystemInterface;
use cce_settings::app::AppAction;
use cce_settings::pages::Page;
use cce_ui::widget::WidgetHost;

/// App shortcuts, resolved once per process from input.kdl
/// (`cce-system-settings` domain → `cce-ui` domain).
struct SettingsKeys {
    open_search: String,
    focus_next: String,
    focus_prev: String,
    focus_ascend: String,
    focus_descend: String,
}

fn settings_keys() -> &'static SettingsKeys {
    static KEYS: std::sync::OnceLock<SettingsKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        let get = cce_ui::input::app_chord;
        SettingsKeys {
            open_search: get("open_search", "/"),
            focus_next: get("focus_next", "ctrl+j"),
            focus_prev: get("focus_prev", "ctrl+k"),
            focus_ascend: get("focus_ascend", "ctrl+u"),
            focus_descend: get("focus_descend", "ctrl+i"),
        }
    })
}

impl SystemInterface {

    pub(crate) fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
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
        cce_ui::widget::hover_animation::set_cursor_pos(lx, ly_no_scroll);
        let mut changed = false;
        {
            let ev = cce_ui::widget::Event::PointerMove { x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
            let dd = self.page_dropdown.id();
            if self.ui_context.propagate_event(&ev, dd) {
                changed = true;
            }
        }

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

        if !drag_handled {
            let event = cce_ui::widget::Event::PointerMove { x: lx, y: ly, local_x: lx, local_y: ly };
            if self.dispatch_page_event(&event) {
                changed = true;
            }
            // Routed drags (6bd): a DragUpdate delivered inside the dispatch surfaces as
            // widget take_change — drain and act per move, as the old page drag hooks did.
            let mut move_actions = Vec::new();
            self.propagate_widget_changes(&mut move_actions);
            if !move_actions.is_empty() {
                changed = true;
                for action in move_actions {
                    self.handle_action(&action);
                }
            }
        }

        let phys_x = x;
        let phys_y = y;

        // Check if cursor hover state changed on any widget
        for w in &self.widgets {
            let is_hovered = phys_x >= w.x && phys_x <= w.x + w.w && phys_y >= w.y && phys_y <= w.y + w.h;
            if w.hovering != is_hovered {
                changed = true;
                break;
            }
        }

        if changed {
            self.needs_rebuild = true;
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
            self.search_box.set_value_string("");
            self.ui_context.clear_focus();
            self.needs_rebuild = true;
        }

        // CSD Close Button Interaction removed

        if cce_ui::widget::context_menu::is_visible() {
            if cce_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll, Some(&mut self.ui_context)) {
                let mut actions = Vec::new();
                self.propagate_widget_changes(&mut actions);
                for action in actions {
                    self.handle_action(&action);
                }
                self.needs_rebuild = true;
                return true;
            }
        }

        let dd_ev = cce_ui::widget::Event::MouseButton { button, state, x: lx_no_scroll, y: ly_no_scroll, local_x: lx_no_scroll, local_y: ly_no_scroll };
        let dd_root = self.page_dropdown.id();
        if self.ui_context.propagate_event(&dd_ev, dd_root) {
            if self.page_dropdown.take_change() {
                let idx = self.page_dropdown.selected;
                if idx < Page::ALL.len() {
                    cce_ui::widget::focus::clear_focus(Some(&mut self.ui_context));
                    self.focused_section = None;
                    let new_page = Page::ALL[idx];
                    self.app.current_page = new_page;
                    self.current_page_shared.store(idx as u8, std::sync::atomic::Ordering::SeqCst);
                    self.scroll_y = 0.0;
                    self.update_status_text();
                }
            }
            self.needs_rebuild = true;
            return true;
        }

        if button != cce_ui::widget::MouseButton::Left && button != cce_ui::widget::MouseButton::Right { return false; }

        let phys_x = self.cursor_x;
        let phys_y = self.cursor_y;

        let mut button_handled = false;
        let mut clicked_action = None;
        let btn_ev = cce_ui::widget::Event::MouseButton { button, state, x: phys_x, y: phys_y, local_x: phys_x, local_y: phys_y };
        let btn_roots: Vec<_> = self.page_buttons[self.scrollable_buttons_start_idx..].iter().map(|(b, _)| b.id()).collect();
        for root in btn_roots {
            if self.ui_context.propagate_event(&btn_ev, root) {
                button_handled = true;
            }
        }
        for (btn, action) in &mut self.page_buttons[self.scrollable_buttons_start_idx..] {
            if btn.take_click() {
                clicked_action = Some(action.clone());
                button_handled = true;
                self.needs_rebuild = true;
            }
        }
        if let Some(action) = clicked_action {
            self.handle_action(&action);
        }
        if button_handled {
            return true;
        }
        let mut actions = Vec::new();
        if button == cce_ui::widget::MouseButton::Left && state == cce_ui::widget::ElementState::Released {
            if self.app.get_current_page_mut().handle_pointer_up(&mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let event = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
        self.dispatch_page_event(&event);

        if state == cce_ui::widget::ElementState::Pressed {
            self.app.get_current_page_mut().handle_pointer_down(lx, ly, &mut self.ui_context);
        }

        self.propagate_widget_changes(&mut actions);

        // Single-slot focus (Phase 6w): if a widget click took the global focus, the
        // section-level highlight yields — exactly as when both lived in FOCUSED_WIDGET.
        if state == cce_ui::widget::ElementState::Pressed && cce_ui::widget::focus::has_focus() {
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

    pub(crate) fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        self.app.get_current_page_mut().propagate_widget_changes(actions);
    }

    pub(crate) fn handle_mouse_wheel_internal(&mut self, delta: &cce_ui::widget::MouseScrollDelta, px: f32, py: f32) -> bool {
        self.scroll_logs.push(format!(
            "handle_mouse_wheel_internal: px={}, py={}, delta={:?}, sidebar_w={}",
            px, py, delta, self.sidebar_width
        ));
        let s = 1.0f32;
        if px >= self.sidebar_width * s {
            let lx = px / s;
            let ly = py / s + self.scroll_y;
            


            let event = cce_ui::widget::Event::MouseWheel { delta: delta.clone(), x: lx, y: ly, local_x: lx, local_y: ly };
            // Page dissolved (6u): one dispatch path for every page — scrollbar, then
            // sections, then the dissolved inner lists (the app-owned ScrollRegions,
            // hit-scoped like the old inner ScrollBoxes), then the manual page scroll
            // below as the fallback, exactly as the non-System pages worked.
            let mut handled = self.dispatch_page_event(&event);
            if !handled {
                handled = self.app.get_current_page_mut().handle_mouse_wheel(delta, lx, ly);
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

            let scroll_speed = 24.0;
            let dy = match delta {
                cce_ui::widget::MouseScrollDelta::LineDelta(_, y) => -y * scroll_speed,
                cce_ui::widget::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
            };
            let old_scroll = self.scroll_y;
            self.scroll_y = (self.scroll_y + dy).max(0.0).min(self.max_scroll_y);
            if (self.scroll_y - old_scroll).abs() > 0.01 {
                let actual_dy = self.scroll_y - old_scroll;
                for w in &mut self.widgets[self.scrollable_widgets_start_idx..] {
                    w.y -= actual_dy;
                }
                for (_, _, _, ty, _, _, bounds) in &mut self.texts[self.scrollable_text_items_start_idx..] {
                    *ty -= actual_dy;
                    if let Some(ref mut b) = bounds {
                        b[1] -= actual_dy;
                        b[3] -= actual_dy;
                    }
                }
                for (btn, _) in &mut self.page_buttons[self.scrollable_buttons_start_idx..] {
                    btn.base_mut().y -= actual_dy;
                }
                self.last_scroll_y = self.scroll_y;
                return true;
            }
        }
        false
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
        let is_pointer_event = matches!(
            event,
            Event::PointerMove { .. } | Event::MouseButton { .. } | Event::MouseWheel { .. }
        );
        if is_pointer_event && !self.page_scroll_bar.dragging && !self.ui_context.is_dragging {
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

        if self.page_scroll_bar.content_h > self.page_scroll_bar.viewport_h {
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
        let extra_roots = {
            let page = self.app.get_current_page_mut();
            page.register_extra_dispatch_roots(&mut self.ui_context);
            page.extra_dispatch_roots()
        };
        for root in extra_roots {
            if self.ui_context.propagate_event(event, root) {
                if !is_pointer_move {
                    return true;
                }
                handled = true;
            }
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
        if cce_ui::widget::context_menu::is_visible() {
            if event.state == cce_ui::widget::ElementState::Pressed
                && event.logical_key == cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape)
            {
                cce_ui::widget::context_menu::hide();
                self.needs_rebuild = true;
                return true;
            }
        }

        if self.search_open {
            if event.state == cce_ui::widget::ElementState::Pressed
                && event.logical_key == cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape)
            {
                self.search_open = false;
                self.search_query.clear();
                self.search_box.set_value_string("");
                self.ui_context.clear_focus();
                self.needs_rebuild = true;
                return true;
            }
        }

        let is_text_box_focused = if let Some(focused) =
            self.ui_context.focused_widget.and_then(|id| self.ui_context.tree.get_ptr(id))
        {
            unsafe { (*focused).as_any().is::<cce_ui::widget::input::TextBox>() }
        } else {
            false
        };

        if !self.search_open && !is_text_box_focused {
            if event.state == cce_ui::widget::ElementState::Pressed && !event.repeat {
                if cce_ui::widget::match_key_shortcut(event, &settings_keys().open_search) {
                    self.search_open = true;
                    self.search_box.set_value_string("");
                    self.search_query.clear();
                    cce_ui::widget::WidgetHost::focus(&mut self.search_box);
                    self.ui_context.set_focused(&mut self.search_box);
                    self.needs_rebuild = true;
                    return true;
                }
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
                // app-side index, widget-level focus stays in the global focus module,
                // and the two are single-slot (as when sections and widgets shared the
                // one FOCUSED_WIDGET). Nav within a section walks the page's widget
                // group where the container's child list used to be walked.
                if cce_ui::widget::focus::has_focus() {
                    // (`focus::navigate_focus` is gone — it walked an empty dummy context
                    // and always returned false here; the section machinery below is the
                    // real ctrl-nav.)
                    let groups = self.app.get_current_page_mut().section_widgets();
                    let focused_pos = groups.iter().enumerate().find_map(|(si, g)| {
                        g.iter()
                            .position(|&id| cce_ui::widget::focus::is_focused_id(id))
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
                            cce_ui::widget::focus::set_focused_id(next_id, Some(&mut self.ui_context));
                            if let Some(w) = self.ui_context.get_widget_mut(next_id) {
                                w.focus();
                            }
                            self.needs_rebuild = true;
                            return true;
                        }
                        if ascend {
                            cce_ui::widget::focus::clear_focus(Some(&mut self.ui_context));
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
                                cce_ui::widget::focus::set_focused_id(first, Some(&mut self.ui_context));
                                if let Some(w) = self.ui_context.get_widget_mut(first) {
                                    w.focus();
                                }
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
                use cce_ui::widget::{Key, NamedKey};
                let viewport_h = self.height as f32 - self.header_height - self.status_height
                    - if self.search_open { 42.0 } else { 0.0 };
                let old_scroll = self.scroll_y;
                match &event.logical_key {
                    Key::Named(NamedKey::ArrowDown) => self.scroll_y = (self.scroll_y + 24.0).min(self.max_scroll_y),
                    Key::Named(NamedKey::ArrowUp) => self.scroll_y = (self.scroll_y - 24.0).max(0.0),
                    Key::Named(NamedKey::PageDown) => self.scroll_y = (self.scroll_y + viewport_h).min(self.max_scroll_y),
                    Key::Named(NamedKey::PageUp) => self.scroll_y = (self.scroll_y - viewport_h).max(0.0),
                    Key::Named(NamedKey::Home) => self.scroll_y = 0.0,
                    Key::Named(NamedKey::End) => self.scroll_y = self.max_scroll_y,
                    _ => {}
                }
                if (self.scroll_y - old_scroll).abs() > 0.01 {
                    self.needs_rebuild = true;
                    key_handled = true;
                }
            }
        }

        if self.search_open {
            if self.search_box.take_change() {
                self.search_query = self.search_box.text.clone();
                self.needs_rebuild = true;
            }
            if !self.ui_context.is_focused(&self.search_box) {
                self.search_open = false;
                self.search_query.clear();
                self.search_box.set_value_string("");
                self.needs_rebuild = true;
            }
        }

        key_handled
    }
}

