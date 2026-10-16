mod accounts_file;
pub mod app;
pub mod pages;
pub mod portal_routing;
pub mod power_meter;
pub mod power_plan;
pub use cce_ui::widget::scroll_region;
pub mod widgets;
pub mod watchers;
pub(crate) mod unit_files;

/// Spawn a command and reap it from a background thread, so a finished
/// xdg-open never lingers as a zombie (cce-core's, via cce-ui).
pub(crate) use cce_ui::process::spawn_detached;

/// [`spawn_detached`] for a `tokio::process::Command`: the child is awaited on
/// a runtime task, so it is reaped the moment it exits. A dropped tokio Child
/// was left to the runtime's best-effort orphan queue instead. Needs the
/// runtime `main` enters, as the spawn itself does.
/// How long a page's read (a `pactl`, `nmcli`, `systemctl`, `bluetoothctl`
/// query) may take. They answer in milliseconds; one that has not answered
/// in this long is hung on its daemon, and until 2026-10-16 it froze its page
/// at its last snapshot (or "Loading...") for as long as the daemon was stuck.
pub(crate) const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// `cmd.output()`, given up after `limit`: the child is killed and the
/// error is `TimedOut`, which a page reports as that source being unavailable.
/// For reads only, never an action a person is waiting on (an update, a
/// polkit prompt), which can rightly take minutes.
pub(crate) async fn output_within(
    cmd: &mut tokio::process::Command,
    limit: std::time::Duration,
) -> std::io::Result<std::process::Output> {
    cmd.kill_on_drop(true);
    match tokio::time::timeout(limit, cmd.output()).await {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("no answer in {} s", limit.as_secs()),
        )),
    }
}

pub(crate) fn spawn_awaited(mut cmd: tokio::process::Command) -> std::io::Result<()> {
    let mut child = cmd.spawn()?;
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}
