pub mod audio;
pub mod bluetooth;
pub mod browser;
pub mod default_apps;
pub mod network;
pub mod storage;
pub mod system_info;
pub mod timers;
pub mod processes;
pub mod services;
pub mod accounts;
pub mod packages;
pub mod notifications;
pub mod power;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Accounts,
    Audio,
    Bluetooth,
    Browser,
    DefaultApps,
    Network,
    Notifications,
    Power,
    Storage,
    System,
    Timers,
    Processes,
    Services,
    Packages,
}

impl Page {
    pub const ALL: [Page; 14] = [
        Page::Accounts,
        Page::Audio,
        Page::Bluetooth,
        Page::Browser,
        Page::DefaultApps,
        Page::Network,
        Page::Notifications,
        Page::Packages,
        Page::Power,
        Page::Processes,
        Page::Services,
        Page::Storage,
        Page::System,
        Page::Timers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Accounts => "Accounts",
            Page::Audio => "Audio",
            Page::Bluetooth => "Bluetooth",
            Page::Browser => "Browser",
            Page::DefaultApps => "Default Apps",
            Page::Network => "Network",
            Page::Notifications => "Notifications",
            Page::Power => "Power",
            Page::Storage => "Storage",
            Page::System => "System",
            Page::Timers => "Timers",
            Page::Processes => "Processes",
            Page::Services => "Services",
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

    /// Per-frame upkeep for the dissolved inner lists. A `ScrollRegion`'s
    /// wheel only moves its target; its `tick` is what glides (wheel) or
    /// coasts (trackpad flick) the drawn offset there — a page hosting one
    /// must pump it here or the list freezes after the first notch. True =
    /// the page's geometry changed (the host re-lays it out).
    fn tick(&mut self, _dt: f32) -> bool {
        false
    }
}



#[cfg(test)]
mod list_clip_tests {
    use super::*;
    use cce_ui::widget::ScrollRegion;

    fn render(page: &mut dyn AppPage) -> crate::app::PageContent {
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let mut ctx = cce_ui::context::UiContext::new();
        page.view(10.0, 20.0, 820.0, 640.0, false, &[false; 4], &mut layout, &mut ctx)
    }

    /// A list sits inside its section's clip exactly when the clip it
    /// pushes for its rows survives whole. Clips intersect, so a list
    /// sticking out past the section's content box — laid out at
    /// `left + margin` — gets its row clip narrowed to that box, the cut
    /// that took the edges off the framed lists and the first letter off
    /// the Processes summary. Row texts carry the clip in their bounds
    /// (`[x0, y0, x1, y1]`), row buttons beside them (`[x, y, w, h]`).
    fn assert_list_inside(name: &str, pc: &crate::app::PageContent, list: &ScrollRegion) {
        let text_clips = pc.texts.iter().filter_map(|t| t.6);
        let button_clips = pc.buttons.iter().filter_map(|b| b.2).map(|[x, y, w, h]| [x, y, x + w, y + h]);
        let widest = text_clips
            .chain(button_clips)
            .filter(|b| b[1] >= list.y - 0.5 && b[3] <= list.y + list.h + 0.5)
            .max_by(|a, b| (a[2] - a[0]).total_cmp(&(b[2] - b[0])))
            .unwrap_or_else(|| panic!("{name}: no row drawn under the list's clip"));
        let (l, r) = (list.x, list.x + list.w);
        assert!(
            (widest[0] - l).abs() < 0.5 && (widest[2] - r).abs() < 0.5,
            "{name}: rows clip to [{}, {}], the list spans [{l}, {r}]",
            widest[0],
            widest[2]
        );
    }

    #[test]
    fn services_list_inside_its_section() {
        let mut s = services::ServicesState::default();
        s.loaded = true;
        s.services = (0..5)
            .map(|i| services::ServiceInfo {
                name: format!("s{i}.service"),
                description: "d".into(),
                active_state: "active".into(),
                sub_state: "running".into(),
                is_system: true,
            })
            .collect();
        let pc = render(&mut s);
        assert_list_inside("services", &pc, &s.list);
    }

    #[test]
    fn timers_list_inside_its_section() {
        let mut s = timers::TimersState::default();
        s.loaded = true;
        s.timers = (0..5)
            .map(|i| timers::TimerInfo {
                unit: format!("t{i}.timer"),
                activates: format!("t{i}.service"),
                next_usec: None,
                last_usec: None,
                active: true,
                file_state: "enabled".into(),
                is_system: true,
                editable: false,
            })
            .collect();
        let pc = render(&mut s);
        assert_list_inside("timers", &pc, &s.list);
    }

    #[test]
    fn packages_list_inside_its_section() {
        let mut s = packages::PackagesState::default();
        s.loaded = true;
        s.installed = (0..5)
            .map(|i| packages::PackageInfo { name: format!("p{i}"), version: "1.0".into() })
            .collect();
        let pc = render(&mut s);
        assert_list_inside("packages", &pc, &s.installed_list);
    }

    #[test]
    fn network_list_inside_its_section() {
        let mut s = network::NetworkState::default();
        s.loaded = true;
        s.wifi_enabled = true;
        s.available = (0..3)
            .map(|i| network::WifiNetwork { ssid: format!("net{i}"), signal: 50, secured: true, in_use: i == 0 })
            .collect();
        let pc = render(&mut s);
        assert_list_inside("network", &pc, &s.wifi_list);
    }

    #[test]
    fn accounts_list_inside_its_section() {
        let mut s = accounts::AccountsState::default_mock();
        s.loaded = true;
        s.accounts = vec![serde_json::from_str(
            r#"{"email":"a@example.org","imap":"imap.example.org:993","smtp":"smtp.example.org:465","is_default":true,"password":""}"#,
        )
        .unwrap()];
        let pc = render(&mut s);
        assert_list_inside("accounts", &pc, &s.list);
    }

    #[test]
    fn processes_list_inside_its_section() {
        let mut s = processes::ProcessesState { loaded: true, ..Default::default() };
        s.processes = (0..5)
            .map(|i| processes::ProcessRow {
                pid: i.to_string(),
                cpu: "1.0".into(),
                mem_pct: "1.0".into(),
                rss_kb: 1024,
                command: "p".into(),
                watts: None,
                wakeups: None,
            })
            .collect();
        let pc = render(&mut s);
        assert_list_inside("processes", &pc, &s.cpu_list);
    }
}
