use crate::{SystemInterface, AppWidget, make_text_buffer, make_text_buffer_with_font};
use cce_system_interface::app::PageContent;
use cce_system_interface::pages::Page;
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
        self.search_box.clear_children(&mut self.ui_context);
        self.search_box.set_parent(None, &mut self.ui_context);

        for p in Page::ALL {
            self.app.get_page_mut(p).clear_children(&mut self.ui_context);
        }

        let active_page = self.app.get_current_page_mut();
        self.page_sec_containers = active_page.get_section_containers();

        active_page.link_children(page_root, &mut self.page_sec_containers, &mut self.ui_context);

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
        self.app.get_current_page_mut().view(cx, cy, cw, ch, root_focused, &sec_focused, &mut layout, &mut self.ui_context)
    }

}

