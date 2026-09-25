//! Power modes and the adapter states they are assigned to: a named set of
//! levers per mode, and a small table saying which mode runs when the
//! machine is plugged in and which when it runs on battery.
//!
//! Shared between the two sides of the feature, which is the point of the
//! module: the Power page edits the modes and the assignment, and
//! `cce-power-apply` (this crate's helper binary) applies them as root —
//! from udev when the Mains supply flips, at boot, and on demand when the
//! page changes a lever of the mode that is running right now. One parser,
//! one apply path, one value guard, so the two sides cannot drift.
//!
//! The plan lives at [`PLAN_PATH`], root-owned, because the applier runs as
//! root outside any session: it has no `$HOME` to look in, and a root daemon
//! taking its orders from a user-writable file would be a privilege boundary
//! drawn in the wrong place. Writes go through the helper under pkexec, the
//! same one-prompt path every lever change in this app already takes.
//!
//! ```kdl
//! mode "performance" {
//!     profile "performance"
//!     turbo "on"
//! }
//! mode "power-saver" {
//!     profile "low-power"
//!     igpu_max_mhz 800
//! }
//! assign {
//!     ac "performance"
//!     battery "power-saver"
//! }
//! ```
//!
//! A lever absent from a mode is left alone when that mode becomes active —
//! "not set" means "don't touch", never "reset to a default". The older
//! per-source form of this file (top-level `ac` / `battery` blocks of
//! levers, before modes existed) still parses: each block becomes the mode
//! that source is assigned to by default, which is exactly the behavior it
//! had.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PLAN_PATH: &str = "/etc/cce/power.kdl";
/// Where `ccebuild install-system` puts the helper. The udev rule and the
/// system unit both name this path, so its presence is what "automatic
/// switching is installed" means to the page.
pub const HELPER_SYSTEM_PATH: &str = "/usr/bin/cce-power-apply";
pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/90-cce-power-apply.rules";

/// A power-adapter state. Defaults to `Ac`: a host with no Mains supply has
/// nothing to unplug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Source {
    #[default]
    Ac,
    Battery,
}

impl Source {
    pub const ALL: [Source; 2] = [Source::Ac, Source::Battery];

    /// The key in the `assign` block, and the CLI spelling.
    pub fn key(self) -> &'static str {
        match self {
            Source::Ac => "ac",
            Source::Battery => "battery",
        }
    }

    pub fn parse(s: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|v| v.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::Ac => "Plugged In",
            Source::Battery => "On Battery",
        }
    }

    /// The mode a source runs when the plan says nothing about it. These are
    /// also what the pre-modes file format migrates onto, so an old plan
    /// keeps behaving exactly as it did.
    pub fn default_mode(self) -> Mode {
        match self {
            Source::Ac => Mode::Performance,
            Source::Battery => Mode::PowerSaver,
        }
    }
}

/// A named set of lever values. The set is fixed rather than user-extensible:
/// the page picks a mode from a dropdown, and there is deliberately no
/// naming UI to keep a mode's identity stable across the plan file, the
/// helper's CLI and the assignment table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Mode {
    Performance,
    #[default]
    Balanced,
    PowerSaver,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Performance, Mode::Balanced, Mode::PowerSaver];

    /// The name in the plan file, and the CLI spelling.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Performance => "performance",
            Mode::Balanced => "balanced",
            Mode::PowerSaver => "power-saver",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Performance => "Performance",
            Mode::Balanced => "Balanced",
            Mode::PowerSaver => "Power Saver",
        }
    }
}

/// The levers that make sense per mode. The battery charge limit is
/// deliberately not one: it is a charging policy, not something to flip when
/// the mode changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lever {
    Profile,
    Epp,
    Governor,
    Turbo,
    IgpuMaxMhz,
    Aspm,
    AudioIdleSecs,
    GpuLimitW,
    Animations,
}
impl Lever {
    pub const ALL: [Lever; 9] = [
        Lever::Profile,
        Lever::Epp,
        Lever::Governor,
        Lever::Turbo,
        Lever::IgpuMaxMhz,
        Lever::Aspm,
        Lever::AudioIdleSecs,
        Lever::GpuLimitW,
        Lever::Animations,
    ];

    /// The node name in the plan file, and the CLI spelling.
    pub fn key(self) -> &'static str {
        match self {
            Lever::Profile => "profile",
            Lever::Epp => "epp",
            Lever::Governor => "governor",
            Lever::Turbo => "turbo",
            Lever::IgpuMaxMhz => "igpu_max_mhz",
            Lever::Aspm => "aspm",
            Lever::AudioIdleSecs => "audio_idle_secs",
            Lever::GpuLimitW => "gpu_limit_w",
            Lever::Animations => "animations",
        }
    }

    pub fn parse(s: &str) -> Option<Lever> {
        Lever::ALL.into_iter().find(|l| l.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Lever::Profile => "Power Profile",
            Lever::Epp => "CPU Energy Preference",
            Lever::Governor => "CPU Governor",
            Lever::Turbo => "CPU Turbo Boost",
            Lever::IgpuMaxMhz => "Integrated GPU Max Clock",
            Lever::Aspm => "PCIe Power Management",
            Lever::AudioIdleSecs => "Audio Codec Idle",
            Lever::GpuLimitW => "GPU Power Limit",
            Lever::Animations => "Animations",
        }
    }

    /// Numeric levers are stored as KDL integers; the rest as strings.
    pub fn is_numeric(self) -> bool {
        matches!(self, Lever::IgpuMaxMhz | Lever::AudioIdleSecs | Lever::GpuLimitW)
    }

    /// Shape check on a value before it goes anywhere near sysfs or a shell
    /// line: a sysfs token, an unsigned integer, or on/off for turbo and
    /// animations. This is
    /// the guard `set` and `apply` share; range checks against the hardware
    /// happen in [`apply_lever`], where the ranges can be read.
    pub fn value_ok(self, v: &str) -> bool {
        match self {
            Lever::Turbo | Lever::Animations => v == "on" || v == "off",
            l if l.is_numeric() => v.parse::<u32>().is_ok(),
            _ => sysfs_token_ok(v),
        }
    }
}

/// Sysfs tokens travel into sysfs writes and (from the page) a `pkexec`
/// argv, so only the shapes sysfs itself produces are allowed through —
/// anything else is dropped, not escaped.
pub fn sysfs_token_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The levers of every mode, plus which mode each adapter state runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerPlan {
    modes: BTreeMap<Mode, BTreeMap<Lever, String>>,
    assign: BTreeMap<Source, Mode>,
}

impl PowerPlan {
    /// One mode's levers. Absent and empty are the same thing to every
    /// caller, so a mode nothing has been set on reads as an empty set.
    pub fn levers(&self, mode: Mode) -> impl Iterator<Item = (Lever, &str)> {
        self.modes
            .get(&mode)
            .into_iter()
            .flat_map(|m| m.iter().map(|(l, v)| (*l, v.as_str())))
    }

    pub fn get(&self, mode: Mode, lever: Lever) -> Option<&str> {
        self.modes.get(&mode)?.get(&lever).map(String::as_str)
    }

    /// Record a value on a mode, or clear it with `None`. Rejects a
    /// malformed value rather than storing it.
    pub fn put(&mut self, mode: Mode, lever: Lever, value: Option<&str>) -> Result<(), String> {
        match value {
            None => {
                if let Some(set) = self.modes.get_mut(&mode) {
                    set.remove(&lever);
                }
            }
            Some(v) if lever.value_ok(v) => {
                self.modes.entry(mode).or_default().insert(lever, v.to_string());
            }
            Some(v) => return Err(format!("{:?} is not a valid value for {}", v, lever.key())),
        }
        Ok(())
    }

    /// The mode an adapter state runs; unassigned falls back to the source's
    /// own default rather than to "do nothing", so a fresh plan still has a
    /// mode to edit and apply.
    pub fn assigned(&self, source: Source) -> Mode {
        self.assign.get(&source).copied().unwrap_or_else(|| source.default_mode())
    }

    pub fn assign(&mut self, source: Source, mode: Mode) {
        self.assign.insert(source, mode);
    }

    /// No lever set on any mode. The assignment alone is not content: it
    /// changes nothing until some mode has a lever in it.
    pub fn is_empty(&self) -> bool {
        self.modes.values().all(BTreeMap::is_empty)
    }

    pub fn parse(text: &str) -> Result<PowerPlan, String> {
        let doc: kdl::KdlDocument = text.parse().map_err(|e: kdl::KdlError| e.to_string())?;
        let mut plan = PowerPlan::default();
        for node in doc.nodes() {
            let name = node.name().value();
            match name {
                "mode" => {
                    let key = node
                        .get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| "mode needs a name, e.g. mode \"balanced\"".to_string())?;
                    let mode = Mode::parse(key).ok_or_else(|| format!("unknown mode {:?}", key))?;
                    plan.read_levers(node, mode, key)?;
                }
                "assign" => {
                    let Some(children) = node.children() else { continue };
                    for child in children.nodes() {
                        let sname = child.name().value();
                        let source = Source::parse(sname)
                            .ok_or_else(|| format!("unknown power source {:?} under assign", sname))?;
                        let key = child
                            .get(0)
                            .and_then(|e| e.value().as_string())
                            .ok_or_else(|| format!("assign.{} needs a mode name", sname))?;
                        let mode = Mode::parse(key)
                            .ok_or_else(|| format!("unknown mode {:?} assigned to {}", key, sname))?;
                        plan.assign(source, mode);
                    }
                }
                // The pre-modes file: a bare block of levers per adapter
                // state. Each becomes that state's default mode, which is
                // what it was already doing.
                _ => match Source::parse(name) {
                    Some(source) => {
                        let mode = source.default_mode();
                        plan.read_levers(node, mode, name)?;
                        plan.assign(source, mode);
                    }
                    None => return Err(format!("unknown block {:?}", name)),
                },
            }
        }
        Ok(plan)
    }

    /// The lever children of one block, into `mode`. `what` names the block
    /// in errors, since the same reader serves both file formats.
    fn read_levers(&mut self, node: &kdl::KdlNode, mode: Mode, what: &str) -> Result<(), String> {
        let Some(children) = node.children() else { return Ok(()) };
        for child in children.nodes() {
            let name = child.name().value();
            let Some(lever) = Lever::parse(name) else {
                return Err(format!("unknown lever {:?} under {}", name, what));
            };
            let value = match child.get(0).map(|e| e.value()) {
                Some(v) if v.as_string().is_some() => v.as_string().unwrap().to_string(),
                Some(v) if v.as_i64().is_some() => v.as_i64().unwrap().to_string(),
                _ => return Err(format!("{}.{} needs one string or integer value", what, name)),
            };
            self.put(mode, lever, Some(&value))?;
        }
        Ok(())
    }

    pub fn to_kdl(&self) -> String {
        let mut doc = kdl::KdlDocument::new();
        for mode in Mode::ALL {
            let mut block = kdl::KdlNode::new("mode");
            block.push(kdl::KdlEntry::new(mode.key()));
            let children = block.ensure_children();
            for (lever, value) in self.levers(mode) {
                let mut node = kdl::KdlNode::new(lever.key());
                if lever.is_numeric() {
                    // Validated on the way in, so this parse cannot fail;
                    // fall back to the string form rather than panicking.
                    match value.parse::<i64>() {
                        Ok(n) => node.push(kdl::KdlEntry::new(n)),
                        Err(_) => node.push(kdl::KdlEntry::new(value)),
                    }
                } else {
                    node.push(kdl::KdlEntry::new(value));
                }
                children.nodes_mut().push(node);
            }
            doc.nodes_mut().push(block);
        }
        let mut assign = kdl::KdlNode::new("assign");
        let children = assign.ensure_children();
        for source in Source::ALL {
            // Resolved, not just what was stored: the file then says out
            // loud what the applier will do on every adapter state.
            let mut node = kdl::KdlNode::new(source.key());
            node.push(kdl::KdlEntry::new(self.assigned(source).key()));
            children.nodes_mut().push(node);
        }
        doc.nodes_mut().push(assign);
        doc.fmt();
        let mut out = String::from(
            "// Power modes and their adapter-state assignment, edited from the\n\
             // System Interface's Power page and applied by cce-power-apply (udev,\n\
             // boot, and on each change). A lever missing from a mode is left\n\
             // untouched when that mode becomes active.\n",
        );
        out.push_str(&doc.to_string());
        out
    }

    /// The plan on disk; a missing file is an empty plan, an unreadable or
    /// malformed one is an error (the applier must not guess at half a plan).
    pub fn load_from(path: &Path) -> Result<PowerPlan, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => PowerPlan::parse(&text).map_err(|e| format!("{}: {}", path.display(), e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PowerPlan::default()),
            Err(e) => Err(format!("{}: {}", path.display(), e)),
        }
    }

    pub fn load() -> Result<PowerPlan, String> {
        Self::load_from(Path::new(PLAN_PATH))
    }

    /// Write atomically (temp file + rename) so a reader never sees a torn
    /// plan; the directory is created if this is the first write.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("kdl.tmp");
        std::fs::write(&tmp, self.to_kdl())?;
        std::fs::rename(&tmp, path)
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(Path::new(PLAN_PATH))
    }
}

/// Which source the machine is on, from (type, online) pairs of the
/// power_supply class: any Mains supply that is online means plugged in.
/// No Mains supply at all (a desktop) reads as plugged in too — there is
/// nothing to unplug.
pub fn source_from_supplies<'a>(supplies: impl IntoIterator<Item = (&'a str, bool)>) -> Source {
    let mut saw_mains = false;
    for (kind, online) in supplies {
        if kind == "Mains" {
            saw_mains = true;
            if online {
                return Source::Ac;
            }
        }
    }
    if saw_mains { Source::Battery } else { Source::Ac }
}

fn read_trim(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// The live source, from /sys/class/power_supply.
pub fn current_source() -> Source {
    let mut pairs: Vec<(String, bool)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") {
        for e in rd.flatten() {
            let p = e.path();
            let kind = read_trim(&p.join("type")).unwrap_or_default();
            let online = read_trim(&p.join("online")).is_some_and(|s| s == "1");
            pairs.push((kind, online));
        }
    }
    source_from_supplies(pairs.iter().map(|(k, o)| (k.as_str(), *o)))
}

/// The state of the root side that applies a mode on plug and unplug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Automation {
    /// No helper at the system path, or no udev rule to start it. The plan
    /// is only applied when the page itself changes a lever.
    #[default]
    Missing,
    /// Both installed, but the helper predates modes or one of the levers:
    /// it cannot parse a plan that uses them, so it applies nothing on plug
    /// or unplug — and it rejects the page's own `set`/`assign` calls too.
    Stale,
    Ready,
}

/// Whether a helper binary speaks the current CLI: modes, and every lever
/// this page can send it.
///
/// Asked by running it with no arguments, which prints its usage and exits
/// 2 without touching anything. Deliberately NOT a string search inside the
/// file: a hit would prove freshness but a miss proves nothing (link-time
/// constant merging eats literals), and a false "stale" is the worse error.
///
/// This exists because a stale `/usr/bin/cce-power-apply` fails in the one
/// way nothing reports: it takes the pkexec prompt, reads the mode name as
/// an adapter state, and exits 2 — so a pick costs the user an
/// authentication and changes nothing. A helper that knows modes but
/// predates a lever fails the same way for that lever alone, which is why
/// the lever list is part of the check.
pub fn helper_speaks_modes(path: &Path) -> bool {
    std::process::Command::new(path)
        .output()
        .is_ok_and(|out| usage_speaks_modes(&String::from_utf8_lossy(&out.stderr)))
}

/// The usage text of a helper that knows about modes names `apply-mode`
/// (the pre-modes one lists only `apply`, `set` and `show`), and its
/// `levers:` line names every lever it accepts.
fn usage_speaks_modes(usage: &str) -> bool {
    let levers: Vec<&str> = usage
        .lines()
        .find_map(|l| l.trim().strip_prefix("levers:"))
        .map(|l| l.split_whitespace().collect())
        .unwrap_or_default();
    usage.contains("apply-mode") && Lever::ALL.iter().all(|l| levers.contains(&l.key()))
}

/// Whether the root-side pieces are in place — the helper at its system
/// path, the udev rule that starts it, and a helper new enough to read the
/// plan this app writes.
pub fn automation_status() -> Automation {
    let helper = Path::new(HELPER_SYSTEM_PATH);
    if !helper.exists() || !Path::new(UDEV_RULE_PATH).exists() {
        return Automation::Missing;
    }
    if helper_speaks_modes(helper) { Automation::Ready } else { Automation::Stale }
}

// ── Applying (root) ─────────────────────────────────────────────────────

fn write_sysfs(path: &Path, value: &str) -> Result<(), String> {
    std::fs::write(path, value).map_err(|e| format!("{}: {}", path.display(), e))
}

/// `/sys/devices/system/cpu/cpu<N>` for every core, sorted.
fn cpu_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir("/sys/devices/system/cpu")
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.strip_prefix("cpu").is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Every /sys/class/drm/card* exposing the i915/xe clock knob.
fn drm_cards_with_freq() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir("/sys/class/drm")
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("card"))
                        && p.join("gt_max_freq_mhz").exists()
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Write one lever to every interface it covers. Runs as root; the page
/// never calls this directly, it goes through the helper under pkexec.
/// Errors name the file and the reason so the unit's journal is useful.
pub fn apply_lever(lever: Lever, value: &str) -> Result<(), String> {
    if !lever.value_ok(value) {
        return Err(format!("{:?} is not a valid value for {}", value, lever.key()));
    }
    match lever {
        Lever::Profile => write_sysfs(Path::new("/sys/firmware/acpi/platform_profile"), value),
        Lever::Epp | Lever::Governor => {
            // Every core: both are per-cpu and a partial write would leave the
            // package split across preferences.
            let file = if lever == Lever::Epp { "energy_performance_preference" } else { "scaling_governor" };
            let mut any = false;
            for cpu in cpu_dirs() {
                let p = cpu.join("cpufreq").join(file);
                if p.exists() {
                    any = true;
                    write_sysfs(&p, value)?;
                }
            }
            if any { Ok(()) } else { Err(format!("no cpufreq/{} on this host", file)) }
        }
        Lever::Turbo => {
            let no_turbo = if value == "on" { "0" } else { "1" };
            write_sysfs(Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo"), no_turbo)
        }
        Lever::IgpuMaxMhz => {
            let mhz: u32 = value.parse().map_err(|_| "bad MHz".to_string())?;
            let cards = drm_cards_with_freq();
            if cards.is_empty() {
                return Err("no DRM card exposes gt_max_freq_mhz".to_string());
            }
            for card in cards {
                // Bounded by the hardware's own reported range, per card.
                let rd = |n: &str| read_trim(&card.join(n)).and_then(|s| s.parse::<u32>().ok());
                let lo = rd("gt_RPn_freq_mhz").unwrap_or(0);
                let hi = rd("gt_RP0_freq_mhz").unwrap_or(u32::MAX);
                if mhz < lo || mhz > hi {
                    return Err(format!("{} MHz is outside {}'s {}–{} MHz range", mhz, card.display(), lo, hi));
                }
                write_sysfs(&card.join("gt_max_freq_mhz"), value)?;
            }
            Ok(())
        }
        Lever::Aspm => write_sysfs(Path::new("/sys/module/pcie_aspm/parameters/policy"), value),
        Lever::AudioIdleSecs => {
            let secs: u32 = value.parse().map_err(|_| "bad seconds".to_string())?;
            if secs > 3600 {
                return Err("audio idle timeout above an hour".to_string());
            }
            write_sysfs(Path::new("/sys/module/snd_hda_intel/parameters/power_save"), value)
        }
        Lever::Animations => write_animations(value),
        Lever::GpuLimitW => {
            // nvidia-smi validates the watts against the card's own min/max
            // and refuses anything outside them, so it is the range check.
            let out = std::process::Command::new("nvidia-smi")
                .args(["-pl", value])
                .output()
                .map_err(|e| format!("nvidia-smi: {}", e))?;
            if out.status.success() {
                Ok(())
            } else {
                let msg = String::from_utf8_lossy(&out.stderr);
                let msg = if msg.trim().is_empty() { String::from_utf8_lossy(&out.stdout) } else { msg };
                Err(format!("nvidia-smi -pl {}: {}", value, msg.trim()))
            }
        }
    }
}

/// Record the animations switch where every session reads it
/// ([`cce_ui::motion::STATE_PATH`]). Not sysfs, but the same shape of lever:
/// root writes it when the mode changes, and the compositor and every cce-ui
/// client follow it without a restart. Under /run, not in anyone's config:
/// this runs as root with no session and no `$HOME`, and a tmpfs file is
/// rewritten at boot by the same coldplug run that applies the rest of the
/// mode, so it can never outlive the plan that set it.
fn write_animations(value: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let path = Path::new(cce_ui::motion::STATE_PATH);
    let dir = path.parent().expect("STATE_PATH has a parent");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    // Temp + rename so a reader never sees a torn value; world-readable
    // because every session's processes read it.
    let tmp = path.with_extension("tmp");
    write_sysfs(&tmp, value)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))
        .map_err(|e| format!("{}: {}", tmp.display(), e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {}", path.display(), e))
}

/// Apply every lever one mode sets. Failures are per lever — a missing
/// NVIDIA driver must not stop the CPU profile from landing — and come back
/// to the caller, which logs them.
pub fn apply_mode(plan: &PowerPlan, mode: Mode) -> Vec<(Lever, Result<(), String>)> {
    plan.levers(mode)
        .map(|(lever, value)| (lever, apply_lever(lever, value)))
        .collect::<Vec<_>>()
}

/// Apply whichever mode is assigned to one adapter state.
pub fn apply_source(plan: &PowerPlan, source: Source) -> Vec<(Lever, Result<(), String>)> {
    apply_mode(plan, plan.assigned(source))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levers_of(plan: &PowerPlan, mode: Mode) -> Vec<(Lever, String)> {
        plan.levers(mode).map(|(l, v)| (l, v.to_string())).collect()
    }

    #[test]
    fn kdl_round_trip_keeps_modes_assignment_and_types() {
        let mut plan = PowerPlan::default();
        plan.put(Mode::Performance, Lever::Profile, Some("performance")).unwrap();
        plan.put(Mode::Performance, Lever::Turbo, Some("on")).unwrap();
        plan.put(Mode::PowerSaver, Lever::Profile, Some("low-power")).unwrap();
        plan.put(Mode::PowerSaver, Lever::IgpuMaxMhz, Some("800")).unwrap();
        plan.put(Mode::PowerSaver, Lever::GpuLimitW, Some("40")).unwrap();
        plan.assign(Source::Battery, Mode::Balanced);
        let text = plan.to_kdl();
        // Numbers are written as KDL integers, tokens as strings.
        assert!(text.contains("igpu_max_mhz 800"), "{text}");
        assert!(text.contains("profile \"low-power\""), "{text}");
        assert!(text.contains("mode \"power-saver\""), "{text}");
        assert!(text.contains("battery \"balanced\""), "{text}");
        // The file says every assignment out loud, so what comes back is the
        // same plan with the AC default written down — and writing it again
        // is a fixed point.
        let back = PowerPlan::parse(&text).unwrap();
        assert_eq!(levers_of(&back, Mode::Performance), levers_of(&plan, Mode::Performance));
        assert_eq!(levers_of(&back, Mode::PowerSaver), levers_of(&plan, Mode::PowerSaver));
        for source in Source::ALL {
            assert_eq!(back.assigned(source), plan.assigned(source));
        }
        assert_eq!(back.to_kdl(), text);
    }

    #[test]
    fn parse_accepts_empty_and_partial_files() {
        assert_eq!(PowerPlan::parse("").unwrap(), PowerPlan::default());
        let p = PowerPlan::parse("mode \"balanced\" {\n  epp \"power\"\n}\n").unwrap();
        assert_eq!(p.get(Mode::Balanced, Lever::Epp), Some("power"));
        assert_eq!(p.get(Mode::Performance, Lever::Epp), None);
        // Nothing assigned: each adapter state keeps its default mode.
        assert_eq!(p.assigned(Source::Ac), Mode::Performance);
        assert_eq!(p.assigned(Source::Battery), Mode::PowerSaver);
    }

    #[test]
    fn the_pre_modes_file_migrates_onto_the_default_modes() {
        // What /etc/cce/power.kdl looked like before modes existed: a bare
        // block of levers per adapter state, applied on plug and unplug.
        let old = "ac {\n  profile \"performance\"\n}\nbattery {\n  profile \"low-power\"\n  igpu_max_mhz 800\n}\n";
        let p = PowerPlan::parse(old).unwrap();
        // Each block landed on the mode its source runs, so the same levers
        // still apply on the same adapter states.
        assert_eq!(p.assigned(Source::Ac), Mode::Performance);
        assert_eq!(p.assigned(Source::Battery), Mode::PowerSaver);
        assert_eq!(p.get(Mode::Performance, Lever::Profile), Some("performance"));
        assert_eq!(p.get(Mode::PowerSaver, Lever::IgpuMaxMhz), Some("800"));
        assert!(levers_of(&p, Mode::Balanced).is_empty());
        // And it rewrites in the new shape.
        assert!(p.to_kdl().contains("mode \"performance\""));
        assert_eq!(PowerPlan::parse(&p.to_kdl()).unwrap(), p);
    }

    #[test]
    fn parse_rejects_unknown_names_and_bad_values() {
        assert!(PowerPlan::parse("mode \"balanced\" {\n  brightness 50\n}\n").is_err());
        assert!(PowerPlan::parse("mode \"turbo-max\" {\n}\n").is_err());
        assert!(PowerPlan::parse("assign {\n  ac \"turbo-max\"\n}\n").is_err());
        assert!(PowerPlan::parse("assign {\n  usb \"balanced\"\n}\n").is_err());
        assert!(PowerPlan::parse("levers {\n  profile \"performance\"\n}\n").is_err());
        // A shell metacharacter never survives into the plan.
        assert!(PowerPlan::parse("mode \"balanced\" {\n  profile \"x;reboot\"\n}\n").is_err());
        // Turbo is on/off only.
        assert!(PowerPlan::parse("mode \"balanced\" {\n  turbo \"yes\"\n}\n").is_err());
        // A numeric lever given a token.
        assert!(PowerPlan::parse("mode \"balanced\" {\n  gpu_limit_w \"max\"\n}\n").is_err());
    }

    #[test]
    fn put_none_clears_and_bad_values_are_refused() {
        let mut plan = PowerPlan::default();
        plan.put(Mode::Balanced, Lever::Governor, Some("powersave")).unwrap();
        assert!(plan.put(Mode::Balanced, Lever::Governor, Some("$(rm)")).is_err());
        assert_eq!(plan.get(Mode::Balanced, Lever::Governor), Some("powersave"));
        plan.put(Mode::Balanced, Lever::Governor, None).unwrap();
        assert!(plan.is_empty());
        // An assignment alone is not content — it changes nothing until a
        // mode has a lever in it.
        plan.assign(Source::Ac, Mode::Balanced);
        assert!(plan.is_empty());
    }

    #[test]
    fn assignment_is_per_source_and_two_states_may_share_a_mode() {
        let mut plan = PowerPlan::default();
        plan.put(Mode::Balanced, Lever::Epp, Some("balance_power")).unwrap();
        plan.assign(Source::Ac, Mode::Balanced);
        plan.assign(Source::Battery, Mode::Balanced);
        assert_eq!(plan.assigned(Source::Ac), Mode::Balanced);
        assert_eq!(plan.assigned(Source::Battery), Mode::Balanced);
        assert_eq!(PowerPlan::parse(&plan.to_kdl()).unwrap(), plan);
        // Both states named, so nothing was left to a default.
        assert!(plan.to_kdl().contains("ac \"balanced\""));
    }

    #[test]
    fn value_guard_shapes() {
        assert!(Lever::Profile.value_ok("balance_power"));
        assert!(Lever::Profile.value_ok("low-power"));
        assert!(!Lever::Profile.value_ok(""));
        assert!(!Lever::Profile.value_ok("a b"));
        assert!(!Lever::Profile.value_ok("x;reboot"));
        assert!(Lever::Turbo.value_ok("off"));
        assert!(!Lever::Turbo.value_ok("0"));
        assert!(Lever::Animations.value_ok("on"));
        assert!(!Lever::Animations.value_ok("false"));
        assert!(Lever::AudioIdleSecs.value_ok("10"));
        assert!(!Lever::AudioIdleSecs.value_ok("-1"));
        assert!(!Lever::AudioIdleSecs.value_ok("ten"));
    }

    #[test]
    fn source_follows_the_mains_supply() {
        // Battery discharging, USB-C sources idle, AC offline → battery.
        let unplugged = [("Battery", false), ("USB", false), ("Mains", false)];
        assert_eq!(source_from_supplies(unplugged), Source::Battery);
        let plugged = [("Battery", false), ("Mains", true)];
        assert_eq!(source_from_supplies(plugged), Source::Ac);
        // A desktop with no Mains device has nothing to unplug.
        assert_eq!(source_from_supplies([("Battery", false)]), Source::Ac);
        assert_eq!(source_from_supplies([]), Source::Ac);
    }

    #[test]
    fn the_usage_probe_tells_a_mode_helper_from_a_pre_modes_one() {
        // What this binary prints today.
        let levers = Lever::ALL.iter().map(|l| l.key()).collect::<Vec<_>>().join(" ");
        let current = format!(
            "usage: cce-power-apply apply [ac|battery]\n       cce-power-apply apply-mode <mode>\n \
             modes:  performance balanced power-saver\n levers: {levers}\n"
        );
        assert!(usage_speaks_modes(&current));
        // One that knows modes but predates a lever takes the prompt and
        // rejects that lever — as stale, for that pick, as a pre-modes one.
        let older = current.replace(" animations", "");
        assert!(!usage_speaks_modes(&older), "{older}");
        // What the pre-modes one printed — the copy that silently rejects
        // every pick the page makes.
        assert!(!usage_speaks_modes(
            "usage: cce-power-apply apply [ac|battery]\n       cce-power-apply set <ac|battery> <lever> <value|unset>\n       cce-power-apply show\n"
        ));
        // A helper that cannot be run at all is not a helper that speaks.
        assert!(!helper_speaks_modes(Path::new("/nonexistent/cce-power-apply")));
    }

    #[test]
    fn keys_round_trip() {
        for l in Lever::ALL {
            assert_eq!(Lever::parse(l.key()), Some(l));
        }
        for s in Source::ALL {
            assert_eq!(Source::parse(s.key()), Some(s));
        }
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.key()), Some(m));
        }
        assert_eq!(Lever::parse("brightness"), None);
        assert_eq!(Mode::parse("ac"), None);
    }

    #[test]
    fn save_and_load_through_a_temp_dir() {
        let dir = std::env::temp_dir().join(format!("cce-power-plan-test-{}", std::process::id()));
        let path = dir.join("nested").join("power.kdl");
        // Missing file is an empty plan, not an error.
        assert_eq!(PowerPlan::load_from(&path).unwrap(), PowerPlan::default());
        let mut plan = PowerPlan::default();
        plan.put(Mode::PowerSaver, Lever::Aspm, Some("powersave")).unwrap();
        plan.assign(Source::Ac, Mode::Balanced);
        plan.assign(Source::Battery, Mode::PowerSaver);
        plan.save_to(&path).unwrap();
        assert_eq!(PowerPlan::load_from(&path).unwrap(), plan);
        // Garbage on disk is reported, not silently emptied.
        std::fs::write(&path, "mode \"balanced\" {\n  profile \n").unwrap();
        assert!(PowerPlan::load_from(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
