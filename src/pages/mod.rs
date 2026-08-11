pub mod audio;
pub mod bluetooth;
pub mod default_apps;
pub mod network;
pub mod storage;
pub mod system_info;
pub mod processes;
pub mod services;
pub mod fonts;
pub mod accounts;
pub mod packages;
pub mod notifications;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Accounts,
    Audio,
    Bluetooth,
    DefaultApps,
    Network,
    Notifications,
    Storage,
    System,
    Processes,
    Services,
    Fonts,
    Packages,
}

impl Page {
    pub const ALL: [Page; 12] = [
        Page::Accounts,
        Page::Audio,
        Page::Bluetooth,
        Page::DefaultApps,
        Page::Fonts,
        Page::Network,
        Page::Notifications,
        Page::Packages,
        Page::Processes,
        Page::Services,
        Page::Storage,
        Page::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Accounts => "Accounts",
            Page::Audio => "Audio",
            Page::Bluetooth => "Bluetooth",
            Page::DefaultApps => "Default Apps",
            Page::Network => "Network",
            Page::Notifications => "Notifications",
            Page::Storage => "Storage",
            Page::System => "System",
            Page::Processes => "Processes",
            Page::Services => "Services",
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
    /// Per-section event/nav widget groups (Phase 6w: SectionContainer dissolved). One
    /// inner Vec per section — same count and order as the old section containers (the
    /// outer length drives the `sec_focused` flags) — holding the widgets that used to
    /// hang under that section's container, in the old link order. The widgets dispatch
    /// directly as propagate roots and the groups drive the app-side ctrl-nav.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>>;

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

    /// Extra top-level event-dispatch roots beyond the section containers: the per-row
    /// widgets that used to hang under a `List`'s ScrollBox (dissolved — the rows now
    /// dispatch directly; `Adapted` hit-gates presses/wheel so misses fall through).
    fn extra_dispatch_roots(&mut self) -> Vec<cce_ui::widget::WidgetId> {
        Vec::new()
    }

    /// Refresh the extra dispatch roots' registrations (id-rooted router: roots resolve
    /// through the registry). The row Vecs are rebuilt on data refresh, so pages
    /// re-register the current allocations right before each dispatch — the same
    /// liveness contract the pointer-rooted dispatch had. Default no-op.
    fn register_extra_dispatch_roots(&mut self, _ctx: &mut cce_ui::context::UiContext) {}

    /// The dissolved inner lists' wheel (`ScrollBox::mouse_wheel`, hit-scoped). Runs after
    /// the widget dispatch and before the manual whole-page scroll fallback — the legacy
    /// "inner ScrollBoxes take the wheel first" order.
    fn handle_mouse_wheel(&mut self, _delta: &cce_ui::widget::MouseScrollDelta, _lx: f32, _ly: f32) -> bool {
        false
    }

    /// The dissolved inner lists' hover/focus-scoped keyboard scrolling
    /// (`ScrollBox::keyboard_input`). Runs before the whole-page scroll-key fallback.
    fn handle_key_input(&mut self, _event: &cce_ui::widget::KeyEvent) -> bool {
        false
    }
}


