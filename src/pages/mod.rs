pub mod audio;
pub mod network;
pub mod display;
pub mod layout;
pub mod storage;
pub mod system_info;
pub mod keybindings;
pub mod input;
pub mod hardware;
pub mod services;
pub mod interface;
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
    Interface,
}

impl Page {
    pub const ALL: [Page; 11] = [
        Page::Accounts,
        Page::Audio,
        Page::Interface,
        Page::Display,
        Page::Input,
        Page::Layout,
        Page::Hardware,
        Page::Radios,
        Page::Services,
        Page::Storage,
        Page::System,
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
            Page::Interface => "Interface",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}

