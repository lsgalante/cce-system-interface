use crate::{SystemInterface, AppWidget, make_text_buffer, make_text_buffer_with_font};
use cce_system_interface::app::PageContent;
use cce_system_interface::pages::{self, Page};
use cce_ui::widget::{Element, TextItem, PageSelector};

impl SystemInterface {

    pub(crate) fn rebuild_layout(&mut self, sw: f32, sh: f32) {
        self.ui_context.clear_hierarchy();
        self.switcher.clear_children(&mut self.ui_context);
        for page in &mut self.pages {
            self.switcher.add_child(page.as_ptr(), &mut self.ui_context);
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);

        // ── Rebuild Element Focus Hierarchy ──
        for page in &mut self.pages {
            page.clear_children(&mut self.ui_context);
        }
        let page_root = &mut self.pages[page_idx];
        for c in &mut self.page_sec_containers {
            c.clear_children(&mut self.ui_context);
            c.set_parent(None, &mut self.ui_context);
        }

        // Clear all widgets' hierarchy links
        self.search_box.clear_children(&mut self.ui_context); self.search_box.set_parent(None, &mut self.ui_context);
        self.app.processes.services_search_box.clear_children(&mut self.ui_context); self.app.processes.services_search_box.set_parent(None, &mut self.ui_context);
        self.app.packages.search_box.clear_children(&mut self.ui_context); self.app.packages.search_box.set_parent(None, &mut self.ui_context);
        self.app.packages.installed_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.packages.installed_list_box.scroll_box.set_parent(None, &mut self.ui_context);
        self.app.packages.updates_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.packages.updates_list_box.scroll_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.email_box.clear_children(&mut self.ui_context); self.app.accounts.email_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.password_box.clear_children(&mut self.ui_context); self.app.accounts.password_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.imap_box.clear_children(&mut self.ui_context); self.app.accounts.imap_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.smtp_box.clear_children(&mut self.ui_context); self.app.accounts.smtp_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.oauth_client_id_box.clear_children(&mut self.ui_context); self.app.accounts.oauth_client_id_box.set_parent(None, &mut self.ui_context);
        self.app.accounts.oauth_client_secret_box.clear_children(&mut self.ui_context); self.app.accounts.oauth_client_secret_box.set_parent(None, &mut self.ui_context);

        self.app.processes.services_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.processes.services_list_box.scroll_box.set_parent(None, &mut self.ui_context);
        self.app.processes.cpu_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.processes.cpu_list_box.scroll_box.set_parent(None, &mut self.ui_context);

        self.app.network.wifi_list_box.scroll_box.clear_children(&mut self.ui_context); self.app.network.wifi_list_box.scroll_box.set_parent(None, &mut self.ui_context);

        for sb in &mut self.app.interface.windows.spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        self.app.interface.windows.cascade_offset_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.cascade_offset_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.edge_gap_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.edge_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.top_gap_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.top_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.grid_gap_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.grid_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.transition_duration_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.transition_duration_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.status_height_spinbox.clear_children(&mut self.ui_context); self.app.interface.windows.status_height_spinbox.set_parent(None, &mut self.ui_context);

        for cs in &mut self.app.interface.color_selectors {
            cs.clear_children(&mut self.ui_context);
            cs.set_parent(None, &mut self.ui_context);
        }
        self.app.interface.menubar_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.menubar_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.button_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.button_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.section_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.section_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.label_alignment_menu.clear_children(&mut self.ui_context);
        self.app.interface.label_alignment_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.label_offset_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.label_offset_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.label_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.label_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_padding_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_show_grid_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_show_grid_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_snap_enabled_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_snap_enabled_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_uniform_background_toggle.clear_children(&mut self.ui_context);
        self.app.interface.graph_uniform_background_toggle.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_cell_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_cell_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_gap_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_gap_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.graph_gap_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.graph_gap_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.page_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.page_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.desktop_grid_scale_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.desktop_grid_scale_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.desktop_line_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.desktop_line_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.grid_min_col_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.grid_min_col_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.spinbox_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.spinbox_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.spinbox_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.spinbox_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.toggle_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.toggle_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.toggle_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.toggle_corner_radius_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.color_selector_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.plate_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.plate_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.page_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.page_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.layer_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.layer_opacity_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.color_selector_preview_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_preview_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_preview_margin_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_preview_margin_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.textbox_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.textbox_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.textbox_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.textbox_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.slider_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.slider_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.color_selector_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.color_selector_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.menubar_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.menubar_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.breadcrumb_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.breadcrumb_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.section_label_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.section_label_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.nested_section_label_font_selector.clear_children(&mut self.ui_context);
        self.app.interface.nested_section_label_font_selector.set_parent(None, &mut self.ui_context);
        self.app.interface.font_selector_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.font_selector_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.font_selector_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.font_selector_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.dropdown_height_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.dropdown_height_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.dropdown_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.dropdown_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.button_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.button_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.notification_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.notification_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.backplate_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.backplate_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.custom_multicontrol.clear_children(&mut self.ui_context);
        self.app.interface.custom_multicontrol.set_parent(None, &mut self.ui_context);
        self.app.interface.status_box_corner_radius_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.status_box_corner_radius_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.status_padding_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.status_padding_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.sans_box.clear_children(&mut self.ui_context); self.app.interface.sans_box.set_parent(None, &mut self.ui_context);
        self.app.interface.serif_box.clear_children(&mut self.ui_context); self.app.interface.serif_box.set_parent(None, &mut self.ui_context);
        self.app.interface.mono_box.clear_children(&mut self.ui_context); self.app.interface.mono_box.set_parent(None, &mut self.ui_context);
        self.app.interface.borders_menu.clear_children(&mut self.ui_context); self.app.interface.borders_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.borders_box.clear_children(&mut self.ui_context); self.app.interface.borders_box.set_parent(None, &mut self.ui_context);
        self.app.interface.status_menu.clear_children(&mut self.ui_context); self.app.interface.status_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.status_box.clear_children(&mut self.ui_context); self.app.interface.status_box.set_parent(None, &mut self.ui_context);
        self.app.interface.fuzzel_menu.clear_children(&mut self.ui_context); self.app.interface.fuzzel_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.fuzzel_box.clear_children(&mut self.ui_context); self.app.interface.fuzzel_box.set_parent(None, &mut self.ui_context);
        self.app.interface.terminal_menu.clear_children(&mut self.ui_context); self.app.interface.terminal_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.terminal_box.clear_children(&mut self.ui_context); self.app.interface.terminal_box.set_parent(None, &mut self.ui_context);

        self.app.system_info.cpu_gov_menu.clear_children(&mut self.ui_context); self.app.system_info.cpu_gov_menu.set_parent(None, &mut self.ui_context);
        self.app.system_info.gpu_gov_menu.clear_children(&mut self.ui_context); self.app.system_info.gpu_gov_menu.set_parent(None, &mut self.ui_context);
        self.app.system_info.notifications_enable_toggle.clear_children(&mut self.ui_context); self.app.system_info.notifications_enable_toggle.set_parent(None, &mut self.ui_context);
        self.app.system_info.notifications_bell_toggle.clear_children(&mut self.ui_context); self.app.system_info.notifications_bell_toggle.set_parent(None, &mut self.ui_context);
        self.app.system_info.notifications_duration_spinbox.clear_children(&mut self.ui_context); self.app.system_info.notifications_duration_spinbox.set_parent(None, &mut self.ui_context);
        self.app.system_info.status_separators_toggle.clear_children(&mut self.ui_context); self.app.system_info.status_separators_toggle.set_parent(None, &mut self.ui_context);
        self.app.system_info.status_underline_toggle.clear_children(&mut self.ui_context); self.app.system_info.status_underline_toggle.set_parent(None, &mut self.ui_context);
        self.app.system_info.status_padding_spinbox.clear_children(&mut self.ui_context); self.app.system_info.status_padding_spinbox.set_parent(None, &mut self.ui_context);

        self.app.input.rate_spinbox.clear_children(&mut self.ui_context); self.app.input.rate_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.delay_spinbox.clear_children(&mut self.ui_context); self.app.input.delay_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_toggle.clear_children(&mut self.ui_context); self.app.input.scroll_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.scroll_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.natural_toggle.clear_children(&mut self.ui_context); self.app.input.natural_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.scroll_speed_spinbox.clear_children(&mut self.ui_context); self.app.input.scroll_speed_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.pointer_toggle.clear_children(&mut self.ui_context); self.app.input.pointer_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.pointer_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.pointer_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.trackpad_toggle.clear_children(&mut self.ui_context); self.app.input.trackpad_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.trackpad_friction_spinbox.clear_children(&mut self.ui_context); self.app.input.trackpad_friction_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.dwtp_toggle.clear_children(&mut self.ui_context); self.app.input.dwtp_toggle.set_parent(None, &mut self.ui_context);
        self.app.input.trackpoint_accel_speed_spinbox.clear_children(&mut self.ui_context); self.app.input.trackpoint_accel_speed_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.trackpoint_accel_profile_menu.clear_children(&mut self.ui_context); self.app.input.trackpoint_accel_profile_menu.set_parent(None, &mut self.ui_context);
        self.app.input.cursor_theme_menu.clear_children(&mut self.ui_context); self.app.input.cursor_theme_menu.set_parent(None, &mut self.ui_context);
        self.app.input.cursor_size_spinbox.clear_children(&mut self.ui_context); self.app.input.cursor_size_spinbox.set_parent(None, &mut self.ui_context);
        self.app.input.zoom_in_box.clear_children(&mut self.ui_context); self.app.input.zoom_in_box.set_parent(None, &mut self.ui_context);
        self.app.input.zoom_out_box.clear_children(&mut self.ui_context); self.app.input.zoom_out_box.set_parent(None, &mut self.ui_context);
        self.app.input.keybinds_control.clear_children(&mut self.ui_context); self.app.input.keybinds_control.set_parent(None, &mut self.ui_context);

        for sb in &mut self.app.audio.sink_spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        for sb in &mut self.app.audio.source_spinboxes {
            sb.clear_children(&mut self.ui_context);
            sb.set_parent(None, &mut self.ui_context);
        }
        for slider in &mut self.app.audio.sink_sliders {
            slider.clear_children(&mut self.ui_context);
            slider.set_parent(None, &mut self.ui_context);
        }
        for slider in &mut self.app.audio.source_sliders {
            slider.clear_children(&mut self.ui_context);
            slider.set_parent(None, &mut self.ui_context);
        }

        self.app.display.brightness_spinbox.clear_children(&mut self.ui_context); self.app.display.brightness_spinbox.set_parent(None, &mut self.ui_context);
        self.app.display.brightness_slider.clear_children(&mut self.ui_context); self.app.display.brightness_slider.set_parent(None, &mut self.ui_context);
        self.app.display.night_light_label.clear_children(&mut self.ui_context); self.app.display.night_light_label.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_enable_toggle.clear_children(&mut self.ui_context); self.app.display.screensaver_enable_toggle.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_lock_screen_toggle.clear_children(&mut self.ui_context); self.app.display.screensaver_lock_screen_toggle.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_timeout_spinbox.clear_children(&mut self.ui_context); self.app.display.screensaver_timeout_spinbox.set_parent(None, &mut self.ui_context);
        self.app.display.screensaver_style_menu.clear_children(&mut self.ui_context); self.app.display.screensaver_style_menu.set_parent(None, &mut self.ui_context);
        for out in &mut self.app.display.outputs {
            out.name_label.clear_children(&mut self.ui_context); out.name_label.set_parent(None, &mut self.ui_context);
            out.resolution_label.clear_children(&mut self.ui_context); out.resolution_label.set_parent(None, &mut self.ui_context);
            if let Some(ref mut scale_lbl) = out.scale_label {
                scale_lbl.clear_children(&mut self.ui_context); scale_lbl.set_parent(None, &mut self.ui_context);
            }
        }


        for menu in &mut self.app.interface.windows.tag_layout_menus {
            menu.clear_children(&mut self.ui_context);
            menu.set_parent(None, &mut self.ui_context);
        }
        self.app.interface.windows.side_panel_behavior_menu.clear_children(&mut self.ui_context);
        self.app.interface.windows.side_panel_behavior_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.side_panel_position_menu.clear_children(&mut self.ui_context);
        self.app.interface.windows.side_panel_position_menu.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.side_panel_width_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.side_panel_width_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.side_panel_border_gap_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.side_panel_border_gap_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.side_panel_border_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.side_panel_border_opacity_spinbox.set_parent(None, &mut self.ui_context);

        self.app.interface.windows.fullscreen_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.fullscreen_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.cascade_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.cascade_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.grid_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.grid_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.floating_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.floating_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.pinned_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.pinned_opacity_spinbox.set_parent(None, &mut self.ui_context);
        self.app.interface.windows.popup_opacity_spinbox.clear_children(&mut self.ui_context);
        self.app.interface.windows.popup_opacity_spinbox.set_parent(None, &mut self.ui_context);

        use cce_ui::widget::focus::link_parent_child;
        match self.app.current_page {
            Page::Accounts => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Accounts").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Modify Accounts").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                if self.app.accounts.editing_oauth_creds {
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.oauth_client_id_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.oauth_client_secret_box, &mut self.ui_context);
                } else if self.app.accounts.adding_new {
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.email_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.password_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.imap_box, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.accounts.smtp_box, &mut self.ui_context);
                }
            }

            Page::System => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Host Info").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("OS & Kernel").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("CPU Gov").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("GPU Gov").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Power Profile").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Backup Status").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Notifications").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Status Bar").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Updates").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..9 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.system_info.cpu_gov_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.system_info.gpu_gov_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.system_info.notifications_enable_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.system_info.notifications_bell_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.system_info.notifications_duration_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.system_info.status_separators_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.system_info.status_underline_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.system_info.status_padding_spinbox, &mut self.ui_context);
            }
            Page::Processes => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Processes List").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Services Control").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.processes.cpu_list_box.scroll_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.processes.services_search_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.processes.services_list_box.scroll_box, &mut self.ui_context);
            }
            Page::Radios => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Wi-Fi Networks").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Radio Switches").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.network.wifi_list_box.scroll_box, &mut self.ui_context);
            }
            Page::Interface => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Custom Parameters").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Layout").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Status").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Controls").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Indicators").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Notification").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Surfaces").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Fonts").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Containers").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Windows").with_layout(cce_ui::widget::AdaptiveGridLayout { min_col_width: 140.0, gap: 8.0, padding_x: 0.0, padding_y: 0.0 }));
                
                for i in 0..10 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                
                // Section 0: Custom Parameters
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.interface.custom_multicontrol, &mut self.ui_context);
                
                // Section 1: Layout (parent of Grid Layout)
                // (Layout widgets)
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.color_selectors[1], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.color_selectors[2], &mut self.ui_context);
                // (Grid Layout child widgets)
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.interface.grid_min_col_width_spinbox, &mut self.ui_context);

                
                // Section 2: Status
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[7], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[3], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.color_selectors[22], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.status_box_corner_radius_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.interface.status_padding_spinbox, &mut self.ui_context);
                
                // Section 3: Controls (parent of: Slider, MenuBar, Toggles, Spinbox, ColorSelector, Textbox, FontSelector)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[5], &mut self.ui_context);
                // (Slider child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[4], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.slider_height_spinbox, &mut self.ui_context);
                
                // (MenuBar child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[8], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[10], &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.button_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.menubar_opacity_spinbox, &mut self.ui_context);
                 // (Toggles child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[11], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[12], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.toggle_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.toggle_corner_radius_spinbox, &mut self.ui_context);
                
                // (Breadcrumb child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selectors[14], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.breadcrumb_font_selector, &mut self.ui_context);
 
                // (Spinbox child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.spinbox_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.spinbox_corner_radius_spinbox, &mut self.ui_context);
                
                // (ColorSelector child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_preview_corner_radius_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_preview_margin_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_corner_radius_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.color_selector_font_selector, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.menubar_font_selector, &mut self.ui_context);
                
                // (Textbox child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.textbox_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.textbox_corner_radius_spinbox, &mut self.ui_context);
                
                // (FontSelector child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.font_selector_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.font_selector_corner_radius_spinbox, &mut self.ui_context);
                
                // (Dropdown child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.dropdown_height_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.dropdown_corner_radius_spinbox, &mut self.ui_context);
                
                // (Button child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.button_corner_radius_spinbox, &mut self.ui_context);
                
                // (Labels child widgets)
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.interface.label_margin_spinbox, &mut self.ui_context);
                
                // Section 4: Indicators (parent of Primary Highlight)
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.interface.color_selectors[9], &mut self.ui_context);
                
                // Section 5: Notification
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.interface.color_selectors[16], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.interface.notification_opacity_spinbox, &mut self.ui_context);
                
                // Section 6: Surfaces
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[17], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.backplate_corner_radius_spinbox, &mut self.ui_context);
                // (Plate child widgets)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[0], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.plate_corner_radius_spinbox, &mut self.ui_context);
                // (Popover child widgets)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[15], &mut self.ui_context);
                // (Desktop child widgets)
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[6], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.color_selectors[23], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.desktop_grid_scale_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.interface.desktop_line_width_spinbox, &mut self.ui_context);
                
                // Section 7: Fonts (parent of System Fonts and Program Fonts)
                // (System Fonts)
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.sans_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.serif_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.mono_box, &mut self.ui_context);
                
                // (Program Fonts)
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.borders_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.borders_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.status_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.status_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.fuzzel_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.fuzzel_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.terminal_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[7], &mut self.app.interface.terminal_box, &mut self.ui_context);
 
                // Section 8: Containers (parent of ScrollingList, Sections, Graph)
                // (Sections child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.section_padding_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.page_margin_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.section_label_font_selector, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.label_alignment_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.label_offset_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.nested_section_label_font_selector, &mut self.ui_context);
                // (ScrollingList child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[13], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[20], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[21], &mut self.ui_context);
                // (Page child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[18], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.page_opacity_spinbox, &mut self.ui_context);
                // (Layer child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.color_selectors[19], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.layer_opacity_spinbox, &mut self.ui_context);
                // (Graph child widgets)
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_show_grid_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_snap_enabled_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_uniform_background_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_cell_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_gap_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[8], &mut self.app.interface.graph_gap_width_spinbox, &mut self.ui_context);

                // Section 9: Windows (parent of Fullscreen, Cascade, Grid, Floating, Movement, Default Layouts, Side Panel, Effects)
                // Fullscreen
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.spinboxes[0], &mut self.ui_context);
                
                // Cascade
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.spinboxes[1], &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.cascade_offset_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.edge_gap_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.top_gap_spinbox, &mut self.ui_context);
                
                // Grid
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.grid_gap_spinbox, &mut self.ui_context);
                
                // Floating
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.spinboxes[3], &mut self.ui_context);
                
                // Movement
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.transition_duration_spinbox, &mut self.ui_context);
                
                // Default Layouts
                for menu in &mut self.app.interface.windows.tag_layout_menus {
                    link_parent_child(&mut self.page_sec_containers[9], menu, &mut self.ui_context);
                }

                // Side Panel
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.side_panel_behavior_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.side_panel_position_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.side_panel_width_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.side_panel_border_gap_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.side_panel_border_opacity_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.pinned_opacity_spinbox, &mut self.ui_context);

                // Popup
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.popup_opacity_spinbox, &mut self.ui_context);

                // Fullscreen
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.fullscreen_opacity_spinbox, &mut self.ui_context);

                // Cascade
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.cascade_opacity_spinbox, &mut self.ui_context);

                // Grid
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.grid_opacity_spinbox, &mut self.ui_context);

                // Floating
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.floating_opacity_spinbox, &mut self.ui_context);

                // Effects
                link_parent_child(&mut self.page_sec_containers[9], &mut self.app.interface.windows.blur_toggle, &mut self.ui_context);
            }


            Page::Input => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Touchpad").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Trackpoint").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Keyboard").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Cursor").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Scrolling").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Inertial Input").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Keyboard Bindings").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..7 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.dwtp_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_speed_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_profile_menu, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.rate_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.delay_spinbox, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_theme_menu, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.cursor_size_spinbox, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_friction_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.natural_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.input.scroll_speed_spinbox, &mut self.ui_context);
                
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.pointer_friction_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[4], &mut self.app.input.trackpad_friction_spinbox, &mut self.ui_context);

                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.input.zoom_in_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[5], &mut self.app.input.zoom_out_box, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[6], &mut self.app.input.keybinds_control, &mut self.ui_context);
            }
            Page::Audio => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Output").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Input").with_layout(cce_ui::widget::VerticalLayout::default()));
                link_parent_child(page_root, &mut self.page_sec_containers[0], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[1], &mut self.ui_context);
                
                for sb in &mut self.app.audio.sink_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[0], &mut **sb, &mut self.ui_context);
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[1], &mut **sb, &mut self.ui_context);
                }
                for slider in &mut self.app.audio.sink_sliders {
                    link_parent_child(&mut self.page_sec_containers[0], &mut **slider, &mut self.ui_context);
                }
                for slider in &mut self.app.audio.source_sliders {
                    link_parent_child(&mut self.page_sec_containers[1], &mut **slider, &mut self.ui_context);
                }
            }
            Page::Display => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Brightness").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Night Light").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Outputs").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Screensaver Settings").with_layout(cce_ui::widget::VerticalLayout::default()));
                link_parent_child(page_root, &mut self.page_sec_containers[0], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[1], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[2], &mut self.ui_context);
                link_parent_child(page_root, &mut self.page_sec_containers[3], &mut self.ui_context);

                // Section 0: Brightness
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.display.brightness_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.display.brightness_slider, &mut self.ui_context);

                // Section 1: Night Light
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.display.night_light_label, &mut self.ui_context);

                // Section 2: Outputs
                for out in &mut self.app.display.outputs {
                    link_parent_child(&mut self.page_sec_containers[2], &mut out.name_label, &mut self.ui_context);
                    link_parent_child(&mut self.page_sec_containers[2], &mut out.resolution_label, &mut self.ui_context);
                    if let Some(ref mut scale_lbl) = out.scale_label {
                        link_parent_child(&mut self.page_sec_containers[2], scale_lbl, &mut self.ui_context);
                    }
                }

                // Section 3: Screensaver Settings
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_enable_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_lock_screen_toggle, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_timeout_spinbox, &mut self.ui_context);
                link_parent_child(&mut self.page_sec_containers[3], &mut self.app.display.screensaver_style_menu, &mut self.ui_context);
            }
            Page::Packages => {
                self.page_sec_containers.clear();
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("Packages").with_layout(cce_ui::widget::VerticalLayout::default()));
                self.page_sec_containers.push(cce_ui::widget::SectionContainer::new("System Update").with_layout(cce_ui::widget::VerticalLayout::default()));
                for i in 0..2 {
                    link_parent_child(page_root, &mut self.page_sec_containers[i], &mut self.ui_context);
                }
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.search_box, &mut self.ui_context);
                match self.app.packages.active_tab {
                    pages::packages::PackageTab::Installed => {
                        link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.installed_list_box.scroll_box, &mut self.ui_context);
                    }
                    pages::packages::PackageTab::Updates => {
                        link_parent_child(&mut self.page_sec_containers[0], &mut self.app.packages.updates_list_box.scroll_box, &mut self.ui_context);
                    }
                }
            }
            _ => {}
        }

        self.sidebar_width = self.menubar.sidebar_w();
        self.header_height = 0.0; // No CSD Titlebar
        let s = 1.0f32;
        let mut widgets = Vec::new();
        let mut text_items = Vec::new();
        let mut page_buttons = Vec::new();

        cce_ui::widget::hover_animation::reset_frame_registration();
        cce_ui::widget::popovers::clear();
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        cce_ui::widget::hover_animation::set_cursor_pos(self.cursor_x / s, self.cursor_y / s);

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw / s - self.sidebar_width;
        let mut lch = sh / s - self.header_height - self.status_height;
        if self.search_open {
            lch -= 42.0;
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.menubar.set_selected_page(page_idx);
        self.switcher.set_active_index(Some(page_idx));

        // Update root window size, background color, opacity, corner radius, and children
        self.root_window.set_rect(0.0, 0.0, sw / s, sh / s);
        let win_r = self.app.interface.backplate_color[0] as f32 / 255.0;
        let win_g = self.app.interface.backplate_color[1] as f32 / 255.0;
        let win_b = self.app.interface.backplate_color[2] as f32 / 255.0;
        let win_a = 1.0f32;
        self.root_window.background_color = Some(cce_ui::color::to_linear([win_r, win_g, win_b, win_a]));
        self.root_window.radius = self.app.interface.backplate_corner_radius as f32;
        self.root_window.clear_children(&mut self.ui_context);
        self.root_window.add_child(self.menubar.as_ptr(), &mut self.ui_context);
        self.root_window.add_child(self.switcher.as_ptr(), &mut self.ui_context);
        if self.search_open {
            use cce_ui::widget::focus::link_parent_child;
            link_parent_child(&mut self.root_window, &mut self.search_box, &mut self.ui_context);
        }

        // Position sidebar and switcher below the titlebar
        let mut dummy_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.menubar, 0.0, self.header_height, self.sidebar_width, sh / s - self.header_height, &mut self.ui_context);
        let switcher_h = if self.search_open {
            sh / s - self.header_height - 42.0
        } else {
            sh / s - self.header_height
        };
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.switcher, self.sidebar_width, self.header_height, sw / s - self.sidebar_width, switcher_h, &mut self.ui_context);

        // Render root window recursively
        let mut window_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut window_pc, &mut self.root_window, 0.0, 0.0, sw / s, sh / s, &mut self.ui_context);

        let mut search_pc = PageContent::new();
        if self.search_open {
            search_pc.rects.push((
                [0.08, 0.08, 0.12, 1.0],
                self.sidebar_width,
                sh / s - 42.0,
                sw / s - self.sidebar_width,
                42.0,
                0.0,
                (false, false, false, false),
            ));
            search_pc.rects.push((
                [0.18, 0.18, 0.24, 1.0],
                self.sidebar_width,
                sh / s - 42.0,
                sw / s - self.sidebar_width,
                1.0,
                0.0,
                (false, false, false, false),
            ));
            cce_ui::layout::render_widget(
                &mut search_pc,
                &mut self.search_box,
                self.sidebar_width + 12.0,
                sh / s - 36.0,
                sw / s - self.sidebar_width - 24.0,
                30.0,
                &mut self.ui_context,
            );
        }

        // CSD Titlebar removed

        for pc_part in &[window_pc] {
            for (c, x, y, w, h, r, corners) in &pc_part.rects {
                widgets.push(AppWidget {
                    x: *x * s, y: *y * s, w: *w * s, h: *h * s,
                    color: *c, hover_color: *c,
                    hovering: false,
                    radius: *r * s,
                    corners: *corners,
                });
            }
            for (t, size, x, y, tc, font_opt, bounds) in &pc_part.texts {
                text_items.push(TextItem {
                    buffer: make_text_buffer_with_font(
                        &mut self.font_system,
                        t,
                        *size * s,
                        font_opt.as_deref(),
                        &self.sans_serif_family,
                        &self.serif_family,
                        &self.monospace_family,
                    ),
                    x: *x * s, y: *y * s,
                    color: glyphon::Color::rgb(
                        (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                    ),
                    bounds: *bounds,
                });
            }
        }

        self.scrollable_widgets_start_idx = widgets.len();
        self.scrollable_text_items_start_idx = text_items.len();
        self.scrollable_buttons_start_idx = page_buttons.len();

        // Page content in LOGICAL coordinates, then scale to physical
        let pc = self.render_page_content(lcx, lcy, lcw, lch);

        if self.search_open && !self.search_query.is_empty() && !self.pages[page_idx].scroll_bar.dragging {
            let query_lower = self.search_query.to_lowercase();
            let mut first_match_y = None;
            for (t, _, _, y, _, _, _) in &pc.texts {
                if t.to_lowercase().contains(&query_lower) {
                    first_match_y = Some(*y);
                    break;
                }
            }
            if let Some(y) = first_match_y {
                let mut max_y = 0.0f32;
                for (_, _, y, _, h, _, _) in &pc.rects {
                    max_y = max_y.max(y + h);
                }
                for (_, size, _, y, _, _, _) in &pc.texts {
                    max_y = max_y.max(y + size);
                }
                for (btn, _) in &pc.buttons {
                    let base = btn.base().unwrap();
                    max_y = max_y.max(base.y + base.h);
                }
                let local_max_scroll_y = (max_y - lch).max(0.0);
                self.scroll_y = (y - 100.0).clamp(0.0, local_max_scroll_y);
            }
        }

        let mut popovers = Vec::new();
        Self::collect_popover_rects(&self.pages[page_idx], &mut popovers, &self.ui_context);

        let mut max_y = 0.0f32;
        for (_, _, y, _, h, _, _) in &pc.rects {
            max_y = max_y.max(y + h);
        }
        for (_, size, _, y, _, _, _) in &pc.texts {
            max_y = max_y.max(y + size);
        }
        for (btn, _) in &pc.buttons {
            let base = btn.base().unwrap();
            max_y = max_y.max(base.y + base.h);
        }
        self.max_scroll_y = (max_y - lch).max(0.0);
        eprintln!("SCROLL_DEBUG: max_y={}, lch={}, max_scroll_y={}, scroll_y={}", max_y, lch, self.max_scroll_y, self.scroll_y);
        static mut FRAME_COUNT: usize = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT > 5 {
                self.scroll_y = self.scroll_y.min(self.max_scroll_y);
            }
        }

        let page_root = &mut self.pages[page_idx];
        if page_root.scroll_bar.dragging {
            self.scroll_y = page_root.scroll_bar.scroll_y;
        } else {
            page_root.scroll_y = self.scroll_y;
            page_root.scroll_bar.scroll_y = self.scroll_y;
        }
        page_root.content_h = max_y;
        page_root.scroll_bar.update(self.scroll_y, max_y, lch);

        let scroll_offset_y = self.scroll_y;

        for (c, x, y, w, h, r, corners) in &pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false,
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &pc.texts {
            let shifted_bounds = bounds.map(|[bl, bt, br, bb]| {
                [bl, bt - scroll_offset_y, br, bb - scroll_offset_y]
            });

            let matched = self.search_open && !self.search_query.is_empty() && t.to_lowercase().contains(&self.search_query.to_lowercase());

            if matched {
                let text_buf = make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                );
                let scale = cce_ui::scale::scale_factor();
                let text_w = text_buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0) / scale;

                let pad_x = 4.0;
                let pad_y = 2.0;
                let rect_x = *x - pad_x;
                let rect_y = *y - pad_y;
                let rect_w = text_w + 2.0 * pad_x;
                let rect_h = *size + 2.0 * pad_y;

                widgets.push(AppWidget {
                    x: rect_x * s,
                    y: (rect_y - scroll_offset_y) * s,
                    w: rect_w * s,
                    h: rect_h * s,
                    color: [0.65, 0.45, 0.05, 0.4],
                    hover_color: [0.65, 0.45, 0.05, 0.4],
                    hovering: false,
                    radius: 3.0 * s,
                    corners: (true, true, true, true),
                });
            }

            let text_color = if self.search_open && !self.search_query.is_empty() {
                if matched {
                    [1.0, 0.95, 0.80, 1.0]
                } else {
                    [tc[0] * 0.25, tc[1] * 0.25, tc[2] * 0.25, tc[3] * 0.25]
                }
            } else {
                *tc
            };

            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: (*y - scroll_offset_y) * s,
                color: glyphon::Color::rgb(
                    (text_color[0] * 255.0) as u8, (text_color[1] * 255.0) as u8, (text_color[2] * 255.0) as u8,
                ),
                bounds: shifted_bounds,
            });
        }
        for (btn, action) in &pc.buttons {
            let base = btn.base().unwrap();
            let bg = btn.bg.unwrap_or([0.16, 0.16, 0.24, 1.0]);
            let hover_bg = btn.hover_bg.unwrap_or([0.25, 0.30, 0.26, 1.0]);
            widgets.push(AppWidget {
                x: base.x * s, y: (base.y - scroll_offset_y) * s, w: base.w * s, h: base.h * s,
                color: bg, hover_color: hover_bg,
                hovering: false,
                radius: cce_ui::layout::button_corner_radius() * s,
                corners: (true, true, true, true),
            });
            let label = base.label.as_deref().unwrap_or("");
            let label_size = 12.0;
            let buf = make_text_buffer(&mut self.font_system, label, label_size * s);
            let scale = cce_ui::scale::scale_factor();
            let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0) / scale;
            let lh = label_size * s * 1.4;
            let mut left_align = btn.left_align;

            // Auto-detect if inside a ScrollBox to apply left alignment by default
            if !left_align && base.w >= 60.0 {
                if self.app.current_page == Page::Processes {
                    let sb1 = &self.app.processes.cpu_list_box;
                    let (sb1_x, sb1_y, sb1_w, sb1_h) = sb1.rect();
                    if base.x >= sb1_x - 1.0 && base.x + base.w <= sb1_x + sb1_w + 1.0
                       && base.y >= sb1_y - 1.0 && base.y + base.h <= sb1_y + sb1_h + 1.0 {
                        left_align = true;
                    }
                    let sb2 = &self.app.processes.services_list_box;
                    let (sb2_x, sb2_y, sb2_w, sb2_h) = sb2.rect();
                    if base.x >= sb2_x - 1.0 && base.x + base.w <= sb2_x + sb2_w + 1.0
                       && base.y >= sb2_y - 1.0 && base.y + base.h <= sb2_y + sb2_h + 1.0 {
                        left_align = true;
                    }
                }
            }

            let text_x = if left_align {
                base.x * s + 8.0 * s
            } else {
                base.x * s + (base.w * s - tw) / 2.0
            };

            let label_color = btn.label_color.unwrap_or([0.83, 0.83, 0.83, 1.0]);
            text_items.push(TextItem {
                buffer: buf,
                x: text_x, y: (base.y - scroll_offset_y) * s + (base.h * s - lh) / 2.0,
                color: glyphon::Color::rgb(
                    (label_color[0] * 255.0) as u8,
                    (label_color[1] * 255.0) as u8,
                    (label_color[2] * 255.0) as u8,
                ),
                bounds: None,
            });
            let mut btn_clone = btn.clone();
            if let Some(base_mut) = btn_clone.base_mut() {
                base_mut.x *= s;
                base_mut.y = (base_mut.y - scroll_offset_y) * s;
                base_mut.w *= s;
                base_mut.h *= s;
            }
            page_buttons.push((btn_clone, action.clone()));
        }

        // Render popovers on top of everything (both backgrounds and texts)
        let mut popover_pc = PageContent::new();
        cce_ui::layout::render_popovers(&mut popover_pc, &mut self.ui_context);

        for (c, x, y, w, h, r, corners) in &popover_pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false,
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &popover_pc.texts {
            let shifted_bounds = bounds.map(|[bl, bt, br, bb]| {
                [bl, bt - scroll_offset_y, br, bb - scroll_offset_y]
            });
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: (*y - scroll_offset_y) * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
                bounds: shifted_bounds,
            });
        }







        for (c, x, y, w, h, r, corners) in &search_pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: *y * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false,
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &search_pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: *y * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
                bounds: *bounds,
            });
        }

        self.widgets = widgets;
        self.text_items = text_items;
        self.page_buttons = page_buttons;
        self.needs_rebuild = false;
        self.last_scroll_y = self.scroll_y;
        self.ui_context.clear_dirty();
    }

    pub(crate) fn render_page_content(&mut self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use pages::*;
        use cce_ui::layout::AdaptiveGrid;
        let margin = cce_ui::layout::page_margin();
        let cx = cx + margin;
        let cy = cy + margin;
        let cw = (cw - 2.0 * margin).max(1.0);
        let ch = (ch - 2.0 * margin).max(1.0);
        let mut layout = AdaptiveGrid::new(260.0, 20.0);
        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        let root_focused = cce_ui::widget::focus::is_focused(&self.pages[page_idx]);
        let sec_focused: Vec<bool> = self.page_sec_containers.iter()
            .map(|c| cce_ui::widget::focus::is_focused(c))
            .collect();
        match self.app.current_page {
            Page::Accounts => accounts::view(&mut self.app.accounts, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Audio => audio::view(&mut self.app.audio, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Display => display::view(&mut self.app.display, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Radios => network::view(&mut self.app.network, cx, cy, cw, ch, root_focused, &mut layout, &mut self.ui_context),
            Page::Processes => processes::view(&mut self.app.processes, cx, cy, cw, ch, root_focused, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Input => input::view(&mut self.app.input, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::System => system_info::view(&mut self.app.system_info, cx, cy, cw, ch, root_focused, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Storage => storage::view(&self.app.storage, cx, cy, cw, ch, &mut layout, &mut self.ui_context),
            Page::Interface => interface::view(&mut self.app.interface, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
            Page::Packages => packages::view(&mut self.app.packages, cx, cy, cw, ch, &sec_focused, &mut layout, &mut self.ui_context),
        }
    }

}

