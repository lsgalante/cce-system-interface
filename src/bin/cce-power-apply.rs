//! cce-power-apply — the root side of the Power page's modes and their
//! adapter-state assignment.
//!
//! The System Interface's Power page keeps a named set of levers per power
//! mode plus a table saying which mode runs plugged in and which on battery
//! (`/etc/cce/power.kdl`, see `cce_settings::power_plan`). Something has to
//! write those levers into sysfs as root whenever the charger comes or goes,
//! with no session and no prompt — that is this binary:
//!
//! - `apply [ac|battery]` — apply the mode assigned to the current (or the
//!   named) adapter state. Run by `cce-power-apply.service`, which udev
//!   starts when the Mains supply appears at boot or flips online/offline
//!   (`udev/90-cce-power-apply.rules`), and by
//!   `cce-power-apply-resume.service` after every wake. It reads the source
//!   again when it finishes and re-applies if it moved. Per-lever failures
//!   are logged and do not fail the run: a missing NVIDIA driver must not
//!   hide the CPU profile that did land.
//! - `sleep` — lower PCIe ASPM to `powersupersave` before the machine
//!   sleeps, when the running mode sets it to anything else. Run by
//!   `cce-power-apply-sleep.service`; the resume unit's `apply` puts the
//!   mode's own value back.
//! - `apply-mode <mode>` — apply one mode by name, whatever is plugged in.
//! - `set <mode> <lever> <value|unset>` — record one lever on a mode and,
//!   when that mode is the one running, apply it now. Run by the Power page
//!   under pkexec, the app's standard privileged path.
//! - `assign <ac|battery> <mode>` — point an adapter state at a mode and,
//!   when that state is the live one, apply the mode now.
//! - `show` — print the plan and the live adapter state.
//!
//! Installed to `/usr/bin` by `ccebuild install-system` (the udev rule and
//! the unit name that path); `ccebuild install` also drops a copy in
//! `~/.local/bin`, which the page falls back to under pkexec before the root
//! side is installed.

use cce_settings::power_plan::{
    apply_lever, apply_mode, current_source, Lever, Mode, PowerPlan, Source, PLAN_PATH,
};

fn usage() -> ! {
    eprintln!(
        "usage: cce-power-apply apply [ac|battery]\n       \
                cce-power-apply sleep\n       \
                cce-power-apply apply-mode <mode>\n       \
                cce-power-apply set <mode> <lever> <value|unset>\n       \
                cce-power-apply assign <ac|battery> <mode>\n       \
                cce-power-apply show\n\
         modes:  {}\n\
         levers: {}",
        Mode::ALL.iter().map(|m| m.key()).collect::<Vec<_>>().join(" "),
        Lever::ALL.iter().map(|l| l.key()).collect::<Vec<_>>().join(" ")
    );
    std::process::exit(2)
}

fn load_plan() -> Result<PowerPlan, i32> {
    PowerPlan::load().map_err(|e| {
        eprintln!("cce-power-apply: {}", e);
        1
    })
}

/// Apply one mode and report each lever. `what` names what asked for it —
/// the adapter state, or the mode itself — so the journal says why.
fn run_mode(plan: &PowerPlan, mode: Mode, what: &str) -> i32 {
    let results = apply_mode(plan, mode);
    if results.is_empty() {
        println!("cce-power-apply: {} runs {}, which sets nothing", what, mode.key());
        return 0;
    }
    for (lever, result) in &results {
        match result {
            Ok(()) => println!("{} [{}]: {} = {}", what, mode.key(), lever.key(), plan.get(mode, *lever).unwrap_or("")),
            Err(e) => eprintln!("cce-power-apply: {} [{}] {}: {}", what, mode.key(), lever.key(), e),
        }
    }
    0
}

/// How many times one `apply` follows the source changing under it before
/// it gives up and leaves the next udev event to finish the job. A charger
/// with a bad contact can flap for as long as it likes.
const MAX_APPLY_PASSES: usize = 4;

fn cmd_apply(forced: Option<&str>) -> i32 {
    let plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    if let Some(s) = forced {
        let source = Source::parse(s).unwrap_or_else(|| usage());
        return run_mode(&plan, plan.assigned(source), source.key());
    }
    // Read the source again after applying, and go round once more if it
    // moved. A udev event that lands while this run is still going cannot
    // start another one: `systemctl start` on a oneshot that is already
    // activating merges into the running job. Until 2026-10-02 that lost a
    // plug-in at resume: a run started by an unplug as the lid closed was
    // frozen with the rest of user space, finished two hours later on
    // resume, and applied the battery mode it had read before sleeping while
    // the plug-in's event merged into it. Animations stayed off on AC until
    // the charger was replugged.
    let mut source = current_source();
    let mut pass = 1;
    loop {
        let code = run_mode(&plan, plan.assigned(source), source.key());
        let now = current_source();
        if now == source {
            return code;
        }
        if pass == MAX_APPLY_PASSES {
            eprintln!(
                "cce-power-apply: source still changing after {} passes; applied {}, now {}",
                pass, source.key(), now.key()
            );
            return code;
        }
        println!("cce-power-apply: source changed to {} while applying {}; applying again", now.key(), source.key());
        source = now;
        pass += 1;
    }
}

/// The ASPM policy every sleep starts under, whatever mode is running.
const SLEEP_ASPM: &str = "powersupersave";

/// Ready the machine to sleep. This laptop has only s2idle, where the
/// package reaches S0ix only if its PCIe links can drop into L1 substates,
/// and the AC mode's `aspm "performance"` forbids that. Nothing re-applies
/// the plan while the machine sleeps, so a suspend begun on the charger kept
/// that mode through an unplug: the battery history before 2026-10-04 shows
/// sleeps begun on battery drawing about 1 W, and those begun on AC up to
/// 4 W, enough to empty a full battery in a day and a half closed.
///
/// Only ASPM moves. The rest of a mode either has no effect in s2idle or,
/// like the NVIDIA power limit, costs seconds of nvidia-smi on every lid
/// close. And only when the running mode sets it: the resume unit's `apply`
/// then restores that mode's value, so a mode that leaves ASPM alone is not
/// woken into a policy it never chose.
fn cmd_sleep() -> i32 {
    let plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    let source = current_source();
    let mode = plan.assigned(source);
    match plan.get(mode, Lever::Aspm) {
        None => {
            println!("cce-power-apply: sleep: {} [{}] sets no aspm; left as is", source.key(), mode.key());
            0
        }
        Some(v) if v == SLEEP_ASPM => 0,
        Some(v) => match apply_lever(Lever::Aspm, SLEEP_ASPM) {
            Ok(()) => {
                println!("cce-power-apply: sleep: aspm {} -> {} ({} [{}] restores it on wake)", v, SLEEP_ASPM, source.key(), mode.key());
                0
            }
            Err(e) => {
                // Never fail the unit: a sleep that drains faster beats one
                // systemd refuses to start.
                eprintln!("cce-power-apply: sleep: aspm: {}", e);
                0
            }
        },
    }
}

fn cmd_apply_mode(rest: &[String]) -> i32 {
    let [mode] = rest else { usage() };
    let mode = Mode::parse(mode).unwrap_or_else(|| usage());
    let plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    run_mode(&plan, mode, "mode")
}

fn cmd_set(rest: &[String]) -> i32 {
    let [mode, lever, value] = rest else { usage() };
    let mode = Mode::parse(mode).unwrap_or_else(|| usage());
    let lever = Lever::parse(lever).unwrap_or_else(|| usage());
    let value: Option<&str> = if value == "unset" { None } else { Some(value.as_str()) };
    let mut plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    if let Err(e) = plan.put(mode, lever, value) {
        eprintln!("cce-power-apply: {}", e);
        return 2;
    }
    if let Err(e) = plan.save() {
        eprintln!("cce-power-apply: writing {}: {}", PLAN_PATH, e);
        return 1;
    }
    // Only the mode the machine is running right now touches sysfs; editing
    // any other mode is a plan change and nothing more.
    if mode == plan.assigned(current_source()) {
        if let Some(v) = value {
            if let Err(e) = apply_lever(lever, v) {
                eprintln!("cce-power-apply: {} {}: {}", mode.key(), lever.key(), e);
                return 1;
            }
        }
    }
    0
}

fn cmd_assign(rest: &[String]) -> i32 {
    let [source, mode] = rest else { usage() };
    let source = Source::parse(source).unwrap_or_else(|| usage());
    let mode = Mode::parse(mode).unwrap_or_else(|| usage());
    let mut plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    plan.assign(source, mode);
    if let Err(e) = plan.save() {
        eprintln!("cce-power-apply: writing {}: {}", PLAN_PATH, e);
        return 1;
    }
    // Reassigning the live adapter state hands the machine to another mode
    // now, not at the next unplug.
    if source == current_source() {
        return run_mode(&plan, mode, source.key());
    }
    0
}

fn cmd_show() -> i32 {
    match load_plan() {
        Ok(plan) => {
            print!("{}", plan.to_kdl());
            let source = current_source();
            println!("// live: {} running {}", source.key(), plan.assigned(source).key());
            0
        }
        Err(code) => code,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("apply") => cmd_apply(args.get(1).map(String::as_str)),
        Some("sleep") => cmd_sleep(),
        Some("apply-mode") => cmd_apply_mode(&args[1..]),
        Some("set") => cmd_set(&args[1..]),
        Some("assign") => cmd_assign(&args[1..]),
        Some("show") => cmd_show(),
        _ => usage(),
    };
    std::process::exit(code);
}
