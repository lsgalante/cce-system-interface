use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{DisplayList, PaintCtx, Prim};
use cce_ui::widget::WidgetHostExt;
use crate::{SystemInterface, make_text_buffer_with_font};
use cce_settings::app::PageContent;
use cce_settings::pages::Page;
use cce_ui::widget::WidgetHost;

/// What a rebuild leaves for `display_list` to emit, every frame, until the
/// next one. The page is in PAGE coordinates and goes out under the scroll
/// offset (`PaintCtx::append_translated`), so the wheel's fast path moves the
/// page by changing `scroll_y` alone; the popovers are in window coordinates,
/// placed at the rebuild.
#[derive(Default)]
pub(crate) struct CachedFrame {
    /// The page in the order it was painted: each section's title, content
    /// and well, the page buttons where the page declared them (the search
    /// dims or lights the page's own text and glyphs, not theirs), then the
    /// search's highlights.
    pub page: DisplayList,
    /// The open popovers — the page dropdown's, then the page widgets' — in
    /// the order each drew itself.
    pub popovers: DisplayList,
}

impl SystemInterface {

    /// Lay the page out and record it into the cached frame, keeping what
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
        // A page button the page declares differently this rebuild (a tab
        // whose face follows the selection, a list row now naming another
        // item) is a new widget, and keyboard focus on the old one would die
        // with it. Remember where the focused widget sat: if the focus is
        // lost, the pass hands it to whatever stands there now, before the
        // buttons are painted. A widget that lives on keeps it.
        self.refocus_rect = self
            .ui_context
            .focused_widget
            .and_then(|id| self.ui_context.get_widget(id))
            .map(|w| w.rect());
        // SectionContainer dissolved (Phase 6w): no per-rebuild section clones to
        // relink — the page's widgets dispatch directly (an open popover is
        // registered as `compose::place` lays its widget out, in the view pass below).
        self.ui_context.clear_hierarchy();

        self.sidebar_width = 0.0;
        self.header_height = 0.0; // No CSD Titlebar

        self.ui_context.clear_popovers();
        // The page scroll, where the toolkit's popover and context-menu hit
        // tests (and a TreeList's) still read it.
        cce_ui::widget::hover_animation::set_scroll_offset(self.scroll_y);

        let cursor = (self.cursor_x, self.cursor_y);

        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw - self.sidebar_width;
        let mut lch = sh - self.header_height - self.status_height;
        if self.search_open {
            lch -= 42.0;
        }

        let page_idx = Page::ALL.iter().position(|&p| p == self.app.current_page).unwrap_or(0);
        self.ui_context[self.page_dropdown].selected = page_idx;

        // root plate container DISSOLVED (Phase 6s): top-level widgets stay parentless
        // (`place` registers their popovers); the window plate, the root aggregate's
        // emission order, and the StatusBar's root plate container-coupled theming are
        // all replicated by hand in display_list.

        // The page switcher, at the status bar's right end.
        let dropdown_h = cce_ui::layout::dropdown_height();
        let size = self.ui_context[self.page_dropdown].measure(
            cce_ui::widget::LayoutConstraints::new(0.0, 500.0, dropdown_h, dropdown_h),
            &self.ui_context,
        );
        let dropdown_w = size.width;
        let dropdown_gap = (self.status_height - dropdown_h) / 2.0;
        let dropdown_x = sw - dropdown_w - dropdown_gap;
        let dropdown_y = sh - self.status_height + dropdown_gap;
        // The dropdown sits flush against the window's rounded bottom-right corner; with the
        // root plate container dissolved, hand it the plate frame for its concentric-corner cut.
        self.ui_context[self.page_dropdown].set_corner_frame(Some(((0.0, 0.0, sw, sh), 12.0, (true, true, true, true))));
        // Laid out here (and its open menu registered); painted live in
        // display_list (`paint_root_into`), chevron and all.
        cce_ui::compose::place_h(self.page_dropdown, dropdown_x, dropdown_y, dropdown_w, dropdown_h, &mut self.ui_context);
        let switcher_h = if self.search_open {
            sh - self.header_height - 42.0 - self.status_height
        } else {
            sh - self.header_height - self.status_height
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

        // The search box, while open: laid out here in its band at the
        // window's foot, painted live in display_list. The search box stands
        // on the root plate: the window-edge inset, the toolkit's textbox
        // height centred in the 42px band.
        if self.search_open {
            let inset = cce_ui::layout::root_plate_inset();
            let box_h = cce_ui::layout::textbox_height();
            cce_ui::compose::place_h(
                self.search_box,
                self.sidebar_width + inset,
                sh - 42.0 + (42.0 - box_h) / 2.0,
                sw - self.sidebar_width - 2.0 * inset,
                box_h,
                &mut self.ui_context,
            );
        }

        // The page, in page coordinates (before the scroll).
        let page = self.render_page_content(lcx, lcy, lcw, lch).finish();
        let max_y = page.bottom();

        let searching = self.search_open && !self.search_query.is_empty();
        let query = self.search_query.to_lowercase();
        let matches = |t: &str| searching && t.to_lowercase().contains(&query);

        // A search jumps the page to its first match.
        if searching && !self.ui_context[self.page_scroll_bar].dragging {
            let first_match_y = page.list.items.iter().find_map(|item| match &item.prim {
                Prim::Text { text, y, .. } if matches(text) => Some(*y),
                _ => None,
            });
            if let Some(y) = first_match_y {
                let local_max_scroll_y = (max_y - lch).max(0.0);
                self.scroll_y = (y - 100.0).clamp(0.0, local_max_scroll_y);
            }
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

        let scroll_y = self.scroll_y;

        // The search: a matched run is lit and gets a highlight under it, the
        // rest of the page's text and its glyphs dim to a quarter. The page
        // buttons are not the page's text: they are painted after this.
        let mut highlights = PaintCtx::new();
        let mut items = page.list.items;
        if searching {
            for item in &mut items {
                match &mut item.prim {
                    Prim::Text { text, x, y, font_size, color, font, .. } => {
                        if matches(text) {
                            let buf = make_text_buffer_with_font(
                                &mut self.font_system,
                                text,
                                *font_size,
                                font.as_deref(),
                                &self.sans_serif_family,
                                &self.serif_family,
                                &self.monospace_family,
                            );
                            let scale = cce_ui::scale::scale_factor();
                            let text_w = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0) / scale;
                            let (pad_x, pad_y) = (4.0, 2.0);
                            highlights.rounded_rect(
                                Rect { x: *x - pad_x, y: *y - pad_y, width: text_w + 2.0 * pad_x, height: *font_size + 2.0 * pad_y },
                                3.0,
                                (true, true, true, true),
                                [0.65, 0.45, 0.05, 0.4],
                            );
                            *color = cce_settings::app::rgb([1.0, 0.95, 0.80, 1.0]);
                        } else {
                            *color = color.map(|c| (c as f32 * 0.25) as u8);
                        }
                    }
                    Prim::Image { alpha, .. } => *alpha *= 0.25,
                    _ => {}
                }
            }
        }

        // The page buttons: widgets the context keeps, made from what the
        // page declared and painted into the page where it declared them,
        // each under the clip in force there. A Tab step's focus that lived
        // on a button the page declared differently this time goes to
        // whatever now stands at its rect (see `focus_stepped`) before they
        // are painted, so the focus ring is on the right one.
        cce_settings::app::sync_page_buttons(&mut self.page_buttons, page.buttons, &mut self.ui_context);
        if let Some((fx, fy, fw, fh)) = self.refocus_rect.take() {
            if self.ui_context.focused_widget.is_none() {
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
        }
        let mut page_items = items.into_iter();
        let mut painted = PaintCtx::new();
        let mut drawn = 0;
        for i in 0..self.page_buttons.len() {
            let hovered = self.page_button_over(&self.page_buttons[i], cursor.0, cursor.1);
            let b = &mut self.page_buttons[i];
            b.hovered = hovered;
            self.ui_context[b.handle].set_hovered(hovered);
            painted.append_items(page_items.by_ref().take(b.at.saturating_sub(drawn)));
            drawn = drawn.max(b.at);
            let w = &self.ui_context[b.handle];
            match b.clip {
                Some(clip) => painted.clip(clip, |pc| cce_ui::widget::painter::paint_root_into(&self.ui_context, w, pc)),
                None => cce_ui::widget::painter::paint_root_into(&self.ui_context, w, &mut painted),
            }
        }
        painted.append_items(page_items);
        // The search's highlights over the page: translucent, and the text
        // they light is drawn after all geometry anyway.
        painted.append_items(highlights.finish().items);
        let items = painted.finish().items;

        // Popovers draw INTO the frame (Phase 6t; there is no engine xdg
        // popup), over the whole window, so the dropdown's open-upward menu
        // renders where it hit-tests. The page dropdown's is in window coords;
        // a page widget's is in page coords, lifted by this rebuild's scroll —
        // the same subtraction the old popup positioner applied at creation.
        // dl-text occlusion comes from the context's popover registration.
        // The context menu is not drawn here: display_list paints it straight
        // into the frame with `context_menu::paint_with_labels`.
        let chrome_id = self.page_dropdown.id();
        let mut chrome_pop = PaintCtx::new();
        let mut page_pop = PaintCtx::new();
        for &pop_id in &self.ui_context.active_popovers {
            let Some(popover) = self.ui_context.get_widget(pop_id) else { continue };
            if pop_id == chrome_id {
                popover.render_popover(&mut chrome_pop);
            } else {
                popover.render_popover(&mut page_pop);
            }
        }
        let mut popovers = chrome_pop.finish().items;
        popovers.extend(page_pop.finish().items.into_iter().map(|i| i.translated(0.0, -scroll_y)));

        self.frame = CachedFrame { page: DisplayList { items }, popovers: DisplayList { items: popovers } };
        self.needs_rebuild = false;
        self.laid_out_page = Some(self.app.current_page);
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
