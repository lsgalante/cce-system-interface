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
//! Two levers are not sysfs: **animations** and the two **idle timeouts**
//! (`idle_display_off_secs`, `idle_sleep_secs`). The applier records them
//! as files under `/run/cce` ([`cce_ui::motion::STATE_PATH`],
//! [`IDLE_DISPLAY_OFF_PATH`], [`IDLE_SLEEP_PATH`]) that the compositor and
//! the toolkit follow without a reload; the idle files override the
//! compositor's `idle { }` block while they exist, so battery can darken
//! the display sooner than the desk does.
//!
//! The battery **charge limit** is in the plan too, but belongs to no mode
//! (`charge_limit { start 75; end 80 }`, [`ChargeLimit`]): it is a charging
//! policy, and one that changed on every plug and unplug would defeat it.
//! The applier writes it on every `apply` — at boot, on each adapter change
//! and after every wake — because the firmware does not keep it: until
//! 2026-10-06 the Power page wrote sysfs once and recorded nothing, and a
//! battery that ran flat came back charging to 100%.
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
/// the mode changes, so it is plan-wide ([`ChargeLimit`]).
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
    IdleDisplayOffSecs,
    IdleSleepSecs,
}
impl Lever {
    pub const ALL: [Lever; 11] = [
        Lever::Profile,
        Lever::Epp,
        Lever::Governor,
        Lever::Turbo,
        Lever::IgpuMaxMhz,
        Lever::Aspm,
        Lever::AudioIdleSecs,
        Lever::GpuLimitW,
        Lever::Animations,
        Lever::IdleDisplayOffSecs,
        Lever::IdleSleepSecs,
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
            Lever::IdleDisplayOffSecs => "idle_display_off_secs",
            Lever::IdleSleepSecs => "idle_sleep_secs",
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
            Lever::IdleDisplayOffSecs => "Display Off After",
            Lever::IdleSleepSecs => "Sleep After",
        }
    }

    /// Numeric levers are stored as KDL integers; the rest as strings.
    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Lever::IgpuMaxMhz
                | Lever::AudioIdleSecs
                | Lever::GpuLimitW
                | Lever::IdleDisplayOffSecs
                | Lever::IdleSleepSecs
        )
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

/// The battery's charge window, in whole percent: charging stops at `end`
/// and, once stopped, resumes only below `start` — so a pack held at 80%
/// is not topped up from 79% every few minutes. Either half absent means
/// "leave that threshold alone", like an absent lever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChargeLimit {
    pub start: Option<u32>,
    pub end: Option<u32>,
}

impl ChargeLimit {
    pub fn is_unset(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }

    /// The kernel's ranges (start 0–99, end 1–100), and a start below the
    /// end, since a window that resumes at or above where it stops is not
    /// one the firmware will take.
    pub fn check(&self) -> Result<(), String> {
        if self.start.is_some_and(|s| s > 99) {
            return Err("charge_limit start must be 0–99".to_string());
        }
        if self.end.is_some_and(|e| e == 0 || e > 100) {
            return Err("charge_limit end must be 1–100".to_string());
        }
        if let (Some(s), Some(e)) = (self.start, self.end) {
            if s >= e {
                return Err(format!("charge_limit start {} must be below end {}", s, e));
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for ChargeLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.start, self.end) {
            (Some(s), Some(e)) => write!(f, "start {}%, end {}%", s, e),
            (Some(s), None) => write!(f, "start {}%", s),
            (None, Some(e)) => write!(f, "end {}%", e),
            (None, None) => write!(f, "not set"),
        }
    }
}

/// The levers of every mode, which mode each adapter state runs, and the
/// plan-wide battery charge limit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerPlan {
    modes: BTreeMap<Mode, BTreeMap<Lever, String>>,
    assign: BTreeMap<Source, Mode>,
    charge: ChargeLimit,
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

    pub fn charge_limit(&self) -> ChargeLimit {
        self.charge
    }

    /// Record the charge window. Rejects one that fails
    /// [`ChargeLimit::check`] rather than storing it.
    pub fn set_charge_limit(&mut self, limit: ChargeLimit) -> Result<(), String> {
        limit.check()?;
        self.charge = limit;
        Ok(())
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
                "charge_limit" => {
                    let mut limit = ChargeLimit::default();
                    if let Some(children) = node.children() {
                        for child in children.nodes() {
                            let cname = child.name().value();
                            let pct = child
                                .get(0)
                                .and_then(|e| e.value().as_i64())
                                .and_then(|n| u32::try_from(n).ok())
                                .ok_or_else(|| format!("charge_limit.{} needs one whole percent", cname))?;
                            match cname {
                                "start" => limit.start = Some(pct),
                                "end" => limit.end = Some(pct),
                                _ => return Err(format!("unknown setting {:?} under charge_limit", cname)),
                            }
                        }
                    }
                    plan.set_charge_limit(limit)?;
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
        if !self.charge.is_unset() {
            let mut block = kdl::KdlNode::new("charge_limit");
            let children = block.ensure_children();
            for (name, pct) in [("start", self.charge.start), ("end", self.charge.end)] {
                if let Some(pct) = pct {
                    let mut node = kdl::KdlNode::new(name);
                    node.push(kdl::KdlEntry::new(i64::from(pct)));
                    children.nodes_mut().push(node);
                }
            }
            doc.nodes_mut().push(block);
        }
        doc.fmt();
        let mut out = String::from(
            "// Power modes and their adapter-state assignment, edited from the\n\
             // System Interface's Power page and applied by cce-power-apply (udev,\n\
             // boot, and on each change). A lever missing from a mode is left\n\
             // untouched when that mode becomes active. The charge limit belongs\n\
             // to no mode and is re-applied on every change.\n",
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
///
/// The answer is kept per path and modification time: the Power page asks
/// on every five-second refresh, and until 2026-10-05 each one ran the
/// helper. A reinstalled helper has a new mtime and is asked again.
pub fn helper_speaks_modes(path: &Path) -> bool {
    type Seen = Vec<(PathBuf, Option<std::time::SystemTime>, bool)>;
    static SEEN: std::sync::Mutex<Seen> = std::sync::Mutex::new(Vec::new());
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    if let Some(&(_, _, ok)) = SEEN.lock().unwrap().iter().find(|(p, t, _)| p == path && *t == mtime) {
        return ok;
    }
    let ok = std::process::Command::new(path)
        .output()
        .is_ok_and(|out| usage_speaks_modes(&String::from_utf8_lossy(&out.stderr)));
    let mut seen = SEEN.lock().unwrap();
    seen.retain(|(p, _, _)| p != path);
    seen.push((path.to_path_buf(), mtime, ok));
    ok
}

/// The usage text of a helper that knows about modes names `apply-mode`
/// (the pre-modes one lists only `apply`, `set` and `show`), and its
/// `levers:` line names every lever it accepts. It must also name
/// `charge-limit`: a helper from before the charge limit cannot parse a
/// plan holding one, so it would apply nothing on plug or unplug.
fn usage_speaks_modes(usage: &str) -> bool {
    let levers: Vec<&str> = usage
        .lines()
        .find_map(|l| l.trim().strip_prefix("levers:"))
        .map(|l| l.split_whitespace().collect())
        .unwrap_or_default();
    usage.contains("apply-mode")
        && usage.contains("charge-limit")
        && Lever::ALL.iter().all(|l| levers.contains(&l.key()))
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
        Lever::Animations => write_run_state(Path::new(cce_ui::motion::STATE_PATH), value),
        Lever::IdleDisplayOffSecs | Lever::IdleSleepSecs => {
            let secs: u32 = value.parse().map_err(|_| "bad seconds".to_string())?;
            if secs > 86_400 {
                return Err("idle timeout above a day".to_string());
            }
            let path = if lever == Lever::IdleDisplayOffSecs { IDLE_DISPLAY_OFF_PATH } else { IDLE_SLEEP_PATH };
            write_run_state(Path::new(path), value)
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

/// Every battery with a charge-limit knob, `/sys/class/power_supply/BAT*`,
/// sorted.
fn charge_batteries() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir("/sys/class/power_supply")
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("BAT"))
                        && p.join("charge_control_end_threshold").exists()
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Whether the start threshold goes in before the end. The firmware
/// (thinkpad_acpi) refuses a start above the end in force and an end below
/// the start in force, so a window moving up must raise its end first and
/// one moving down must lower its start first; `current_end` is what the
/// battery reports now. Written start-then-end blindly, 75/80 → 85/90
/// fails on the start.
fn start_first(limit: ChargeLimit, current_end: Option<u32>) -> bool {
    match (limit.start, current_end) {
        (Some(s), Some(cur)) => s <= cur,
        _ => true,
    }
}

/// Write the charge window to every battery that has one. A threshold
/// already at its value is not rewritten; a start the battery has no file
/// for is reported after the end has landed, since the end is the half that
/// protects the pack.
pub fn apply_charge_limit(limit: ChargeLimit) -> Result<(), String> {
    limit.check()?;
    if limit.is_unset() {
        return Ok(());
    }
    let batteries = charge_batteries();
    if batteries.is_empty() {
        return Err("no battery exposes charge_control_end_threshold".to_string());
    }
    const START: &str = "charge_control_start_threshold";
    const END: &str = "charge_control_end_threshold";
    let mut missing_start = Vec::new();
    for bat in batteries {
        let read = |file: &str| read_trim(&bat.join(file)).and_then(|s| s.parse::<u32>().ok());
        let start = match limit.start {
            Some(s) if bat.join(START).exists() => Some((START, s)),
            Some(_) => {
                missing_start.push(bat.display().to_string());
                None
            }
            None => None,
        };
        let end = limit.end.map(|e| (END, e));
        let order = if start_first(limit, read(END)) { [start, end] } else { [end, start] };
        for (file, pct) in order.into_iter().flatten() {
            if read(file) != Some(pct) {
                write_sysfs(&bat.join(file), &pct.to_string())?;
            }
        }
    }
    if missing_start.is_empty() {
        Ok(())
    } else {
        Err(format!("no {} on {}; only the end was applied", START, missing_start.join(", ")))
    }
}

/// Where the idle-timeout levers land, in seconds (0 = never). The
/// compositor's idle manager polls both and lets a present file override
/// its `idle { }` block; a missing file means "the config's value". The
/// paths are repeated in `cce-fx`'s `idle.rs` (it cannot depend on this
/// crate), so a rename must land on both sides.
pub const IDLE_DISPLAY_OFF_PATH: &str = "/run/cce/idle_display_off";
pub const IDLE_SLEEP_PATH: &str = "/run/cce/idle_sleep";

/// Record a session-wide switch where every session reads it: the
/// animations file ([`cce_ui::motion::STATE_PATH`]) and the idle-timeout
/// files. Not sysfs, but the same shape of lever: root writes it when the
/// mode changes, and the compositor and every cce-ui client follow it
/// without a restart. Under /run, not in anyone's config: this runs as root
/// with no session and no `$HOME`, and a tmpfs file is rewritten at boot by
/// the same coldplug run that applies the rest of the mode, so it can never
/// outlive the plan that set it.
fn write_run_state(path: &Path, value: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent().expect("run-state path has a parent");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    // Temp + rename so a reader never sees a torn value; world-readable
    // because every session's processes read it.
    let tmp = path.with_extension("tmp");
    write_sysfs(&tmp, value)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))
        .map_err(|e| format!("{}: {}", tmp.display(), e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {}", path.display(), e))
}

/// The order levers are written in. The governor goes first: intel_pstate
/// refuses any energy preference but "performance" while the `performance`
/// governor is in force (EBUSY), and switching governors resets the
/// preference to the one it cached. Written in plan order (alphabetical by
/// `Lever`), the battery mode's `epp "power"` failed on every unplug and
/// left all cores at EPP=performance under the powersave governor. Every
/// other lever is independent and keeps plan order.
fn apply_order<'a>(levers: impl Iterator<Item = (Lever, &'a str)>) -> Vec<(Lever, &'a str)> {
    let mut v: Vec<(Lever, &str)> = levers.collect();
    v.sort_by_key(|(lever, _)| (*lever != Lever::Governor) as u8);
    v
}

/// Apply every lever one mode sets. Failures are per lever — a missing
/// NVIDIA driver must not stop the CPU profile from landing — and come back
/// to the caller, which logs them.
pub fn apply_mode(plan: &PowerPlan, mode: Mode) -> Vec<(Lever, Result<(), String>)> {
    apply_order(plan.levers(mode))
        .into_iter()
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
    fn governor_is_written_before_epp() {
        // The power-saver mode as shipped: plan order puts epp before
        // governor, and intel_pstate rejects that pairing.
        let mut plan = PowerPlan::default();
        plan.put(Mode::PowerSaver, Lever::Profile, Some("low-power")).unwrap();
        plan.put(Mode::PowerSaver, Lever::Epp, Some("power")).unwrap();
        plan.put(Mode::PowerSaver, Lever::Governor, Some("powersave")).unwrap();
        plan.put(Mode::PowerSaver, Lever::Turbo, Some("off")).unwrap();
        let order: Vec<Lever> = apply_order(plan.levers(Mode::PowerSaver)).into_iter().map(|(l, _)| l).collect();
        assert_eq!(order[0], Lever::Governor, "{order:?}");
        // The rest keep plan order, and nothing is dropped or duplicated.
        assert_eq!(order, vec![Lever::Governor, Lever::Profile, Lever::Epp, Lever::Turbo]);
        // A mode without a governor is untouched.
        let mut bare = PowerPlan::default();
        bare.put(Mode::Balanced, Lever::Epp, Some("balance_power")).unwrap();
        let order: Vec<Lever> = apply_order(bare.levers(Mode::Balanced)).into_iter().map(|(l, _)| l).collect();
        assert_eq!(order, vec![Lever::Epp]);
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
        assert!(Lever::IdleDisplayOffSecs.value_ok("0"));
        assert!(Lever::IdleSleepSecs.value_ok("1800"));
        assert!(!Lever::IdleSleepSecs.value_ok("30m"));
        assert!(Lever::IdleDisplayOffSecs.is_numeric());
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
            "usage: cce-power-apply apply [ac|battery]\n       cce-power-apply apply-mode <mode>\n       \
             cce-power-apply charge-limit <start|unset> <end|unset>\n \
             modes:  performance balanced power-saver\n levers: {levers}\n"
        );
        assert!(usage_speaks_modes(&current));
        // One that knows modes but predates a lever takes the prompt and
        // rejects that lever — as stale, for that pick, as a pre-modes one.
        let older = current.replace(" animations", "");
        assert!(!usage_speaks_modes(&older), "{older}");
        // One from before the charge limit cannot even parse a plan that
        // holds one.
        let no_charge = current.replace("charge-limit", "");
        assert!(!usage_speaks_modes(&no_charge), "{no_charge}");
        // What the pre-modes one printed — the copy that silently rejects
        // every pick the page makes.
        assert!(!usage_speaks_modes(
            "usage: cce-power-apply apply [ac|battery]\n       cce-power-apply set <ac|battery> <lever> <value|unset>\n       cce-power-apply show\n"
        ));
        // A helper that cannot be run at all is not a helper that speaks.
        assert!(!helper_speaks_modes(Path::new("/nonexistent/cce-power-apply")));
    }

    #[test]
    fn the_charge_limit_is_plan_wide_and_round_trips() {
        let text = "mode \"performance\" { aspm \"performance\"; }\n\
                    charge_limit { start 75; end 80; }\n";
        let plan = PowerPlan::parse(text).unwrap();
        assert_eq!(plan.charge_limit(), ChargeLimit { start: Some(75), end: Some(80) });
        // It is in no mode's levers, and survives a write and a re-read.
        assert_eq!(levers_of(&plan, Mode::Performance), [(Lever::Aspm, "performance".to_string())]);
        let again = PowerPlan::parse(&plan.to_kdl()).unwrap();
        assert_eq!(again.charge_limit(), plan.charge_limit());
        assert_eq!(levers_of(&again, Mode::Performance), levers_of(&plan, Mode::Performance));
        // Half a window keeps only that half; none writes no block at all.
        let end_only = PowerPlan::parse("charge_limit { end 60; }").unwrap();
        assert_eq!(end_only.charge_limit(), ChargeLimit { start: None, end: Some(60) });
        assert_eq!(PowerPlan::parse(&end_only.to_kdl()).unwrap().charge_limit(), end_only.charge_limit());
        assert!(!PowerPlan::default().to_kdl().contains("charge_limit"));
    }

    #[test]
    fn a_charge_window_must_be_one_the_firmware_takes() {
        for bad in [
            "charge_limit { start 80; end 80; }",
            "charge_limit { start 90; end 80; }",
            "charge_limit { end 0; }",
            "charge_limit { end 101; }",
            "charge_limit { start 100; }",
            "charge_limit { start -1; }",
            "charge_limit { end \"80\"; }",
            "charge_limit { stop 80; }",
        ] {
            assert!(PowerPlan::parse(bad).is_err(), "{bad}");
        }
        let mut plan = PowerPlan::default();
        assert!(plan.set_charge_limit(ChargeLimit { start: Some(85), end: Some(80) }).is_err());
        assert!(plan.charge_limit().is_unset(), "a refused window is not stored");
        plan.set_charge_limit(ChargeLimit { start: Some(0), end: Some(100) }).unwrap();
    }

    #[test]
    fn a_charge_window_moving_up_writes_its_end_first() {
        let window = |s, e| ChargeLimit { start: Some(s), end: Some(e) };
        // 75/80 → 85/90: a start of 85 over an end of 80 is refused.
        assert!(!start_first(window(85, 90), Some(80)));
        // 75/80 → 55/60, and the factory 0/100 → 75/80: start first, or the
        // new end would sit below the old start.
        assert!(start_first(window(55, 60), Some(80)));
        assert!(start_first(window(75, 80), Some(100)));
        // An end the battery does not report, or no start to order.
        assert!(start_first(window(75, 80), None));
        assert!(start_first(ChargeLimit { start: None, end: Some(80) }, Some(60)));
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
