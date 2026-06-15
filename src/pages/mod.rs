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
pub mod packages;

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
    Packages,
}

impl Page {
    pub const ALL: [Page; 12] = [
        Page::Accounts,
        Page::Audio,
        Page::Display,
        Page::Hardware,
        Page::Input,
        Page::Interface,
        Page::Packages,
        Page::Radios,
        Page::Services,
        Page::Storage,
        Page::System,
        Page::Layout,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Accounts => "Accounts",
            Page::Audio => "Audio",
            Page::Radios => "Radios",
            Page::Services => "Services",
            Page::Storage => "Storage",
            Page::Display => "Display",
            Page::Layout => "Window Manager",
            Page::System => "System",
            Page::Hardware => "Hardware",
            Page::Input => "Input",
            Page::Interface => "Interface",
            Page::Packages => "Packages",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}


