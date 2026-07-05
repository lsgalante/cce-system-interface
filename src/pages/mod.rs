pub mod audio;
pub mod network;
pub mod storage;
pub mod system_info;
pub mod keybindings;
pub mod processes;
pub mod fonts;
pub mod accounts;
pub mod packages;
pub mod notifications;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Accounts,
    Audio,
    Notifications,
    Radios,
    Storage,
    System,
    Processes,
    Fonts,
    Packages,
}

impl Page {
    pub const ALL: [Page; 9] = [
        Page::Accounts,
        Page::Audio,
        Page::Fonts,
        Page::Notifications,
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
            Page::Notifications => "Notifications",
            Page::Radios => "Radios",
            Page::Storage => "Storage",
            Page::System => "System",
            Page::Processes => "Processes",
            Page::Fonts => "Fonts",
            Page::Packages => "Packages",
        }
    }

    pub fn icon(self) -> &'static str {
        ""
    }

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|&p| p == self).unwrap()
    }
}

pub trait AppPage {
    fn clear_children(&mut self, ctx: &mut cce_ui::context::UiContext);

    fn get_section_containers(&self) -> Vec<cce_ui::widget::SectionContainer>;

    fn link_children(
        &mut self,
        page_root: &mut dyn cce_ui::widget::Element,
        sec_containers: &mut [cce_ui::widget::SectionContainer],
        ctx: &mut cce_ui::context::UiContext,
    );

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn cce_ui::layout::LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent;

    fn propagate_widget_changes(&mut self, actions: &mut Vec<crate::app::AppAction>);

    fn handle_pointer_move(
        &mut self,
        _lx: f32,
        _ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        false
    }

    fn handle_pointer_down(&mut self, _lx: f32, _ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        false
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        false
    }
}


