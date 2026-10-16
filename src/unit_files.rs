//! `systemctl list-unit-files`, cached for the Services and Timers pages.
//!
//! The system scope takes ~280 ms: systemd walks every unit directory to
//! answer it. Both pages asked on every refresh (every 3 s and 5 s), which
//! held the first paint back by that much and kept systemd busy for as long
//! as either page was open. What it reports, the installed units and whether
//! each starts at boot, changes only when a unit is installed, removed,
//! enabled or disabled; so it is re-read after MAX_AGE, and at once after
//! anything either page does (`invalidate`). The live state (`list-units`,
//! `list-timers`, `show`) is still read every refresh.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_AGE: Duration = Duration::from_secs(30);

/// (system scope?, unit type) → when it was read, and what it said.
static CACHE: Mutex<Option<HashMap<(bool, &'static str), (Instant, String)>>> = Mutex::new(None);

/// `systemctl [--user] list-unit-files --type=<kind> --no-legend`, from the
/// cache when it is fresh. A failed or timed-out read is not cached.
pub async fn list(is_system: bool, kind: &'static str) -> String {
    if let Some((at, text)) = CACHE.lock().unwrap().as_ref().and_then(|c| c.get(&(is_system, kind))) {
        if at.elapsed() < MAX_AGE {
            return text.clone();
        }
    }
    let mut cmd = tokio::process::Command::new("systemctl");
    if !is_system {
        cmd.arg("--user");
    }
    cmd.env("SYSTEMD_COLORS", "0").args(["list-unit-files", &format!("--type={kind}"), "--no-legend"]);
    match crate::output_within(&mut cmd, crate::FETCH_TIMEOUT).await {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).into_owned();
            CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert((is_system, kind), (Instant::now(), text.clone()));
            text
        }
        _ => String::new(),
    }
}

/// Forget every cached list: the next read asks systemd.
pub fn invalidate() {
    *CACHE.lock().unwrap() = None;
}
