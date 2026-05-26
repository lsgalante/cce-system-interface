pub mod power;
pub mod audio;
pub mod network;
pub mod display;
pub mod layout;
pub mod storage;
pub mod system_info;
pub mod keybindings;
pub mod input;
pub mod status;
pub mod processors;
pub mod notifications;
pub mod backup;
pub mod typeface;
pub mod services;
pub mod colors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Power,
    Audio,
    Radios,
    Services,
    Storage,
    Display,
    Layout,
    System,
    Processors,
    Input,
    Status,
    Notifications,
    Backup,
    Typefaces,
    Colors,
}

impl Page {
    pub const ALL: [Page; 15] = [
        Page::Audio,
        Page::Backup,
        Page::Colors,
        Page::Display,
        Page::Input,
        Page::Layout,
        Page::Notifications,
        Page::Processors,
        Page::Power,
        Page::Radios,
        Page::Services,
        Page::Status,
        Page::Storage,
        Page::System,
        Page::Typefaces,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Power => "Power",
            Page::Audio => "Audio",
            Page::Radios => "Radios",
            Page::Services => "Services",
            Page::Storage => "Storage",
            Page::Display => "Display",
            Page::Layout => "Layout",
            Page::System => "System",
            Page::Processors => "Processors",
            Page::Input => "Input",
            Page::Status => "Status",
            Page::Notifications => "Notifications",
            Page::Backup => "Backup",
            Page::Typefaces => "Typefaces",
            Page::Colors => "Colors",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}
