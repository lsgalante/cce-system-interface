//! Power: battery facts plus the host's real battery-life levers, all sysfs,
//! organized as named power modes and an assignment of a mode to each power
//! adapter state.
//!
//! Every control is discovered from the interfaces this machine actually
//! exposes (missing ones render as absent, not as dead widgets):
//! - `/sys/firmware/acpi/platform_profile` — firmware power profile
//! - `/sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`
//! - `/sys/class/power_supply/BAT*/charge_control_end_threshold` — capping
//!   charge at 80% is the classic battery-longevity lever
//! - `/sys/devices/system/cpu/intel_pstate/no_turbo` — turbo boost
//!
//! The page is three sections. **Battery** is the pack itself: its facts and
//! the charge limit, which is a charging policy and so belongs to no mode.
//! **Power Mode** edits one mode's levers, chosen by the dropdown at the top
//! of the section — one section rather than one per mode, so the levers sit
//! in the same place whichever mode is being edited. **Mode Assignment**
//! says which mode runs plugged in and which on battery, one dropdown per
//! adapter state.
//!
//! A "Not set" row means "leave that lever alone"; when the edited mode is
//! the one running right now, the row also reports the live sysfs value, and
//! a pick applies immediately. Everything is remembered in the plan
//! (`crate::power_plan`, `/etc/cce/power.kdl`) through `cce-power-apply`
//! under pkexec — the one-prompt path every privileged action in this app
//! takes — and the same helper re-applies the assigned mode from udev when
//! the charger comes or goes. The UI is optimistic and the 5s watcher
//! re-reads the truth, so a dismissed auth prompt reverts the dropdown —
//! honest, with no extra error channel.

use crate::app::{AppAction, PageContent};
use crate::power_plan::{self, Automation, Lever, Mode, PowerPlan, Source};
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
    /// The plan on disk, and the state of the root side that applies it on
    /// plug/unplug (helper + udev rule).
    pub plan: PowerPlan,
    pub automation: Automation,
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


/// The edited mode's column of lever dropdowns. One set, not one per mode:
/// the mode dropdown above it decides whose values it is showing.
#[derive(Debug, Clone)]
pub struct LeverSet {
    /// One dropdown per [`Lever::ALL`] entry, in that order.
    pub dds: Vec<Adapted<Dropdown>>,
    /// The plan value behind each row of each dropdown (options are display
    /// text). Row 0 is always the "Not set" row and holds the empty string;
    /// an empty Vec means the interface is absent on this host and the
    /// dropdown is not painted.
    pub rows: Vec<Vec<String>>,
}

impl Default for LeverSet {
    fn default() -> Self {
        Self {
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
    /// Which mode the lever section is editing. Page state, not plan state:
    /// it says what is on screen, never what the machine runs.
    pub editing: Mode,
    pub dd_mode: Adapted<Dropdown>,
    pub levers: LeverSet,
    /// One mode picker per [`Source::ALL`] entry, in that order.
    pub dd_assign: Vec<Adapted<Dropdown>>,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            loaded: false,
            facts: PowerFacts::default(),
            dd_limit: Dropdown::new(vec!["—".to_string()], 0).with_label("Battery Charge Limit"),
            limit_values: Vec::new(),
            editing: Mode::default(),
            dd_mode: Dropdown::new(mode_options(), 0).with_label("Mode"),
            levers: LeverSet::default(),
            dd_assign: Source::ALL
                .iter()
                .map(|s| Dropdown::new(mode_options(), 0).with_label(s.label()))
                .collect(),
        }
    }
}

fn mode_options() -> Vec<String> {
    Mode::ALL.iter().map(|m| m.label().to_string()).collect()
}

/// Which adapter states the assignment section offers. A host with no
/// battery has one, so the battery row would be an assignment for a state it
/// never enters.
fn sources_shown(f: &PowerFacts) -> Vec<Source> {
    if f.battery_present { vec![Source::Ac, Source::Battery] } else { vec![Source::Ac] }
}

/// Whether the assignment section is worth painting at all — on a host with
/// a single adapter state there is nothing to choose between.
fn assignment_shown(f: &PowerFacts) -> bool {
    sources_shown(f).len() > 1
}

#[derive(Debug, Clone)]
pub enum PowerMessage {
    Refreshed(PowerFacts),
    /// Charge-limit pick, by option index.
    SetLimit(usize),
    /// Which mode the lever section edits, by option index. Page-local: it
    /// writes nothing and applies nothing.
    EditMode(usize),
    /// A lever pick on the mode being edited, by option index.
    Set { lever: Lever, idx: usize },
    /// Which mode an adapter state runs, by option index.
    Assign { source: Source, idx: usize },
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
/// `ccebuild install-system` has put a current one there, else the one
/// installed beside this binary (`~/.local/bin`) — which pkexec will still
/// run as root after the prompt, so the plan works before the root side is
/// installed; only the automatic switching waits on it.
///
/// The system copy has to speak the current CLI to be preferred. A stale
/// one there is worse than none: it takes the pkexec prompt, reads the mode
/// name as an adapter state and exits 2, so every pick costs an
/// authentication and changes nothing. Falling through to the local copy
/// keeps the page working; the Battery section is where the user is told
/// the root side is behind.
fn helper_path() -> Option<PathBuf> {
    let sys = Path::new(power_plan::HELPER_SYSTEM_PATH);
    if sys.exists() && power_plan::helper_speaks_modes(sys) {
        return Some(sys.to_path_buf());
    }
    let beside = std::env::current_exe().ok()?.parent()?.join("cce-power-apply");
    if beside.exists() {
        return Some(beside);
    }
    sys.exists().then(|| sys.to_path_buf())
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
    f.automation = power_plan::automation_status();

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

/// Rebuild the lever dropdowns for the mode being edited. Every lever leads
/// with a "Not set" row meaning "leave it alone"; when the edited mode is
/// the one running right now that row also names the live sysfs value, which
/// is where the page reports what the machine is actually doing. A planned
/// value missing from the host's list gets appended as its own row. Skipped
/// per dropdown while it is open (the default_apps rule: never yank an open
/// menu out from under the pointer — the next refresh normalizes it).
fn fill_levers(levers: &mut LeverSet, f: &PowerFacts, mode: Mode) {
    let running = mode == f.plan.assigned(f.source);
    for (i, lever) in Lever::ALL.iter().enumerate() {
        if levers.dds[i].open {
            continue;
        }
        let mut rows = choices(*lever, f);
        if rows.is_empty() {
            levers.rows[i].clear();
            levers.dds[i].options = vec!["—".to_string()];
            levers.dds[i].selected = 0;
            continue;
        }
        let planned: Option<String> = f.plan.get(mode, *lever).map(str::to_string);
        if let Some(s) = &planned {
            if !rows.iter().any(|(v, _)| v == s) {
                rows.push((s.clone(), display_of(*lever, s)));
            }
        }
        let mut values = Vec::with_capacity(rows.len() + 1);
        let mut options = Vec::with_capacity(rows.len() + 1);
        values.push(String::new());
        options.push(match live(*lever, f) {
            Some(cur) if running => format!("Not set — now {}", display_of(*lever, &cur)),
            _ => "Not set".to_string(),
        });
        for (v, d) in rows {
            values.push(v);
            options.push(d);
        }
        levers.dds[i].selected = planned.and_then(|s| values.iter().position(|v| *v == s)).unwrap_or(0);
        levers.dds[i].options = options;
        levers.rows[i] = values;
    }
}

fn mode_index(mode: Mode) -> usize {
    Mode::ALL.iter().position(|m| *m == mode).unwrap()
}

fn source_index(source: Source) -> usize {
    Source::ALL.iter().position(|s| *s == source).unwrap()
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
    if !state.dd_mode.open {
        state.dd_mode.options = mode_options();
        state.dd_mode.selected = mode_index(state.editing);
    }
    fill_levers(&mut state.levers, &state.facts, state.editing);
    for source in Source::ALL {
        let i = source_index(source);
        if state.dd_assign[i].open {
            continue;
        }
        state.dd_assign[i].options = mode_options();
        state.dd_assign[i].selected = mode_index(state.facts.plan.assigned(source));
    }
}

/// How the page says when a mode runs, in a sentence.
fn when_text(source: Source) -> &'static str {
    match source {
        Source::Ac => "plugged in",
        Source::Battery => "unplugged",
    }
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

    let PowerState { facts, dd_limit, editing, dd_mode, levers, dd_assign, .. } = state;
    let editing = *editing;
    let sources = sources_shown(facts);
    let show_assign = assignment_shown(facts);
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w)
        .with_section_count(if show_assign { 3 } else { 2 });

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
            match f.automation {
                Automation::Ready => {
                    sec.text("Switches automatically on plug and unplug.", 12.0, 0.0, 11.0, TEXT_DIM);
                }
                Automation::Missing => {
                    sec.text("Automatic switching is not installed:", 12.0, 0.0, 11.0, WARN);
                    sec.text("System › System Files installs it.", 12.0, 0.0, 11.0, WARN);
                }
                Automation::Stale => {
                    sec.text("Automatic switching is out of date and", 12.0, 0.0, 11.0, WARN);
                    sec.text("applies nothing on plug or unplug.", 12.0, 0.0, 11.0, WARN);
                    sec.text("Run: ccebuild install-system", 12.0, 0.0, 11.0, WARN);
                }
            }
        }
    });

    // ── The edited mode's levers, behind the picker that chooses it ──
    {
        let f = &*facts;
        let running = f.plan.assigned(f.source) == editing;
        let applies_when: Vec<&str> = sources
            .iter()
            .filter(|s| f.plan.assigned(**s) == editing)
            .map(|s| when_text(*s))
            .collect();
        builder.add_section(&mut final_pc, "Power Mode", focused(1), |sec| {
            let sec_w = sec.cw;
            {
                let mut stack = sec.vstack(8.0);
                dd_mode.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
                stack.add_widget(dd_mode, sec_w - 28.0, 44.0, ctx);
            }
            sec.spacing(4.0);
            if running {
                sec.text("Running now — picks apply immediately.", 12.0, 0.0, 11.0, GOOD);
            } else if applies_when.is_empty() {
                sec.text("Assigned to no adapter state.", 12.0, 0.0, 11.0, TEXT_DIM);
            } else {
                sec.text(&format!("Applied when {}.", applies_when.join(" and ")), 12.0, 0.0, 11.0, TEXT_DIM);
            }
            sec.text("Not set leaves a lever alone.", 12.0, 0.0, 11.0, TEXT_DIM);
            sec.spacing(6.0);
            let mut stack = sec.vstack(8.0);
            for i in 0..Lever::ALL.len() {
                if levers.rows[i].is_empty() {
                    continue;
                }
                levers.dds[i].set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
                stack.add_widget(&mut levers.dds[i], sec_w - 28.0, 44.0, ctx);
            }
        });
    }

    // ── Which mode each power adapter state runs ──
    if show_assign {
        let f = &*facts;
        builder.add_section(&mut final_pc, "Mode Assignment", focused(2), |sec| {
            let sec_w = sec.cw;
            sec.text("Which mode runs in each adapter state.", 12.0, 0.0, 11.0, TEXT_DIM);
            sec.text(&format!("{} right now.", f.source.label()), 12.0, 0.0, 11.0, GOOD);
            sec.spacing(6.0);
            let mut stack = sec.vstack(8.0);
            for source in sources.iter().copied() {
                let dd = &mut dd_assign[source_index(source)];
                dd.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
                stack.add_widget(dd, sec_w - 28.0, 44.0, ctx);
            }
        });
    }

    final_pc
}

pub fn update(state: &mut PowerState, msg: PowerMessage) {
    match msg {
        PowerMessage::Refreshed(facts) => {
            // The page opens on the mode the machine is actually running, so
            // the first thing on screen describes the present rather than a
            // mode nothing is using. Only the first read moves it — after
            // that the pick is the user's.
            if !state.loaded {
                state.editing = facts.plan.assigned(facts.source);
            }
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
        PowerMessage::EditMode(idx) => {
            // Page-local: switching which mode is on screen writes nothing
            // and applies nothing, so it needs no privileged call.
            if let Some(mode) = Mode::ALL.get(idx).copied() {
                state.editing = mode;
                rebuild_options(state);
            }
        }
        PowerMessage::Set { lever, idx } => {
            let i = lever_index(lever);
            let Some(value) = state.levers.rows.get(i).and_then(|r| r.get(idx)).cloned() else {
                return;
            };
            let value: Option<&str> = if value.is_empty() { None } else { Some(value.as_str()) };
            // The row values come from sysfs reads and the plan, but the guard
            // makes the argv safe by construction rather than by data flow.
            if value.is_some_and(|v| !lever.value_ok(v)) {
                return;
            }
            let mode = state.editing;
            let Some(helper) = helper_path() else {
                log::error!("[power] cce-power-apply not found at {} or beside this binary", power_plan::HELPER_SYSTEM_PATH);
                return;
            };
            // Detached, like run_privileged: the helper records the pick and,
            // when this mode is the running one, applies it; the watcher's
            // next read reports what actually happened.
            let _ = std::process::Command::new("pkexec")
                .arg(&helper)
                .arg("set")
                .arg(mode.key())
                .arg(lever.key())
                .arg(value.unwrap_or("unset"))
                .spawn();
            // Optimistic mirror of what the helper will make true.
            let _ = state.facts.plan.put(mode, lever, value);
            if mode == state.facts.plan.assigned(state.facts.source) {
                if let Some(v) = value {
                    set_live(lever, v, &mut state.facts);
                }
            }
            rebuild_options(state);
        }
        PowerMessage::Assign { source, idx } => {
            let Some(mode) = Mode::ALL.get(idx).copied() else {
                return;
            };
            let Some(helper) = helper_path() else {
                log::error!("[power] cce-power-apply not found at {} or beside this binary", power_plan::HELPER_SYSTEM_PATH);
                return;
            };
            let _ = std::process::Command::new("pkexec")
                .arg(&helper)
                .arg("assign")
                .arg(source.key())
                .arg(mode.key())
                .spawn();
            state.facts.plan.assign(source, mode);
            // Reassigning the live state hands the machine to a different
            // mode; mirror its levers so the page agrees with what the helper
            // is applying until the watcher's next read.
            if source == state.facts.source {
                let values: Vec<(Lever, String)> =
                    state.facts.plan.levers(mode).map(|(l, v)| (l, v.to_string())).collect();
                for (lever, value) in values {
                    set_live(lever, &value, &mut state.facts);
                }
            }
            rebuild_options(state);
        }
    }
}

impl crate::pages::AppPage for PowerState {
    // Sections: [Battery, Power Mode, Mode Assignment] — ids mirror the
    // view's load gate AND its per-interface presence gates (the d13a901
    // lesson: never report a widget the view didn't paint).
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
        let mut mode_sec = vec![self.dd_mode.id()];
        mode_sec.extend(
            (0..Lever::ALL.len())
                .filter(|i| !self.levers.rows[*i].is_empty())
                .map(|i| self.levers.dds[i].id()),
        );
        out.push(mode_sec);
        if assignment_shown(&self.facts) {
            let ids = sources_shown(&self.facts)
                .into_iter()
                .map(|s| self.dd_assign[source_index(s)].id())
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
        if self.dd_mode.take_change() {
            actions.push(AppAction::Power(PowerMessage::EditMode(self.dd_mode.selected)));
        }
        for (i, lever) in Lever::ALL.iter().enumerate() {
            if self.levers.dds[i].take_change() {
                actions.push(AppAction::Power(PowerMessage::Set {
                    lever: *lever,
                    idx: self.levers.dds[i].selected,
                }));
            }
        }
        for source in Source::ALL {
            let i = source_index(source);
            if self.dd_assign[i].take_change() {
                actions.push(AppAction::Power(PowerMessage::Assign {
                    source,
                    idx: self.dd_assign[i].selected,
                }));
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
        plan.put(Mode::Performance, Lever::Profile, Some("performance")).unwrap();
        plan.put(Mode::PowerSaver, Lever::Profile, Some("low-power")).unwrap();
        PowerFacts {
            battery_present: true,
            status: "Discharging".to_string(),
            capacity_pct: 92,
            health_pct: Some(83),
            power_w: Some(7.2),
            ac_online: Some(false),
            source: Source::Battery,
            plan,
            automation: Automation::Ready,
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

    /// Loaded, editing whichever mode the machine is running — which is what
    /// the page opens on.
    fn loaded() -> PowerState {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        st.editing = st.facts.plan.assigned(st.facts.source);
        rebuild_options(&mut st);
        st
    }

    const PROFILE: usize = 0;
    const TURBO: usize = 3;
    const IGPU: usize = 4;
    const AUDIO: usize = 6;

    #[test]
    fn the_lever_section_shows_the_edited_mode_behind_not_set() {
        let st = loaded();
        // On battery, so Power Saver is running; its plan says low-power.
        assert_eq!(st.editing, Mode::PowerSaver);
        assert_eq!(st.dd_mode.options, ["Performance", "Balanced", "Power Saver"]);
        assert_eq!(st.dd_mode.selected, 2);
        assert_eq!(st.levers.rows[PROFILE], ["", "low-power", "balanced", "performance"]);
        assert_eq!(st.levers.dds[PROFILE].selected, 1);
        // Nothing planned for turbo in this mode → Not set.
        assert_eq!(st.levers.dds[TURBO].selected, 0);
        assert_eq!(st.levers.rows[TURBO], ["", "on", "off"]);
    }

    #[test]
    fn the_running_mode_reports_the_live_value_on_its_not_set_row() {
        let mut st = loaded();
        // Power Saver is running: sysfs says balanced and turbo on, and the
        // Not set row is where the page says so.
        assert_eq!(st.levers.dds[PROFILE].options[0], "Not set — now Balanced");
        assert_eq!(st.levers.dds[TURBO].options[0], "Not set — now Enabled");
        // A mode that is not running has no live value to report.
        st.editing = Mode::Balanced;
        rebuild_options(&mut st);
        assert_eq!(st.levers.dds[PROFILE].options[0], "Not set");
        assert_eq!(st.levers.dds[PROFILE].selected, 0);
    }

    #[test]
    fn the_page_opens_on_the_mode_the_machine_is_running() {
        let mut st = PowerState::default();
        // Default state edits Balanced; the first read is on battery, which
        // runs Power Saver.
        assert_eq!(st.editing, Mode::Balanced);
        update(&mut st, PowerMessage::Refreshed(facts()));
        assert_eq!(st.editing, Mode::PowerSaver);
        assert_eq!(st.dd_mode.selected, mode_index(Mode::PowerSaver));
        // A later read does not yank the section away from the user's pick.
        update(&mut st, PowerMessage::EditMode(mode_index(Mode::Performance)));
        let mut plugged = facts();
        plugged.source = Source::Ac;
        update(&mut st, PowerMessage::Refreshed(plugged));
        assert_eq!(st.editing, Mode::Performance);
    }

    #[test]
    fn switching_the_edited_mode_swaps_the_lever_values() {
        let mut st = loaded();
        assert_eq!(st.levers.dds[PROFILE].selected, 1); // low-power
        update(&mut st, PowerMessage::EditMode(mode_index(Mode::Performance)));
        assert_eq!(st.editing, Mode::Performance);
        assert_eq!(st.dd_mode.selected, 0);
        assert_eq!(st.levers.dds[PROFILE].selected, 3); // performance
        update(&mut st, PowerMessage::EditMode(mode_index(Mode::Balanced)));
        assert_eq!(st.levers.dds[PROFILE].selected, 0); // nothing planned
        // Editing is page state: it changes no assignment and no plan.
        assert_eq!(st.facts.plan.assigned(Source::Battery), Mode::PowerSaver);
        assert_eq!(st.facts.plan.get(Mode::Balanced, Lever::Profile), None);
    }

    #[test]
    fn assignment_dropdowns_follow_the_plan_and_pick_a_mode_per_state() {
        let mut st = loaded();
        assert_eq!(st.dd_assign[source_index(Source::Ac)].selected, mode_index(Mode::Performance));
        assert_eq!(st.dd_assign[source_index(Source::Battery)].selected, mode_index(Mode::PowerSaver));
        // Reassigning the live state hands the machine to that mode, and the
        // lever section — still editing Power Saver — stops claiming to run.
        update(
            &mut st,
            PowerMessage::Assign { source: Source::Battery, idx: mode_index(Mode::Balanced) },
        );
        assert_eq!(st.facts.plan.assigned(Source::Battery), Mode::Balanced);
        assert_eq!(st.dd_assign[source_index(Source::Battery)].selected, mode_index(Mode::Balanced));
        assert_eq!(st.editing, Mode::PowerSaver);
        assert_eq!(st.levers.dds[PROFILE].options[0], "Not set");
        // Both states may run the same mode.
        update(
            &mut st,
            PowerMessage::Assign { source: Source::Ac, idx: mode_index(Mode::Balanced) },
        );
        assert_eq!(st.facts.plan.assigned(Source::Ac), Mode::Balanced);
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
        st.levers.dds[PROFILE].open = true;
        st.levers.dds[PROFILE].selected = 2;
        let mut newer = facts();
        newer.profile = "low-power".to_string();
        st.facts = newer;
        rebuild_options(&mut st);
        // Open menu untouched; the others refreshed.
        assert_eq!(st.levers.dds[PROFILE].selected, 2);
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
        // Every interface present: the charge limit, the mode picker plus all
        // eight levers, and one assignment per adapter state.
        assert_eq!(counts(&mut st), [1, 9, 2]);
        // A host without a charge-limit knob or turbo file reports fewer.
        st.facts.charge_limit = None;
        st.facts.turbo = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 8, 2]);
        // No cpufreq governors and no NVIDIA driver: both drop out too.
        st.facts.governors.clear();
        st.facts.gpu_limit_w = None;
        st.facts.gpu_default_w = None;
        st.facts.gpu_min_w = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 6, 2]);
        // No Intel render clocks, an ASPM-less kernel and no snd_hda_intel is
        // down to profile and epp.
        st.facts.igpu_max_mhz = None;
        st.facts.igpu_min_mhz = None;
        st.facts.aspm_policies.clear();
        st.facts.hda_idle_secs = None;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 3, 2]);
        // A desktop: one adapter state, so there is nothing to assign.
        st.facts.battery_present = false;
        rebuild_options(&mut st);
        assert_eq!(counts(&mut st), [0, 3]);
    }

    #[test]
    fn igpu_rows_come_from_the_hardware_range() {
        let mut st = loaded();
        // RP0, the rounded midpoint, RPn — no invented numbers, behind the
        // Not set row.
        assert_eq!(st.levers.rows[IGPU], ["", "1500", "800", "100"]);
        assert_eq!(st.levers.dds[IGPU].selected, 0);
        // A planned cap that is none of the three earns its own row.
        st.facts.plan.put(Mode::PowerSaver, Lever::IgpuMaxMhz, Some("1300")).unwrap();
        rebuild_options(&mut st);
        assert_eq!(st.levers.rows[IGPU], ["", "1500", "800", "100", "1300"]);
        assert_eq!(st.levers.dds[IGPU].selected, 4);
        assert_eq!(st.levers.dds[IGPU].options[4], "1300 MHz");
        // And a live cap off the list still shows on the Not set row.
        st.facts.igpu_mhz = Some(1200);
        rebuild_options(&mut st);
        assert_eq!(st.levers.dds[IGPU].options[0], "Not set — now 1200 MHz");
    }

    #[test]
    fn audio_rows_are_the_three_timeouts_behind_not_set() {
        let mut st = loaded();
        assert_eq!(st.levers.rows[AUDIO], ["", "0", "1", "10"]);
        assert_eq!(st.levers.dds[AUDIO].options[1], "Never suspend");
        assert_eq!(st.levers.dds[AUDIO].selected, 0);
        st.facts.plan.put(Mode::PowerSaver, Lever::AudioIdleSecs, Some("30")).unwrap();
        rebuild_options(&mut st);
        assert_eq!(st.levers.rows[AUDIO], ["", "0", "1", "10", "30"]);
        assert_eq!(st.levers.dds[AUDIO].options[4], "After 30 s idle");
    }

    #[test]
    fn aspm_rows_come_from_the_kernels_own_list() {
        let mut st = loaded();
        let i = lever_index(Lever::Aspm);
        st.facts.plan.put(Mode::PowerSaver, Lever::Aspm, Some("powersave")).unwrap();
        rebuild_options(&mut st);
        // fetch strips the brackets; the selection lands behind Not set.
        assert_eq!(st.levers.rows[i], ["", "default", "performance", "powersave"]);
        assert_eq!(st.levers.dds[i].selected, 3);
        assert_eq!(st.levers.dds[i].options[3], "Powersave");
    }

    #[test]
    fn gpu_rows_are_default_and_min_with_an_off_list_live_value() {
        let mut st = loaded();
        let i = lever_index(Lever::GpuLimitW);
        // Current == default: two rows, no duplicate.
        assert_eq!(st.levers.rows[i], ["", "80", "5"]);
        assert_eq!(st.levers.dds[i].selected, 0);
        // A live limit that is neither default nor minimum is reported on the
        // Not set row rather than silently selecting the wrong one.
        st.facts.gpu_limit_w = Some(60);
        rebuild_options(&mut st);
        assert_eq!(st.levers.rows[i], ["", "80", "5"]);
        assert_eq!(st.levers.dds[i].selected, 0);
        assert_eq!(st.levers.dds[i].options[0], "Not set — now 60 W");
    }

    #[test]
    fn the_three_sections_paint_and_the_assignment_one_drops_on_a_desktop() {
        let mut ctx = cce_ui::context::UiContext::new();
        let mut paint = |st: &mut PowerState, sections: usize| {
            let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
            let sec_focused = vec![false; sections];
            let pc = st.view(10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
            assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
        };
        let mut st = loaded();
        paint(&mut st, 3);
        // A desktop has one adapter state and so nothing to assign.
        st.facts.battery_present = false;
        rebuild_options(&mut st);
        paint(&mut st, 2);
        // And the loading gate paints its one line.
        let mut empty = PowerState::default();
        paint(&mut empty, 1);
    }

    #[test]
    fn a_stale_root_helper_is_named_on_the_page() {
        let mut ctx = cce_ui::context::UiContext::new();
        let mut lines = |st: &mut PowerState| -> Vec<String> {
            let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
            let sec_focused = vec![false; 3];
            let pc = st.view(10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
            pc.texts.iter().map(|t| t.0.clone()).collect()
        };
        let mut st = loaded();
        assert!(lines(&mut st).iter().any(|l| l.contains("Switches automatically")));
        // A helper too old to read a plan with modes applies nothing on plug
        // or unplug, and the page says so rather than claiming it switches.
        st.facts.automation = Automation::Stale;
        let out = lines(&mut st);
        assert!(out.iter().any(|l| l.contains("out of date")), "{out:?}");
        assert!(out.iter().any(|l| l.contains("ccebuild install-system")), "{out:?}");
        st.facts.automation = Automation::Missing;
        assert!(lines(&mut st).iter().any(|l| l.contains("not installed")));
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
