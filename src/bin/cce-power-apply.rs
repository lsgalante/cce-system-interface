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
//!   (`udev/90-cce-power-apply.rules`). Per-lever failures are logged and do
//!   not fail the run: a missing NVIDIA driver must not hide the CPU profile
//!   that did land.
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

fn cmd_apply(forced: Option<&str>) -> i32 {
    let source = match forced {
        None => current_source(),
        Some(s) => Source::parse(s).unwrap_or_else(|| usage()),
    };
    let plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    run_mode(&plan, plan.assigned(source), source.key())
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
        Some("apply-mode") => cmd_apply_mode(&args[1..]),
        Some("set") => cmd_set(&args[1..]),
        Some("assign") => cmd_assign(&args[1..]),
        Some("show") => cmd_show(),
        _ => usage(),
    };
    std::process::exit(code);
}
