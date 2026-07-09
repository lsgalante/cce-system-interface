use crate::{SystemInterface, AppWidget, make_text_buffer_with_font};
use cce_settings::app::PageContent;
use cce_settings::pages::Page;
use cce_ui::widget::Element;

type RectTuple = ([f32; 4], f32, f32, f32, f32, f32, (bool, bool, bool, bool));
type TextTuple = (String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>);

/// One child's contribution to the dissolved root's window assembly, replicating the
/// legacy `render_widget(root Backplate)` aggregate exactly: plain quads are skipped
/// when they are a rounded child's own bg (the rounded pass carries it), clipped to
/// the window, and corner-resolved against the root's rounded rect (a quad flush with
/// a window corner picks up the plate radius there); rounded quads are clipped;
/// text bounds are clamped to the window (unbounded labels become window-bounded).
fn collect_window_child(
    w: &dyn Element,
    ctx: &cce_ui::context::UiContext,
    win_w: f32,
    win_h: f32,
    plate_radius: f32,
    plain: &mut Vec<RectTuple>,
    rounded: &mut Vec<RectTuple>,
    texts: &mut Vec<TextTuple>,
) {
    let (cx, cy, cw, ch) = w.rect();
    let child_rounded = w.rounded_corners() != (false, false, false, false);
    for (qx, qy, qw, qh, qc) in w.all_quads(ctx) {
        if child_rounded && (qx - cx).abs() < 0.1 && (qy - cy).abs() < 0.1 && (qw - cw).abs() < 0.1 && (qh - ch).abs() < 0.1 {
            continue;
        }
        let x0 = qx.max(0.0);
        let y0 = qy.max(0.0);
        let x1 = (qx + qw).min(win_w);
        let y1 = (qy + qh).min(win_h);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let corners = (
            x0 <= 1.5 && y0 <= 1.5,
            x1 >= win_w - 1.5 && y0 <= 1.5,
            x1 >= win_w - 1.5 && y1 >= win_h - 1.5,
            x0 <= 1.5 && y1 >= win_h - 1.5,
        );
        if corners == (false, false, false, false) {
            plain.push((qc, x0, y0, x1 - x0, y1 - y0, 0.0, (false, false, false, false)));
        } else {
            plain.push((qc, x0, y0, x1 - x0, y1 - y0, plate_radius, corners));
        }
    }
    for (qx, qy, qw, qh, qr, qc, qcorners) in w.all_rounded_quads(ctx) {
        let x0 = qx.max(0.0);
        let y0 = qy.max(0.0);
        let x1 = (qx + qw).min(win_w);
        let y1 = (qy + qh).min(win_h);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        rounded.push((qc, x0, y0, x1 - x0, y1 - y0, qr, qcorners));
    }
    for (label, font, bounds) in w.text_labels_with_font_and_bounds(ctx) {
        let cb = match bounds {
            Some(b) => {
                let bx0 = b[0].max(0.0);
                let by0 = b[1].max(0.0);
                let bx1 = b[2].min(win_w);
                let by1 = b[3].min(win_h);
                if bx1 <= bx0 || by1 <= by0 {
                    continue;
                }
                Some([bx0, by0, bx1, by1])
            }
            None => Some([0.0, 0.0, win_w, win_h]),
        };
        let color = [
            label.color[0] as f32 / 255.0,
            label.color[1] as f32 / 255.0,
            label.color[2] as f32 / 255.0,
            1.0,
        ];
        texts.push((label.text.clone(), label.font_size, label.x, label.y, color, font, cb));
    }
}

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

        self.sidebar_width = 0.0;
        self.header_height = 0.0; // No CSD Titlebar
        let mut widgets = Vec::new();
        let mut texts = Vec::new();
        let mut page_buttons = Vec::new();

        cce_ui::widget::hover_animation::reset_frame_registration();
        cce_ui::widget::popovers::clear();
        self.ui_context.clear_popovers();
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);
        cce_ui::widget::hover_animation::set_cursor_pos(self.cursor_x, self.cursor_y);

        let s = 1.0f32;
        let cursor_phys_x = self.cursor_x;
        let cursor_phys_y = self.cursor_y;
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
        self.page_dropdown.selected = page_idx;
        self.switcher.set_active_index(Some(page_idx));

        // Root Backplate DISSOLVED (Phase 6s): top-level widgets stay parentless
        // (render_widget registers them); the window plate, the root aggregate's
        // emission order, and the StatusBar's Backplate-coupled theming are all
        // replicated by hand below.

        // Position sidebar and switcher below the titlebar
        let mut dummy_pc = PageContent::new();
        let dropdown_h = 18.0f32;
        let size = self.page_dropdown.measure(
            cce_ui::widget::LayoutConstraints::new(0.0, 500.0, dropdown_h, dropdown_h),
            &self.ui_context,
        );
        let dropdown_w = size.width;
        let dropdown_gap = (self.status_height - dropdown_h) / 2.0;
        let dropdown_x = logical_sw - dropdown_w - dropdown_gap;
        let dropdown_y = logical_sh - self.status_height + dropdown_gap;
        // The dropdown sits flush against the window's rounded bottom-right corner; with the
        // root Backplate dissolved, hand it the plate frame for its concentric-corner cut.
        self.page_dropdown.set_corner_frame(Some(((0.0, 0.0, logical_sw, logical_sh), 12.0, (true, true, true, true))));
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.page_dropdown, dropdown_x, dropdown_y, dropdown_w, dropdown_h, &mut self.ui_context);
        let switcher_h = if self.search_open {
            logical_sh - self.header_height - 42.0 - self.status_height
        } else {
            logical_sh - self.header_height - self.status_height
        };
        cce_ui::layout::render_widget(&mut dummy_pc, &mut self.switcher, self.sidebar_width, self.header_height, logical_sw - self.sidebar_width, switcher_h, &mut self.ui_context);

        // Assemble the window exactly as the legacy `render_widget(root Backplate)`
        // aggregate did: every child plain quad (clipped to the window, with the root's
        // corner resolution against its rounded rect), then the translucent window
        // plate, then every child rounded quad, then the root-clamped text — in the old
        // child order [switcher, statusbar, dropdown, search box]. The StatusBar widget
        // is dissolved outright: its theming was Backplate-parent-coupled (statusbar
        // theme color falling back to STATUS_BG, bottom corners rounded at the root's
        // radius, statusbar text color/font), replicated here as tuples.
        let plate_radius = 12.0f32;
        let mut window_pc = PageContent::new();
        {
            let mut plain: Vec<RectTuple> = Vec::new();
            let mut rounded: Vec<RectTuple> = Vec::new();
            let mut wtexts: Vec<TextTuple> = Vec::new();

            collect_window_child(&self.switcher, &self.ui_context, logical_sw, logical_sh, plate_radius, &mut plain, &mut rounded, &mut wtexts);

            // The dissolved status bar's slot in the child order.
            let sb_y = logical_sh - self.status_height;
            let sb_theme = cce_ui::colors::backplate_statusbar_color();
            let sb_bg = if sb_theme[3] > 0.001 { sb_theme } else { cce_ui::color::STATUS_BG };
            rounded.push((sb_bg, 0.0, sb_y, logical_sw, self.status_height, plate_radius, (false, false, true, true)));
            if !self.status_text.is_empty() {
                let (_, sb_font_size) = cce_ui::layout::statusbar_font_parsed();
                let sb_size = if sb_font_size > 0.0 { sb_font_size } else { 12.0 };
                let c = cce_ui::colors::backplate_statusbar_text_color();
                // Same u8 round-trip the legacy TextLabel path applied.
                let sb_color = [
                    ((c[0] * 255.0) as u8) as f32 / 255.0,
                    ((c[1] * 255.0) as u8) as f32 / 255.0,
                    ((c[2] * 255.0) as u8) as f32 / 255.0,
                    1.0,
                ];
                let sb_text_y = cce_ui::layout::align_text_y(sb_y, self.status_height, sb_size, 0.0);
                wtexts.push((self.status_text.clone(), sb_size, 12.0, sb_text_y, sb_color, None, Some([0.0, 0.0, logical_sw, logical_sh])));
            }

            collect_window_child(&self.page_dropdown, &self.ui_context, logical_sw, logical_sh, plate_radius, &mut plain, &mut rounded, &mut wtexts);
            if self.search_open {
                collect_window_child(&self.search_box, &self.ui_context, logical_sw, logical_sh, plate_radius, &mut plain, &mut rounded, &mut wtexts);
            }

            window_pc.rects.extend(plain);
            let mut plate = cce_ui::color::to_linear([0x0a as f32 / 255.0, 0x1a as f32 / 255.0, 0x0e as f32 / 255.0, 1.0]);
            if plate[3] > 0.001 {
                plate[3] = cce_ui::color::active_backplate_opacity();
            }
            window_pc.rects.push((plate, 0.0, 0.0, logical_sw, logical_sh, plate_radius, (true, true, true, true)));
            window_pc.rects.extend(rounded);
            window_pc.texts.extend(wtexts);
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        let active_page_widget = &self.pages[page_idx];
        if self.app.current_page == Page::System {
            self.scroll_y = active_page_widget.scroll_y;
            self.max_scroll_y = (active_page_widget.content_h - active_page_widget.base.base.h).max(0.0);
        }

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
                texts.push((t.clone(), *size, *x, *y, *tc, font_opt.clone(), *bounds));
            }
        }

        self.scrollable_widgets_start_idx = widgets.len();
        self.scrollable_text_items_start_idx = texts.len();
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
            let mut wy = (*y - scroll_offset_y) * s;
            let ww = *w * s;
            let mut wh = *h * s;

            let viewport_bottom = logical_sh - self.status_height;
            if wy >= viewport_bottom || wy + wh <= 0.0 {
                continue;
            }
            if wy < 0.0 {
                let diff = 0.0 - wy;
                wy = 0.0;
                wh = (wh - diff).max(0.0);
            }
            if wy + wh > viewport_bottom {
                wh = (viewport_bottom - wy).max(0.0);
            }

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

            let viewport_bottom = logical_sh - self.status_height;
            let final_bounds = match shifted_bounds {
                Some(b) => Some([
                    b[0],
                    b[1].max(0.0),
                    b[2],
                    b[3].min(viewport_bottom),
                ]),
                None => Some([
                    0.0,
                    0.0,
                    logical_sw,
                    viewport_bottom,
                ]),
            };

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
                let mut wy = (rect_y - scroll_offset_y) * s;
                let ww = rect_w * s;
                let mut wh = rect_h * s;

                if wy < viewport_bottom && wy + wh > 0.0 {
                    if wy < 0.0 {
                        let diff = 0.0 - wy;
                        wy = 0.0;
                        wh = (wh - diff).max(0.0);
                    }
                    if wy + wh > viewport_bottom {
                        wh = (viewport_bottom - wy).max(0.0);
                    }
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

            texts.push((t.clone(), *size, *x, *y - scroll_offset_y, text_color, font_opt.clone(), final_bounds));
        }
        for (btn, action) in &pc.buttons {
            let base = btn.base().unwrap();
            let bg = btn.bg.unwrap_or([0.16, 0.16, 0.24, 1.0]);
            let hover_bg = btn.hover_bg.unwrap_or([0.25, 0.30, 0.26, 1.0]);
            let wx = base.x * s;
            let mut wy = (base.y - scroll_offset_y) * s;
            let ww = base.w * s;
            let mut wh = base.h * s;

            let viewport_bottom = logical_sh - self.status_height;
            if wy >= viewport_bottom || wy + wh <= 0.0 {
                continue;
            }
            if wy < 0.0 {
                let diff = 0.0 - wy;
                wy = 0.0;
                wh = (wh - diff).max(0.0);
            }
            if wy + wh > viewport_bottom {
                wh = (viewport_bottom - wy).max(0.0);
            }

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
                 btn.widget_font().as_deref(),
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

            let scale_factor = cce_ui::scale::scale_factor();
            let logical_tw = tw / scale_factor;
            let logical_lh = lh / scale_factor;
            let text_x = if left_align {
                base.x + 8.0
            } else {
                base.x + (base.w - logical_tw) / 2.0
            };

            let label_color = btn.label_color.unwrap_or([0.83, 0.83, 0.83, 1.0]);
            let button_bounds = Some([
                0.0,
                0.0,
                logical_sw,
                viewport_bottom,
            ]);
            texts.push((
                label.to_string(),
                label_size,
                text_x,
                (base.y - scroll_offset_y) + (base.h - logical_lh) / 2.0,
                label_color,
                btn.widget_font(),
                button_bounds,
            ));
            let mut btn_clone = btn.clone();
            if let Some(base_mut) = btn_clone.base_mut() {
                base_mut.x *= s;
                base_mut.y = (base_mut.y - scroll_offset_y) * s;
                base_mut.w *= s;
                base_mut.h *= s;
            }
            page_buttons.push((btn_clone, action.clone()));
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
            texts.push((t.clone(), *size, *x, *y, *tc, font_opt.clone(), *bounds));
        }

        // Popovers + context menu draw INTO the frame (Phase 6t): the same collector the
        // engine's xdg popup used, but emitted on top of the whole window — the dropdown's
        // open-upward popover finally renders where it hit-tests. Kept out of
        // widgets/texts so the wheel fast-path can't scroll them; dl-text occlusion comes
        // from the ui_context popover registration (and the engine's context-menu overlay
        // rect under draws_own_popovers).
        let mut popover_pc = PageContent::new();
        {
            // Chrome popover (the page dropdown) is in window coords; page-widget popovers
            // (notifications/fonts menus) are in page coords and shift with the viewport —
            // the same scroll subtraction the old popup positioner applied at creation.
            let chrome_ptr = self.page_dropdown.as_ptr();
            let mut page_pop_pc = PageContent::new();
            for popover_ptr in &self.ui_context.active_popovers {
                unsafe {
                    if std::ptr::addr_eq(*popover_ptr, chrome_ptr) {
                        (**popover_ptr).render_popover(&mut popover_pc);
                    } else {
                        (**popover_ptr).render_popover(&mut page_pop_pc);
                    }
                }
            }
            for (c, x, y, w, h, r, corners) in page_pop_pc.rects {
                popover_pc.rects.push((c, x, y - self.scroll_y, w, h, r, corners));
            }
            for (t, size, x, y, tc, font, bounds) in page_pop_pc.texts {
                let shifted = bounds.map(|[l, tb, rr, b]| [l, tb - self.scroll_y, rr, b - self.scroll_y]);
                popover_pc.texts.push((t, size, x, y - self.scroll_y, tc, font, shifted));
            }
        }
        if cce_ui::widget::context_menu::is_visible() {
            use cce_ui::layout::RenderTarget;
            let cx = cce_ui::widget::context_menu::x();
            let cy = cce_ui::widget::context_menu::y();
            let cw = cce_ui::widget::context_menu::w();
            let ch = cce_ui::widget::context_menu::h();

            popover_pc.rect([0.22, 0.22, 0.28, 1.0], cx, cy, cw, ch);
            popover_pc.rect([0.06, 0.06, 0.09, 1.0], cx + 1.0, cy + 1.0, cw - 2.0, ch - 2.0);

            if let Some(h_idx) = cce_ui::widget::context_menu::hovered_item() {
                let iy = cy + h_idx as f32 * 24.0;
                popover_pc.rect([0.20, 0.40, 0.65, 0.6], cx + 2.0, iy + 2.0, cw - 4.0, 20.0);
            }

            for (idx, opt) in cce_ui::widget::context_menu::options().iter().enumerate() {
                let iy = cy + idx as f32 * 24.0 + (24.0 - 12.0) / 2.0;
                let text_color = if idx == 0 {
                    [0.44, 0.44, 0.47, 1.0]
                } else if cce_ui::widget::context_menu::hovered_item() == Some(idx) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.80, 0.80, 0.83, 1.0]
                };
                popover_pc.text_with_bounds(opt, cx + 8.0, iy, 12.0, text_color, Some([cx, cy, cx + cw, cy + ch]));
            }
        }
        self.popover_widgets = popover_pc.rects.iter().map(|(c, x, y, w, h, r, corners)| AppWidget {
            x: *x, y: *y, w: *w, h: *h,
            color: *c, hover_color: *c,
            hovering: false,
            radius: *r,
            corners: *corners,
        }).collect();
        self.popover_texts = popover_pc.texts;

        self.widgets = widgets;
        self.texts = texts;
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

