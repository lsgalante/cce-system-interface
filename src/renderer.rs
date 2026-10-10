use cce_ui::widget::WidgetHostExt;
use crate::{SystemInterface, AppWidget, make_text_buffer_with_font};
use cce_settings::app::{ControlCarve, PageContent};
use cce_settings::pages::Page;
use cce_ui::widget::WidgetHost;


impl SystemInterface {

    /// Lay the page out and flatten it into the cached frame, keeping what
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
        let mut widgets = Vec::new();
        let mut texts = Vec::new();
        let mut page_buttons = Vec::new();
        let mut page_icon_images: Vec<crate::PlacedIcon> = Vec::new();

        cce_ui::widget::hover_animation::reset_frame_registration();
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
        self.ui_context[self.page_dropdown].selected = page_idx;

        // root plate container DISSOLVED (Phase 6s): top-level widgets stay parentless
        // (render_widget registers them); the window plate, the root aggregate's
        // emission order, and the StatusBar's root plate container-coupled theming are all
        // replicated by hand below.

        // Position sidebar and switcher below the titlebar
        let mut dummy_pc = PageContent::new();
        let dropdown_h = cce_ui::layout::dropdown_height();
        let size = self.ui_context[self.page_dropdown].measure(
            cce_ui::widget::LayoutConstraints::new(0.0, 500.0, dropdown_h, dropdown_h),
            &self.ui_context,
        );
        let dropdown_w = size.width;
        let dropdown_gap = (self.status_height - dropdown_h) / 2.0;
        let dropdown_x = logical_sw - dropdown_w - dropdown_gap;
        let dropdown_y = logical_sh - self.status_height + dropdown_gap;
        // The dropdown sits flush against the window's rounded bottom-right corner; with the
        // root plate container dissolved, hand it the plate frame for its concentric-corner cut.
        self.ui_context[self.page_dropdown].set_corner_frame(Some(((0.0, 0.0, logical_sw, logical_sh), 12.0, (true, true, true, true))));
        cce_ui::compose::render_widget_h(&mut dummy_pc, self.page_dropdown, dropdown_x, dropdown_y, dropdown_w, dropdown_h, &mut self.ui_context);
        // The dropdown is laid out by this pass and painted live in display_list
        // (`paint_root_into`), chevron and all.
        let switcher_h = if self.search_open {
            logical_sh - self.header_height - 42.0 - self.status_height
        } else {
            logical_sh - self.header_height - self.status_height
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

        // The window chrome — the page dropdown and, while open, the search box — is
        // painted live in display_list, each as it paints itself (`paint_root_into`).
        // Until 2026-10-08 it was flattened here through the legacy tuple views (plain
        // quads, rounded quads, then the text walk) into the cached frame, which drew
        // the dropdown without its relief and the search box twice.

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
            // The search box stands on the root plate: the window-edge inset.
            // The toolkit's textbox height, centred in the 42px band.
            let inset = cce_ui::layout::root_plate_inset();
            let box_h = cce_ui::layout::textbox_height();
            // Laid out here; painted live in display_list.
            let mut layout_only = PageContent::new();
            cce_ui::compose::render_widget_h(
                &mut layout_only,
                self.search_box,
                self.sidebar_width + inset,
                sh - 42.0 + (42.0 - box_h) / 2.0,
                sw - self.sidebar_width - 2.0 * inset,
                box_h,
                &mut self.ui_context,
            );
        }

        // CSD Titlebar removed

        self.scrollable_widgets_start_idx = widgets.len();
        self.scrollable_text_items_start_idx = texts.len();
        self.scrollable_buttons_start_idx = page_buttons.len();

        // Page content in LOGICAL coordinates, then scale to physical
        let pc = self.render_page_content(lcx, lcy, lcw, lch);
        self.page_reliefs = pc.reliefs.clone();
        self.page_control_reliefs = pc.control_reliefs.iter().map(|&c| (c, None)).collect();
        self.page_control_relief_marks.clear();



        if self.search_open && !self.search_query.is_empty() && !self.ui_context[self.page_scroll_bar].dragging {
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
                for (btn, _, _) in &pc.buttons {
                    let base = btn.base();
                    max_y = max_y.max(base.y + base.h);
                }
                let local_max_scroll_y = (max_y - lch).max(0.0);
                self.scroll_y = (y - 100.0).clamp(0.0, local_max_scroll_y);
            }
        }

        let mut max_y = 0.0f32;
        for (_, _, y, _, h, _, _) in &pc.rects {
            max_y = max_y.max(y + h);
        }
        for (_, size, _, y, _, _, _) in &pc.texts {
            max_y = max_y.max(y + size);
        }
        for (btn, _, _) in &pc.buttons {
            let base = btn.base();
            max_y = max_y.max(base.y + base.h);
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

        let scroll_offset_y = self.scroll_y;

        // Where each page rect landed in `widgets` (culled rects collapse onto
        // the next survivor), so a carve's mark — "the rect I was claimed
        // before" — survives the cull. One extra entry for "after the last".
        let mut rect_widget_index: Vec<usize> = Vec::with_capacity(pc.rects.len() + 1);
        for (c, x, y, w, h, r, corners) in pc.rects.iter() {
            rect_widget_index.push(widgets.len());
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
        rect_widget_index.push(widgets.len());
        for &mark in &pc.control_relief_marks {
            self.page_control_relief_marks.push(rect_widget_index[mark.min(rect_widget_index.len() - 1)]);
        }
        for (t, size, x, y, tc, font_opt, bounds) in pc.texts.iter() {
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
        for (btn, action, clip) in &pc.buttons {
            let base = btn.base();
            // The button's own idle and hover faces: the page's colours when
            // it passed them, else the toolkit's for the button's kind — a
            // list row's transparent-until-hover wash (`PageContent::list_row`).
            let mut probe = (**btn).clone();
            probe.set_hovered(false);
            let bg = cce_ui::widget::Paint::color(&probe);
            probe.set_hovered(true);
            let hover_bg = cce_ui::widget::Paint::color(&probe);
            // Clamp to the emission-time clip rect (page coords) so a partially
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
            let wx = px0 * s;
            let mut wy = (py0 - scroll_offset_y) * s;
            let ww = (px1 - px0) * s;
            let mut wh = (py1 - py0) * s;

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
            // Buttons never reach the toolkit's flat-path bridge here: this
            // app collects them into `pc.buttons` and draws them itself, so
            // `render_widget` — where a widget's carves are offered — never
            // sees one. Ask each button for the inset face its own `paint`
            // draws (`Button::inset_face`, the same source the drawn one
            // reads) and carve it right after the button's quad, where its
            // paint puts it — the mark is "before the next widget". Page
            // coords, pre-scroll, like everything else in this list.
            if cce_ui::layout::control_relief() {
                let rect = cce_ui::scene::layout::Rect { x: base.x, y: base.y, width: base.w, height: base.h };
                if let Some(mut plate) = btn.plate(rect) {
                    // This pass's clone of the button a Tab step focused (its
                    // registered rect, window coords, matches `refocus_rect`;
                    // the view-pass tail hands it the focus): light its rim
                    // now, since the carve is collected here, before that.
                    let near = |a: f32, b: f32| (a - b).abs() < 0.5;
                    let refocused = self
                        .refocus_rect
                        .is_some_and(|(fx, fy, fw, fh)| near(wx, fx) && near(wy, fy) && near(ww, fw) && near(wh, fh));
                    if refocused {
                        plate.tint = Some(cce_ui::widget::ControlPlate::focus_tint());
                    }
                    // Under the button's own clip, as its quad is clamped:
                    // drawn whole, a row half-scrolled out of a list kept
                    // its plate standing past the list's edge.
                    let carve = ControlCarve::Plate {
                        x: plate.rect.x,
                        y: plate.rect.y,
                        w: plate.rect.width,
                        h: plate.rect.height,
                        radius: plate.radii.0,
                        depth: plate.depth,
                        color: plate.face_fill(),
                        tint: plate.tint,
                    };
                    self.page_control_reliefs.push((carve, *clip));
                    self.page_control_relief_marks.push(widgets.len());
                }
            }
            // An icon face replaces the label entirely (as it does in
            // `Button::paint`). The rect comes from the button's own
            // `icon_rect` so the glyph lands where the paint path would put
            // it, and it keeps the button's clip: the page clip alone let a
            // row half-scrolled out of a list show its whole glyph past the
            // list's edge.
            let has_icon = if let Some((image, irect, alpha)) =
                btn.icon_rect(cce_ui::scene::layout::Rect {
                    x: base.x,
                    y: base.y - scroll_offset_y,
                    width: base.w,
                    height: base.h,
                })
            {
                page_icon_images.push(crate::PlacedIcon {
                    image,
                    x: irect.x,
                    y: irect.y,
                    w: irect.width,
                    h: irect.height,
                    alpha,
                    clip: clip.map(|[x, y, w, h]| [x, y - scroll_offset_y, w, h]),
                });
                true
            } else {
                false
            };

            // The label is skipped for an icon face, but NOT the dispatch clone
            // below it: an icon button still has to be clickable.
            if !has_icon {
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
            // Label bounds: the button's own (clip-clamped) box — a label longer
            // than its button truncates instead of spilling over the neighbor.
            let button_bounds = Some([
                wx,
                wy.max(0.0),
                (wx + ww).min(logical_sw),
                (wy + wh).min(viewport_bottom),
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
            }
            let mut btn_clone = btn.clone();
            {
                // The dispatch clone hit-tests at the CLAMPED rect, so clicks in
                // a row's clipped-away region fall through to what's visible there.
                let base_mut = btn_clone.base_mut();
                base_mut.x = wx;
                base_mut.y = wy;
                base_mut.w = ww;
                base_mut.h = wh;
            }
            page_buttons.push((btn_clone, action.clone()));
        }

        // The page's own glyphs (`PageContent::icon`): shifted by the scroll
        // as the texts are, cut at the clip they were placed under (a list's
        // box) by display_list, and dimmed with the text a search leaves
        // unmatched.
        let searching = self.search_open && !self.search_query.is_empty();
        for ic in &pc.icons {
            page_icon_images.push(crate::PlacedIcon {
                image: ic.image,
                x: ic.x,
                y: ic.y - scroll_offset_y,
                w: ic.w,
                h: ic.h,
                alpha: if searching { ic.alpha * 0.25 } else { ic.alpha },
                clip: ic.clip.map(|[x, y, w, h]| [x, y - scroll_offset_y, w, h]),
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
            texts.push((t.clone(), *size, *x, *y, *tc, font_opt.clone(), *bounds));
        }

        // Popovers + context menu draw INTO the frame (Phase 6t; the engine xdg popup no
        // longer exists) — emitted on top of the whole window, so the dropdown's
        // open-upward popover renders where it hit-tests. Kept out of
        // widgets/texts so the wheel fast-path can't scroll them; dl-text occlusion comes
        // from the ui_context popover registration (and the engine's context-menu overlay
        // rect).
        let mut popover_pc = PageContent::new();
        {
            // Chrome popover (the page dropdown) is in window coords; page-widget popovers
            // (notifications/fonts menus) are in page coords and shift with the viewport —
            // the same scroll subtraction the old popup positioner applied at creation.
            let chrome_id = self.page_dropdown.id();
            let mut page_pop_pc = PageContent::new();
            for &pop_id in &self.ui_context.active_popovers {
                let Some(popover) = self.ui_context.get_widget(pop_id) else { continue };
                if pop_id == chrome_id {
                    popover.render_popover(&mut popover_pc);
                } else {
                    popover.render_popover(&mut page_pop_pc);
                }
            }
            // The chrome popover's rects are already in place: the page
            // carves' marks shift past them so the interleave stays true.
            let rect_base = popover_pc.rects.len();
            for (c, x, y, w, h, r, corners) in page_pop_pc.rects {
                popover_pc.rects.push((c, x, y - self.scroll_y, w, h, r, corners));
            }
            for (t, size, x, y, tc, font, bounds) in page_pop_pc.texts {
                let shifted = bounds.map(|[l, tb, rr, b]| [l, tb - self.scroll_y, rr, b - self.scroll_y]);
                popover_pc.texts.push((t, size, x, y - self.scroll_y, tc, font, shifted));
            }
            for ic in page_pop_pc.icons {
                popover_pc.icons.push(cce_settings::app::PageIcon {
                    y: ic.y - self.scroll_y,
                    clip: ic.clip.map(|[x, y, w, h]| [x, y - self.scroll_y, w, h]),
                    ..ic
                });
            }
            for (carve, mark) in page_pop_pc.control_reliefs.into_iter().zip(page_pop_pc.control_relief_marks) {
                popover_pc.control_reliefs.push(carve.shifted_y(-self.scroll_y));
                popover_pc.control_relief_marks.push(rect_base + mark);
            }
        }
        // The context menu is NOT collected here: display_list paints it
        // straight into the frame with `context_menu::paint_with_labels`,
        // the toolkit's lit plate and its labels in the menu font — and its
        // marks (✓ ● ○) and page chevrons as cce-icons glyphs, which the
        // bare `text_labels()` this used to loop over cannot draw.
        self.popover_widgets = popover_pc.rects.iter().map(|(c, x, y, w, h, r, corners)| AppWidget {
            x: *x, y: *y, w: *w, h: *h,
            color: *c, hover_color: *c,
            hovering: false,
            radius: *r,
            corners: *corners,
        }).collect();
        self.popover_texts = popover_pc.texts;
        self.popover_icon_images = popover_pc
            .icons
            .iter()
            .map(|ic| crate::PlacedIcon { image: ic.image, x: ic.x, y: ic.y, w: ic.w, h: ic.h, alpha: ic.alpha, clip: ic.clip })
            .collect();
        self.popover_control_reliefs = popover_pc.control_reliefs;
        self.popover_control_relief_marks = popover_pc.control_relief_marks;

        self.widgets = widgets;
        self.texts = texts;
        // The dispatch copies are the context's: last frame's go back, this frame's go in.
        for (h, _) in std::mem::take(&mut self.page_buttons) {
            self.ui_context.remove(h);
        }
        let ctx = &mut self.ui_context;
        self.page_buttons = page_buttons.into_iter().map(|(b, a)| (ctx.insert(b), a)).collect();
        self.page_icon_images = page_icon_images;

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
        self.last_scroll_y = self.scroll_y;
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

