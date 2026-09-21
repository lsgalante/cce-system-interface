//! Power: battery facts plus the host's real battery-life levers, all sysfs,
//! kept as two plans — one for when the machine is plugged in, one for when
//! it runs on battery.
//!
//! Every control is discovered from the interfaces this machine actually
//! exposes (missing ones render as absent, not as dead widgets):
//! - `/sys/firmware/acpi/platform_profile` — firmware power profile
//! - `/sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`
//! - `/sys/class/power_supply/BAT*/charge_control_end_threshold` — capping
//!   charge at 80% is the classic battery-longevity lever
//! - `/sys/devices/system/cpu/intel_pstate/no_turbo` — turbo boost
//!
//! The page is three sections: the battery itself (facts and the charge
//! limit, which is a charging policy and so not per source), then one column
//! of levers per power source. The column for the source that is active right
//! now shows the LIVE sysfs values and a pick there applies immediately; the
//! other column shows what is planned for that source, with a "Not set" row
//! meaning "leave it alone". Both are remembered in the plan
//! (`crate::power_plan`, `/etc/cce/power.kdl`) through `cce-power-apply`
//! under pkexec — the one-prompt path every privileged action in this app
//! takes — and the same helper re-applies the plan from udev when the charger
//! comes or goes. The UI is optimistic and the 5s watcher re-reads the truth,
//! so a dismissed auth prompt reverts the dropdown — honest, with no extra
//! error channel.

use crate::app::{AppAction, PageContent};
use crate::power_plan::{self, Lever, PowerPlan, Source};
use cce_ui::layout::{LayoutStrategy, PageLayoutBuilder};
use cce_ui::widget::{Adapted, Dropdown, WidgetHost};
use std::path::{Path, PathBuf};

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
    /// Which plan is in force right now, from the Mains supply.
    pub source: Source,
    /// The per-source plan on disk, and whether the root side that applies
    /// it on plug/unplug (helper + udev rule) is installed.
    pub plan: PowerPlan,
    pub automation: bool,
    /// platform_profile choices in sysfs spelling, and the active one.
    pub profiles: Vec<String>,
    pub profile: String,
    /// EPP choices in sysfs spelling, and cpu0's current value.
    pub epps: Vec<String>,
    pub epp: String,
    /// energy_now / energy_full in µWh — the Wh readout, and what the
    /// time-remaining estimate divides by power_now.
    pub energy_now_uwh: Option<f64>,
    pub energy_full_uwh: Option<f64>,
    /// manufacturer + model_name, the pack's own identification.
    pub vendor: String,
    pub model: String,
    /// charge_control_end_threshold, when the battery has one.
    pub charge_limit: Option<u32>,
    /// intel_pstate no_turbo, inverted to "turbo enabled".
    pub turbo: Option<bool>,
    /// scaling_available_governors, and cpu0's active one. Moved here from the
    /// System page, which drove it through an unversioned pkexec helper script
    /// whose path had not survived two renames of this app.
    pub governors: Vec<String>,
    pub governor: String,
    /// Intel GPU render-clock ceiling in MHz: (current, hardware max RP0,
    /// hardware min RPn). This is the iGPU — on a hybrid laptop it is the GPU
    /// actually drawing power when the discrete one is asleep or driverless.
    pub igpu_mhz: Option<u32>,
    pub igpu_max_mhz: Option<u32>,
    pub igpu_min_mhz: Option<u32>,
    /// PCIe Active State Power Management: the policy list the kernel offers,
    /// and the one in brackets.
    pub aspm_policies: Vec<String>,
    pub aspm: String,
    /// snd_hda_intel power_save: seconds of idle before the audio codec
    /// suspends, 0 meaning never.
    pub hda_idle_secs: Option<u32>,
    /// NVIDIA power limit in watts: (current, default, minimum). None whenever
    /// nvidia-smi cannot reach a driver — including a host where the module is
    /// simply not loaded — which is what gates the dropdown away.
    pub gpu_limit_w: Option<u32>,
    pub gpu_default_w: Option<u32>,
    pub gpu_min_w: Option<u32>,
}

/// One power source's column of lever dropdowns.
#[derive(Debug, Clone)]
pub struct LeverColumn {
    pub source: Source,
    /// One dropdown per [`Lever::ALL`] entry, in that order.
    pub dds: Vec<Adapted<Dropdown>>,
    /// The plan value behind each row of each dropdown (options are display
    /// text). Empty string is the "Not set" row; an empty Vec means the
    /// interface is absent on this host and the dropdown is not painted.
    pub rows: Vec<Vec<String>>,
}

impl LeverColumn {
    fn new(source: Source) -> Self {
        Self {
            source,
            dds: Lever::ALL
                .iter()
                .map(|l| Dropdown::new(vec!["—".to_string()], 0).with_label(l.label()))
                .collect(),
            rows: vec![Vec::new(); Lever::ALL.len()],
        }
    }
}

fn lever_index(lever: Lever) -> usize {
    Lever::ALL.iter().position(|l| *l == lever).unwrap()
}

#[derive(Debug, Clone)]
pub struct PowerState {
    pub loaded: bool,
    pub facts: PowerFacts,
    pub dd_limit: Adapted<Dropdown>,
    /// Sysfs value per charge-limit dropdown row (options are display text).
    pub limit_values: Vec<u32>,
    pub ac: LeverColumn,
    pub battery: LeverColumn,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            loaded: false,
            facts: PowerFacts::default(),
            dd_limit: Dropdown::new(vec!["—".to_string()], 0).with_label("Battery Charge Limit"),
            limit_values: Vec::new(),
            ac: LeverColumn::new(Source::Ac),
            battery: LeverColumn::new(Source::Battery),
        }
    }
}

impl PowerState {
    pub fn column_mut(&mut self, source: Source) -> &mut LeverColumn {
        match source {
            Source::Ac => &mut self.ac,
            Source::Battery => &mut self.battery,
        }
    }
}

/// Which lever columns the page shows. A host with no battery has one power
/// source, so the battery column would be a plan for a state it never enters.
fn columns_shown(f: &PowerFacts) -> Vec<Source> {
    if f.battery_present { vec![Source::Ac, Source::Battery] } else { vec![Source::Ac] }
}

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Refreshed(PowerFacts),
    /// Charge-limit pick, by option index.
    SetLimit(usize),
    /// A lever pick in one source's column, by option index.
    Set { source: Source, lever: Lever, idx: usize },
}

/// Root action via pkexec, the app's standard privileged path. Detached: the
/// polkit prompt runs in its own process, the UI never blocks, and the
/// watcher's next read reports what actually happened.
fn run_privileged(cmd: String) {
    let _ = std::process::Command::new("pkexec")
        .args(["sh", "-c", &cmd])
        .spawn();
}

/// The helper that records and applies the plan: the system copy when
/// `ccebuild install-system` has put it there, else the one installed beside
/// this binary (`~/.local/bin`) — which pkexec will still run as root after
/// the prompt, so the plan works before the root side is installed; only the
/// automatic switching waits on it.
fn helper_path() -> Option<PathBuf> {
    let sys = Path::new(power_plan::HELPER_SYSTEM_PATH);
    if sys.exists() {
        return Some(sys.to_path_buf());
    }
    let beside = std::env::current_exe().ok()?.parent()?.join("cce-power-apply");
    beside.exists().then_some(beside)
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

/// Seconds of runtime left while discharging, or to full while charging, from
/// energy over draw. sysfs has no time field — UPower computes exactly this,
/// and reading it here is what lets the Power page stay all-sysfs rather than
/// taking a D-Bus dependency for one line.
fn battery_seconds(f: &PowerFacts) -> Option<i64> {
    let now = f.energy_now_uwh?;
    let full = f.energy_full_uwh?;
    let draw = f.power_w? as f64 * 1e6;
    if draw <= 0.0 {
        return None;
    }
    let remaining_uwh = match f.status.as_str() {
        "Discharging" => now,
        "Charging" => (full - now).max(0.0),
        _ => return None,
    };
    Some(((remaining_uwh / draw) * 3600.0).round() as i64)
}

fn humanize_secs(secs: i64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// The first /sys/class/drm/card* exposing `gt_max_freq_mhz` (i915/xe).
fn drm_card_with_freq() -> Option<std::path::PathBuf> {
    let rd = std::fs::read_dir("/sys/class/drm").ok()?;
    let mut cards: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("card"))
                && p.join("gt_max_freq_mhz").exists()
        })
        .collect();
    cards.sort();
    cards.into_iter().next()
}

/// The first battery under /sys/class/power_supply, by convention BAT*.
pub(crate) fn battery_dir() -> Option<std::path::PathBuf> {
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
        f.energy_now_uwh = b("energy_now").and_then(|s| s.parse().ok());
        f.energy_full_uwh = full;
        f.vendor = b("manufacturer").unwrap_or_default();
        f.model = b("model_name").unwrap_or_default();
        f.charge_limit = b("charge_control_end_threshold").and_then(|s| s.parse().ok());
    }
    f.ac_online = read_trim("/sys/class/power_supply/AC/online").map(|s| s == "1");
    f.source = power_plan::current_source();
    // An unreadable plan shows as empty here; the applier is the side that
    // refuses to act on it, and logs why.
    f.plan = PowerPlan::load().unwrap_or_else(|e| {
        log::warn!("[power] {}", e);
        PowerPlan::default()
    });
    f.automation = power_plan::automation_installed();

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

    // The first DRM card exposing the i915/xe clock knobs. Globbed rather than
    // fixed at card0: the render node's number depends on probe order, and on a
    // hybrid machine the Intel one is not necessarily first.
    if let Some(card) = drm_card_with_freq() {
        let g = |name: &str| read_trim(&format!("{}/{}", card.display(), name))
            .and_then(|s| s.parse::<u32>().ok());
        f.igpu_mhz = g("gt_max_freq_mhz");
        f.igpu_max_mhz = g("gt_RP0_freq_mhz");
        f.igpu_min_mhz = g("gt_RPn_freq_mhz");
    }

    if let Some(pol) = read_trim("/sys/module/pcie_aspm/parameters/policy") {
        // "[default] performance powersave powersupersave" — brackets mark the
        // active one, and the list is whatever this kernel was built with.
        for tok in pol.split_whitespace() {
            let bare = tok.trim_start_matches('[').trim_end_matches(']');
            if tok.starts_with('[') {
                f.aspm = bare.to_string();
            }
            f.aspm_policies.push(bare.to_string());
        }
    }

    f.hda_idle_secs =
        read_trim("/sys/module/snd_hda_intel/parameters/power_save").and_then(|s| s.parse().ok());

    if let Some(govs) = read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors") {
        f.governors = govs.split_whitespace().map(String::from).collect();
        f.governor = read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").unwrap_or_default();
    }

    // One nvidia-smi call for all three watt figures; any failure (no driver,
    // module not loaded, no card) leaves them None and hides the dropdown.
    if let Ok(out) = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=power.limit,power.default_limit,power.min_limit", "--format=csv,noheader,nounits"])
        .output()
        .await
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(row) = text.lines().next() {
                let w: Vec<Option<u32>> = row
                    .split(',')
                    .map(|c| c.trim().parse::<f32>().ok().map(|v| v.round() as u32))
                    .collect();
                f.gpu_limit_w = w.first().copied().flatten();
                f.gpu_default_w = w.get(1).copied().flatten();
                f.gpu_min_w = w.get(2).copied().flatten();
            }
        }
    }

    f
}

// ── Lever rows ──────────────────────────────────────────────────────────

/// The choices this host offers for a lever, as (plan value, display text)
/// in menu order. Empty means the interface is absent and the dropdown is
/// not painted. Numeric levers get the hardware's own figures — ceiling,
/// midpoint, floor for the iGPU; default and minimum for the dGPU — never
/// invented numbers.
fn choices(lever: Lever, f: &PowerFacts) -> Vec<(String, String)> {
    let tokens = |list: &[String]| list.iter().map(|t| (t.clone(), pretty(t))).collect::<Vec<_>>();
    match lever {
        Lever::Profile => tokens(&f.profiles),
        Lever::Epp => tokens(&f.epps),
        Lever::Governor => tokens(&f.governors),
        Lever::Aspm => tokens(&f.aspm_policies),
        Lever::Turbo => {
            if f.turbo.is_some() {
                vec![("on".to_string(), "Enabled".to_string()), ("off".to_string(), "Disabled".to_string())]
            } else {
                Vec::new()
            }
        }
        Lever::IgpuMaxMhz => {
            let mut out = Vec::new();
            if let (Some(hi), Some(lo)) = (f.igpu_max_mhz, f.igpu_min_mhz) {
                out.push((hi.to_string(), format!("{} MHz  (full)", hi)));
                let mid = ((hi + lo) / 2 / 100) * 100;
                if mid > lo && mid < hi {
                    out.push((mid.to_string(), format!("{} MHz", mid)));
                }
                out.push((lo.to_string(), format!("{} MHz  (minimum)", lo)));
            }
            out
        }
        Lever::AudioIdleSecs => {
            if f.hda_idle_secs.is_some() {
                [0u32, 1, 10].iter().map(|v| (v.to_string(), display_of(Lever::AudioIdleSecs, &v.to_string()))).collect()
            } else {
                Vec::new()
            }
        }
        Lever::GpuLimitW => {
            let mut out: Vec<(String, String)> = Vec::new();
            if let Some(d) = f.gpu_default_w {
                out.push((d.to_string(), format!("{} W  (default)", d)));
            }
            if let Some(m) = f.gpu_min_w {
                if Some(m) != f.gpu_default_w {
                    out.push((m.to_string(), format!("{} W  (minimum)", m)));
                }
            }
            out
        }
    }
}

/// What sysfs says right now, in plan-value spelling.
fn live(lever: Lever, f: &PowerFacts) -> Option<String> {
    let nonempty = |s: &String| if s.is_empty() { None } else { Some(s.clone()) };
    match lever {
        Lever::Profile => nonempty(&f.profile),
        Lever::Epp => nonempty(&f.epp),
        Lever::Governor => nonempty(&f.governor),
        Lever::Aspm => nonempty(&f.aspm),
        Lever::Turbo => f.turbo.map(|t| if t { "on" } else { "off" }.to_string()),
        Lever::IgpuMaxMhz => f.igpu_mhz.map(|v| v.to_string()),
        Lever::AudioIdleSecs => f.hda_idle_secs.map(|v| v.to_string()),
        Lever::GpuLimitW => f.gpu_limit_w.map(|v| v.to_string()),
    }
}

/// Display text for a value that is not one of the host's listed choices
/// (the charge-limit rule: an off-list value gets its own row rather than
/// silently matching the wrong one).
fn display_of(lever: Lever, value: &str) -> String {
    match lever {
        Lever::Turbo => if value == "on" { "Enabled" } else { "Disabled" }.to_string(),
        Lever::IgpuMaxMhz => format!("{} MHz", value),
        Lever::AudioIdleSecs => {
            if value == "0" { "Never suspend".to_string() } else { format!("After {} s idle", value) }
        }
        Lever::GpuLimitW => format!("{} W", value),
        _ => pretty(value),
    }
}

/// Reflect a value the user just applied into the live facts, so the next
/// watcher read (which will say the same thing) does not rebuild the menus.
fn set_live(lever: Lever, value: &str, f: &mut PowerFacts) {
    match lever {
        Lever::Profile => f.profile = value.to_string(),
        Lever::Epp => f.epp = value.to_string(),
        Lever::Governor => f.governor = value.to_string(),
        Lever::Aspm => f.aspm = value.to_string(),
        Lever::Turbo => f.turbo = Some(value == "on"),
        Lever::IgpuMaxMhz => f.igpu_mhz = value.parse().ok(),
        Lever::AudioIdleSecs => f.hda_idle_secs = value.parse().ok(),
        Lever::GpuLimitW => f.gpu_limit_w = value.parse().ok(),
    }
}

/// Rebuild one column's dropdowns. The active source's column shows the live
/// value; the other shows the plan, behind a leading "Not set" row. Either
/// way a shown value missing from the host's list gets appended as its own
/// row. Skipped per dropdown while it is open (the default_apps rule: never
/// yank an open menu out from under the pointer — the next refresh
/// normalizes it).
fn fill_column(col: &mut LeverColumn, f: &PowerFacts, active: bool) {
    for (i, lever) in Lever::ALL.iter().enumerate() {
        if col.dds[i].open {
            continue;
        }
        let mut rows = choices(*lever, f);
        if rows.is_empty() {
            col.rows[i].clear();
            col.dds[i].options = vec!["—".to_string()];
            col.dds[i].selected = 0;
            continue;
        }
        let shown: Option<String> =
            if active { live(*lever, f) } else { f.plan.get(col.source, *lever).map(str::to_string) };
        if let Some(s) = &shown {
            if !rows.iter().any(|(v, _)| v == s) {
                rows.push((s.clone(), display_of(*lever, s)));
            }
        }
        let mut values = Vec::with_capacity(rows.len() + 1);
        let mut options = Vec::with_capacity(rows.len() + 1);
        if !active {
            values.push(String::new());
            options.push("Not set".to_string());
        }
        for (v, d) in rows {
            values.push(v);
            options.push(d);
        }
        col.dds[i].selected = shown.and_then(|s| values.iter().position(|v| *v == s)).unwrap_or(0);
        col.dds[i].options = options;
        col.rows[i] = values;
    }
}

/// Rebuild every dropdown's options/selection from fresh facts.
fn rebuild_options(state: &mut PowerState) {
    let f = &state.facts;
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
    let active = f.source;
    fill_column(&mut state.ac, f, active == Source::Ac);
    fill_column(&mut state.battery, f, active == Source::Battery);
}

pub fn view(state: &mut PowerState, cx: f32, cy: f32, cw: f32, ch: f32, _root_focused: bool, sec_focused: &[bool], layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let focused = |i: usize| sec_focused.get(i).copied().unwrap_or(false);

    if !state.loaded {
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);
        builder.add_section_spanned(&mut final_pc, "", 1, focused(0), |sec| {
            sec.text("Reading power interfaces...", 12.0, 0.0, 12.0, TEXT_DIM);
        });
        return final_pc;
    }

    let PowerState { facts, dd_limit, ac, battery, .. } = state;
    let sources = columns_shown(facts);
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1 + sources.len());

    // ── Battery: facts, the charge limit, and whether switching is wired up ──
    builder.add_section(&mut final_pc, "Battery", focused(0), |sec| {
        let sec_w = sec.cw;
        let f = &*facts;
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
            sec.text(&line, 12.0, 0.0, 12.0, status_color);

            // Folded in from the System page's battery section, which showed
            // the same pack in a second place until 2026-08-23.
            let mut detail = String::new();
            if let (Some(now), Some(full)) = (f.energy_now_uwh, f.energy_full_uwh) {
                detail.push_str(&format!("{:.1} / {:.1} Wh", now / 1e6, full / 1e6));
            }
            if let Some(secs) = battery_seconds(f) {
                if !detail.is_empty() {
                    detail.push_str("  ·  ");
                }
                let what = if f.status == "Charging" { "to full" } else { "remaining" };
                detail.push_str(&format!("{} {}", humanize_secs(secs), what));
            }
            if !detail.is_empty() {
                sec.text(&detail, 12.0, 0.0, 12.0, TEXT_DIM);
            }

            if let Some(h) = f.health_pct {
                sec.text(
                    &format!("Health: {}% of design capacity", h),
                    12.0,
                    0.0,
                    12.0,
                    if h >= 80 { TEXT_DIM } else { WARN },
                );
            }
            let pack = format!("{} {}", f.vendor, f.model);
            if !pack.trim().is_empty() {
                sec.text(pack.trim(), 11.0, 0.0, 11.0, TEXT_DIM);
            }
        } else {
            sec.text("No battery detected", 12.0, 0.0, 12.0, TEXT_DIM);
        }

        if f.charge_limit.is_some() {
            sec.spacing(10.0);
            let mut stack = sec.vstack(8.0);
            dd_limit.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(dd_limit, sec_w - 28.0, 44.0, ctx);
        }

        if f.battery_present {
            sec.spacing(10.0);
            if f.automation {
                sec.text("Switches automatically on plug and unplug.", 12.0, 0.0, 11.0, TEXT_DIM);
            } else {
                sec.text("Automatic switching is not installed:", 12.0, 0.0, 11.0, WARN);
                sec.text("System › System Files installs it.", 12.0, 0.0, 11.0, WARN);
            }
        }
    });

    // ── One column of levers per power source ──
    for (k, source) in sources.iter().enumerate() {
        let source = *source;
        let active = source == facts.source;
        let label = if facts.battery_present { source.label() } else { "Settings" };
        let col: &mut LeverColumn = match source {
            Source::Ac => &mut *ac,
            Source::Battery => &mut *battery,
        };
        let f = &*facts;
        builder.add_section(&mut final_pc, label, focused(1 + k), |sec| {
            let sec_w = sec.cw;
            if f.battery_present {
                if active {
                    sec.text("Active now — picks apply immediately.", 12.0, 0.0, 11.0, GOOD);
                } else {
                    let when = if source == Source::Battery { "unplugged" } else { "plugged in" };
                    sec.text(&format!("Applied when {}.", when), 12.0, 0.0, 11.0, TEXT_DIM);
                    sec.text("Not set leaves a lever alone.", 12.0, 0.0, 11.0, TEXT_DIM);
                }
                sec.spacing(6.0);
            }
            let mut stack = sec.vstack(8.0);
            for i in 0..Lever::ALL.len() {
                if col.rows[i].is_empty() {
                    continue;
                }
                col.dds[i].set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
                stack.add_widget(&mut col.dds[i], sec_w - 28.0, 44.0, ctx);
            }
        });
    }

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
        PowerMessage::SetLimit(idx) => {
            if let Some(v) = state.limit_values.get(idx).copied() {
                if (1..=100).contains(&v) {
                    run_privileged(format!(
                        "for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do echo {} > \"$f\"; done",
                        v
                    ));
                    state.facts.charge_limit = Some(v);
                }
            }
        }
        PowerMessage::Set { source, lever, idx } => {
            let i = lever_index(lever);
            let Some(value) = state.column_mut(source).rows.get(i).and_then(|r| r.get(idx)).cloned() else {
                return;
            };
            let value: Option<&str> = if value.is_empty() { None } else { Some(value.as_str()) };
            // The row values come from sysfs reads and the plan, but the guard
            // makes the argv safe by construction rather than by data flow.
            if value.is_some_and(|v| !lever.value_ok(v)) {
                return;
            }
            let Some(helper) = helper_path() else {
                log::error!("[power] cce-power-apply not found at {} or beside this binary", power_plan::HELPER_SYSTEM_PATH);
                return;
            };
            // Detached, like run_privileged: the helper records the pick and,
            // when this is the live source, applies it; the watcher's next
            // read reports what actually happened.
            let _ = std::process::Command::new("pkexec")
                .arg(&helper)
                .arg("set")
                .arg(source.key())
                .arg(lever.key())
                .arg(value.unwrap_or("unset"))
                .spawn();
            // Optimistic mirror of what the helper will make true.
            let _ = state.facts.plan.put(source, lever, value);
            if source == state.facts.source {
                if let Some(v) = value {
                    set_live(lever, v, &mut state.facts);
                }
            }
        }
    }
}

impl crate::pages::AppPage for PowerState {
    // Sections: [Battery, <one per shown source>] — ids mirror the view's
    // load gate AND its per-interface presence gates (the d13a901 lesson:
    // never report a widget the view didn't paint).
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        if !self.loaded {
            return vec![Vec::new()];
        }
        let mut out = Vec::new();
        let mut first = Vec::new();
        if self.facts.charge_limit.is_some() {
            first.push(self.dd_limit.id());
        }
        out.push(first);
        for source in columns_shown(&self.facts) {
            let col = self.column_mut(source);
            let ids = (0..Lever::ALL.len())
                .filter(|i| !col.rows[*i].is_empty())
                .map(|i| col.dds[i].id())
                .collect();
            out.push(ids);
        }
        out
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
        if self.dd_limit.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetLimit(self.dd_limit.selected)));
        }
        for col in [&mut self.ac, &mut self.battery] {
            for (i, lever) in Lever::ALL.iter().enumerate() {
                if col.dds[i].take_change() {
                    actions.push(AppAction::Power(PowerMessage::Set {
                        source: col.source,
                        lever: *lever,
                        idx: col.dds[i].selected,
                    }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::AppPage;

    fn facts() -> PowerFacts {
        let mut plan = PowerPlan::default();
        plan.put(Source::Ac, Lever::Profile, Some("performance")).unwrap();
        plan.put(Source::Battery, Lever::Profile, Some("low-power")).unwrap();
        PowerFacts {
            battery_present: true,
            status: "Discharging".to_string(),
            capacity_pct: 92,
            health_pct: Some(83),
            power_w: Some(7.2),
            ac_online: Some(false),
            source: Source::Battery,
            plan,
            automation: true,
            energy_now_uwh: Some(44_900_000.0),
            energy_full_uwh: Some(74_900_000.0),
            vendor: "SMP".to_string(),
            model: "5B11M90061".to_string(),
            profiles: vec!["low-power".into(), "balanced".into(), "performance".into()],
            profile: "balanced".to_string(),
            epps: vec!["default".into(), "performance".into(), "balance_power".into(), "power".into()],
            epp: "balance_power".to_string(),
            charge_limit: Some(80),
            turbo: Some(true),
            governors: vec!["performance".into(), "powersave".into()],
            governor: "powersave".to_string(),
            gpu_limit_w: Some(80),
            gpu_default_w: Some(80),
            gpu_min_w: Some(5),
            igpu_mhz: Some(1500),
            igpu_max_mhz: Some(1500),
            igpu_min_mhz: Some(100),
            aspm_policies: vec!["default".into(), "performance".into(), "powersave".into()],
            aspm: "default".to_string(),
            hda_idle_secs: Some(10),
        }
    }

    fn loaded() -> PowerState {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        st
    }

    const PROFILE: usize = 0;
    const TURBO: usize = 3;
    const IGPU: usize = 4;
    const AUDIO: usize = 6;

    #[test]
    fn active_column_shows_live_values_without_a_not_set_row() {
        let st = loaded();
        // On battery: the battery column is the live one. The plan says
        // low-power for it, but sysfs says balanced, and sysfs is what shows.
        assert_eq!(st.battery.dds[PROFILE].options, ["Low Power", "Balanced", "Performance"]);
        assert_eq!(st.battery.dds[PROFILE].selected, 1);
        assert_eq!(st.battery.rows[PROFILE], ["low-power", "balanced", "performance"]);
        assert_eq!(st.battery.dds[TURBO].selected, 0); // enabled
        assert_eq!(st.battery.rows[TURBO], ["on", "off"]);
    }

    #[test]
    fn inactive_column_shows_the_plan_behind_not_set() {
        let mut st = loaded();
        assert_eq!(st.ac.dds[PROFILE].options, ["Not set", "Low Power", "Balanced", "Performance"]);
        assert_eq!(st.ac.dds[PROFILE].selected, 3); // planned: performance
        assert_eq!(st.ac.rows[PROFILE][0], ""); // the unset row
        // Nothing planned for AC turbo → Not set.
        assert_eq!(st.ac.dds[TURBO].selected, 0);
        // Plugging in swaps which column is live.
        st.facts.source = Source::Ac;
        rebuild_options(&mut st);
        assert_eq!(st.ac.dds[PROFILE].options, ["Low Power", "Balanced", "Performance"]);
        assert_eq!(st.ac.dds[PROFILE].selected, 1);
        assert_eq!(st.battery.dds[PROFILE].options.len(), 4);
        assert_eq!(st.battery.dds[PROFILE].selected, 1); // planned: low-power, after Not set
    }

    #[test]
    fn charge_limit_rows_map_current_and_off_list_values() {
        let mut st = loaded();
        assert_eq!(st.dd_limit.selected, 1); // 80
        assert_eq!(st.limit_values, [100, 80, 60]);
        // An off-list threshold gets its own row instead of a wrong match.
        st.facts.charge_limit = Some(75);
        rebuild_options(&mut st);
        assert_eq!(st.limit_values, [100, 80, 60, 75]);
        assert_eq!(st.dd_limit.selected, 3);
        assert!(st.dd_limit.options[3].contains("75%"));
    }

    #[test]
    fn open_dropdown_is_left_alone_on_refresh() {
        let mut st = loaded();
        st.battery.dds[PROFILE].open = true;
        st.battery.dds[PROFILE].selected = 2;
        let mut newer = facts();
        newer.profile = "low-power".to_string();
        st.facts = newer;
        rebuild_options(&mut st);
        // Open menu untouched; the others refreshed.
        assert_eq!(st.battery.dds[PROFILE].selected, 2);
    }

    #[test]
    fn battery_time_is_energy_over_draw_and_follows_direction() {
        // sysfs has no time field — this replaces what UPower used to compute
        // for the System page's battery section.
        let mut f = facts();
        f.status = "Discharging".to_string();
        f.power_w = Some(50.0);
        // 44.9 Wh left at 50 W ≈ 53.9 min.
        assert_eq!(humanize_secs(battery_seconds(&f).unwrap()), "53m");

        // Charging counts the GAP to full, not what is already in the pack.
        f.status = "Charging".to_string();
        // (74.9 - 44.9) = 30 Wh at 50 W = 36 min.
        assert_eq!(humanize_secs(battery_seconds(&f).unwrap()), "36m");

        // Idle on AC, or no draw at all, has no meaningful estimate.
        f.status = "Full".to_string();
        assert_eq!(battery_seconds(&f), None);
        f.status = "Discharging".to_string();
        f.power_w = Some(0.0);
        assert_eq!(battery_seconds(&f), None);
    }

    #[test]
    fn humanize_secs_splits_hours() {
        assert_eq!(humanize_secs(54 * 60), "54m");
        assert_eq!(humanize_secs(3 * 3600 + 7 * 60), "3h 7m");
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
        // Not loaded: one section, nothing reported (the view paints only the
        // loading line).
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        let counts = |st: &mut PowerState| st.section_widgets().iter().map(Vec::len).collect::<Vec<_>>();
        // Every interface present: the charge limit, then all eight levers in
        // each of the two source columns.
        assert_eq!(counts(&mut st), [1, 8, 8]);
        // A host without a charge-limit knob or turbo file reports fewer.
        st.facts.charge_limit = None;
        st.facts.turbo = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 7, 7]);
        // No cpufreq governors and no NVIDIA driver: both drop out too.
        st.facts.governors.clear();
        st.facts.gpu_limit_w = None;
        st.facts.gpu_default_w = None;
        st.facts.gpu_min_w = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 5, 5]);
        // No Intel render clocks, an ASPM-less kernel and no snd_hda_intel is
        // down to profile and epp.
        st.facts.igpu_max_mhz = None;
        st.facts.igpu_min_mhz = None;
        st.facts.aspm_policies.clear();
        st.facts.hda_idle_secs = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 2, 2]);
        // A desktop: no battery, so only one source column exists.
        st.facts.battery_present = false;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 2]);
    }

    #[test]
    fn igpu_rows_come_from_the_hardware_range() {
        let mut st = loaded();
        // RP0, the rounded midpoint, RPn — no invented numbers; the live column
        // has no Not set row.
        assert_eq!(st.battery.rows[IGPU], ["1500", "800", "100"]);
        assert_eq!(st.battery.dds[IGPU].selected, 0);
        // A live cap that is none of the three earns its own row.
        st.facts.igpu_mhz = Some(1200);
        rebuild_options(&mut st);
        assert_eq!(st.battery.rows[IGPU], ["1500", "800", "100", "1200"]);
        assert_eq!(st.battery.dds[IGPU].selected, 3);
        assert_eq!(st.battery.dds[IGPU].options[3], "1200 MHz");
        // And so does a planned value in the other column.
        st.facts.plan.put(Source::Ac, Lever::IgpuMaxMhz, Some("1300")).unwrap();
        rebuild_options(&mut st);
        assert_eq!(st.ac.rows[IGPU], ["", "1500", "800", "100", "1300"]);
        assert_eq!(st.ac.dds[IGPU].selected, 4);
    }

    #[test]
    fn audio_rows_are_the_three_timeouts_plus_an_off_list_current() {
        let mut st = loaded();
        assert_eq!(st.battery.rows[AUDIO], ["0", "1", "10"]);
        assert_eq!(st.battery.dds[AUDIO].options[0], "Never suspend");
        assert_eq!(st.battery.dds[AUDIO].selected, 2);
        st.facts.hda_idle_secs = Some(30);
        rebuild_options(&mut st);
        assert_eq!(st.battery.rows[AUDIO], ["0", "1", "10", "30"]);
        assert_eq!(st.battery.dds[AUDIO].options[3], "After 30 s idle");
    }

    #[test]
    fn aspm_current_is_the_bracketed_policy() {
        // fetch strips the brackets; the selection must land on the active one.
        let mut st = loaded();
        st.facts.aspm = "powersave".to_string();
        rebuild_options(&mut st);
        let i = lever_index(Lever::Aspm);
        assert_eq!(st.battery.dds[i].selected, 2);
        assert_eq!(st.battery.dds[i].options[2], "Powersave");
    }

    #[test]
    fn gpu_rows_are_default_min_and_an_off_list_current() {
        let mut st = loaded();
        let i = lever_index(Lever::GpuLimitW);
        // Current == default: two rows, no duplicate.
        assert_eq!(st.battery.rows[i], ["80", "5"]);
        assert_eq!(st.battery.dds[i].selected, 0);
        // A current limit that is neither default nor minimum earns its own
        // row rather than silently selecting the wrong one.
        st.facts.gpu_limit_w = Some(60);
        rebuild_options(&mut st);
        assert_eq!(st.battery.rows[i], ["80", "5", "60"]);
        assert_eq!(st.battery.dds[i].selected, 2);
    }

    #[test]
    fn set_live_mirrors_each_lever() {
        let mut f = facts();
        set_live(Lever::Profile, "performance", &mut f);
        set_live(Lever::Turbo, "off", &mut f);
        set_live(Lever::IgpuMaxMhz, "800", &mut f);
        set_live(Lever::GpuLimitW, "40", &mut f);
        assert_eq!(f.profile, "performance");
        assert_eq!(f.turbo, Some(false));
        assert_eq!(f.igpu_mhz, Some(800));
        assert_eq!(f.gpu_limit_w, Some(40));
        // Round trip: what set_live wrote is what live() reads back.
        assert_eq!(live(Lever::Turbo, &f).as_deref(), Some("off"));
        assert_eq!(live(Lever::GpuLimitW, &f).as_deref(), Some("40"));
    }

}
