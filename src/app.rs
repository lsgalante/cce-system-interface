use cce_ui::layout::RenderTarget;

use crate::pages::audio;
use crate::pages::display;
use crate::pages::input;
use crate::pages::layout;
use crate::pages::network;
use crate::pages::hardware;
use crate::pages::system_info;
use crate::pages::storage;
use crate::pages::services;
use crate::pages::interface;
use crate::pages::accounts;
use crate::pages::packages;
use crate::pages::Page;

pub struct AppState {
    pub current_page: Page,
    pub audio: audio::AudioState,
    pub display: display::DisplayState,
    pub network: network::NetworkState,
    pub layout: layout::LayoutState,
    pub input: input::InputState,
    pub hardware: hardware::HardwareState,
    pub system_info: system_info::SystemState,
    pub storage: storage::StorageState,
    pub services: services::ServicesState,
    pub interface: interface::InterfaceState,
    pub accounts: accounts::AccountsState,
    pub packages: packages::PackagesState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_page: Page::ALL[0],
            audio: audio::AudioState::default(),
            display: display::DisplayState::default(),
            network: network::NetworkState::default(),
            layout: layout::LayoutState::default(),
            input: input::InputState::default(),
            hardware: hardware::HardwareState::default(),
            system_info: system_info::SystemState::default(),
            storage: storage::StorageState::default(),
            services: services::ServicesState::default(),
            interface: interface::InterfaceState::default(),
            accounts: accounts::AccountsState::default_mock(),
            packages: packages::PackagesState::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Exit,
    Audio(audio::AudioMessage),
    Display(display::DisplayMessage),
    Radios(network::NetworkMessage),
    Layout(layout::LayoutMessage),
    Input(input::InputMessage),
    Hardware(hardware::HardwareMessage),
    SystemInfo(system_info::SystemMessage),
    Storage(storage::StorageMessage),
    Services(services::ServicesMessage),
    Interface(interface::InterfaceMessage),
    Accounts(accounts::AccountsMessage),
    Packages(packages::PackagesMessage),
}


#[derive(Default)]
pub struct PageContent {
    pub rects: Vec<([f32; 4], f32, f32, f32, f32, f32, (bool, bool, bool, bool))>,
    pub texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    pub buttons: Vec<(cce_ui::widget::Button, AppAction)>,
}

impl PageContent {
    pub fn new() -> Self {
        Self { rects: Vec::new(), texts: Vec::new(), buttons: Vec::new() }
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.rects.push((color, x, y, w, h, 0.0, (true, true, true, true)));
    }

    pub fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.texts.push((content.to_string(), size, x, y, color, None, None));
    }

    pub fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), None));
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
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
        self.rects.push((color, x, y, w, h, 0.0, (true, true, true, true)));
    }

    fn rect_with_radius(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32) {
        self.rects.push((color, x, y, w, h, radius, (true, true, true, true)));
    }

    fn rect_with_radius_corners(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32, radius: f32, corners: (bool, bool, bool, bool)) {
        self.rects.push((color, x, y, w, h, radius, corners));
    }

    fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.texts.push((content.to_string(), size, x, y, color, None, None));
    }

    fn text_with_font(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str) {
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), None));
    }

    fn text_with_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], bounds: Option<[f32; 4]>) {
        self.texts.push((content.to_string(), size, x, y, color, None, bounds));
    }

    fn text_with_font_and_bounds(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4], font: &str, bounds: Option<[f32; 4]>) {
        self.texts.push((content.to_string(), size, x, y, color, Some(font.to_string()), bounds));
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
    }
    
    fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32, bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4], action: AppAction) {
        self.pc.button_left(label, x, y, w, h, bg, hover_bg, label_color, action);
        self.content_y = self.content_y.max(y + h);
    }
}





