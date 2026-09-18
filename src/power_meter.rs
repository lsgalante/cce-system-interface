//! Per-process power estimation for the Processes page.
//!
//! Nothing on a laptop reports watts per process, so this is an attribution
//! model, not a measurement. What IS measured, per sample:
//!
//! - the whole-machine draw: `BAT0/power_now` while discharging, or the
//!   RAPL `psys` domain when its counter is readable (it is root-only by
//!   kernel default since the PLATYPUS fix, so on AC it usually is not);
//! - per-domain CPU energy from RAPL `core` / `uncore` (uncore ≈ the iGPU on
//!   Intel client parts), again only when readable;
//! - the discrete GPU's draw from nvidia-smi, only while the card is awake;
//! - per process: CPU ticks from `/proc/<pid>/stat`, iGPU engine busy time
//!   from the i915 `fdinfo` of every `/dev/dri` fd it holds, context switches
//!   from `status` (wakeups/s, powertop's idle metric), and whether it holds a
//!   `/dev/nvidia*` fd.
//!
//! The split then works on *deltas* between two snapshots. Each measured
//! source is reduced by its own rolling minimum over the last minute — the
//! floor is the screen, radios and idle silicon, which no process owns — and
//! only the part above the floor is handed out, in proportion to each pid's
//! share of the activity that source responds to. Battery-only mode has one
//! budget (total minus floor, minus whatever the dGPU took) split by CPU time
//! with iGPU time folded in; RAPL mode has a budget per domain. Per-pid
//! results are smoothed with a ~10s exponential average so the list does not
//! reorder on every tick.
//!
//! Everything below [`Meter`] is pure and unit-tested on synthetic snapshots;
//! [`sample`] is the only thing that touches the machine.

use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;
use std::time::Instant;

/// `/proc` CPU times are in USER_HZ ticks, which Linux fixes at 100 on every
/// architecture whatever the kernel's own HZ is.
pub const USER_HZ: f64 = 100.0;
/// One nanosecond of iGPU engine time counted against one nanosecond of CPU
/// core time in the battery-only split. A busy render engine and a busy core
/// land in the same handful of watts on this class of part, so 1:1 is as
/// honest as any other single figure; RAPL mode does not use it.
pub const GPU_NS_WEIGHT: f64 = 1.0;
/// How far back the rolling-minimum floor looks.
pub const FLOOR_WINDOW_SECS: f64 = 60.0;
/// Time constant of the per-pid smoothing.
pub const SMOOTH_TAU_SECS: f64 = 10.0;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcSample {
    /// utime + stime, USER_HZ ticks.
    pub cpu_ticks: u64,
    /// Summed i915 engine busy time across the pid's DRM clients.
    pub gpu_ns: u64,
    /// voluntary + nonvoluntary context switches.
    pub ctxt_switches: u64,
    /// Holds an open `/dev/nvidia*` fd.
    pub dgpu: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RaplDomain {
    /// sysfs `name`: `package-0`, `core`, `uncore`, `psys`, …
    pub name: String,
    pub energy_uj: u64,
    /// `max_energy_range_uj` — the counter wraps at this.
    pub range_uj: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// Seconds on an arbitrary monotonic clock.
    pub t: f64,
    /// Battery draw, only while discharging (the field is meaningless
    /// otherwise, so the sampler leaves it None).
    pub battery_w: Option<f64>,
    pub rapl: Vec<RaplDomain>,
    /// nvidia-smi power.draw, None while the card is runtime-suspended or
    /// there is no driver.
    pub dgpu_w: Option<f64>,
    pub procs: HashMap<u32, ProcSample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One snapshot so far, or a zero-length interval: nothing to attribute yet.
    Warming,
    /// RAPL core/uncore readable: CPU and iGPU budgets are measured per domain.
    Rapl,
    /// Only the battery's total is known: one budget, split by activity.
    Battery,
    /// On AC without RAPL access, or no battery: wakeups only.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PidPower {
    /// Smoothed estimate; None when the mode cannot produce one.
    pub watts: Option<f32>,
    pub wakeups_per_s: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub mode: Mode,
    /// Whole-machine draw this interval, when any source reports it.
    pub total_w: Option<f32>,
    /// Rolling-minimum of `total_w`: the unattributable baseline.
    pub floor_w: Option<f32>,
    /// Sum of every pid's smoothed estimate.
    pub attributed_w: f32,
    pub per_pid: HashMap<u32, PidPower>,
}

impl Attribution {
    fn empty(mode: Mode) -> Self {
        Self { mode, total_w: None, floor_w: None, attributed_w: 0.0, per_pid: HashMap::new() }
    }
}

/// Rolling minimum over a time window.
#[derive(Debug, Default)]
struct Floor {
    samples: VecDeque<(f64, f64)>,
}

impl Floor {
    /// Record `w` at time `t`; return the minimum over the window ending now.
    fn push(&mut self, t: f64, w: f64) -> f64 {
        self.samples.push_back((t, w));
        while let Some(&(t0, _)) = self.samples.front() {
            if t - t0 > FLOOR_WINDOW_SECS {
                self.samples.pop_front();
            } else {
                break;
            }
        }
        self.samples.iter().map(|&(_, w)| w).fold(f64::INFINITY, f64::min)
    }
}

/// Holds the previous snapshot, the floors and the smoothing state between
/// ticks. One per page; the fetch keeps it in a static.
#[derive(Debug, Default)]
pub struct Meter {
    prev: Option<Snapshot>,
    floors: HashMap<&'static str, Floor>,
    ema: HashMap<u32, f64>,
}

/// Counter delta with wrap-around at `range` (0 = no wrap known).
fn wrapped_delta(prev: u64, next: u64, range: u64) -> u64 {
    if next >= prev {
        next - prev
    } else if range > 0 {
        range - prev + next
    } else {
        0
    }
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }

    fn floor(&mut self, key: &'static str, t: f64, w: f64) -> f64 {
        self.floors.entry(key).or_default().push(t, w)
    }

    /// Budget a source hands out: its reading above its own rolling minimum.
    fn budget(&mut self, key: &'static str, t: f64, w: Option<f64>) -> f64 {
        match w {
            Some(w) => (w - self.floor(key, t, w)).max(0.0),
            None => 0.0,
        }
    }

    pub fn tick(&mut self, snap: Snapshot) -> Attribution {
        let Some(prev) = self.prev.replace(snap) else {
            // Instantaneous sources seed their floors now, so the next
            // tick's reading is measured against this one rather than
            // against itself. RAPL is a counter and needs the interval.
            let s = self.prev.as_ref().unwrap();
            let (t, bat, dgpu) = (s.t, s.battery_w, s.dgpu_w);
            if let Some(w) = bat {
                self.floor("total", t, w);
            }
            if let Some(w) = dgpu {
                self.floor("dgpu", t, w);
            }
            return Attribution::empty(Mode::Warming);
        };
        let snap = self.prev.as_ref().unwrap();
        let dt = snap.t - prev.t;
        if dt <= 0.0 {
            return Attribution::empty(Mode::Warming);
        }

        // RAPL watts per domain over the interval.
        let rapl_w = |name: &str| -> Option<f64> {
            let a = prev.rapl.iter().find(|d| d.name == name)?;
            let b = snap.rapl.iter().find(|d| d.name == name)?;
            Some(wrapped_delta(a.energy_uj, b.energy_uj, b.range_uj) as f64 / 1e6 / dt)
        };
        let core = rapl_w("core");
        let uncore = rapl_w("uncore");
        let psys = rapl_w("psys");
        let package = snap
            .rapl
            .iter()
            .find(|d| d.name.starts_with("package"))
            .and_then(|d| rapl_w(&d.name));

        let total = snap.battery_w.or(psys).or(package);
        let mode = if core.is_some() {
            Mode::Rapl
        } else if snap.battery_w.is_some() {
            Mode::Battery
        } else {
            Mode::Unavailable
        };

        // Per-pid activity over the interval, pids present in both snapshots.
        struct Delta {
            cpu_ns: f64,
            gpu_ns: f64,
            wakeups: f64,
            dgpu: bool,
        }
        let deltas: HashMap<u32, Delta> = snap
            .procs
            .iter()
            .filter_map(|(pid, b)| {
                let a = prev.procs.get(pid)?;
                Some((
                    *pid,
                    Delta {
                        cpu_ns: b.cpu_ticks.saturating_sub(a.cpu_ticks) as f64 * 1e9 / USER_HZ,
                        gpu_ns: b.gpu_ns.saturating_sub(a.gpu_ns) as f64,
                        wakeups: b.ctxt_switches.saturating_sub(a.ctxt_switches) as f64 / dt,
                        dgpu: b.dgpu,
                    },
                ))
            })
            .collect();

        let t = snap.t;
        let snap_dgpu = snap.dgpu_w;
        let floor = total.map(|w| self.floor("total", t, w));
        let dgpu_budget = self.budget("dgpu", t, snap_dgpu);
        let dgpu_holders = deltas.values().filter(|d| d.dgpu).count() as f64;

        // Raw per-pid watts for this interval.
        let mut raw: HashMap<u32, f64> = HashMap::new();
        let share = |get: &dyn Fn(&Delta) -> f64| -> HashMap<u32, f64> {
            let sum: f64 = deltas.values().map(|d| get(d)).sum();
            if sum <= 0.0 {
                return HashMap::new();
            }
            deltas.iter().map(|(pid, d)| (*pid, get(d) / sum)).collect()
        };
        match mode {
            Mode::Rapl => {
                let cpu_budget = self.budget("core", t, core);
                let gpu_budget = self.budget("uncore", t, uncore);
                for (pid, s) in share(&|d| d.cpu_ns) {
                    *raw.entry(pid).or_default() += s * cpu_budget;
                }
                for (pid, s) in share(&|d| d.gpu_ns) {
                    *raw.entry(pid).or_default() += s * gpu_budget;
                }
            }
            Mode::Battery => {
                let dynamic = (total.unwrap() - floor.unwrap()).max(0.0);
                let budget = (dynamic - dgpu_budget).max(0.0);
                for (pid, s) in share(&|d| d.cpu_ns + GPU_NS_WEIGHT * d.gpu_ns) {
                    *raw.entry(pid).or_default() += s * budget;
                }
            }
            Mode::Warming | Mode::Unavailable => {}
        }
        if mode != Mode::Unavailable && dgpu_holders > 0.0 {
            for (pid, d) in &deltas {
                if d.dgpu {
                    *raw.entry(*pid).or_default() += dgpu_budget / dgpu_holders;
                }
            }
        }

        // Smooth, and forget pids that are gone.
        let alpha = 1.0 - (-dt / SMOOTH_TAU_SECS).exp();
        self.ema.retain(|pid, _| deltas.contains_key(pid));
        let mut per_pid = HashMap::with_capacity(deltas.len());
        let mut attributed = 0.0;
        for (pid, d) in &deltas {
            let watts = if mode == Mode::Unavailable {
                None
            } else {
                let r = raw.get(pid).copied().unwrap_or(0.0);
                let e = match self.ema.get(pid) {
                    Some(&e) => e + alpha * (r - e),
                    None => r,
                };
                self.ema.insert(*pid, e);
                attributed += e;
                Some(e as f32)
            };
            per_pid.insert(*pid, PidPower { watts, wakeups_per_s: d.wakeups as f32 });
        }

        Attribution {
            mode,
            total_w: total.map(|w| w as f32),
            floor_w: floor.map(|w| w as f32),
            attributed_w: attributed as f32,
            per_pid,
        }
    }
}

// ── The machine-facing half ──

fn read_trim(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn clock_secs() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// utime + stime from a `/proc/<pid>/stat` line. The comm field can contain
/// spaces and parens, so fields are counted from the LAST `)`.
pub fn parse_stat_ticks(stat: &str) -> Option<u64> {
    let rest = &stat[stat.rfind(')')? + 1..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    // After ')': state ppid pgrp session tty tpgid flags minflt cminflt
    // majflt cmajflt utime stime …
    Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
}

/// voluntary + nonvoluntary context switches from `/proc/<pid>/status`.
pub fn parse_status_switches(status: &str) -> u64 {
    status
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.ends_with("ctxt_switches") { v.trim().parse::<u64>().ok() } else { None }
        })
        .sum()
}

/// (client id, summed engine busy ns) from one DRM fd's fdinfo. Capacity
/// lines are counts, not times, and are skipped.
pub fn parse_fdinfo_engines(fdinfo: &str) -> Option<(u64, u64)> {
    let mut client = None;
    let mut ns = 0u64;
    for l in fdinfo.lines() {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        if k == "drm-client-id" {
            client = v.parse().ok();
        } else if k.starts_with("drm-engine-") && !k.starts_with("drm-engine-capacity") {
            ns += v.split_whitespace().next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
        }
    }
    client.map(|c| (c, ns))
}

fn sample_proc(dir: &std::path::Path) -> Option<ProcSample> {
    let cpu_ticks = parse_stat_ticks(&std::fs::read_to_string(dir.join("stat")).ok()?)?;
    let ctxt_switches =
        std::fs::read_to_string(dir.join("status")).map(|s| parse_status_switches(&s)).unwrap_or(0);

    // fd links are only readable for our own processes; others simply
    // contribute no GPU time.
    let mut clients: HashMap<u64, u64> = HashMap::new();
    let mut dgpu = false;
    if let Ok(fds) = std::fs::read_dir(dir.join("fd")) {
        for fd in fds.flatten() {
            let Ok(target) = std::fs::read_link(fd.path()) else { continue };
            let Some(target) = target.to_str() else { continue };
            if target.starts_with("/dev/nvidia") {
                dgpu = true;
            } else if target.starts_with("/dev/dri/") {
                let info = dir.join("fdinfo").join(fd.file_name());
                if let Some((client, ns)) =
                    std::fs::read_to_string(info).ok().and_then(|s| parse_fdinfo_engines(&s))
                {
                    // Several fds can share one client; count it once.
                    clients.insert(client, ns);
                }
            }
        }
    }
    Some(ProcSample { cpu_ticks, gpu_ns: clients.values().sum(), ctxt_switches, dgpu })
}

fn sample_rapl() -> Vec<RaplDomain> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/powercap") else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("intel-rapl:")) {
            continue;
        }
        let (Some(name), Some(energy)) = (read_trim(&p.join("name")), read_trim(&p.join("energy_uj")))
        else {
            continue;
        };
        let Ok(energy_uj) = energy.parse() else { continue };
        let range_uj = read_trim(&p.join("max_energy_range_uj")).and_then(|s| s.parse().ok()).unwrap_or(0);
        out.push(RaplDomain { name, energy_uj, range_uj });
    }
    out
}

fn sample_battery_w() -> Option<f64> {
    let dir = crate::pages::power::battery_dir()?;
    if read_trim(&dir.join("status"))? != "Discharging" {
        return None;
    }
    read_trim(&dir.join("power_now"))?.parse::<f64>().ok().map(|uw| uw / 1e6)
}

/// Whether the NVIDIA card is awake. Polling nvidia-smi wakes a suspended
/// card — a few watts, every tick, for a number that would be zero — so the
/// caller only queries the draw when this says active.
pub fn dgpu_awake() -> bool {
    let Ok(devs) = std::fs::read_dir("/sys/bus/pci/drivers/nvidia") else { return false };
    devs.flatten()
        .any(|d| read_trim(&d.path().join("power/runtime_status")).as_deref() == Some("active"))
}

/// One pass over the machine. `dgpu_w` is passed in because it comes from an
/// async nvidia-smi call the caller owns.
pub fn sample(dgpu_w: Option<f64>) -> Snapshot {
    let mut procs = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for e in entries.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
            if let Some(s) = sample_proc(&e.path()) {
                procs.insert(pid, s);
            }
        }
    }
    Snapshot { t: clock_secs(), battery_w: sample_battery_w(), rapl: sample_rapl(), dgpu_w, procs }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(cpu_ticks: u64, gpu_ns: u64, ctxt: u64) -> ProcSample {
        ProcSample { cpu_ticks, gpu_ns, ctxt_switches: ctxt, dgpu: false }
    }

    fn snap(t: f64, battery_w: Option<f64>, procs: &[(u32, ProcSample)]) -> Snapshot {
        Snapshot { t, battery_w, rapl: Vec::new(), dgpu_w: None, procs: procs.iter().cloned().collect() }
    }

    fn w(a: &Attribution, pid: u32) -> f32 {
        a.per_pid[&pid].watts.unwrap()
    }

    #[test]
    fn first_tick_is_warming() {
        let mut m = Meter::new();
        let a = m.tick(snap(0.0, Some(20.0), &[(1, proc(0, 0, 0))]));
        assert_eq!(a.mode, Mode::Warming);
        assert!(a.per_pid.is_empty());
    }

    #[test]
    fn battery_mode_splits_dynamic_by_cpu_share() {
        let mut m = Meter::new();
        m.tick(snap(0.0, Some(10.0), &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0))]));
        // 10 W floor (the minimum seen), 16 W now: 6 W to split 3:1.
        let a = m.tick(snap(3.0, Some(16.0), &[(1, proc(300, 0, 0)), (2, proc(100, 0, 0))]));
        assert_eq!(a.mode, Mode::Battery);
        assert_eq!(a.total_w, Some(16.0));
        assert_eq!(a.floor_w, Some(10.0));
        assert!((w(&a, 1) - 4.5).abs() < 1e-4);
        assert!((w(&a, 2) - 1.5).abs() < 1e-4);
        assert!((a.attributed_w - 6.0).abs() < 1e-4);
    }

    #[test]
    fn igpu_time_counts_in_battery_mode() {
        let mut m = Meter::new();
        m.tick(snap(0.0, Some(10.0), &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0))]));
        // pid 1: 1 s of CPU (100 ticks). pid 2: 1 s of render engine. Equal.
        let a = m.tick(snap(1.0, Some(12.0), &[(1, proc(100, 0, 0)), (2, proc(0, 1_000_000_000, 0))]));
        assert!((w(&a, 1) - w(&a, 2)).abs() < 1e-4);
    }

    #[test]
    fn floor_is_the_rolling_minimum_and_never_goes_negative() {
        let mut m = Meter::new();
        m.tick(snap(0.0, Some(15.0), &[(1, proc(0, 0, 0))]));
        let a = m.tick(snap(3.0, Some(12.0), &[(1, proc(50, 0, 0))]));
        // The new low IS the floor: nothing above it to attribute.
        assert_eq!(a.floor_w, Some(12.0));
        assert_eq!(w(&a, 1), 0.0);
        // 59 s after the 12 W low it is still in the window: 14 W now is
        // 2 W above it, and that goes to the one busy pid.
        let a = m.tick(snap(62.0, Some(14.0), &[(1, proc(100, 0, 0))]));
        assert_eq!(a.floor_w, Some(12.0));
        assert!(w(&a, 1) > 0.0);
        // Once the low has aged out, the floor rises to what is left.
        let a = m.tick(snap(70.0, Some(14.0), &[(1, proc(200, 0, 0))]));
        assert_eq!(a.floor_w, Some(14.0));
    }

    #[test]
    fn wakeups_are_switches_per_second_even_when_unavailable() {
        let mut m = Meter::new();
        m.tick(snap(0.0, None, &[(1, proc(0, 0, 100))]));
        let a = m.tick(snap(2.0, None, &[(1, proc(0, 0, 160))]));
        assert_eq!(a.mode, Mode::Unavailable);
        assert_eq!(a.per_pid[&1].watts, None);
        assert_eq!(a.per_pid[&1].wakeups_per_s, 30.0);
    }

    #[test]
    fn exited_pids_drop_out_and_new_ones_wait_a_tick() {
        let mut m = Meter::new();
        m.tick(snap(0.0, Some(10.0), &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0))]));
        m.tick(snap(3.0, Some(12.0), &[(1, proc(100, 0, 0)), (2, proc(100, 0, 0))]));
        let a = m.tick(snap(6.0, Some(12.0), &[(1, proc(200, 0, 0)), (3, proc(5, 0, 0))]));
        assert!(!a.per_pid.contains_key(&2));
        assert!(!a.per_pid.contains_key(&3), "no previous sample to diff against");
        assert!(!m.ema.contains_key(&2));
    }

    #[test]
    fn smoothing_moves_toward_the_new_value_not_onto_it() {
        let mut m = Meter::new();
        m.tick(snap(0.0, Some(10.0), &[(1, proc(0, 0, 0))]));
        let a = m.tick(snap(3.0, Some(16.0), &[(1, proc(300, 0, 0))]));
        assert!((w(&a, 1) - 6.0).abs() < 1e-4, "first estimate is taken as-is");
        // Activity stops: raw drops to 0, the average decays toward it.
        let a = m.tick(snap(6.0, Some(10.0), &[(1, proc(300, 0, 0))]));
        let v = w(&a, 1);
        assert!(v > 0.0 && v < 6.0, "{v}");
    }

    fn rapl(name: &str, uj: u64, range: u64) -> RaplDomain {
        RaplDomain { name: name.to_string(), energy_uj: uj, range_uj: range }
    }

    #[test]
    fn rapl_mode_budgets_core_by_cpu_and_uncore_by_gpu() {
        let mut m = Meter::new();
        let mut s0 = snap(0.0, None, &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0))]);
        s0.rapl = vec![rapl("package-0", 0, 0), rapl("core", 0, 0), rapl("uncore", 0, 0)];
        m.tick(s0);
        // Interval 1 establishes the floors (idle: 1 W core, 0.5 W uncore).
        let mut s1 = snap(1.0, None, &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0))]);
        s1.rapl = vec![rapl("package-0", 2_000_000, 0), rapl("core", 1_000_000, 0), rapl("uncore", 500_000, 0)];
        m.tick(s1);
        // Interval 2: core 5 W (4 above floor), uncore 2.5 W (2 above); pid 1
        // did all the CPU work, pid 2 all the GPU work.
        let mut s2 = snap(2.0, None, &[(1, proc(100, 0, 0)), (2, proc(0, 1_000_000_000, 0))]);
        s2.rapl = vec![rapl("package-0", 10_000_000, 0), rapl("core", 6_000_000, 0), rapl("uncore", 3_000_000, 0)];
        let a = m.tick(s2);
        assert_eq!(a.mode, Mode::Rapl);
        // Both pids already had a (zero) estimate from interval 1, so the
        // 4 W and 2 W raw figures arrive through the smoothing step.
        let alpha = 1.0 - (-1.0f64 / SMOOTH_TAU_SECS).exp();
        assert!((w(&a, 1) as f64 - alpha * 4.0).abs() < 1e-4, "{}", w(&a, 1));
        assert!((w(&a, 2) as f64 - alpha * 2.0).abs() < 1e-4, "{}", w(&a, 2));
        // Total falls back to the package domain when neither battery nor psys report.
        assert_eq!(a.total_w, Some(8.0));
    }

    #[test]
    fn rapl_counter_wraps() {
        assert_eq!(wrapped_delta(900, 100, 1000), 200);
        assert_eq!(wrapped_delta(100, 900, 1000), 800);
        assert_eq!(wrapped_delta(900, 100, 0), 0, "unknown range: no wrap guess");
    }

    #[test]
    fn dgpu_budget_is_split_equally_among_fd_holders() {
        let mut m = Meter::new();
        let mut s0 = snap(0.0, Some(20.0), &[(1, proc(0, 0, 0)), (2, proc(0, 0, 0)), (3, proc(0, 0, 0))]);
        s0.dgpu_w = Some(4.0);
        m.tick(s0);
        let hold = |ticks| ProcSample { cpu_ticks: ticks, gpu_ns: 0, ctxt_switches: 0, dgpu: true };
        // Total +10 W, of which the dGPU rose 6 W: that 6 goes 3+3 to the two
        // holders, and the remaining 4 follows CPU time (all pid 3's).
        let mut s1 = snap(3.0, Some(30.0), &[(1, hold(0)), (2, hold(0)), (3, proc(100, 0, 0))]);
        s1.dgpu_w = Some(10.0);
        let a = m.tick(s1);
        assert!((w(&a, 1) - 3.0).abs() < 1e-4);
        assert!((w(&a, 2) - 3.0).abs() < 1e-4);
        assert!((w(&a, 3) - 4.0).abs() < 1e-4);
    }

    /// Real-machine smoke: `cargo test -p cce-system-interface --lib
    /// live_sample -- --ignored --nocapture`. Prints the sampler's cost, the
    /// mode this host lands in, and the top estimates.
    #[test]
    #[ignore]
    fn live_sample() {
        let mut m = Meter::new();
        let t0 = Instant::now();
        let s0 = sample(None);
        let cost = t0.elapsed();
        eprintln!("sample: {:?} for {} pids, battery={:?}, rapl={:?}", cost, s0.procs.len(), s0.battery_w,
            s0.rapl.iter().map(|d| d.name.as_str()).collect::<Vec<_>>());
        m.tick(s0);
        std::thread::sleep(std::time::Duration::from_secs(3));
        let a = m.tick(sample(None));
        eprintln!("mode={:?} total={:?} floor={:?} attributed={}", a.mode, a.total_w, a.floor_w, a.attributed_w);
        let mut rows: Vec<_> = a.per_pid.iter().collect();
        rows.sort_by(|x, y| y.1.watts.partial_cmp(&x.1.watts).unwrap());
        for (pid, p) in rows.iter().take(8) {
            eprintln!("  {pid}: {:?} W, {:.0} wake/s", p.watts, p.wakeups_per_s);
        }
    }

    #[test]
    fn stat_ticks_survive_spaces_and_parens_in_comm() {
        let line = "42 (Web Content (x)) S 1 42 42 0 -1 4194560 100 0 0 0 250 75 0 0 20 0 1 0 12345 0 0";
        assert_eq!(parse_stat_ticks(line), Some(325));
        assert_eq!(parse_stat_ticks("garbage"), None);
    }

    #[test]
    fn status_switches_sum_both_kinds() {
        let s = "Name:\tx\nvoluntary_ctxt_switches:\t120\nnonvoluntary_ctxt_switches:\t5\n";
        assert_eq!(parse_status_switches(s), 125);
    }

    #[test]
    fn fdinfo_engines_sum_time_and_skip_capacity() {
        let s = "drm-driver:\ti915\ndrm-client-id:\t7\ndrm-engine-render:\t1000 ns\ndrm-engine-copy:\t10 ns\ndrm-engine-capacity-video:\t2\ndrm-engine-video:\t5 ns\n";
        assert_eq!(parse_fdinfo_engines(s), Some((7, 1015)));
        assert_eq!(parse_fdinfo_engines("drm-driver:\ti915\n"), None);
    }
}
