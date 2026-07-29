use cce_ui::layout::RenderTarget;

use crate::pages::audio;
use crate::pages::network;
use crate::pages::processes;
use crate::pages::system_info;
use crate::pages::storage;
use crate::pages::fonts;
use crate::pages::accounts;
use crate::pages::packages;
use crate::pages::notifications;
use crate::pages::Page;

pub struct AppState {
    pub current_page: Page,
    pub audio: audio::AudioState,
    pub network: network::NetworkState,
    pub processes: processes::ProcessesState,
    pub system_info: system_info::SystemState,
    pub storage: storage::StorageState,
    pub fonts: fonts::FontsState,
    pub accounts: accounts::AccountsState,
    pub packages: packages::PackagesState,
    pub notifications: notifications::NotificationsState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_page: Page::ALL[0],
            audio: audio::AudioState::default(),
            network: network::NetworkState::default(),
            processes: processes::ProcessesState::default(),
            system_info: system_info::SystemState::default(),
            storage: storage::StorageState::default(),
            fonts: fonts::FontsState::default(),
            accounts: accounts::AccountsState::default_mock(),
            packages: packages::PackagesState::default(),
            notifications: notifications::NotificationsState::default(),
        }
    }
}

impl AppState {
    pub fn get_page(&self, page: Page) -> &dyn crate::pages::AppPage {
        match page {
            Page::Accounts => &self.accounts,
            Page::Audio => &self.audio,
            Page::Packages => &self.packages,
            Page::Processes => &self.processes,
            Page::Radios => &self.network,
            Page::Storage => &self.storage,
            Page::System => &self.system_info,
            Page::Fonts => &self.fonts,
            Page::Notifications => &self.notifications,
        }
    }

    pub fn get_page_mut(&mut self, page: Page) -> &mut dyn crate::pages::AppPage {
        match page {
            Page::Accounts => &mut self.accounts,
            Page::Audio => &mut self.audio,
            Page::Packages => &mut self.packages,
            Page::Processes => &mut self.processes,
            Page::Radios => &mut self.network,
            Page::Storage => &mut self.storage,
            Page::System => &mut self.system_info,
            Page::Fonts => &mut self.fonts,
            Page::Notifications => &mut self.notifications,
        }
    }

    pub fn get_current_page(&self) -> &dyn crate::pages::AppPage {
        self.get_page(self.current_page)
    }

    pub fn get_current_page_mut(&mut self) -> &mut dyn crate::pages::AppPage {
        self.get_page_mut(self.current_page)
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Exit,
    Audio(audio::AudioMessage),
    Radios(network::NetworkMessage),
    Processes(processes::ProcessesMessage),
    SystemInfo(system_info::SystemMessage),
    Storage(storage::StorageMessage),
    Fonts(fonts::FontsMessage),
    Accounts(accounts::AccountsMessage),
    Packages(packages::PackagesMessage),
    Notifications(notifications::NotificationsMessage),
}


pub struct PageContent {
    pub rects: Vec<([f32; 4], f32, f32, f32, f32, f32, (bool, bool, bool, bool))>,
    pub texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    pub buttons: Vec<(cce_ui::widget::Adapted<cce_ui::widget::Button>, AppAction)>,
    /// Section wells claimed via `RenderTarget::section_relief` — the body box
    /// plus the title tab box, carved into the window plate by display_list as
    /// recess prims (page coordinates, pre-scroll).
    pub reliefs: Vec<((f32, f32, f32, f32), Option<(f32, f32, f32, f32)>)>,
    pub clip_stack: Vec<[f32; 4]>,
    pub measure_only: bool,
}

impl Default for PageContent {
    fn default() -> Self {
        Self {
            rects: Vec::new(),
            texts: Vec::new(),
            buttons: Vec::new(),
            reliefs: Vec::new(),
            clip_stack: Vec::new(),
            measure_only: true,
        }
    }
}

impl PageContent {
    pub fn new() -> Self {
        Self { rects: Vec::new(), texts: Vec::new(), buttons: Vec::new(), reliefs: Vec::new(), clip_stack: Vec::new(), measure_only: false }
    }

    fn get_clipped_rect(&self, x: f32, y: f32, w: f32, h: f32) -> Option<(f32, f32, f32, f32)> {
        if let Some(&clip) = self.clip_stack.last() {
            let cx = clip[0];
            let cy = clip[1];
            let cw = clip[2];
            let ch = clip[3];
            let rx1 = x.max(cx);
            let ry1 = y.max(cy);
            let rx2 = (x + w).min(cx + cw);
            let ry2 = (y + h).min(cy + ch);
            if rx1 < rx2 && ry1 < ry2 {
                Some((rx1, ry1, rx2 - rx1, ry2 - ry1))
            } else {
                None
            }
        } else {
            Some((x, y, w, h))
        }
    }

    fn get_clipped_bounds(&self, bounds: Option<[f32; 4]>) -> Option<[f32; 4]> {
        if let Some(&clip) = self.clip_stack.last() {
            if let Some(b) = bounds {
                let rx1 = b[0].max(clip[0]);
                let ry1 = b[1].max(clip[1]);
                let rx2 = b[2].min(clip[0] + clip[2]);
                let ry2 = b[3].min(clip[1] + clip[3]);
                Some([rx1, ry1, rx2.max(rx1), ry2.max(ry1)])
            } else {
                Some([clip[0], clip[1], clip[0] + clip[2], clip[1] + clip[3]])
            }
        } else {
            bounds
        }
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        if self.measure_only { return; }
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, 0.0, (true, true, true, true)));
        }
    }

    pub fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    pub fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
        if self.measure_only { return; }
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color);
        self.buttons.push((btn, action));
    }

    pub fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                       bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                       action: AppAction) {
        if self.measure_only { return; }
        let btn = cce_ui::widget::Button::new(x, y, w, h)
            .with_label(label)
            .with_bg(bg)
            .with_hover_bg(hover_bg)
            .with_label_color(label_color)
            .with_left_align(true);
        self.buttons.push((btn, action));
    }
}

impl RenderTarget for PageContent {
    fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        if self.measure_only { return; }
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, 0.0, (true, true, true, true)));
        }
    }

    fn rect_with_radius(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32) {
        if self.measure_only { return; }
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, radius, (true, true, true, true)));
        }
    }

    fn rect_with_radius_corners(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, corners: (bool, bool, bool, bool)) {
        if self.measure_only { return; }
        if let Some((cx, cy, cw, ch)) = self.get_clipped_rect(x, y, w, h) {
            self.rects.push((color, cx, cy, cw, ch, radius, corners));
        }
    }

    fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(None);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    fn text_with_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], bounds: Option<[f32; 4]>) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(bounds);
        self.texts.push((content.to_string(), size, x, y, color, None, cb));
    }

    fn text_with_font_and_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str, bounds: Option<[f32; 4]>) {
        if self.measure_only { return; }
        let cb = self.get_clipped_bounds(bounds);
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), cb));
    }

    fn section_relief_style(&self) -> bool {
        cce_ui::layout::control_relief()
    }

    fn section_relief(&mut self, f: &cce_ui::layout::SectionFrame) -> bool {
        // Focused sections keep the legacy green outline (the ctrl-nav feedback);
        // relief-off styling keeps the outline everywhere.
        if f.focused || !cce_ui::layout::control_relief() {
            return false;
        }
        if !self.measure_only {
            self.reliefs.push(((f.x, f.y, f.w, f.h), f.tab));
        }
        true
    }

    fn push_clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        if self.measure_only { return; }
        let clip = if let Some(&parent_clip) = self.clip_stack.last() {
            let cx = x.max(parent_clip[0]);
            let cy = y.max(parent_clip[1]);
            let cw = (x + w).min(parent_clip[0] + parent_clip[2]) - cx;
            let ch = (y + h).min(parent_clip[1] + parent_clip[3]) - cy;
            [cx, cy, cw.max(0.0), ch.max(0.0)]
        } else {
            [x, y, w, h]
        };
        self.clip_stack.push(clip);
    }

    fn pop_clip_rect(&mut self) {
        if self.measure_only { return; }
        self.clip_stack.pop();
    }
}

pub trait SectionContextExt {
    fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: AppAction);
    fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: AppAction);
}

impl<'a> SectionContextExt for cce_ui::layout::SectionContext<'a, PageContent> {
    fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: AppAction) {
        self.pc.button(label, x, y, w, h, bg, hover_bg, label_color, action);
        self.content_y = self.content_y.max(y + h);
        for height in &mut self.grid.col_heights {
            *height = height.max(self.content_y);
        }
    }
    
    fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: AppAction) {
        self.pc.button_left(label, x, y, w, h, bg, hover_bg, label_color, action);
        self.content_y = self.content_y.max(y + h);
        for height in &mut self.grid.col_heights {
            *height = height.max(self.content_y);
        }
    }
}





