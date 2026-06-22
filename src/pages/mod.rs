pub mod audio;
pub mod network;
pub mod display;
pub mod storage;
pub mod system_info;
pub mod keybindings;
pub mod input;
pub mod processes;
pub mod interface;
pub mod accounts;
pub mod packages;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Accounts,
    Audio,
    Radios,
    Storage,
    Display,
    System,
    Processes,
    Input,
    Interface,
    Packages,
}

impl Page {
    pub const ALL: [Page; 10] = [
        Page::Accounts,
        Page::Audio,
        Page::Display,
        Page::Input,
        Page::Interface,
        Page::Packages,
        Page::Processes,
        Page::Radios,
        Page::Storage,
        Page::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Accounts => "Accounts",
            Page::Audio => "Audio",
            Page::Radios => "Radios",
            Page::Storage => "Storage",
            Page::Display => "Display",
            Page::System => "System",
            Page::Processes => "Processes",
            Page::Input => "Input",
            Page::Interface => "Interface",
            Page::Packages => "Packages",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}


