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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Power,
    Audio,
    Radios,
    Storage,
    Display,
    Layout,
    System,
    Processors,
    Input,
    Status,
}

impl Page {
    pub const ALL: [Page; 10] = [
        Page::Audio,
        Page::Display,
        Page::Input,
        Page::Layout,
        Page::Processors,
        Page::Power,
        Page::Radios,
        Page::Status,
        Page::Storage,
        Page::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Power => "Power",
            Page::Audio => "Audio",
            Page::Radios => "Radios",
            Page::Storage => "Storage",
            Page::Display => "Display",
            Page::Layout => "Layout",
            Page::System => "System",
            Page::Processors => "Processors",
            Page::Input => "Input",
            Page::Status => "Status",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }
}
