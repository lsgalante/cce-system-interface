use clear_ui::layout::RenderTarget;

use crate::pages::audio;
use crate::pages::display;
use crate::pages::input;
use crate::pages::layout;
use crate::pages::network;
use crate::pages::notifications;
use crate::pages::power;
use crate::pages::processors;
use crate::pages::status;
use crate::pages::storage;
use crate::pages::system_info;
use crate::pages::backup;
use crate::pages::Page;

pub struct AppState {
    pub current_page: Page,
    pub power: power::PowerState,
    pub audio: audio::AudioState,
    pub display: display::DisplayState,
    pub network: network::NetworkState,
    pub layout: layout::LayoutState,
    pub input: input::InputState,
    pub processors: processors::ProcessorsState,
    pub system_info: system_info::SystemState,
    pub status: status::StatusState,
    pub storage: storage::StorageState,
    pub notifications: notifications::NotificationsState,
    pub backup: backup::BackupState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_page: Page::ALL[0],
            power: power::PowerState::default(),
            audio: audio::AudioState::default(),
            display: display::DisplayState::default(),
            network: network::NetworkState::default(),
            layout: layout::LayoutState::default(),
            input: input::InputState::default(),
            processors: processors::ProcessorsState::default(),
            system_info: system_info::SystemState::default(),
            status: status::StatusState::default(),
            storage: storage::StorageState::default(),
            notifications: notifications::read_notifications_config(),
            backup: backup::BackupState::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Power(power::PowerMessage),
    Audio(audio::AudioMessage),
    Display(display::DisplayMessage),
    Radios(network::NetworkMessage),
    Layout(layout::LayoutMessage),
    Input(input::InputMessage),
    Processors(processors::ProcessorsMessage),
    SystemInfo(system_info::SystemMessage),
    Status(status::StatusMessage),
    Storage(storage::StorageMessage),
    Notifications(notifications::NotificationsMessage),
    Backup(backup::BackupMessage),
}

pub struct PageContent {
    pub rects: Vec<([f32; 4], f32, f32, f32, f32)>,
    pub texts: Vec<(String, f32, f32, f32, [f32; 4])>,
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
}

impl PageContent {
    pub fn new() -> Self {
        Self { rects: Vec::new(), texts: Vec::new(), buttons: Vec::new() }
    }

    pub fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.rects.push((color, x, y, w, h));
    }

    pub fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.texts.push((content.to_string(), size, x, y, color));
    }

    pub fn button(&mut self, label: &str, x: f32, y: f32, w: f32, h: f32,
                  bg: [f32; 4], hover_bg: [f32; 4], label_color: [f32; 4],
                  action: AppAction) {
        self.buttons.push(ContentButton {
            x, y, w, h, bg, hover_bg,
            label: label.to_string(), label_size: 12.0, label_color,
            action,
        });
    }
}

impl RenderTarget for PageContent {
    fn rect(&mut self, color: [f32; 4], x: f32, y: f32, w: f32, h: f32) {
        self.rects.push((color, x, y, w, h));
    }

    fn text(&mut self, content: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        self.texts.push((content.to_string(), size, x, y, color));
    }
}
