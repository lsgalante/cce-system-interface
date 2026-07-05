use crate::{SystemInterface, AppWidget, make_text_buffer_with_font};
use cce_settings::app::PageContent;
use cce_settings::pages::Page;
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
        let mut widgets = Vec::new();
        let mut text_items = Vec::new();
        let mut page_buttons = Vec::new();

        cce_ui::widget::hover_animation::reset_frame_registration();
        cce_ui::widget::popovers::clear();
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        cce_ui::widget::hover_animation::set_cursor_pos(self.cursor_x, self.cursor_y);

        let s = self.scale_factor as f32;
        let cursor_phys_x = self.cursor_x * s;
        let cursor_phys_y = self.cursor_y * s;
        let check_hover = |wx: f32, wy: f32, ww: f32, wh: f32| -> bool {
            cursor_phys_x >= wx && cursor_phys_x <= wx + ww && cursor_phys_y >= wy && cursor_phys_y <= wy + wh
        };
        let logical_sw = sw;
        let logical_sh = sh;

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = logical_sw - self.sidebar_width;
        let mut lch = logical_sh - self.header_height - self.status_height;
        if self.search_open {
            lch -= 42.0;
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.menubar.set_selected_page(page_idx);
        self.switcher.set_active_index(Some(page_idx));

        // Update root window size, background color, opacity, corner radius, and children
        self.root_window.set_rect(0.0, 0.0, logical_sw, logical_sh);
        let win_r = 0x0a as f32 / 255.0;
        let win_g = 0x1a as f32 / 255.0;
        let win_b = 0x0e as f32 / 255.0;
        let win_a = 1.0f32;
        self.root_window.background_color = Some(cce_ui::color::to_linear([win_r, win_g, win_b, win_a]));
        self.root_window.radius = 12.0;
        self.root_window.clear_children(&mut self.ui_context);
        self.root_window.add_child(self.menubar.as_ptr(), &mut self.ui_context);
        self.root_window.add_child(self.switcher.as_ptr(), &mut self.ui_context);
        self.root_window.add_child(self.statusbar.as_ptr(), &mut self.ui_context);
        if self.search_open {
            use cce_ui::widget::focus::link_parent_child;
            link_parent_child(&mut self.root_window, &mut self.search_box, &mut self.ui_context);
        }

        // Position sidebar and switcher below the titlebar
        let mut dummy_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.menubar, 0.0, self.header_height, self.sidebar_width, logical_sh - self.header_height - self.status_height, &mut self.ui_context);
        let switcher_h = if self.search_open {
            logical_sh - self.header_height - 42.0 - self.status_height
        } else {
            logical_sh - self.header_height - self.status_height
        };
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.switcher, self.sidebar_width, self.header_height, logical_sw - self.sidebar_width, switcher_h, &mut self.ui_context);
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.statusbar, 0.0, logical_sh - self.status_height, logical_sw, self.status_height, &mut self.ui_context);
        // Render root window recursively
        let mut window_pc = PageContent::new();
        cce_ui::layout::render_widget(&mut window_pc, &mut self.root_window, 0.0, 0.0, logical_sw, logical_sh, &mut self.ui_context);

        let mut search_pc = PageContent::new();
        if self.search_open {
            search_pc.rects.push((
                [0.08, 0.08, 0.12, 1.0],
                self.sidebar_width,
                sh - 42.0,
                sw - self.sidebar_width,
                42.0,
                0.0,
                (false, false, false, false),
            ));
            search_pc.rects.push((
                [0.18, 0.18, 0.24, 1.0],
                self.sidebar_width,
                sh - 42.0,
                sw - self.sidebar_width,
                1.0,
                0.0,
                (false, false, false, false),
            ));
            cce_ui::layout::render_widget(
                &mut search_pc,
                &mut self.search_box,
                self.sidebar_width + 12.0,
                sh - 36.0,
                sw - self.sidebar_width - 24.0,
                30.0,
                &mut self.ui_context,
            );
        }

        // CSD Titlebar removed

        eprintln!("WINDOW_PC_RECTS_LEN: {}", window_pc.rects.len());
        for pc_part in &[window_pc] {
            for (idx, (c, x, y, w, h, r, corners)) in pc_part.rects.iter().enumerate() {
                eprintln!("WINDOW_PC_RECT idx={}: color={:?}, x={}, y={}, w={}, h={}", idx, c, x, y, w, h);
                let wx = *x * s;
                let wy = *y * s;
                let ww = *w * s;
                let wh = *h * s;
                widgets.push(AppWidget {
                    x: wx, y: wy, w: ww, h: wh,
                    color: *c, hover_color: *c,
                    hovering: check_hover(wx, wy, ww, wh),
                    radius: *r * s,
                    corners: *corners,
                });
            }
            for (idx, (t, size, x, y, tc, font_opt, bounds)) in pc_part.texts.iter().enumerate() {
                eprintln!("WINDOW_PC_TEXT idx={}: text='{}', size={}, x={}, y={}, bounds={:?}", idx, t, size, x, y, bounds);
                text_items.push(TextItem {
                    buffer: make_text_buffer_with_font(
                        &mut self.font_system,
                        t,
                        *size,
                        font_opt.as_deref(),
                        &self.sans_serif_family,
                        &self.serif_family,
                        &self.monospace_family,
                    ),
                    x: *x, y: *y,
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

        for (idx, (c, x, y, w, h, r, corners)) in pc.rects.iter().enumerate() {
            eprintln!("PAGE_PC_RECT idx={}: color={:?}, x={}, y={}, w={}, h={}", idx, c, x, y, w, h);
            let wx = *x * s;
            let wy = (*y - scroll_offset_y) * s;
            let ww = *w * s;
            let wh = *h * s;
            widgets.push(AppWidget {
                x: wx, y: wy, w: ww, h: wh,
                color: *c, hover_color: *c,
                hovering: check_hover(wx, wy, ww, wh),
                radius: *r * s,
                corners: *corners,
            });
        }
        for (idx, (t, size, x, y, tc, font_opt, bounds)) in pc.texts.iter().enumerate() {
            eprintln!("PAGE_PC_TEXT idx={}: text='{}', size={}, x={}, y={}, bounds={:?}", idx, t, size, x, y, bounds);
            let shifted_bounds = bounds.map(|[bl, bt, br, bb]| {
                [bl, bt - scroll_offset_y, br, bb - scroll_offset_y]
            });

            let matched = self.search_open && !self.search_query.is_empty() && t.to_lowercase().contains(&self.search_query.to_lowercase());

            if matched {
                let text_buf = make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size,
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

                let wx = rect_x * s;
                let wy = (rect_y - scroll_offset_y) * s;
                let ww = rect_w * s;
                let wh = rect_h * s;
                widgets.push(AppWidget {
                    x: wx,
                    y: wy,
                    w: ww,
                    h: wh,
                    color: [0.65, 0.45, 0.05, 0.4],
                    hover_color: [0.65, 0.45, 0.05, 0.4],
                    hovering: check_hover(wx, wy, ww, wh),
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
                    *size,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x, y: *y - scroll_offset_y,
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
            let wx = base.x * s;
            let wy = (base.y - scroll_offset_y) * s;
            let ww = base.w * s;
            let wh = base.h * s;
            widgets.push(AppWidget {
                x: wx, y: wy, w: ww, h: wh,
                color: bg, hover_color: hover_bg,
                hovering: check_hover(wx, wy, ww, wh),
                radius: cce_ui::layout::button_corner_radius() * s,
                corners: (true, true, true, true),
            });
            let label = base.label.as_deref().unwrap_or("");
            let label_size = 12.0;
            let buf = make_text_buffer_with_font(
                &mut self.font_system,
                label,
                label_size,
                None,
                &self.sans_serif_family,
                &self.serif_family,
                &self.monospace_family,
            );
            let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
            let lh = buf.metrics().line_height;
            let mut left_align = btn.justify == cce_ui::widget::Justification::Left;

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

            let logical_tw = tw / s;
            let logical_lh = lh / s;
            let text_x = if left_align {
                base.x + 8.0
            } else {
                base.x + (base.w - logical_tw) / 2.0
            };

            let label_color = btn.label_color.unwrap_or([0.83, 0.83, 0.83, 1.0]);
            text_items.push(TextItem {
                buffer: buf,
                x: text_x, y: (base.y - scroll_offset_y) + (base.h - logical_lh) / 2.0,
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
            let wx = *x * s;
            let wy = (*y - scroll_offset_y) * s;
            let ww = *w * s;
            let wh = *h * s;
            widgets.push(AppWidget {
                x: wx, y: wy, w: ww, h: wh,
                color: *c, hover_color: *c,
                hovering: check_hover(wx, wy, ww, wh),
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
                    *size,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x, y: *y - scroll_offset_y,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
                bounds: shifted_bounds,
            });
        }







        for (c, x, y, w, h, r, corners) in &search_pc.rects {
            let wx = *x * s;
            let wy = *y * s;
            let ww = *w * s;
            let wh = *h * s;
            widgets.push(AppWidget {
                x: wx, y: wy, w: ww, h: wh,
                color: *c, hover_color: *c,
                hovering: check_hover(wx, wy, ww, wh),
                radius: *r * s,
                corners: *corners,
            });
        }
        for (t, size, x, y, tc, font_opt, bounds) in &search_pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x, y: *y,
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

