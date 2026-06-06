use clear_ui::layout::{RenderTarget, Radial};

use crate::pages::audio;
use crate::pages::display;
use crate::pages::input;
use crate::pages::layout;
use crate::pages::network;
use crate::pages::notifications;
use crate::pages::hardware;
use crate::pages::status;
use crate::pages::storage;
use crate::pages::system_info;
use crate::pages::backup;
use crate::pages::typeface;
use crate::pages::services;
use crate::pages::interface;
use crate::pages::screensaver;
use crate::pages::accounts;
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
    pub status: status::StatusState,
    pub storage: storage::StorageState,
    pub notifications: notifications::NotificationsState,
    pub backup: backup::BackupState,
    pub typeface: typeface::TypefaceState,
    pub services: services::ServicesState,
    pub interface: interface::InterfaceState,
    pub screensaver: screensaver::ScreensaverState,
    pub accounts: accounts::AccountsState,
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
            status: status::StatusState::default(),
            storage: storage::StorageState::default(),
            notifications: notifications::read_notifications_config(),
            backup: backup::BackupState::default(),
            typeface: typeface::TypefaceState::default(),
            services: services::ServicesState::default(),
            interface: interface::InterfaceState::default(),
            screensaver: screensaver::read_screensaver_config(),
            accounts: accounts::AccountsState::default_mock(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Audio(audio::AudioMessage),
    Display(display::DisplayMessage),
    Radios(network::NetworkMessage),
    Layout(layout::LayoutMessage),
    Input(input::InputMessage),
    Hardware(hardware::HardwareMessage),
    SystemInfo(system_info::SystemMessage),
    Status(status::StatusMessage),
    Storage(storage::StorageMessage),
    Notifications(notifications::NotificationsMessage),
    Backup(backup::BackupMessage),
    Typeface(typeface::TypefaceMessage),
    Services(services::ServicesMessage),
    Interface(interface::InterfaceMessage),
    Screensaver(screensaver::ScreensaverMessage),
    Accounts(accounts::AccountsMessage),
}


#[derive(Default)]
pub struct PageContent {
    pub rects: Vec<([f32; 4], f32, f32, f32, f32)>,
    pub texts: Vec<(String, f32, f32, f32, [f32; 4], Option<String>, Option<[f32; 4]>)>,
    pub buttons: Vec<ContentButton>,
}

#[derive(Clone)]
pub struct ContentButton {
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub bg: [f32; 4],
    pub hover_bg: [f32; 4],
    pub label: String,
    pub label_size: f32,
    pub label_color: [f32; 4],
    pub action: AppAction,
    pub left_align: bool,
}

impl PageContent {
    pub fn new() -> Self {
        Self { rects: Vec::new(), texts: Vec::new(), buttons: Vec::new() }
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.rects.push((color, x, y, w, h));
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
        self.buttons.push(ContentButton {
            x, y, w, h, bg, hover_bg,
            label: label.to_string(), label_size: 12.0, label_color,
            action,
            left_align: false,
        });
    }

    pub fn button_left(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                       bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                       action: AppAction) {
        self.buttons.push(ContentButton {
            x, y, w, h, bg, hover_bg,
            label: label.to_string(), label_size: 12.0, label_color,
            action,
            left_align: true,
        });
    }
}

impl RenderTarget for PageContent {
    fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.rects.push((color, x, y, w, h));
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



