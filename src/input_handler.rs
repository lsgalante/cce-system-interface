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

        // Drag updates are high-priority overrides
        let mut drag_handled = false;
        if self.app.current_page == Page::Audio {
            if let Some(idx) = self.audio_sink_dragging {
                drag_handled = true;
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
                drag_handled = true;
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
            }
        }
        if self.app.current_page == Page::Display {
            if self.display_brightness_dragging {
                drag_handled = true;
                if self.app.display.brightness_slider.drag_update(lx, ly) {
                    changed = true;
                    let val = self.app.display.brightness_slider.value();
                    let pct = (val * 100.0).round() as u32;
                    self.app.display.brightness_spinbox.value = pct as i32;
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(pct)));
                }
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
        let mut actions = Vec::new();
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
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let event = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
        if let Some(root) = self.get_page_root_widget() {
            self.ui_context.propagate_event(&event, root);
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
        match self.app.current_page {
            Page::Interface => {
                for (i, cp) in self.app.interface.color_selectors.iter_mut().enumerate() {
                    if cp.take_change() {
                        actions.push(AppAction::Interface(match i {
                            0 => pages::interface::InterfaceMessage::SetPageLowColor(cp.color),
                            1 => pages::interface::InterfaceMessage::SetHighColor(cp.color),
                            2 => pages::interface::InterfaceMessage::SetVisualGuidesColor(cp.color),
                            3 => pages::interface::InterfaceMessage::SetDisabledColor(cp.color),
                            4 => pages::interface::InterfaceMessage::SetSliderTrackColor(cp.color),
                            5 => pages::interface::InterfaceMessage::SetColorBordersColor(cp.color),
                            6 => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                            7 => pages::interface::InterfaceMessage::SetNormalColor(cp.color),
                            8 => pages::interface::InterfaceMessage::SetPaginatorSidebarColor(cp.color),
                            9 => pages::interface::InterfaceMessage::SetPrimaryHighlightColor(cp.color),
                            10 => pages::interface::InterfaceMessage::SetMenubarTabLabelColor(cp.color),
                            11 => pages::interface::InterfaceMessage::SetToggleEnabledColor(cp.color),
                            12 => pages::interface::InterfaceMessage::SetToggleDisabledColor(cp.color),
                            13 => pages::interface::InterfaceMessage::SetScrollingListBgColor(cp.color),
                            14 => pages::interface::InterfaceMessage::SetBreadcrumbBgColor(cp.color),
                            15 => pages::interface::InterfaceMessage::SetPopoverBgColor(cp.color),
                            16 => pages::interface::InterfaceMessage::SetNotificationBgColor(cp.color),
                            17 => pages::interface::InterfaceMessage::SetBackplateColor(cp.color),
                            18 => pages::interface::InterfaceMessage::SetPageColor(cp.color),
                            19 => pages::interface::InterfaceMessage::SetLayerColor(cp.color),
                            20 => pages::interface::InterfaceMessage::SetScrollingListEntryBgColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha]),
                            21 => pages::interface::InterfaceMessage::SetScrollingListEntryHighlightColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha]),
                            22 => pages::interface::InterfaceMessage::SetStatusBoxBackgroundColor(cp.color),
                            23 => pages::interface::InterfaceMessage::SetDesktopGridColor([cp.color[0], cp.color[1], cp.color[2], cp.alpha]),
                            _ => pages::interface::InterfaceMessage::SetDesktopBackground(cp.color),
                        }));
                    }
                }
                if self.app.interface.notification_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetNotificationOpacity(self.app.interface.notification_opacity_spinbox.value as f32 / 100.0)));
                }
                if self.app.interface.desktop_grid_scale_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDesktopGridScale(self.app.interface.desktop_grid_scale_spinbox.value as u16)));
                }
                if self.app.interface.desktop_line_width_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetDesktopLineWidth(self.app.interface.desktop_line_width_spinbox.value as u16)));
                }
                if self.app.interface.menubar_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetMenubarOpacity(self.app.interface.menubar_opacity_spinbox.value as f32 / 100.0)));
                }


                if self.app.interface.status_box_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusBoxCornerRadius(self.app.interface.status_box_corner_radius_spinbox.value as u16)));
                }
                if self.app.interface.status_padding_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusPadding(self.app.interface.status_padding_spinbox.value as u16)));
                }
                if self.app.interface.status_module_spacing_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetStatusModuleSpacing(self.app.interface.status_module_spacing_spinbox.value as u16)));
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
                if self.app.interface.backplate_corner_radius_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::SetBackplateCornerRadius(self.app.interface.backplate_corner_radius_spinbox.value as u16)));
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
                if tf.fullscreen_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetFullscreenOpacity(tf.fullscreen_opacity_spinbox.value as u16))));
                }
                if tf.cascade_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetCascadeOpacity(tf.cascade_opacity_spinbox.value as u16))));
                }
                if tf.grid_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetGridOpacity(tf.grid_opacity_spinbox.value as u16))));
                }
                if tf.floating_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetFloatingOpacity(tf.floating_opacity_spinbox.value as u16))));
                }
                if tf.pinned_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetPinnedOpacity(tf.pinned_opacity_spinbox.value as u16))));
                }
                if tf.popup_opacity_spinbox.take_change() {
                    actions.push(AppAction::Interface(pages::interface::InterfaceMessage::Windows(pages::interface::WindowsMessage::SetPopupOpacity(tf.popup_opacity_spinbox.value as u16))));
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
            Page::System => {
                if self.app.system_info.cpu_gov_menu.take_change() {
                    if self.app.system_info.cpu_gov_menu.selected == 0 {
                        actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::SetCpuPerformance));
                    } else {
                        actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::SetCpuPowersave));
                    }
                }
                if self.app.system_info.gpu_gov_menu.take_change() {
                    if self.app.system_info.gpu_gov_menu.selected == 0 {
                        actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::SetGpuDefault));
                    } else {
                        actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::SetGpuPowersave));
                    }
                }
                if self.app.system_info.notifications_enable_toggle.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::ToggleNotificationsEnable));
                }
                if self.app.system_info.notifications_bell_toggle.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::ToggleNotificationsBell));
                }
                if self.app.system_info.notifications_duration_spinbox.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::SetNotificationsDuration(self.app.system_info.notifications_duration_spinbox.value)));
                }
                if self.app.system_info.status_separators_toggle.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::StatusToggleSeparators));
                }
                if self.app.system_info.status_underline_toggle.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::StatusToggleUnderline));
                }
                if self.app.system_info.status_padding_spinbox.take_change() {
                    actions.push(AppAction::SystemInfo(pages::system_info::SystemMessage::StatusSetPadding(self.app.system_info.status_padding_spinbox.value as u16)));
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
        self.scroll_logs.push(format!(
            "handle_mouse_wheel_internal: px={}, py={}, delta={:?}, sidebar_w={}",
            px, py, delta, self.sidebar_width
        ));
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

            let event = cce_ui::widget::Event::MouseWheel { delta: delta.clone(), x: lx, y: ly, local_x: lx, local_y: ly };
            let mut handled = false;
            if let Some(root) = self.get_page_root_widget() {
                if self.ui_context.propagate_event(&event, root) {
                    handled = true;
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
        if let Some(root) = self.get_page_root_widget() {
            if self.ui_context.propagate_event(&event_wrapper, root) {
                let mut actions = Vec::new();
                self.propagate_widget_changes(&mut actions);
                for a in actions {
                    self.handle_action(&a);
                }
                self.needs_rebuild = true;
                return true;
            }
        }

        false
    }
}

