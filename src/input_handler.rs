use crate::SystemInterface;
use cce_system_interface::app::AppAction;
use cce_system_interface::pages::{self, Page};
use cce_ui::widget::Element;

impl SystemInterface {

    pub(crate) fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_x = x;
        self.cursor_y = y;
        let s = 1.0f32;
        let lx_no_scroll = x / s;
        let ly_no_scroll = y / s;
        
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
        for w in &mut self.widgets {
            let was = w.hovering;
            w.hovering = self.cursor_x >= w.x && self.cursor_x <= w.x + w.w
                && self.cursor_y >= w.y && self.cursor_y <= w.y + w.h;
            if w.hovering != was {
                w.color = if w.hovering { w.hover_color } else { w.color };
                changed = true;
            }
        }

        if self.app.current_page == Page::Interface {
            for cp in &mut self.app.interface.color_selectors {
                if cp.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.interface.tab_margin_spinbox_x.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.menubar_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.tab_margin_spinbox_y.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.button_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.section_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_alignment_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_offset_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.label_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_show_grid_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_snap_enabled_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_uniform_background_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_cell_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_gap_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.graph_gap_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.page_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.grid_min_col_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.spinbox_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.spinbox_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.toggle_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.toggle_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.plate_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.page_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.layer_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }


            if self.app.interface.color_selector_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_preview_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_preview_margin_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.color_selector_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.menubar_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.breadcrumb_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.section_label_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.nested_section_label_font_selector.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.textbox_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.textbox_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.slider_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.font_selector_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.font_selector_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.dropdown_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.dropdown_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.button_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.notification_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.window_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.window_corner_radius_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.sans_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.serif_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.mono_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.borders_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.status_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.fuzzel_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.terminal_size_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.custom_multicontrol.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for sb in &mut self.app.interface.windows.spinboxes {
                if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.interface.windows.cascade_offset_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.edge_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.top_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.grid_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.transition_duration_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.status_height_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for menu in &mut self.app.interface.windows.tag_layout_menus {
                if menu.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.interface.windows.side_panel_behavior_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.side_panel_position_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.side_panel_width_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.side_panel_border_gap_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.side_panel_border_opacity_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.transparency_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.interface.windows.blur_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.delay_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.tap_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.scroll_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.scroll_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.pointer_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.pointer_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpad_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpad_friction_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.dwtp_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_profile_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.zoom_in_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.zoom_out_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.input.keybinds_control.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Audio {
            if let Some(idx) = self.audio_sink_dragging {
                if idx < self.app.audio.sink_sliders.len() {
                    if self.app.audio.sink_sliders[idx].drag_update(lx, ly) {
                        changed = true;
                        let val = self.app.audio.sink_sliders[idx].value();
                        if idx < self.app.audio.sink_spinboxes.len() {
                            self.app.audio.sink_spinboxes[idx].value = (val * 100.0).round() as i32;
                        }
                        if idx < self.app.audio.sinks.len() {
                            let id = self.app.audio.sinks[idx].id;
                            self.handle_action(&AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, val)));
                        }
                    }
                }
            } else if let Some(idx) = self.audio_source_dragging {
                if idx < self.app.audio.source_sliders.len() {
                    if self.app.audio.source_sliders[idx].drag_update(lx, ly) {
                        changed = true;
                        let val = self.app.audio.source_sliders[idx].value();
                        if idx < self.app.audio.source_spinboxes.len() {
                            self.app.audio.source_spinboxes[idx].value = (val * 100.0).round() as i32;
                        }
                        if idx < self.app.audio.sources.len() {
                            let id = self.app.audio.sources[idx].id;
                            self.handle_action(&AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, val)));
                        }
                    }
                }
            } else {
                for sb in &mut self.app.audio.sink_spinboxes {
                    if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    if sb.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for slider in &mut self.app.audio.sink_sliders {
                    if slider.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
                for slider in &mut self.app.audio.source_sliders {
                    if slider.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
            }
        }
        if self.app.current_page == Page::Display {
            if self.display_brightness_dragging {
                if self.app.display.brightness_slider.drag_update(lx, ly) {
                    changed = true;
                    let val = self.app.display.brightness_slider.value();
                    let pct = (val * 100.0).round() as u32;
                    self.app.display.brightness_spinbox.value = pct as i32;
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                }
            } else {
                if self.app.display.brightness_slider.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if self.app.display.brightness_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.display.night_light_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for out in &mut self.app.display.outputs {
                if out.name_label.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if out.resolution_label.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if let Some(ref mut scale_lbl) = out.scale_label {
                    if scale_lbl.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                }
            }
            if self.app.display.screensaver_enable_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_lock_screen_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_timeout_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.display.screensaver_style_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }

        if self.app.current_page == Page::Hardware {
            if self.app.hardware.cpu_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_usage_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_temp_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            for gpu_lbl in &mut self.app.hardware.gpu_labels {
                if gpu_lbl.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.hardware.cpu_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.cpu_gov_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.hardware.gpu_gov_menu.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }

        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            if pkgs.search_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let matching = pkgs.installed.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                        .count();
                    for i in 0..matching.min(pkgs.installed_items.len()) {
                        if pkgs.installed_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                            changed = true;
                        }
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                        changed = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let matching = pkgs.updates.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query))
                        .count();
                    for i in 0..matching.min(pkgs.updates_items.len()) {
                        if pkgs.updates_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                            changed = true;
                        }
                    }
                }
            }
        }



        if self.app.current_page == Page::Services {
            if self.app.services.search_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.list_box.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            let query = if self.app.services.search_box.editing {
                self.app.services.search_box.edit_buffer.to_lowercase()
            } else {
                self.app.services.search_box.text.to_lowercase()
            };
            let matching_count = self.app.services.services.iter()
                .filter(|s| s.is_system == (self.app.services.active_tab == pages::services::ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .count();
            for i in 0..matching_count.min(self.app.services.service_items.len()) {
                if self.app.services.service_items[i].cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }
            if self.app.services.notifications_enable_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.notifications_bell_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.notifications_duration_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_label.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_separators_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_underline_toggle.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
            if self.app.services.status_padding_spinbox.cursor_moved(lx, ly, &mut self.ui_context) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Accounts {
            if self.app.accounts.editing_oauth_creds {
                if self.app.accounts.oauth_client_id_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.oauth_client_secret_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
            } else if self.app.accounts.adding_new {
                if self.app.accounts.email_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.password_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.imap_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
                if self.app.accounts.smtp_box.cursor_moved(lx, ly, &mut self.ui_context) { changed = true; }
            }
        }
        if changed { self.needs_rebuild = true; }
        changed
    }

    pub(crate) fn handle_mouse_input_internal(&mut self, button: cce_ui::widget::MouseButton, state: cce_ui::widget::ElementState) -> bool {
        let s = 1.0f32;
        let lx_no_scroll = self.cursor_x / s;
        let ly_no_scroll = self.cursor_y / s;

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
                            pages::interface::write_config_value("last_page", &format!("\"{}\"", new_page.label().to_lowercase()));
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
                        pages::interface::write_config_value("last_page", &format!("\"{}\"", new_page.label().to_lowercase()));
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
        if button == cce_ui::widget::MouseButton::Left && state == cce_ui::widget::ElementState::Released {
            if self.app.current_page == Page::Audio {
                let mut ended = false;
                if let Some(idx) = self.audio_sink_dragging {
                    if idx < self.app.audio.sink_sliders.len() {
                        self.app.audio.sink_sliders[idx].drag_end();
                    }
                    self.audio_sink_dragging = None;
                    ended = true;
                }
                if let Some(idx) = self.audio_source_dragging {
                    if idx < self.app.audio.source_sliders.len() {
                        self.app.audio.source_sliders[idx].drag_end();
                    }
                    self.audio_source_dragging = None;
                    ended = true;
                }
                if ended {
                    self.needs_rebuild = true;
                }
            }
            if self.app.current_page == Page::Display {
                if self.display_brightness_dragging {
                    self.app.display.brightness_slider.drag_end();
                    self.display_brightness_dragging = false;
                    self.needs_rebuild = true;
                }
            }

            let (px, py) = (self.cursor_x, self.cursor_y);
            for (btn, action) in &self.page_buttons.clone() {
                let base = btn.base().unwrap();
                if px >= base.x && px <= base.x + base.w && py >= base.y && py <= base.y + base.h {
                    self.handle_action(action);
                    self.needs_rebuild = true;
                    return true;
                }
            }
        }
        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let mut actions = Vec::new();

        if state == cce_ui::widget::ElementState::Pressed {
            let mut clicked_any_focusable = false;
            match self.app.current_page {
                Page::Accounts => {
                    let accs = &mut self.app.accounts;
                    if accs.editing_oauth_creds {
                        if accs.oauth_client_id_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.oauth_client_secret_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    } else if accs.adding_new {
                        if accs.email_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.password_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.imap_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if accs.smtp_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                }

                Page::Interface => {
                    if self.app.interface.custom_multicontrol.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for cp in &mut self.app.interface.color_selectors {
                        if cp.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if self.app.interface.tab_margin_spinbox_x.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.menubar_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.tab_margin_spinbox_y.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.button_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.section_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_alignment_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_offset_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.label_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_show_grid_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_snap_enabled_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_uniform_background_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_cell_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_gap_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.graph_gap_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.page_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.grid_min_col_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.spinbox_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.spinbox_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.toggle_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.toggle_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.plate_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.page_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.layer_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }


                    if self.app.interface.color_selector_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_preview_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_preview_margin_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.color_selector_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.menubar_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.breadcrumb_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.section_label_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.nested_section_label_font_selector.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.textbox_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.textbox_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.slider_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.font_selector_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.font_selector_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.dropdown_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.dropdown_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.button_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.notification_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.window_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.interface.window_corner_radius_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    let tf = &mut self.app.interface;
                    if tf.sans_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.serif_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.mono_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.borders_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.borders_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.status_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.status_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.fuzzel_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.fuzzel_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.terminal_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.terminal_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for sb in &mut tf.windows.spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if tf.windows.cascade_offset_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.edge_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.top_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.grid_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.transition_duration_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.status_height_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for menu in &mut tf.windows.tag_layout_menus {
                        if menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    if tf.windows.side_panel_behavior_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.side_panel_position_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.side_panel_width_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.side_panel_border_gap_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.side_panel_border_opacity_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.transparency_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if tf.windows.blur_toggle.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Input => {
                    if self.app.input.rate_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.delay_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.scroll_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.pointer_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpad_friction_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_speed_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_profile_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.zoom_in_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.zoom_out_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.input.keybinds_control.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }

                Page::Audio => {
                    for sb in &mut self.app.audio.sink_spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for sb in &mut self.app.audio.source_spinboxes {
                        if sb.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for slider in &mut self.app.audio.sink_sliders {
                        if slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                    for slider in &mut self.app.audio.source_sliders {
                        if slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    }
                }
                Page::Display => {
                    if self.app.display.brightness_slider.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.brightness_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.night_light_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    for out in &mut self.app.display.outputs {
                        if out.name_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if out.resolution_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        if let Some(ref mut scale_lbl) = out.scale_label {
                            if scale_lbl.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                    }
                    if self.app.display.screensaver_timeout_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if self.app.display.screensaver_style_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Services => {
                    let srv = &mut self.app.services;
                    if srv.search_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.notifications_duration_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.status_label.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if srv.status_padding_spinbox.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Packages => {
                    let pkgs = &mut self.app.packages;
                    if pkgs.search_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    match pkgs.active_tab {
                        pages::packages::PackageTab::Installed => {
                            if pkgs.installed_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                        pages::packages::PackageTab::Updates => {
                            if pkgs.updates_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                        }
                    }
                }
                Page::Hardware => {
                    let hw = &mut self.app.hardware;
                    if hw.cpu_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if hw.cpu_gov_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                    if hw.gpu_gov_menu.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                Page::Radios => {
                    let net = &mut self.app.network;
                    if net.wifi_list_box.hit_test(lx, ly, &self.ui_context) { clicked_any_focusable = true; }
                }
                _ => {}
            }

            if !clicked_any_focusable {
                cce_ui::widget::focus::clear_focus();
            }
        }

        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Interface {
            let tf = &mut self.app.interface.windows;
            for (i, sb) in tf.spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    actions.push(AppAction::Interface(
                        pages::interface::InterfaceMessage::Windows(
                            pages::interface::WindowsMessage::SetWidth(
                                pages::interface::WidthParam::ALL[i],
                                sb.value as u16,
                            )
                        )
                    ));
                }
            }
            let sb = &mut tf.cascade_offset_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetCascadeOffset(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.edge_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetEdgeGap(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.top_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetTopGap(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.grid_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetGridGap(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.transition_duration_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetTransitionDuration(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.status_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetStatusHeight(sb.value as u16)
                    )
                ));
            }
        }
        if self.app.current_page == Page::Interface {
            let tf = &mut self.app.interface.windows;
            for (idx, menu) in tf.tag_layout_menus.iter_mut().enumerate() {
                if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
                if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
                if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                    actions.push(AppAction::Interface(
                        pages::interface::InterfaceMessage::Windows(
                            pages::interface::WindowsMessage::SetTagLayout(idx + 1, menu.selected)
                        )
                    ));
                }
            }
            let menu = &mut tf.side_panel_behavior_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetSidePanelBehavior(menu.selected)
                    )
                ));
            }
            let menu = &mut tf.side_panel_position_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetSidePanelPosition(menu.selected)
                    )
                ));
            }
            let sb = &mut tf.side_panel_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetSidePanelWidth(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.side_panel_border_gap_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetSidePanelBorderGap(sb.value as u16)
                    )
                ));
            }
            let sb = &mut tf.side_panel_border_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::SetSidePanelBorderOpacity(sb.value as u16)
                    )
                ));
            }
            let toggle = &mut tf.transparency_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::ToggleTransparency
                    )
                ));
            }
            let toggle2 = &mut tf.blur_toggle;
            toggle2.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle2.take_click() {
                actions.push(AppAction::Interface(
                    pages::interface::InterfaceMessage::Windows(
                        pages::interface::WindowsMessage::ToggleBlur
                    )
                ));
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Interface {
            for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if !cp.hit_test(lx, ly, &self.ui_context) { cp.unfocus(); }
                cp.mouse_input(button, state, lx, ly, &mut self.ui_context);
                if cp.take_click() {
                    actions.push(AppAction::Interface(match i {
                        0 => pages::interface::InterfaceMessage::PickPageLowColor,
                        1 => pages::interface::InterfaceMessage::PickHighColor,
                        2 => pages::interface::InterfaceMessage::PickVisualGuides,
                        3 => pages::interface::InterfaceMessage::PickDisabledColor,
                        4 => pages::interface::InterfaceMessage::PickSeparatorColor,
                        5 => pages::interface::InterfaceMessage::PickSliderTrackColor,
                        6 => pages::interface::InterfaceMessage::PickColorBordersColor,
                        7 => pages::interface::InterfaceMessage::PickLowColor,
                        8 => pages::interface::InterfaceMessage::PickNormalColor,
                        9 => pages::interface::InterfaceMessage::PickPaginatorSidebarColor,
                        10 => pages::interface::InterfaceMessage::PickPrimaryHighlightColor,
                        11 => pages::interface::InterfaceMessage::PickMenubarTabLabelColor,
                        12 => pages::interface::InterfaceMessage::PickToggleEnabledColor,
                        13 => pages::interface::InterfaceMessage::PickToggleDisabledColor,
                        14 => pages::interface::InterfaceMessage::PickScrollingListBgColor,
                        15 => pages::interface::InterfaceMessage::PickBreadcrumbBgColor,
                        16 => pages::interface::InterfaceMessage::PickPopoverBgColor,
                        17 => pages::interface::InterfaceMessage::PickNotificationBgColor,
                        18 => pages::interface::InterfaceMessage::PickWindowColor,
                        19 => pages::interface::InterfaceMessage::PickPageColor,
                        20 => pages::interface::InterfaceMessage::PickLayerColor,
                        _ => pages::interface::InterfaceMessage::PickLowColor,
                    }));
                }
                if cp.color != old {
                    actions.push(AppAction::Interface(match i {
                        0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                        1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                        2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                        3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                        4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                        5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                        6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                        7 => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                        8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                        9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                        10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                        11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                        12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                        13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                        14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                        15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                        16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                        17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                        18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                        19 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                        20 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                        _ => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                    }));
                }
            }
            let sb = &mut self.app.interface.menubar_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.notification_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.window_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.window_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_x;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(sb.value as u16)));
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_y;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(sb.value as u16)));
            }
            let sb = &mut self.app.interface.button_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(sb.value as u16)));
            }
            let sb = &mut self.app.interface.section_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(sb.value as u16)));
            }
            let menu = &mut self.app.interface.label_alignment_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(menu.selected)));
            }
            let sb = &mut self.app.interface.label_offset_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(sb.value as i16)));
            }
            let sb = &mut self.app.interface.label_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.plate_padding_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(sb.value as u16)));
            }
            let toggle = &mut self.app.interface.graph_show_grid_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphShowGrid(toggle.toggled())));
            }
            let toggle = &mut self.app.interface.graph_snap_enabled_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphSnapEnabled(toggle.toggled())));
            }
            let toggle = &mut self.app.interface.graph_uniform_background_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphUniformBackground(toggle.toggled())));
            }
            let sb = &mut self.app.interface.graph_gap_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapWidth(sb.value as u16)));
            }
            let sb = &mut self.app.interface.graph_cell_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.graph_gap_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.page_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.grid_min_col_width_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(sb.value as u16)));
            }
            let sb = &mut self.app.interface.spinbox_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.spinbox_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.toggle_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.toggle_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.plate_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.plate_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.page_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(sb.value as f32 / 100.0)));
            }
            let sb = &mut self.app.interface.layer_opacity_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(sb.value as f32 / 100.0)));
            }


            let sb = &mut self.app.interface.color_selector_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_preview_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.color_selector_preview_margin_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(sb.value as u16)));
            }
            let sb = &mut self.app.interface.textbox_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.textbox_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.slider_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.font_selector_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.font_selector_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.dropdown_height_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(sb.value as u16)));
            }
            let sb = &mut self.app.interface.dropdown_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(sb.value as u16)));
            }
            let sb = &mut self.app.interface.button_corner_radius_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(sb.value as u16)));
            }
        }

        if self.app.current_page == Page::Interface {
            let mc = &mut self.app.interface.custom_multicontrol;
            if state == cce_ui::widget::ElementState::Pressed && !mc.hit_test(lx, ly, &self.ui_context) {
                mc.unfocus();
            }
            if mc.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Input {
            let sb = &mut self.app.input.rate_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.delay_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.scroll_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
            }
            let sb = &mut self.app.input.scroll_speed_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
            }
            let sb = &mut self.app.input.pointer_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
            }
            let sb = &mut self.app.input.trackpad_friction_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
            }
            let sb = &mut self.app.input.trackpoint_accel_speed_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
            }
            let sb = &mut self.app.input.cursor_size_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
            }
            let kc = &mut self.app.input.keybinds_control;
            if state == cce_ui::widget::ElementState::Pressed && !kc.hit_test(lx, ly, &self.ui_context) {
                kc.unfocus();
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Services {
            let sb = &mut self.app.services.notifications_duration_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(sb.value)));
            }
            let sb2 = &mut self.app.services.status_padding_spinbox;
            if !sb2.hit_test(lx, ly, &self.ui_context) { sb2.unfocus(); }
            let old2 = sb2.value;
            if sb2.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb2.value != old2 {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(sb2.value as u16)));
            }
        }
        if self.app.current_page == Page::Input {
            let toggle = &mut self.app.input.tap_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
            }
            let toggle = &mut self.app.input.scroll_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
            }
            let toggle = &mut self.app.input.natural_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleNaturalScroll));
            }
            let toggle = &mut self.app.input.pointer_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
            }
            let toggle = &mut self.app.input.trackpad_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
            }
            let toggle = &mut self.app.input.dwtp_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
            }
            let menu = &mut self.app.input.trackpoint_accel_profile_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(menu.selected)));
            }
            let menu = &mut self.app.input.cursor_theme_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorTheme(menu.selected)));
            }

            let tb = &mut self.app.input.zoom_in_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
            }

            let tb = &mut self.app.input.zoom_out_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
            }
            let kc = &mut self.app.input.keybinds_control;
            if kc.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if kc.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ReloadKeybinds));
                self.needs_rebuild = true;
            }
        }
        if self.app.current_page == Page::Services {
            let toggle = &mut self.app.services.notifications_enable_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsEnable));
            }
            let toggle = &mut self.app.services.notifications_bell_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsBell));
            }
            if state == cce_ui::widget::ElementState::Pressed {
                let lbl1 = &mut self.app.services.status_label;
                if !lbl1.hit_test(lx, ly, &self.ui_context) { lbl1.unfocus(); }
                lbl1.mouse_input(button, state, lx, ly, &mut self.ui_context);
            }
            let toggle = &mut self.app.services.status_separators_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleSeparators));
            }
            let toggle2 = &mut self.app.services.status_underline_toggle;
            toggle2.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle2.take_click() {
                actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleUnderline));
            }
        }
        if self.app.current_page == Page::Display {
            let toggle = &mut self.app.display.screensaver_enable_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverEnable));
            }

            let toggle = &mut self.app.display.screensaver_lock_screen_toggle;
            toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if toggle.take_click() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverLockScreen));
            }

            let menu = &mut self.app.display.screensaver_style_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(menu.selected)));
            }
        }
        if self.app.current_page == Page::Hardware {
            let menu = &mut self.app.hardware.cpu_gov_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                if menu.selected == 0 {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                } else {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                }
            }

            let menu = &mut self.app.hardware.gpu_gov_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                if menu.selected == 0 {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                } else {
                    actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                }
            }
        }

        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Audio {
            if button == cce_ui::widget::MouseButton::Left {
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    if slider.hit_test(lx, ly, &self.ui_context) {
                        slider.drag_begin(lx, ly);
                        self.audio_sink_dragging = Some(i);
                        self.needs_rebuild = true;
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    if slider.hit_test(lx, ly, &self.ui_context) {
                        slider.drag_begin(lx, ly);
                        self.audio_source_dragging = Some(i);
                        self.needs_rebuild = true;
                    }
                }
            }
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    let id = self.app.audio.sinks[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                    let id = self.app.audio.sources[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                }
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Display {
            if button == cce_ui::widget::MouseButton::Left {
                let slider = &mut self.app.display.brightness_slider;
                if slider.hit_test(lx, ly, &self.ui_context) {
                    slider.drag_begin(lx, ly);
                    self.display_brightness_dragging = true;
                    self.needs_rebuild = true;
                }
            }
            let sb = &mut self.app.display.brightness_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
            }
            let lbl = &mut self.app.display.night_light_label;
            if !lbl.hit_test(lx, ly, &self.ui_context) { lbl.unfocus(); }
            lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);

            for out in &mut self.app.display.outputs {
                let lbl = &mut out.name_label;
                if !lbl.hit_test(lx, ly, &self.ui_context) { lbl.unfocus(); }
                lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);

                let lbl2 = &mut out.resolution_label;
                if !lbl2.hit_test(lx, ly, &self.ui_context) { lbl2.unfocus(); }
                lbl2.mouse_input(button, state, lx, ly, &mut self.ui_context);

                if let Some(ref mut scale_lbl) = out.scale_label {
                    if !scale_lbl.hit_test(lx, ly, &self.ui_context) { scale_lbl.unfocus(); }
                    scale_lbl.mouse_input(button, state, lx, ly, &mut self.ui_context);
                }
            }

            let sb = &mut self.app.display.screensaver_timeout_spinbox;
            if !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(sb.value)));
            }
        }

        if self.app.current_page == Page::Accounts {
            if self.app.accounts.editing_oauth_creds {
                let tb = &mut self.app.accounts.oauth_client_id_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.oauth_client_secret_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            } else if self.app.accounts.adding_new {
                let tb = &mut self.app.accounts.email_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
                if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                    let email_val = tb.text.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }

                let tb = &mut self.app.accounts.password_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.imap_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }

                let tb = &mut self.app.accounts.smtp_box;
                if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
                if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            }
        }
        if self.app.current_page == Page::Interface {
            let tb = &mut self.app.interface.sans_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(tb.text.clone())));
            }

            let tb = &mut self.app.interface.serif_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(tb.text.clone())));
            }

            let tb = &mut self.app.interface.mono_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(tb.text.clone())));
            }

            let menu = &mut self.app.interface.borders_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.borders_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(tb.text.clone())));
            }

            let menu = &mut self.app.interface.status_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.status_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(tb.text.clone())));
            }

            let menu = &mut self.app.interface.fuzzel_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.fuzzel_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(tb.text.clone())));
            }

            let menu = &mut self.app.interface.terminal_menu;
            if state == cce_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly, &self.ui_context) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalMenu(menu.selected)));
            }

            let tb = &mut self.app.interface.terminal_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if state == cce_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(tb.text.clone())));
            }





            let fs = &mut self.app.interface.color_selector_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.menubar_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.breadcrumb_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBreadcrumbFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.section_label_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionLabelFont(fs.font_family.clone())));
            }

            let fs = &mut self.app.interface.nested_section_label_font_selector;
            if state == cce_ui::widget::ElementState::Pressed && !fs.hit_test(lx, ly, &self.ui_context) { fs.unfocus(); }
            if fs.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if fs.take_change() {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelFont(fs.font_family.clone())));
            }

            let sb = &mut self.app.interface.borders_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(sb.value)));
            }

            let sb = &mut self.app.interface.status_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(sb.value)));
            }

            let sb = &mut self.app.interface.fuzzel_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelSize(sb.value)));
            }

            let sb = &mut self.app.interface.terminal_size_box;
            if state == cce_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly, &self.ui_context) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalSize(sb.value)));
            }




        }
        if self.app.current_page == Page::Services {
            let tb = &mut self.app.services.search_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            let srv = &mut self.app.services;
            if srv.list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            let query = if srv.search_box.editing {
                srv.search_box.edit_buffer.to_lowercase()
            } else {
                srv.search_box.text.to_lowercase()
            };
            let matching_count = srv.services.iter()
                .filter(|s| s.is_system == (srv.active_tab == pages::services::ServiceTab::System))
                .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                .count();
            for i in 0..matching_count.min(srv.service_items.len()) {
                if srv.service_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                }
            }
        }
        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            let tb = &mut pkgs.search_box;
            if state == cce_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly, &self.ui_context) {
                tb.unfocus();
            }
            if tb.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                        self.needs_rebuild = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let filtered: Vec<&pages::packages::PackageInfo> = pkgs.installed.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query) || p.version.to_lowercase().contains(&query))
                        .collect();
                    let matching = filtered.len();
                    for i in 0..matching.min(pkgs.installed_items.len()) {
                        if pkgs.installed_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                        }
                        if pkgs.installed_items[i].take_click() {
                            let pkg_name = filtered[i].name.clone();
                            actions.push(AppAction::Packages(pages::packages::PackagesMessage::SelectPackage(Some(pkg_name))));
                        }
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                        self.needs_rebuild = true;
                    }
                    let query = if pkgs.search_box.editing {
                        pkgs.search_box.edit_buffer.to_lowercase()
                    } else {
                        pkgs.search_box.text.to_lowercase()
                    };
                    let filtered: Vec<&pages::packages::UpdateInfo> = pkgs.updates.iter()
                        .filter(|p| p.name.to_lowercase().contains(&query))
                        .collect();
                    let matching = filtered.len();
                    for i in 0..matching.min(pkgs.updates_items.len()) {
                        if pkgs.updates_items[i].mouse_input(button, state, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                        }
                        if pkgs.updates_items[i].take_click() {
                            let pkg_name = filtered[i].name.clone();
                            actions.push(AppAction::Packages(pages::packages::PackagesMessage::SelectPackage(Some(pkg_name))));
                        }
                    }
                }
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
        }
        if state == cce_ui::widget::ElementState::Pressed && self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                self.needs_rebuild = true;
            }
            net.wifi_toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if net.wifi_toggle.take_click() {
                actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleWifi));
            }
            net.bt_toggle.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if net.bt_toggle.take_click() {
                actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleBluetooth));
            }
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
        match self.app.current_page {
            Page::Interface => {
                for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                    if cp.take_change() {
                        actions.push(AppAction::Interface(match i {
                            0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                            1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                            2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                            4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                            5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                            6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                            7 => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                            8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                            9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                            10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                            11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                            12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                            13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                            14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                            15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                            16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                            17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                            18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                            19 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                            20 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                            _ => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                        }));
                    }
                }
                if self.app.interface.notification_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(self.app.interface.notification_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.menubar_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(self.app.interface.menubar_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.tab_margin_spinbox_x.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(self.app.interface.tab_margin_spinbox_x.value as u16)));
                }
                if self.app.interface.tab_margin_spinbox_y.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(self.app.interface.tab_margin_spinbox_y.value as u16)));
                }
                if self.app.interface.button_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(self.app.interface.button_padding_spinbox.value as u16)));
                }
                if self.app.interface.section_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(self.app.interface.section_padding_spinbox.value as u16)));
                }
                if self.app.interface.label_alignment_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(self.app.interface.label_alignment_menu.selected)));
                }
                if self.app.interface.label_offset_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(self.app.interface.label_offset_spinbox.value as i16)));
                }
                if self.app.interface.label_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(self.app.interface.label_margin_spinbox.value as u16)));
                }
                if self.app.interface.plate_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(self.app.interface.plate_padding_spinbox.value as u16)));
                }
                if self.app.interface.graph_show_grid_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphShowGrid(self.app.interface.graph_show_grid_toggle.toggled())));
                }
                if self.app.interface.graph_snap_enabled_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphSnapEnabled(self.app.interface.graph_snap_enabled_toggle.toggled())));
                }
                if self.app.interface.graph_uniform_background_toggle.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphUniformBackground(self.app.interface.graph_uniform_background_toggle.toggled())));
                }
                if self.app.interface.graph_cell_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(self.app.interface.graph_cell_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.graph_gap_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(self.app.interface.graph_gap_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.page_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(self.app.interface.page_margin_spinbox.value as u16)));
                }
                if self.app.interface.grid_min_col_width_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(self.app.interface.grid_min_col_width_spinbox.value as u16)));
                }
                if self.app.interface.spinbox_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(self.app.interface.spinbox_height_spinbox.value as u16)));
                }
                if self.app.interface.spinbox_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(self.app.interface.spinbox_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.toggle_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(self.app.interface.toggle_height_spinbox.value as u16)));
                }
                if self.app.interface.toggle_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(self.app.interface.toggle_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.plate_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(self.app.interface.plate_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.plate_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(self.app.interface.plate_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.page_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(self.app.interface.page_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.layer_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(self.app.interface.layer_opacity_spinbox.value as f32 / 100.0)));
                }


                if self.app.interface.color_selector_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(self.app.interface.color_selector_height_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(self.app.interface.color_selector_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_preview_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(self.app.interface.color_selector_preview_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.color_selector_preview_margin_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(self.app.interface.color_selector_preview_margin_spinbox.value as u16)));
                }
                if self.app.interface.textbox_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(self.app.interface.textbox_height_spinbox.value as u16)));
                }
                if self.app.interface.textbox_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(self.app.interface.textbox_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.slider_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(self.app.interface.slider_height_spinbox.value as u16)));
                }
                if self.app.interface.font_selector_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(self.app.interface.font_selector_height_spinbox.value as u16)));
                }
                if self.app.interface.font_selector_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(self.app.interface.font_selector_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.dropdown_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(self.app.interface.dropdown_height_spinbox.value as u16)));
                }
                if self.app.interface.dropdown_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(self.app.interface.dropdown_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.button_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(self.app.interface.button_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.sans_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(self.app.interface.sans_box.text.clone())));
                }
                if self.app.interface.serif_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(self.app.interface.serif_box.text.clone())));
                }
                if self.app.interface.mono_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(self.app.interface.mono_box.text.clone())));
                }
                if self.app.interface.borders_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersMenu(self.app.interface.borders_menu.selected)));
                }
                if self.app.interface.borders_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(self.app.interface.borders_box.text.clone())));
                }
                if self.app.interface.status_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusMenu(self.app.interface.status_menu.selected)));
                }
                if self.app.interface.status_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(self.app.interface.status_box.text.clone())));
                }
                if self.app.interface.fuzzel_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelMenu(self.app.interface.fuzzel_menu.selected)));
                }
                if self.app.interface.fuzzel_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(self.app.interface.fuzzel_box.text.clone())));
                }
                if self.app.interface.terminal_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalMenu(self.app.interface.terminal_menu.selected)));
                }
                if self.app.interface.terminal_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(self.app.interface.terminal_box.text.clone())));
                }
                if self.app.interface.color_selector_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorFont(self.app.interface.color_selector_font_selector.font_family.clone())));
                }
                if self.app.interface.menubar_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarFont(self.app.interface.menubar_font_selector.font_family.clone())));
                }
                if self.app.interface.section_label_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSectionLabelFont(self.app.interface.section_label_font_selector.font_family.clone())));
                }
                if self.app.interface.nested_section_label_font_selector.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelFont(self.app.interface.nested_section_label_font_selector.font_family.clone())));
                }
                if self.app.interface.borders_size_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(self.app.interface.borders_size_box.value)));
                }
                if self.app.interface.status_size_box.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(self.app.interface.status_size_box.value)));
                }

                let tf = &mut self.app.interface.windows;
                for (i, sb) in tf.spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetWidth(pages::interface::WidthParam::ALL[i], sb.value as u16))));
                    }
                }
                if tf.cascade_offset_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetCascadeOffset(tf.cascade_offset_spinbox.value as u16))));
                }
                if tf.edge_gap_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetEdgeGap(tf.edge_gap_spinbox.value as u16))));
                }
                if tf.top_gap_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetTopGap(tf.top_gap_spinbox.value as u16))));
                }
                if tf.grid_gap_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetGridGap(tf.grid_gap_spinbox.value as u16))));
                }
                if tf.transition_duration_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetTransitionDuration(tf.transition_duration_spinbox.value as u16))));
                }
                if tf.status_height_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetStatusHeight(tf.status_height_spinbox.value as u16))));
                }
                for (idx, menu) in tf.tag_layout_menus.iter_mut().enumerate() {
                    if menu.take_change() {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetTagLayout(idx + 1, menu.selected))));
                    }
                }
                if tf.side_panel_behavior_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetSidePanelBehavior(tf.side_panel_behavior_menu.selected))));
                }
                if tf.side_panel_position_menu.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetSidePanelPosition(tf.side_panel_position_menu.selected))));
                }
                if tf.side_panel_width_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetSidePanelWidth(tf.side_panel_width_spinbox.value as u16))));
                }
                if tf.side_panel_border_gap_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetSidePanelBorderGap(tf.side_panel_border_gap_spinbox.value as u16))));
                }
                if tf.side_panel_border_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetSidePanelBorderOpacity(tf.side_panel_border_opacity_spinbox.value as u16))));
                }
            }
            Page::Input => {
                if self.app.input.rate_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                }
                if self.app.input.delay_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                }
                if self.app.input.scroll_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                }
                if self.app.input.scroll_speed_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
                }
                if self.app.input.pointer_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                }
                if self.app.input.trackpad_friction_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                }
                if self.app.input.trackpoint_accel_speed_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                }
                if self.app.input.cursor_size_spinbox.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
                }
                if self.app.input.tap_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
                }
                if self.app.input.scroll_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
                }
                if self.app.input.natural_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleNaturalScroll));
                }
                if self.app.input.pointer_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
                }
                if self.app.input.trackpad_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
                }
                if self.app.input.dwtp_toggle.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
                }
                if self.app.input.trackpoint_accel_profile_menu.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(self.app.input.trackpoint_accel_profile_menu.selected)));
                }
                if self.app.input.cursor_theme_menu.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyCursorTheme(self.app.input.cursor_theme_menu.selected)));
                }
                if self.app.input.zoom_in_box.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
                }
                if self.app.input.zoom_out_box.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
                }
                if self.app.input.keybinds_control.take_change() {
                    actions.push(AppAction::Input(pages::input::InputMessage::ReloadKeybinds));
                }
            }
            Page::Services => {
                if self.app.services.notifications_duration_spinbox.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(self.app.services.notifications_duration_spinbox.value)));
                }
                if self.app.services.status_padding_spinbox.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(self.app.services.status_padding_spinbox.value as u16)));
                }
                if self.app.services.notifications_enable_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsEnable));
                }
                if self.app.services.notifications_bell_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::ToggleNotificationsBell));
                }
                if self.app.services.status_separators_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleSeparators));
                }
                if self.app.services.status_underline_toggle.take_change() {
                    actions.push(AppAction::Services(pages::services::ServicesMessage::StatusToggleUnderline));
                }
            }
            Page::Display => {
                if self.app.display.screensaver_enable_toggle.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverEnable));
                }
                if self.app.display.screensaver_lock_screen_toggle.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::ToggleScreensaverLockScreen));
                }
                if self.app.display.screensaver_style_menu.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(self.app.display.screensaver_style_menu.selected)));
                }
                if self.app.display.brightness_spinbox.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(self.app.display.brightness_spinbox.value as u32)));
                }
                if self.app.display.brightness_slider.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet((self.app.display.brightness_slider.value() * 100.0).round() as u32)));
                }
                if self.app.display.screensaver_timeout_spinbox.take_change() {
                    actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(self.app.display.screensaver_timeout_spinbox.value)));
                }
            }
            Page::Hardware => {
                if self.app.hardware.cpu_gov_menu.take_change() {
                    if self.app.hardware.cpu_gov_menu.selected == 0 {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                    } else {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                    }
                }
                if self.app.hardware.gpu_gov_menu.take_change() {
                    if self.app.hardware.gpu_gov_menu.selected == 0 {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                    } else {
                        actions.push(AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                    }
                }
            }
            Page::Accounts => {
                if self.app.accounts.adding_new && self.app.accounts.email_box.take_change() {
                    let email_val = self.app.accounts.email_box.text.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }
            }
            Page::Audio => {
                for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
                for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                    if sb.take_change() {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    if slider.take_change() {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, slider.value() as f32 / 100.0)));
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    if slider.take_change() {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, slider.value() as f32 / 100.0)));
                    }
                }
            }
            Page::Radios => {
                let net = &mut self.app.network;
                if net.wifi_toggle.take_change() {
                    actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleWifi));
                }
                if net.bt_toggle.take_change() {
                    actions.push(AppAction::Radios(pages::network::NetworkMessage::ToggleBluetooth));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_wheel_internal(&mut self, delta: &cce_ui::widget::MouseScrollDelta, px: f32, py: f32) -> bool {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/cce-scroll-debug.log") {
            use std::io::Write;
            let _ = writeln!(file, "handle_mouse_wheel_internal: px={}, py={}, delta={:?}, sidebar_w={}", px, py, delta, self.sidebar_width);
        }
        let s = 1.0f32;
        if px >= self.sidebar_width * s {
            let lx = px / s;
            let ly = py / s + self.scroll_y;
            
            if self.app.current_page == Page::Input {
                let input = &self.app.input;
                if input.is_over_trackpad(lx, ly, &self.ui_context) {
                    return true;
                }
            }
            if self.app.current_page == Page::Audio {
                let mut actions = Vec::new();
                for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                    let old = sb.value;
                    if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        if sb.value != old {
                            let id = self.app.audio.sinks[i].id;
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                        }
                    }
                }
                for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                    let old = sb.value;
                    if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        if sb.value != old {
                            let id = self.app.audio.sources[i].id;
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                        }
                    }
                }
                for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                    let old = slider.value();
                    if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        let new_val = slider.value();
                        if new_val != old {
                            let id = self.app.audio.sinks[i].id;
                            if i < self.app.audio.sink_spinboxes.len() {
                                self.app.audio.sink_spinboxes[i].value = (new_val * 100.0).round() as i32;
                            }
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, new_val)));
                        }
                    }
                }
                for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                    let old = slider.value();
                    if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                        let new_val = slider.value();
                        if new_val != old {
                            let id = self.app.audio.sources[i].id;
                            if i < self.app.audio.source_spinboxes.len() {
                                self.app.audio.source_spinboxes[i].value = (new_val * 100.0).round() as i32;
                            }
                            actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, new_val)));
                        }
                    }
                }
                for a in &actions {
                    self.handle_action(a);
                }
                if !actions.is_empty() {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            
            if self.app.current_page == Page::Display {
                let mut actions = Vec::new();
                let slider = &mut self.app.display.brightness_slider;
                let old_val = slider.value();
                if slider.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old_val {
                        let pct = (new_val * 100.0).round() as u32;
                        self.app.display.brightness_spinbox.value = pct as i32;
                        actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                    }
                }
                let sb = &mut self.app.display.brightness_spinbox;
                let old = sb.value;
                if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
                    }
                }
                let sb2 = &mut self.app.display.screensaver_timeout_spinbox;
                let old2 = sb2.value;
                if sb2.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb2.value != old2 {
                        actions.push(AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(sb2.value)));
                    }
                }
                for a in &actions {
                    self.handle_action(a);
                }
                if !actions.is_empty() {
                    self.needs_rebuild = true;
                    return true;
                }
            }


            if self.app.current_page == Page::Interface {
                let mut actions = Vec::new();
                let sb = &mut self.app.interface.graph_cell_opacity_spinbox;
                let old = sb.value;
                if sb.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphCellOpacity(sb.value as f32 / 100.0)));
                    }
                }
                let sb2 = &mut self.app.interface.graph_gap_opacity_spinbox;
                let old2 = sb2.value;
                if sb2.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb2.value != old2 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapOpacity(sb2.value as f32 / 100.0)));
                    }
                }
                let sb3 = &mut self.app.interface.window_opacity_spinbox;
                let old3 = sb3.value;
                if sb3.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb3.value != old3 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(sb3.value as f32 / 100.0)));
                    }
                }
                let sb4 = &mut self.app.interface.window_corner_radius_spinbox;
                let old4 = sb4.value;
                if sb4.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb4.value != old4 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(sb4.value as u16)));
                    }
                }
                let sb5 = &mut self.app.interface.plate_opacity_spinbox;
                let old5 = sb5.value;
                if sb5.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb5.value != old5 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(sb5.value as f32 / 100.0)));
                    }
                }
                let sb6 = &mut self.app.interface.plate_corner_radius_spinbox;
                let old6 = sb6.value;
                if sb6.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb6.value != old6 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(sb6.value as u16)));
                    }
                }
                let sb7 = &mut self.app.interface.page_opacity_spinbox;
                let old7 = sb7.value;
                if sb7.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb7.value != old7 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(sb7.value as f32 / 100.0)));
                    }
                }
                let sb8 = &mut self.app.interface.layer_opacity_spinbox;
                let old8 = sb8.value;
                if sb8.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    if sb8.value != old8 {
                        actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(sb8.value as f32 / 100.0)));
                    }
                }
                for a in &actions {
                    self.handle_action(a);
                }
                if !actions.is_empty() {
                    self.needs_rebuild = true;
                    return true;
                }
            }

            if self.app.current_page == Page::Services {
                let srv = &mut self.app.services;
                if srv.list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Packages {
                let pkgs = &mut self.app.packages;
                match pkgs.active_tab {
                    pages::packages::PackageTab::Installed => {
                        if pkgs.installed_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                    pages::packages::PackageTab::Updates => {
                        if pkgs.updates_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
            if self.app.current_page == Page::Hardware {
                let hw = &mut self.app.hardware;
                if hw.cpu_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Radios {
                let net = &mut self.app.network;
                if net.wifi_list_box.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
                    self.needs_rebuild = true;
                    return true;
                }
            }

            let scroll_speed = 24.0;
            let dy = match delta {
                cce_ui::widget::MouseScrollDelta::LineDelta(_, y) => -y * scroll_speed,
                cce_ui::widget::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
            };
            let old_scroll = self.scroll_y;
            self.scroll_y = (self.scroll_y + dy).max(0.0).min(self.max_scroll_y);
            if (self.scroll_y - old_scroll).abs() > 0.01 {
                self.needs_rebuild = true;
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
        let ptr = &mut self.plates[page_idx] as &mut dyn cce_ui::widget::Element as *mut dyn cce_ui::widget::Element;
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

        if self.app.current_page == Page::Interface {
            let mc = &mut self.app.interface.custom_multicontrol;
            if mc.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if cp.keyboard_input(event, &mut self.ui_context) {
                    if cp.color != old {
                        actions.push(AppAction::Interface(match i {
                            0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                            1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                            2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                            4 => pages::interface::InterfaceMessage::SetSeparatorColor(cp.color),
                            5 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                            6 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                            7 => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                            8 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                            9 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                            10 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                            11 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                            12 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                            13 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                            14 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                            15 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                            16 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                            17 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                            18 => pages::interface::InterfaceMessage::SetWindowColor(cp.color),
                            19 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                            20 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                            _ => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                        }));
                    }
                    changed = true;
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.notification_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.window_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetWindowOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.window_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetWindowCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.menubar_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.graph_gap_width_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetGraphGapWidth(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_x;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginX(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.tab_margin_spinbox_y;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTabMarginY(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.button_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetButtonPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.section_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSectionPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.interface.label_alignment_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelAlignment(new_selected)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.label_offset_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetNestedSectionLabelOffset(new_val as i16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.label_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetLabelMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlatePadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.page_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPageMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.grid_min_col_width_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetGridMinColWidth(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.spinbox_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.spinbox_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSpinboxCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.toggle_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetToggleHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.toggle_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetToggleCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlateOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.plate_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPlateCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.page_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetPageOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.layer_opacity_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetLayerOpacity(new_val as f32 / 100.0)));
                }
                self.needs_rebuild = true;
                return true;
            }


            let sb = &mut self.app.interface.color_selector_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_preview_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.color_selector_preview_margin_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetColorSelectorPreviewMargin(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.textbox_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.textbox_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetTextboxCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.slider_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetSliderHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.font_selector_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.font_selector_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetFontSelectorCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.dropdown_height_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownHeight(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.dropdown_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetDropdownCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.interface.button_corner_radius_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Interface(pages::interface::InterfaceMessage::SetButtonCornerRadius(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }



            let mut actions = Vec::new();
            let mut consumed = false;
            
            let tf = &mut self.app.interface;
            let tb = &mut tf.sans_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSans(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.serif_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetSerif(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.mono_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMono(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.borders_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBorders(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.status_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatus(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.fuzzel_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzel(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.interface.terminal_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                if tb.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminal(tb.text.clone())));
                }
                consumed = true;
            }





            let sb = &mut self.app.interface.borders_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBordersSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.status_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.fuzzel_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetFuzzelSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.interface.terminal_size_box;
            if sb.keyboard_input(event, &mut self.ui_context) {
                actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetTerminalSize(sb.value)));
                consumed = true;
            }


            
            for a in &actions {
                self.handle_action(a);
            }
            if consumed {
                self.needs_rebuild = true;
                return true;
            }

            let mut changed = false;
            let mut actions = Vec::new();
            {
                let tf = &mut self.app.interface.windows;
                for (i, sb) in tf.spinboxes.iter_mut().enumerate() {
                    let old = sb.value;
                    if sb.keyboard_input(event, &mut self.ui_context) {
                        if sb.value != old {
                            actions.push(AppAction::Interface(
                                pages::interface::InterfaceMessage::Windows(
                                    pages::interface::WindowsMessage::SetWidth(
                                        pages::interface::WidthParam::ALL[i],
                                        sb.value as u16,
                                    )
                                )
                            ));
                        }
                        changed = true;
                    }
                }
                let sb = &mut tf.cascade_offset_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetCascadeOffset(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.edge_gap_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetEdgeGap(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.top_gap_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetTopGap(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.grid_gap_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetGridGap(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.transition_duration_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetTransitionDuration(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.status_height_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetStatusHeight(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let (menu_changed, old_selected, new_selected) = {
                    let menu = &mut tf.side_panel_behavior_menu;
                    let old = menu.selected;
                    let changed = menu.keyboard_input(event, &mut self.ui_context);
                    (changed, old, menu.selected)
                };
                if menu_changed {
                    if new_selected != old_selected {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetSidePanelBehavior(new_selected)
                            )
                        ));
                    }
                    changed = true;
                }
                let (menu_changed, old_selected, new_selected) = {
                    let menu = &mut tf.side_panel_position_menu;
                    let old = menu.selected;
                    let changed = menu.keyboard_input(event, &mut self.ui_context);
                    (changed, old, menu.selected)
                };
                if menu_changed {
                    if new_selected != old_selected {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetSidePanelPosition(new_selected)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.side_panel_width_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetSidePanelWidth(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.side_panel_border_gap_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetSidePanelBorderGap(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
                let sb = &mut tf.side_panel_border_opacity_spinbox;
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        actions.push(AppAction::Interface(
                            pages::interface::InterfaceMessage::Windows(
                                pages::interface::WindowsMessage::SetSidePanelBorderOpacity(sb.value as u16)
                            )
                        ));
                    }
                    changed = true;
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Services {
            let sb = &mut self.app.services.notifications_duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Services(pages::services::ServicesMessage::SetNotificationsDuration(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.services.status_padding_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Services(pages::services::ServicesMessage::StatusSetPadding(new_val as u16)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.delay_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_speed_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.pointer_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpad_friction_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.cursor_size_spinbox.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyCursorSize));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.zoom_in_box.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyZoomIn));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.zoom_out_box.keyboard_input(event, &mut self.ui_context) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyZoomOut));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.keybinds_control.keyboard_input(event, &mut self.ui_context) {
                if self.app.input.keybinds_control.take_change() {
                    self.handle_action(&AppAction::Input(pages::input::InputMessage::ReloadKeybinds));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Accounts {
            let mut consumed = false;
            if self.app.accounts.editing_oauth_creds {
                let tb = &mut self.app.accounts.oauth_client_id_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.oauth_client_secret_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
            } else if self.app.accounts.adding_new {
                let tb = &mut self.app.accounts.email_box;
                if tb.keyboard_input(event, &mut self.ui_context) {
                    consumed = true;
                    let email_val = tb.edit_buffer.trim().to_lowercase();
                    if email_val.ends_with("@gmail.com") {
                        self.app.accounts.imap_box.text = "imap.gmail.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.gmail.com:465".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
                    } else if email_val.ends_with("@icloud.com") {
                        self.app.accounts.imap_box.text = "imap.mail.me.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.mail.me.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
                    } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                        self.app.accounts.imap_box.text = "outlook.office365.com:993".to_string();
                        self.app.accounts.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                        self.app.accounts.smtp_box.text = "smtp.office365.com:587".to_string();
                        self.app.accounts.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
                    }
                }
                let tb = &mut self.app.accounts.password_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.imap_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
                let tb = &mut self.app.accounts.smtp_box;
                if tb.keyboard_input(event, &mut self.ui_context) { consumed = true; }
            }
            
            if consumed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Audio {
            let mut actions = Vec::new();
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event, &mut self.ui_context) {
                    if sb.value != old {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, slider) in self.app.audio.sink_sliders.iter_mut().enumerate() {
                let old = slider.value();
                if slider.keyboard_input(event, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old {
                        let id = self.app.audio.sinks[i].id;
                        if i < self.app.audio.sink_spinboxes.len() {
                            self.app.audio.sink_spinboxes[i].value = (new_val * 100.0).round() as i32;
                        }
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, new_val)));
                    }
                }
            }
            for (i, slider) in self.app.audio.source_sliders.iter_mut().enumerate() {
                let old = slider.value();
                if slider.keyboard_input(event, &mut self.ui_context) {
                    let new_val = slider.value();
                    if new_val != old {
                        let id = self.app.audio.sources[i].id;
                        if i < self.app.audio.source_spinboxes.len() {
                            self.app.audio.source_spinboxes[i].value = (new_val * 100.0).round() as i32;
                        }
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, new_val)));
                    }
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if !actions.is_empty() {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Display {
            let slider = &mut self.app.display.brightness_slider;
            let old_slider = slider.value();
            if slider.keyboard_input(event, &mut self.ui_context) {
                let new_slider = slider.value();
                if new_slider != old_slider {
                    let pct = (new_slider * 100.0).round() as u32;
                    self.app.display.brightness_spinbox.value = pct as i32;
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.display.brightness_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(new_val as u32)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let sb = &mut self.app.display.screensaver_timeout_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event, &mut self.ui_context) {
                let new_val = sb.value;
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::SetScreensaverTimeout(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
            let (menu_changed, old_selected, new_selected) = {
                let menu = &mut self.app.display.screensaver_style_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if menu_changed {
                if new_selected != old_selected {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::SetScreensaverStyle(new_selected)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }


        if self.app.current_page == Page::Services {
            let srv = &mut self.app.services;
            if srv.list_box.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let tb = &mut srv.search_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Packages {
            let pkgs = &mut self.app.packages;
            match pkgs.active_tab {
                pages::packages::PackageTab::Installed => {
                    if pkgs.installed_list_box.keyboard_input(event, &mut self.ui_context) {
                        self.needs_rebuild = true;
                        return true;
                    }
                }
                pages::packages::PackageTab::Updates => {
                    if pkgs.updates_list_box.keyboard_input(event, &mut self.ui_context) {
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }
            let tb = &mut pkgs.search_box;
            if tb.keyboard_input(event, &mut self.ui_context) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
            let (cpu_changed, old_cpu, new_cpu) = {
                let menu = &mut hw.cpu_gov_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if cpu_changed {
                if new_cpu != old_cpu {
                    if new_cpu == 0 {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPerformance));
                    } else {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetCpuPowersave));
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
            let (gpu_changed, old_gpu, new_gpu) = {
                let menu = &mut hw.gpu_gov_menu;
                let old = menu.selected;
                let changed = menu.keyboard_input(event, &mut self.ui_context);
                (changed, old, menu.selected)
            };
            if gpu_changed {
                if new_gpu != old_gpu {
                    if new_gpu == 0 {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuDefault));
                    } else {
                        self.handle_action(&AppAction::Hardware(pages::hardware::HardwareMessage::SetGpuPowersave));
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.keyboard_input(event, &mut self.ui_context) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

}

