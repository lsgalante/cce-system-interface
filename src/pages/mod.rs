pub mod audio;
pub mod network;
pub mod display;
pub mod layout;
pub mod storage;
pub mod system_info;
pub mod keybindings;
pub mod input;
pub mod status;
pub mod hardware;
pub mod notifications;
pub mod backup;
pub mod typeface;
pub mod services;
pub mod interface;
pub mod screensaver;
pub mod accounts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Accounts,
    Audio,
    Radios,
    Services,
    Storage,
    Display,
    Layout,
    System,
    Hardware,
    Input,
    Status,
    Notifications,
    Backup,
    Typefaces,
    Interface,
    Screensaver,
}

impl Page {
    pub const ALL: [Page; 16] = [
        Page::Accounts,
        Page::Audio,
        Page::Backup,
        Page::Interface,
        Page::Display,
        Page::Input,
        Page::Layout,
        Page::Notifications,
        Page::Hardware,
        Page::Radios,
        Page::Services,
        Page::Status,
        Page::Storage,
        Page::System,
        Page::Typefaces,
        Page::Screensaver,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Accounts => "Accounts",
            Page::Audio => "Audio",
            Page::Radios => "Radios",
            Page::Services => "Services",
            Page::Storage => "Storage",
            Page::Display => "Display",
            Page::Layout => "Layout",
            Page::System => "System",
            Page::Hardware => "Hardware",
            Page::Input => "Input",
            Page::Status => "Status",
            Page::Notifications => "Notifications",
            Page::Backup => "Backup",
            Page::Typefaces => "Typefaces",
            Page::Interface => "Interface",
            Page::Screensaver => "Screensaver",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}

