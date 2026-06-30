use crate::SystemInterface;
use cce_settings::app::AppAction;
use cce_settings::pages::Page;
use cce_ui::widget::Element;

impl SystemInterface {

    pub(crate) fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_x = x;
        self.cursor_y = y;
        let s = 1.0f32;
        let lx_no_scroll = x / s;
        let ly_no_scroll = y / s;

        let sh_logical = self.height as f32 / s;
        if self.search_open && ly_no_scroll >= (sh_logical - 42.0) {
            let changed = self.search_box.cursor_moved(lx_no_scroll, ly_no_scroll, &mut self.ui_context);
            if changed {
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
        if self.menubar.cursor_moved(lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
            changed = true;
        }
        if self.switcher.cursor_moved(lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
            changed = true;
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
            if let Some(root) = self.get_page_root_widget() {
                if self.ui_context.propagate_event(&event, root) {
                    changed = true;
                }
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
            let handled = self.search_box.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context);
            if handled {
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
            if cce_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll) {
                let mut actions = Vec::new();
                self.propagate_widget_changes(&mut actions);
                for action in actions {
                    self.handle_action(&action);
                }
                self.needs_rebuild = true;
                return true;
            }
        }

        let mut handled = false;
        if state == cce_ui::widget::ElementState::Pressed {
            if lx_no_scroll < self.sidebar_width {
                if self.menubar.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                    use cce_ui::widget::MenuController;
                    if let Some((idx, _)) = self.menubar.menu_click() {
                        if idx < Page::ALL.len() {
                            cce_ui::widget::focus::clear_focus();
                            let new_page = Page::ALL[idx];
                            self.app.current_page = new_page;
                            self.current_page_shared.store(idx as u8, std::sync::atomic::Ordering::SeqCst);
                            self.scroll_y = 0.0;
                        }
                    }
                    self.needs_rebuild = true;
                    handled = true;
                }
            } else {
                if self.switcher.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    handled = true;
                }
            }
        } else {
            if self.menubar.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                use cce_ui::widget::MenuController;
                if let Some((idx, _)) = self.menubar.menu_click() {
                    if idx < Page::ALL.len() {
                        cce_ui::widget::focus::clear_focus();
                        let new_page = Page::ALL[idx];
                        self.app.current_page = new_page;
                        self.current_page_shared.store(idx as u8, std::sync::atomic::Ordering::SeqCst);
                        self.scroll_y = 0.0;
                    }
                }
                self.needs_rebuild = true;
                handled = true;
            }
            if self.switcher.mouse_input(button, state, lx_no_scroll, ly_no_scroll, &mut self.ui_context) {
                self.needs_rebuild = true;
                handled = true;
            }
        }
        if handled {
            return true;
        }

        if button != cce_ui::widget::MouseButton::Left && button != cce_ui::widget::MouseButton::Right { return false; }
        let mut actions = Vec::new();
        if button == cce_ui::widget::MouseButton::Left && state == cce_ui::widget::ElementState::Released {
            if self.app.get_current_page_mut().handle_pointer_up(&mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let event = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
        if let Some(root) = self.get_page_root_widget() {
            self.ui_context.propagate_event(&event, root);
        }

        if state == cce_ui::widget::ElementState::Pressed {
            self.app.get_current_page_mut().handle_pointer_down(lx, ly, &mut self.ui_context);
        }

        self.propagate_widget_changes(&mut actions);

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
            let mut handled = false;
            if let Some(root) = self.get_page_root_widget() {
                unsafe {
                    for child in (*root).children(&self.ui_context).into_iter().rev() {
                        let (cx, cy, _, _) = (*child).rect();
                        let mut local_adjusted = event.clone();
                        match &mut local_adjusted {
                            cce_ui::widget::Event::MouseWheel { local_x, local_y, .. } => {
                                *local_x -= cx;
                                *local_y -= cy;
                            }
                            _ => {}
                        }
                        let adjusted_event = (*root).transform_event_for_child(child, local_adjusted, &self.ui_context);
                        if self.ui_context.propagate_event(&adjusted_event, child) {
                            handled = true;
                            break;
                        }
                    }
                }
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
                for ti in &mut self.text_items[self.scrollable_text_items_start_idx..] {
                    ti.y -= actual_dy;
                    if let Some(ref mut b) = ti.bounds {
                        b[1] -= actual_dy;
                        b[3] -= actual_dy;
                    }
                }
                for (btn, _) in &mut self.page_buttons[self.scrollable_buttons_start_idx..] {
                    if let Some(base) = btn.base_mut() {
                        base.y -= actual_dy;
                    }
                }
                self.last_scroll_y = self.scroll_y;
                return true;
            }
        } else {
            let lx = px / s;
            let ly = py / s;
            if self.menubar.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

    pub(crate) fn get_page_root_widget(&mut self) -> Option<*mut (dyn cce_ui::widget::Element + 'static)> {
        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        let ptr = &mut self.pages[page_idx] as &mut dyn cce_ui::widget::Element as *mut dyn cce_ui::widget::Element;
        let static_ptr = unsafe {
            std::mem::transmute::<*mut dyn cce_ui::widget::Element, *mut (dyn cce_ui::widget::Element + 'static)>(ptr)
        };
        Some(static_ptr)
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

        let is_text_box_focused = if let Some(focused) = self.ui_context.focused_widget {
            unsafe { (*focused).as_any().is::<cce_ui::widget::input::TextBox>() }
        } else {
            false
        };

        if !self.search_open && !is_text_box_focused {
            if event.state == cce_ui::widget::ElementState::Pressed && !event.repeat {
                if let cce_ui::widget::Key::Character(ref c) = event.logical_key {
                    if c == "/" {
                        self.search_open = true;
                        self.search_box.set_value_string("");
                        self.search_query.clear();
                        let search_box_ptr = &mut self.search_box as *mut cce_ui::widget::input::TextBox;
                        unsafe {
                            (*search_box_ptr).focus();
                        }
                        self.ui_context.set_focused(&mut self.search_box);
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }
        }

        if event.state == cce_ui::widget::ElementState::Pressed && !event.repeat {
            let is_nav_key = match (&event.logical_key, event.ctrl) {
                (cce_ui::widget::Key::Character(c), true) if c == "j" || c == "J" || c == "k" || c == "K" || c == "u" || c == "U" || c == "i" || c == "I" => true,
                _ => false,
            };
            if is_nav_key {
                if cce_ui::widget::focus::has_focus() {
                    if cce_ui::widget::focus::navigate_focus(&event.logical_key, event.ctrl) {
                        self.needs_rebuild = true;
                        return true;
                    }
                } else {
                    if let Some(root_ptr) = self.get_page_root_widget() {
                        unsafe {
                            let root_ref = &mut *root_ptr;
                            cce_ui::widget::focus::set_focused(root_ref);
                            root_ref.focus();
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
        }

        let event_wrapper = cce_ui::widget::Event::KeyInput(event.clone());
        let mut key_handled = false;
        if let Some(root) = self.get_page_root_widget() {
            let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
            let old_page_scroll = self.pages[page_idx].scroll_y;

            if self.ui_context.propagate_event(&event_wrapper, root) {
                let mut actions = Vec::new();
                self.propagate_widget_changes(&mut actions);
                for a in actions {
                    self.handle_action(&a);
                }
                self.needs_rebuild = true;
                key_handled = true;
            }

            let new_page_scroll = self.pages[page_idx].scroll_y;
            if (new_page_scroll - old_page_scroll).abs() > 0.01 {
                self.scroll_y = new_page_scroll;
                self.needs_rebuild = true;
                key_handled = true;
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

