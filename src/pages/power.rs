//! Power: battery facts plus the host's real battery-life levers, all sysfs.
//!
//! Every control is discovered from the interfaces this machine actually
//! exposes (missing ones render as absent, not as dead widgets):
//! - `/sys/firmware/acpi/platform_profile` — firmware power profile
//! - `/sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`
//! - `/sys/class/power_supply/BAT*/charge_control_end_threshold` — capping
//!   charge at 80% is the classic battery-longevity lever
//! - `/sys/devices/system/cpu/intel_pstate/no_turbo` — turbo boost
//!
//! Reads are plain file reads; writes go through `pkexec sh -c` like every
//! other privileged action in this app (bluetooth, storage, packages). The
//! UI is optimistic and the 5s watcher re-reads the truth, so a dismissed
//! auth prompt reverts the dropdown — honest, with no extra error channel.

use crate::app::{AppAction, PageContent};
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy, RenderTarget};
use cce_ui::widget::{Dropdown, WidgetHost};

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const GOOD: [f32; 4] = [0.56, 0.83, 0.56, 1.0];
const WARN: [f32; 4] = [0.90, 0.75, 0.45, 1.0];

/// Everything read from sysfs in one pass. Plain data; widget state lives in
/// [`PowerState`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PowerFacts {
    pub battery_present: bool,
    pub status: String,
    pub capacity_pct: u32,
    /// energy_full / energy_full_design — how much the cells have aged.
    pub health_pct: Option<u32>,
    /// Instantaneous draw in watts, meaningful while discharging.
    pub power_w: Option<f32>,
    pub ac_online: Option<bool>,
    /// platform_profile choices in sysfs spelling, and the active one.
    pub profiles: Vec<String>,
    pub profile: String,
    /// EPP choices in sysfs spelling, and cpu0's current value.
    pub epps: Vec<String>,
    pub epp: String,
    /// charge_control_end_threshold, when the battery has one.
    pub charge_limit: Option<u32>,
    /// intel_pstate no_turbo, inverted to "turbo enabled".
    pub turbo: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PowerState {
    pub loaded: bool,
    pub facts: PowerFacts,
    pub dd_profile: cce_ui::widget::Adapted<Dropdown>,
    pub dd_epp: cce_ui::widget::Adapted<Dropdown>,
    pub dd_limit: cce_ui::widget::Adapted<Dropdown>,
    pub dd_turbo: cce_ui::widget::Adapted<Dropdown>,
    /// Sysfs value per charge-limit dropdown row (options are display text).
    pub limit_values: Vec<u32>,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            loaded: false,
            facts: PowerFacts::default(),
            dd_profile: Dropdown::new(vec!["—".to_string()], 0).with_label("Power Profile"),
            dd_epp: Dropdown::new(vec!["—".to_string()], 0).with_label("CPU Energy Preference"),
            dd_limit: Dropdown::new(vec!["—".to_string()], 0).with_label("Battery Charge Limit"),
            dd_turbo: Dropdown::new(vec!["Enabled".to_string(), "Disabled".to_string()], 0)
                .with_label("CPU Turbo Boost"),
            limit_values: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Refreshed(PowerFacts),
    /// Dropdown picks, by option index.
    SetProfile(usize),
    SetEpp(usize),
    SetLimit(usize),
    SetTurbo(usize),
}

/// Sysfs tokens travel into a `pkexec sh -c` line, so only the shapes sysfs
/// itself produces are allowed through — anything else is dropped, not
/// escaped. (The lists come from sysfs reads, but the guard makes the write
/// path safe by construction rather than by data-flow argument.)
fn sysfs_token_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Root sysfs write via pkexec, the app's standard privileged-action path.
/// Detached: the polkit prompt runs in its own process, the UI never blocks,
/// and the watcher's next read reports what actually happened.
fn write_sysfs(cmd: String) {
    let _ = std::process::Command::new("pkexec")
        .args(["sh", "-c", &cmd])
        .spawn();
}

/// Display form of a sysfs token: `balance_power` → "Balance Power".
fn pretty(token: &str) -> String {
    token
        .split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// The first battery under /sys/class/power_supply, by convention BAT*.
fn battery_dir() -> Option<std::path::PathBuf> {
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    let mut bats: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("BAT")))
        .collect();
    bats.sort();
    bats.into_iter().next()
}

pub async fn fetch_power_state() -> PowerFacts {
    let mut f = PowerFacts::default();

    if let Some(bat) = battery_dir() {
        let b = |name: &str| read_trim(&format!("{}/{}", bat.display(), name));
        f.battery_present = true;
        f.status = b("status").unwrap_or_else(|| "Unknown".to_string());
        f.capacity_pct = b("capacity").and_then(|s| s.parse().ok()).unwrap_or(0);
        // Health from energy_* or charge_* — batteries report one family.
        let full: Option<f64> = b("energy_full").or_else(|| b("charge_full")).and_then(|s| s.parse().ok());
        let design: Option<f64> =
            b("energy_full_design").or_else(|| b("charge_full_design")).and_then(|s| s.parse().ok());
        if let (Some(full), Some(design)) = (full, design) {
            if design > 0.0 {
                f.health_pct = Some(((full / design) * 100.0).round() as u32);
            }
        }
        f.power_w = b("power_now").and_then(|s| s.parse::<f64>().ok()).map(|uw| (uw / 1e6) as f32);
        f.charge_limit = b("charge_control_end_threshold").and_then(|s| s.parse().ok());
    }
    f.ac_online = read_trim("/sys/class/power_supply/AC/online").map(|s| s == "1");

    if let Some(choices) = read_trim("/sys/firmware/acpi/platform_profile_choices") {
        f.profiles = choices.split_whitespace().map(String::from).collect();
        f.profile = read_trim("/sys/firmware/acpi/platform_profile").unwrap_or_default();
    }

    if let Some(prefs) =
        read_trim("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_available_preferences")
    {
        f.epps = prefs.split_whitespace().map(String::from).collect();
        f.epp = read_trim("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
            .unwrap_or_default();
    }

    f.turbo = read_trim("/sys/devices/system/cpu/intel_pstate/no_turbo").map(|s| s == "0");

    f
}

/// Rebuild every dropdown's options/selection from fresh facts. Skipped per
/// dropdown while it is open (the default_apps rule: never yank an open menu
/// out from under the pointer — the next refresh normalizes it).
fn rebuild_options(state: &mut PowerState) {
    let f = &state.facts;
    if !state.dd_profile.open {
        state.dd_profile.options = f.profiles.iter().map(|p| pretty(p)).collect();
        state.dd_profile.selected =
            f.profiles.iter().position(|p| *p == f.profile).unwrap_or(0);
    }
    if !state.dd_epp.open {
        state.dd_epp.options = f.epps.iter().map(|p| pretty(p)).collect();
        state.dd_epp.selected = f.epps.iter().position(|p| *p == f.epp).unwrap_or(0);
    }
    if !state.dd_limit.open {
        let mut values = vec![100u32, 80, 60];
        if let Some(cur) = f.charge_limit {
            if !values.contains(&cur) {
                values.push(cur);
            }
        }
        state.dd_limit.options = values
            .iter()
            .map(|v| match v {
                100 => "100% — full capacity".to_string(),
                80 => "80% — longevity".to_string(),
                60 => "60% — max longevity".to_string(),
                other => format!("{}% — current", other),
            })
            .collect();
        state.dd_limit.selected = f
            .charge_limit
            .and_then(|cur| values.iter().position(|v| *v == cur))
            .unwrap_or(0);
        state.limit_values = values;
    }
    if !state.dd_turbo.open {
        state.dd_turbo.selected = if f.turbo.unwrap_or(true) { 0 } else { 1 };
    }
}

pub fn view(state: &mut PowerState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let sec_w = sec.cw;
        if !state.loaded {
            sec.text("Reading power interfaces...", 12.0, 0.0, 12.0, TEXT_DIM);
            return;
        }
        let f = &state.facts;

        // ── Battery facts ──
        if f.battery_present {
            let status_color = match f.status.as_str() {
                "Charging" | "Full" => GOOD,
                "Discharging" => WARN,
                _ => TEXT_FG,
            };
            let mut line = format!("{}%  ·  {}", f.capacity_pct, f.status);
            if let (Some(w), "Discharging") = (f.power_w, f.status.as_str()) {
                line.push_str(&format!("  ·  {:.1} W draw", w));
            }
            if let Some(true) = f.ac_online {
                line.push_str("  ·  on AC");
            }
            sec.text("Battery", 13.0, 0.0, 13.0, TEXT_FG);
            sec.text(&line, 12.0, 0.0, 12.0, status_color);
            if let Some(h) = f.health_pct {
                sec.text(
                    &format!("Health: {}% of design capacity", h),
                    12.0,
                    0.0,
                    12.0,
                    if h >= 80 { TEXT_DIM } else { WARN },
                );
            }
        } else {
            sec.text("No battery detected", 12.0, 0.0, 12.0, TEXT_DIM);
        }
        sec.spacing(10.0);

        // ── Levers, one dropdown per interface the host exposes ──
        let mut stack = sec.vstack(8.0);
        if !state.facts.profiles.is_empty() {
            state.dd_profile.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_profile, sec_w - 28.0, 44.0, ctx);
        }
        if !state.facts.epps.is_empty() {
            state.dd_epp.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_epp, sec_w - 28.0, 44.0, ctx);
        }
        if state.facts.charge_limit.is_some() {
            state.dd_limit.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_limit, sec_w - 28.0, 44.0, ctx);
        }
        if state.facts.turbo.is_some() {
            state.dd_turbo.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_turbo, sec_w - 28.0, 44.0, ctx);
        }
    });

    final_pc
}

pub fn update(state: &mut PowerState, msg: PowerMessage) {
    match msg {
        PowerMessage::Refreshed(facts) => {
            state.loaded = true;
            if state.facts != facts {
                state.facts = facts;
                rebuild_options(state);
            }
        }
        PowerMessage::SetProfile(idx) => {
            if let Some(p) = state.facts.profiles.get(idx) {
                if sysfs_token_ok(p) {
                    write_sysfs(format!("echo {} > /sys/firmware/acpi/platform_profile", p));
                    state.facts.profile = p.clone();
                }
            }
        }
        PowerMessage::SetEpp(idx) => {
            if let Some(p) = state.facts.epps.get(idx) {
                if sysfs_token_ok(p) {
                    // Every core: EPP is per-cpu and a partial write would
                    // leave the package split across preferences.
                    write_sysfs(format!(
                        "for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do echo {} > \"$f\"; done",
                        p
                    ));
                    state.facts.epp = p.clone();
                }
            }
        }
        PowerMessage::SetLimit(idx) => {
            if let Some(v) = state.limit_values.get(idx).copied() {
                if (1..=100).contains(&v) {
                    write_sysfs(format!(
                        "for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do echo {} > \"$f\"; done",
                        v
                    ));
                    state.facts.charge_limit = Some(v);
                }
            }
        }
        PowerMessage::SetTurbo(idx) => {
            let no_turbo = if idx == 1 { 1 } else { 0 };
            write_sysfs(format!(
                "echo {} > /sys/devices/system/cpu/intel_pstate/no_turbo",
                no_turbo
            ));
            state.facts.turbo = Some(no_turbo == 0);
        }
    }
}

impl crate::pages::AppPage for PowerState {
    // Sections: [Power] — ids mirror the view's load gate AND its
    // per-interface presence gates (the d13a901 lesson: never report a
    // widget the view didn't paint).
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if !self.loaded {
            return vec![Vec::new()];
        }
        let mut ids = Vec::new();
        if !self.facts.profiles.is_empty() {
            ids.push(self.dd_profile.id());
        }
        if !self.facts.epps.is_empty() {
            ids.push(self.dd_epp.id());
        }
        if self.facts.charge_limit.is_some() {
            ids.push(self.dd_limit.id());
        }
        if self.facts.turbo.is_some() {
            ids.push(self.dd_turbo.id());
        }
        vec![ids]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, root_focused, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        if self.dd_profile.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetProfile(self.dd_profile.selected)));
        }
        if self.dd_epp.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetEpp(self.dd_epp.selected)));
        }
        if self.dd_limit.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetLimit(self.dd_limit.selected)));
        }
        if self.dd_turbo.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetTurbo(self.dd_turbo.selected)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::AppPage;

    fn facts() -> PowerFacts {
        PowerFacts {
            battery_present: true,
            status: "Discharging".to_string(),
            capacity_pct: 92,
            health_pct: Some(83),
            power_w: Some(7.2),
            ac_online: Some(false),
            profiles: vec!["low-power".into(), "balanced".into(), "performance".into()],
            profile: "balanced".to_string(),
            epps: vec!["default".into(), "performance".into(), "balance_power".into(), "power".into()],
            epp: "balance_power".to_string(),
            charge_limit: Some(80),
            turbo: Some(true),
        }
    }

    #[test]
    fn options_rebuild_maps_current_values() {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        assert_eq!(st.dd_profile.options, ["Low Power", "Balanced", "Performance"]);
        assert_eq!(st.dd_profile.selected, 1);
        assert_eq!(st.dd_epp.selected, 2); // balance_power
        assert_eq!(st.dd_limit.selected, 1); // 80
        assert_eq!(st.limit_values, [100, 80, 60]);
        assert_eq!(st.dd_turbo.selected, 0); // enabled

        // An off-list threshold gets its own row instead of a wrong match.
        st.facts.charge_limit = Some(75);
        rebuild_options(&mut st);
        assert_eq!(st.limit_values, [100, 80, 60, 75]);
        assert_eq!(st.dd_limit.selected, 3);
        assert!(st.dd_limit.options[3].contains("75%"));
    }

    #[test]
    fn open_dropdown_is_left_alone_on_refresh() {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        st.dd_profile.open = true;
        st.dd_profile.selected = 2;
        let mut newer = facts();
        newer.profile = "low-power".to_string();
        st.facts = newer;
        rebuild_options(&mut st);
        // Open menu untouched; the others refreshed.
        assert_eq!(st.dd_profile.selected, 2);
    }

    #[test]
    fn sysfs_token_guard() {
        assert!(sysfs_token_ok("balance_power"));
        assert!(sysfs_token_ok("low-power"));
        assert!(!sysfs_token_ok(""));
        assert!(!sysfs_token_ok("a b"));
        assert!(!sysfs_token_ok("x;reboot"));
        assert!(!sysfs_token_ok("$(rm)"));
    }

    #[test]
    fn pretty_prints_tokens() {
        assert_eq!(pretty("low-power"), "Low Power");
        assert_eq!(pretty("balance_performance"), "Balance Performance");
        assert_eq!(pretty("default"), "Default");
    }

    #[test]
    fn section_widgets_mirror_presence_gates() {
        let mut st = PowerState::default();
        // Not loaded: nothing reported (the view paints only the loading line).
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        st.facts = facts();
        // All four interfaces present: all four dropdowns reported.
        assert_eq!(st.section_widgets()[0].len(), 4);
        // A host without a charge-limit knob or turbo file reports fewer.
        st.facts.charge_limit = None;
        st.facts.turbo = None;
        assert_eq!(st.section_widgets()[0].len(), 2);
    }
}
