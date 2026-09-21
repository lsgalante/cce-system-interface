//! The per-power-source plan: which levers to set when the machine is
//! plugged in and which when it runs on battery.
//!
//! Shared between the two sides of the feature, which is the point of the
//! module: the Power page edits the plan and shows it, and `cce-power-apply`
//! (this crate's helper binary) applies it as root — from udev when the
//! Mains supply flips, at boot, and on demand when the page changes a lever
//! for the source that is active right now. One parser, one apply path, one
//! value guard, so the two sides cannot drift.
//!
//! The plan lives at [`PLAN_PATH`], root-owned, because the applier runs as
//! root outside any session: it has no `$HOME` to look in, and a root daemon
//! taking its orders from a user-writable file would be a privilege boundary
//! drawn in the wrong place. Writes go through the helper under pkexec, the
//! same one-prompt path every lever change in this app already takes.
//!
//! ```kdl
//! ac {
//!     profile "performance"
//!     turbo "on"
//! }
//! battery {
//!     profile "low-power"
//!     igpu_max_mhz 800
//! }
//! ```
//!
//! A lever absent from a block is left alone when that source becomes
//! active — "not set" means "don't touch", never "reset to a default".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PLAN_PATH: &str = "/etc/cce/power.kdl";
/// Where `ccebuild install-system` puts the helper. The udev rule and the
/// system unit both name this path, so its presence is what "automatic
/// switching is installed" means to the page.
pub const HELPER_SYSTEM_PATH: &str = "/usr/bin/cce-power-apply";
pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/90-cce-power-apply.rules";

/// Defaults to `Ac`: a host with no Mains supply has nothing to unplug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Source {
    #[default]
    Ac,
    Battery,
}

impl Source {
    pub const ALL: [Source; 2] = [Source::Ac, Source::Battery];

    /// The block name in the plan file, and the CLI spelling.
    pub fn key(self) -> &'static str {
        match self {
            Source::Ac => "ac",
            Source::Battery => "battery",
        }
    }

    pub fn parse(s: &str) -> Option<Source> {
        match s {
            "ac" => Some(Source::Ac),
            "battery" => Some(Source::Battery),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::Ac => "Plugged In",
            Source::Battery => "On Battery",
        }
    }
}

/// The levers that make sense per source. The battery charge limit is
/// deliberately not one: it is a charging policy, not something to flip on
/// unplug.
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
}

impl Lever {
    pub const ALL: [Lever; 8] = [
        Lever::Profile,
        Lever::Epp,
        Lever::Governor,
        Lever::Turbo,
        Lever::IgpuMaxMhz,
        Lever::Aspm,
        Lever::AudioIdleSecs,
        Lever::GpuLimitW,
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
        }
    }

    /// Numeric levers are stored as KDL integers; the rest as strings.
    pub fn is_numeric(self) -> bool {
        matches!(self, Lever::IgpuMaxMhz | Lever::AudioIdleSecs | Lever::GpuLimitW)
    }

    /// Shape check on a value before it goes anywhere near sysfs or a shell
    /// line: a sysfs token, an unsigned integer, or on/off for turbo. This is
    /// the guard `set` and `apply` share; range checks against the hardware
    /// happen in [`apply_lever`], where the ranges can be read.
    pub fn value_ok(self, v: &str) -> bool {
        match self {
            Lever::Turbo => v == "on" || v == "off",
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerPlan {
    pub ac: BTreeMap<Lever, String>,
    pub battery: BTreeMap<Lever, String>,
}

impl PowerPlan {
    pub fn set(&self, source: Source) -> &BTreeMap<Lever, String> {
        match source {
            Source::Ac => &self.ac,
            Source::Battery => &self.battery,
        }
    }

    pub fn set_mut(&mut self, source: Source) -> &mut BTreeMap<Lever, String> {
        match source {
            Source::Ac => &mut self.ac,
            Source::Battery => &mut self.battery,
        }
    }

    pub fn get(&self, source: Source, lever: Lever) -> Option<&str> {
        self.set(source).get(&lever).map(String::as_str)
    }

    /// Record a value, or clear it with `None`. Rejects a malformed value
    /// rather than storing it.
    pub fn put(&mut self, source: Source, lever: Lever, value: Option<&str>) -> Result<(), String> {
        match value {
            None => {
                self.set_mut(source).remove(&lever);
            }
            Some(v) if lever.value_ok(v) => {
                self.set_mut(source).insert(lever, v.to_string());
            }
            Some(v) => return Err(format!("{:?} is not a valid value for {}", v, lever.key())),
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.ac.is_empty() && self.battery.is_empty()
    }

    pub fn parse(text: &str) -> Result<PowerPlan, String> {
        let doc: kdl::KdlDocument = text.parse().map_err(|e: kdl::KdlError| e.to_string())?;
        let mut plan = PowerPlan::default();
        for source in Source::ALL {
            let Some(block) = doc.get(source.key()) else { continue };
            let Some(children) = block.children() else { continue };
            for node in children.nodes() {
                let name = node.name().value();
                let Some(lever) = Lever::parse(name) else {
                    return Err(format!("unknown lever {:?} under {}", name, source.key()));
                };
                let value = match node.get(0).map(|e| e.value()) {
                    Some(v) if v.as_string().is_some() => v.as_string().unwrap().to_string(),
                    Some(v) if v.as_i64().is_some() => v.as_i64().unwrap().to_string(),
                    _ => return Err(format!("{}.{} needs one string or integer value", source.key(), name)),
                };
                plan.put(source, lever, Some(&value))?;
            }
        }
        Ok(plan)
    }

    pub fn to_kdl(&self) -> String {
        let mut doc = kdl::KdlDocument::new();
        for source in Source::ALL {
            let mut block = kdl::KdlNode::new(source.key());
            let children = block.ensure_children();
            for (lever, value) in self.set(source) {
                let mut node = kdl::KdlNode::new(lever.key());
                if lever.is_numeric() {
                    // Validated on the way in, so this parse cannot fail;
                    // fall back to the string form rather than panicking.
                    match value.parse::<i64>() {
                        Ok(n) => node.push(kdl::KdlEntry::new(n)),
                        Err(_) => node.push(kdl::KdlEntry::new(value.as_str())),
                    }
                } else {
                    node.push(kdl::KdlEntry::new(value.as_str()));
                }
                children.nodes_mut().push(node);
            }
            doc.nodes_mut().push(block);
        }
        doc.fmt();
        let mut out = String::from(
            "// Per-power-source settings, edited from the System Interface's Power page\n\
             // and applied by cce-power-apply (udev, boot, and on each change).\n\
             // A lever missing from a block is left untouched for that source.\n",
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

/// Whether the root-side pieces are in place: the helper at its system path
/// and the udev rule that starts it. Without both, the page's plan is only
/// applied when the page itself changes a lever.
pub fn automation_installed() -> bool {
    Path::new(HELPER_SYSTEM_PATH).exists() && Path::new(UDEV_RULE_PATH).exists()
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

/// Apply every lever the plan sets for one source. Failures are per lever —
/// a missing NVIDIA driver must not stop the CPU profile from landing — and
/// come back to the caller, which logs them.
pub fn apply_source(plan: &PowerPlan, source: Source) -> Vec<(Lever, Result<(), String>)> {
    plan.set(source)
        .iter()
        .map(|(lever, value)| (*lever, apply_lever(*lever, value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdl_round_trip_keeps_both_blocks_and_types() {
        let mut plan = PowerPlan::default();
        plan.put(Source::Ac, Lever::Profile, Some("performance")).unwrap();
        plan.put(Source::Ac, Lever::Turbo, Some("on")).unwrap();
        plan.put(Source::Battery, Lever::Profile, Some("low-power")).unwrap();
        plan.put(Source::Battery, Lever::IgpuMaxMhz, Some("800")).unwrap();
        plan.put(Source::Battery, Lever::GpuLimitW, Some("40")).unwrap();
        let text = plan.to_kdl();
        // Numbers are written as KDL integers, tokens as strings.
        assert!(text.contains("igpu_max_mhz 800"), "{text}");
        assert!(text.contains("profile \"low-power\""), "{text}");
        assert_eq!(PowerPlan::parse(&text).unwrap(), plan);
    }

    #[test]
    fn parse_accepts_empty_and_partial_files() {
        assert_eq!(PowerPlan::parse("").unwrap(), PowerPlan::default());
        let p = PowerPlan::parse("battery {\n  epp \"power\"\n}\n").unwrap();
        assert!(p.ac.is_empty());
        assert_eq!(p.get(Source::Battery, Lever::Epp), Some("power"));
        assert_eq!(p.get(Source::Ac, Lever::Epp), None);
    }

    #[test]
    fn parse_rejects_unknown_levers_and_bad_values() {
        assert!(PowerPlan::parse("ac {\n  brightness 50\n}\n").is_err());
        // A shell metacharacter never survives into the plan.
        assert!(PowerPlan::parse("ac {\n  profile \"x;reboot\"\n}\n").is_err());
        // Turbo is on/off only.
        assert!(PowerPlan::parse("ac {\n  turbo \"yes\"\n}\n").is_err());
        // A numeric lever given a token.
        assert!(PowerPlan::parse("ac {\n  gpu_limit_w \"max\"\n}\n").is_err());
    }

    #[test]
    fn put_none_clears_and_bad_values_are_refused() {
        let mut plan = PowerPlan::default();
        plan.put(Source::Ac, Lever::Governor, Some("powersave")).unwrap();
        assert!(plan.put(Source::Ac, Lever::Governor, Some("$(rm)")).is_err());
        assert_eq!(plan.get(Source::Ac, Lever::Governor), Some("powersave"));
        plan.put(Source::Ac, Lever::Governor, None).unwrap();
        assert!(plan.is_empty());
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
    fn keys_round_trip() {
        for l in Lever::ALL {
            assert_eq!(Lever::parse(l.key()), Some(l));
        }
        for s in Source::ALL {
            assert_eq!(Source::parse(s.key()), Some(s));
        }
        assert_eq!(Lever::parse("brightness"), None);
    }

    #[test]
    fn save_and_load_through_a_temp_dir() {
        let dir = std::env::temp_dir().join(format!("cce-power-plan-test-{}", std::process::id()));
        let path = dir.join("nested").join("power.kdl");
        // Missing file is an empty plan, not an error.
        assert_eq!(PowerPlan::load_from(&path).unwrap(), PowerPlan::default());
        let mut plan = PowerPlan::default();
        plan.put(Source::Battery, Lever::Aspm, Some("powersave")).unwrap();
        plan.save_to(&path).unwrap();
        assert_eq!(PowerPlan::load_from(&path).unwrap(), plan);
        // Garbage on disk is reported, not silently emptied.
        std::fs::write(&path, "ac {\n  profile \n").unwrap();
        assert!(PowerPlan::load_from(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
