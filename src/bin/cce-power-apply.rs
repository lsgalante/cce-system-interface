//! cce-power-apply — the root side of the Power page's per-source plan.
//!
//! The System Interface's Power page keeps one set of levers for "plugged
//! in" and one for "on battery" (`/etc/cce/power.kdl`, see
//! `cce_settings::power_plan`). Something has to write those levers into
//! sysfs as root whenever the charger comes or goes, with no session and no
//! prompt — that is this binary:
//!
//! - `apply [ac|battery]` — apply the plan for the current (or the named)
//!   source. Run by `cce-power-apply.service`, which udev starts when the
//!   Mains supply appears at boot or flips online/offline
//!   (`udev/90-cce-power-apply.rules`). Per-lever failures are logged and do
//!   not fail the run: a missing NVIDIA driver must not hide the CPU profile
//!   that did land.
//! - `set <ac|battery> <lever> <value|unset>` — record one lever in the plan
//!   and, when that source is the live one, apply it now. Run by the Power
//!   page under pkexec, the app's standard privileged path.
//! - `show` — print the plan and the live source.
//!
//! Installed to `/usr/bin` by `ccebuild install-system` (the udev rule and
//! the unit name that path); `ccebuild install` also drops a copy in
//! `~/.local/bin`, which the page falls back to under pkexec before the root
//! side is installed.

use cce_settings::power_plan::{apply_lever, apply_source, current_source, Lever, PowerPlan, Source, PLAN_PATH};

fn usage() -> ! {
    eprintln!(
        "usage: cce-power-apply apply [ac|battery]\n       \
                cce-power-apply set <ac|battery> <lever> <value|unset>\n       \
                cce-power-apply show\n\
         levers: {}",
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

fn cmd_apply(forced: Option<&str>) -> i32 {
    let source = match forced {
        None => current_source(),
        Some(s) => Source::parse(s).unwrap_or_else(|| usage()),
    };
    let plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    let results = apply_source(&plan, source);
    if results.is_empty() {
        println!("cce-power-apply: nothing planned for {}", source.key());
        return 0;
    }
    for (lever, result) in &results {
        match result {
            Ok(()) => println!("{}: {} = {}", source.key(), lever.key(), plan.get(source, *lever).unwrap_or("")),
            Err(e) => eprintln!("cce-power-apply: {} {}: {}", source.key(), lever.key(), e),
        }
    }
    0
}

fn cmd_set(rest: &[String]) -> i32 {
    let [source, lever, value] = rest else { usage() };
    let source = Source::parse(source).unwrap_or_else(|| usage());
    let lever = Lever::parse(lever).unwrap_or_else(|| usage());
    let value: Option<&str> = if value == "unset" { None } else { Some(value.as_str()) };
    let mut plan = match load_plan() {
        Ok(p) => p,
        Err(code) => return code,
    };
    if let Err(e) = plan.put(source, lever, value) {
        eprintln!("cce-power-apply: {}", e);
        return 2;
    }
    if let Err(e) = plan.save() {
        eprintln!("cce-power-apply: writing {}: {}", PLAN_PATH, e);
        return 1;
    }
    if source == current_source() {
        if let Some(v) = value {
            if let Err(e) = apply_lever(lever, v) {
                eprintln!("cce-power-apply: {} {}: {}", source.key(), lever.key(), e);
                return 1;
            }
        }
    }
    0
}

fn cmd_show() -> i32 {
    match load_plan() {
        Ok(plan) => {
            print!("{}", plan.to_kdl());
            println!("// live source: {}", current_source().key());
            0
        }
        Err(code) => code,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("apply") => cmd_apply(args.get(1).map(String::as_str)),
        Some("set") => cmd_set(&args[1..]),
        Some("show") => cmd_show(),
        _ => usage(),
    };
    std::process::exit(code);
}
