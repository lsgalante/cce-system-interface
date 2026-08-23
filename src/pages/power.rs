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
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
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

#[derive(Debug, Clone)]
pub struct PowerState {
    pub loaded: bool,
    pub facts: PowerFacts,
    pub dd_profile: cce_ui::widget::Adapted<Dropdown>,
    pub dd_epp: cce_ui::widget::Adapted<Dropdown>,
    pub dd_limit: cce_ui::widget::Adapted<Dropdown>,
    pub dd_turbo: cce_ui::widget::Adapted<Dropdown>,
    pub dd_governor: cce_ui::widget::Adapted<Dropdown>,
    pub dd_gpu: cce_ui::widget::Adapted<Dropdown>,
    pub dd_igpu: cce_ui::widget::Adapted<Dropdown>,
    pub dd_aspm: cce_ui::widget::Adapted<Dropdown>,
    pub dd_audio: cce_ui::widget::Adapted<Dropdown>,
    /// Sysfs value per charge-limit dropdown row (options are display text).
    pub limit_values: Vec<u32>,
    /// Watts per GPU-limit dropdown row (options are display text).
    pub gpu_values: Vec<u32>,
    /// MHz per iGPU-clock row, and idle seconds per audio row.
    pub igpu_values: Vec<u32>,
    pub audio_values: Vec<u32>,
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
            dd_governor: Dropdown::new(vec!["—".to_string()], 0).with_label("CPU Governor"),
            dd_gpu: Dropdown::new(vec!["—".to_string()], 0).with_label("GPU Power Limit"),
            dd_igpu: Dropdown::new(vec!["—".to_string()], 0).with_label("Integrated GPU Max Clock"),
            dd_aspm: Dropdown::new(vec!["—".to_string()], 0).with_label("PCIe Power Management"),
            dd_audio: Dropdown::new(vec!["—".to_string()], 0).with_label("Audio Codec Idle"),
            limit_values: Vec::new(),
            gpu_values: Vec::new(),
            igpu_values: Vec::new(),
            audio_values: Vec::new(),
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
    SetGovernor(usize),
    SetGpuLimit(usize),
    SetIgpuClock(usize),
    SetAspm(usize),
    SetAudioIdle(usize),
}

/// Sysfs tokens travel into a `pkexec sh -c` line, so only the shapes sysfs
/// itself produces are allowed through — anything else is dropped, not
/// escaped. (The lists come from sysfs reads, but the guard makes the write
/// path safe by construction rather than by data-flow argument.)
fn sysfs_token_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Root action via pkexec, the app's standard privileged path. Detached: the
/// polkit prompt runs in its own process, the UI never blocks, and the
/// watcher's next read reports what actually happened. Most callers echo into
/// sysfs; the GPU limit shells out to nvidia-smi, which is why this is not
/// named for sysfs.
fn run_privileged(cmd: String) {
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
        f.energy_now_uwh = b("energy_now").and_then(|s| s.parse().ok());
        f.energy_full_uwh = full;
        f.vendor = b("manufacturer").unwrap_or_default();
        f.model = b("model_name").unwrap_or_default();
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
    if !state.dd_governor.open {
        state.dd_governor.options = f.governors.iter().map(|g| pretty(g)).collect();
        state.dd_governor.selected =
            f.governors.iter().position(|g| *g == f.governor).unwrap_or(0);
    }
    if !state.dd_gpu.open {
        // Default and minimum, plus the current draw when it is neither — the
        // charge-limit rule: an off-list value gets its own row rather than
        // silently matching the wrong one.
        let mut vals: Vec<u32> = Vec::new();
        if let Some(d) = f.gpu_default_w { vals.push(d); }
        if let Some(m) = f.gpu_min_w { if !vals.contains(&m) { vals.push(m); } }
        if let Some(c) = f.gpu_limit_w { if !vals.contains(&c) { vals.push(c); } }
        state.dd_gpu.options = vals
            .iter()
            .map(|w| {
                if Some(*w) == f.gpu_default_w { format!("{} W  (default)", w) }
                else if Some(*w) == f.gpu_min_w { format!("{} W  (minimum)", w) }
                else { format!("{} W", w) }
            })
            .collect();
        state.dd_gpu.selected = f
            .gpu_limit_w
            .and_then(|c| vals.iter().position(|v| *v == c))
            .unwrap_or(0);
        state.gpu_values = vals;
    }
    if !state.dd_igpu.open {
        // Ceiling, midpoint and floor, all from the hardware's own RP0/RPn —
        // no invented numbers, and a current cap that is none of them keeps its
        // own row (the charge-limit rule).
        let mut vals: Vec<u32> = Vec::new();
        if let (Some(hi), Some(lo)) = (f.igpu_max_mhz, f.igpu_min_mhz) {
            vals.push(hi);
            let mid = ((hi + lo) / 2 / 100) * 100;
            if mid > lo && mid < hi {
                vals.push(mid);
            }
            vals.push(lo);
            if let Some(cur) = f.igpu_mhz {
                if !vals.contains(&cur) {
                    vals.push(cur);
                }
            }
        }
        state.dd_igpu.options = vals
            .iter()
            .map(|m| {
                if Some(*m) == f.igpu_max_mhz { format!("{} MHz  (full)", m) }
                else if Some(*m) == f.igpu_min_mhz { format!("{} MHz  (minimum)", m) }
                else { format!("{} MHz", m) }
            })
            .collect();
        state.dd_igpu.selected =
            f.igpu_mhz.and_then(|c| vals.iter().position(|v| *v == c)).unwrap_or(0);
        state.igpu_values = vals;
    }
    if !state.dd_aspm.open {
        state.dd_aspm.options = f.aspm_policies.iter().map(|p| pretty(p)).collect();
        state.dd_aspm.selected =
            f.aspm_policies.iter().position(|p| *p == f.aspm).unwrap_or(0);
    }
    if !state.dd_audio.open {
        // 0 disables suspend entirely; the rest are idle timeouts. The current
        // value earns a row if the kernel came up with something else.
        let mut vals: Vec<u32> = vec![0, 1, 10];
        if let Some(cur) = f.hda_idle_secs {
            if !vals.contains(&cur) {
                vals.push(cur);
            }
        }
        vals.sort_unstable();
        state.dd_audio.options = vals
            .iter()
            .map(|v| if *v == 0 { "Never suspend".to_string() } else { format!("After {} s idle", v) })
            .collect();
        state.dd_audio.selected =
            f.hda_idle_secs.and_then(|c| vals.iter().position(|v| *v == c)).unwrap_or(0);
        state.audio_values = vals;
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
        if !state.facts.governors.is_empty() {
            state.dd_governor.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_governor, sec_w - 28.0, 44.0, ctx);
        }
        if state.facts.charge_limit.is_some() {
            state.dd_limit.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_limit, sec_w - 28.0, 44.0, ctx);
        }
        if state.facts.turbo.is_some() {
            state.dd_turbo.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_turbo, sec_w - 28.0, 44.0, ctx);
        }
        if !state.igpu_values.is_empty() {
            state.dd_igpu.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_igpu, sec_w - 28.0, 44.0, ctx);
        }
        if !state.facts.aspm_policies.is_empty() {
            state.dd_aspm.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_aspm, sec_w - 28.0, 44.0, ctx);
        }
        if state.facts.hda_idle_secs.is_some() {
            state.dd_audio.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_audio, sec_w - 28.0, 44.0, ctx);
        }
        if !state.gpu_values.is_empty() {
            state.dd_gpu.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut state.dd_gpu, sec_w - 28.0, 44.0, ctx);
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
                    run_privileged(format!("echo {} > /sys/firmware/acpi/platform_profile", p));
                    state.facts.profile = p.clone();
                }
            }
        }
        PowerMessage::SetEpp(idx) => {
            if let Some(p) = state.facts.epps.get(idx) {
                if sysfs_token_ok(p) {
                    // Every core: EPP is per-cpu and a partial write would
                    // leave the package split across preferences.
                    run_privileged(format!(
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
                    run_privileged(format!(
                        "for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do echo {} > \"$f\"; done",
                        v
                    ));
                    state.facts.charge_limit = Some(v);
                }
            }
        }
        PowerMessage::SetGovernor(idx) => {
            if let Some(g) = state.facts.governors.get(idx) {
                if sysfs_token_ok(g) {
                    // Every core, like EPP: a partial write leaves the package
                    // split across governors.
                    run_privileged(format!(
                        "for f in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo {} > \"$f\"; done",
                        g
                    ));
                    state.facts.governor = g.clone();
                }
            }
        }
        PowerMessage::SetIgpuClock(idx) => {
            if let Some(m) = state.igpu_values.get(idx).copied() {
                // Bounded by the hardware's own reported range, so the number
                // reaching the shell line can only be one the GPU accepts.
                let within = state.facts.igpu_min_mhz.is_none_or(|lo| m >= lo)
                    && state.facts.igpu_max_mhz.is_none_or(|hi| m <= hi);
                if within {
                    // Every card exposing the knob, like the per-cpu writes.
                    run_privileged(format!(
                        "for f in /sys/class/drm/card*/gt_max_freq_mhz; do echo {} > \"$f\"; done",
                        m
                    ));
                    state.facts.igpu_mhz = Some(m);
                }
            }
        }
        PowerMessage::SetAspm(idx) => {
            if let Some(p) = state.facts.aspm_policies.get(idx) {
                if sysfs_token_ok(p) {
                    run_privileged(format!(
                        "echo {} > /sys/module/pcie_aspm/parameters/policy",
                        p
                    ));
                    state.facts.aspm = p.clone();
                }
            }
        }
        PowerMessage::SetAudioIdle(idx) => {
            if let Some(v) = state.audio_values.get(idx).copied() {
                if v <= 3600 {
                    run_privileged(format!(
                        "echo {} > /sys/module/snd_hda_intel/parameters/power_save",
                        v
                    ));
                    state.facts.hda_idle_secs = Some(v);
                }
            }
        }
        PowerMessage::SetGpuLimit(idx) => {
            if let Some(w) = state.gpu_values.get(idx).copied() {
                // Bounded by what nvidia-smi itself reported, so the number
                // reaching the shell line cannot be arbitrary.
                let within = state.facts.gpu_min_w.is_none_or(|m| w >= m)
                    && state.facts.gpu_default_w.is_none_or(|d| w <= d.max(w));
                if within {
                    run_privileged(format!("nvidia-smi -pl {}", w));
                    state.facts.gpu_limit_w = Some(w);
                }
            }
        }
        PowerMessage::SetTurbo(idx) => {
            let no_turbo = if idx == 1 { 1 } else { 0 };
            run_privileged(format!(
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
        if !self.facts.governors.is_empty() {
            ids.push(self.dd_governor.id());
        }
        if self.facts.charge_limit.is_some() {
            ids.push(self.dd_limit.id());
        }
        if self.facts.turbo.is_some() {
            ids.push(self.dd_turbo.id());
        }
        if !self.igpu_values.is_empty() {
            ids.push(self.dd_igpu.id());
        }
        if !self.facts.aspm_policies.is_empty() {
            ids.push(self.dd_aspm.id());
        }
        if self.facts.hda_idle_secs.is_some() {
            ids.push(self.dd_audio.id());
        }
        if !self.gpu_values.is_empty() {
            ids.push(self.dd_gpu.id());
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
        if self.dd_governor.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetGovernor(self.dd_governor.selected)));
        }
        if self.dd_gpu.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetGpuLimit(self.dd_gpu.selected)));
        }
        if self.dd_igpu.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetIgpuClock(self.dd_igpu.selected)));
        }
        if self.dd_aspm.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetAspm(self.dd_aspm.selected)));
        }
        if self.dd_audio.take_change() {
            actions.push(AppAction::Power(PowerMessage::SetAudioIdle(self.dd_audio.selected)));
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
        // Not loaded: nothing reported (the view paints only the loading line).
        assert_eq!(st.section_widgets(), vec![Vec::new()]);
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        // Every interface present: profile, epp, governor, limit, turbo, igpu,
        // aspm, audio, gpu.
        assert_eq!(st.section_widgets()[0].len(), 9);
        // A host without a charge-limit knob or turbo file reports fewer.
        st.facts.charge_limit = None;
        st.facts.turbo = None;
        assert_eq!(st.section_widgets()[0].len(), 7);
        // No cpufreq governors and no NVIDIA driver: both drop out too. The
        // GPU gate is gpu_values, which rebuild_options derives from the facts
        // — the same predicate the view paints on.
        st.facts.governors.clear();
        st.facts.gpu_limit_w = None;
        st.facts.gpu_default_w = None;
        st.facts.gpu_min_w = None;
        rebuild_options(&mut st);
        assert_eq!(st.section_widgets()[0].len(), 5);
        // A desktop with no Intel render clocks, an ASPM-less kernel and no
        // snd_hda_intel is down to profile and epp.
        st.facts.igpu_max_mhz = None;
        st.facts.igpu_min_mhz = None;
        st.facts.aspm_policies.clear();
        st.facts.hda_idle_secs = None;
        rebuild_options(&mut st);
        assert_eq!(st.section_widgets()[0].len(), 2);
    }

    #[test]
    fn igpu_rows_come_from_the_hardware_range() {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        rebuild_options(&mut st);
        // RP0, the rounded midpoint, RPn — no invented numbers.
        assert_eq!(st.igpu_values, vec![1500, 800, 100]);
        assert_eq!(st.dd_igpu.selected, 0);
        // A cap that is none of the three earns its own row.
        st.facts.igpu_mhz = Some(1200);
        rebuild_options(&mut st);
        assert_eq!(st.igpu_values, vec![1500, 800, 100, 1200]);
        assert_eq!(st.dd_igpu.selected, 3);
    }

    #[test]
    fn aspm_current_is_the_bracketed_policy() {
        // fetch strips the brackets; the selection must land on the active one.
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        st.facts.aspm = "powersave".to_string();
        rebuild_options(&mut st);
        assert_eq!(st.dd_aspm.selected, 2);
        assert_eq!(st.dd_aspm.options[2], "Powersave");
    }

    #[test]
    fn gpu_rows_are_default_min_and_an_off_list_current() {
        let mut st = PowerState::default();
        st.loaded = true;
        st.facts = facts();
        // Current == default: two rows, no duplicate.
        rebuild_options(&mut st);
        assert_eq!(st.gpu_values, vec![80, 5]);
        assert_eq!(st.dd_gpu.selected, 0);
        // A current limit that is neither default nor minimum earns its own
        // row rather than silently selecting the wrong one.
        st.facts.gpu_limit_w = Some(60);
        rebuild_options(&mut st);
        assert_eq!(st.gpu_values, vec![80, 5, 60]);
        assert_eq!(st.dd_gpu.selected, 2);
    }
}
