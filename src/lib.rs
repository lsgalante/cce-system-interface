mod accounts_file;
pub mod app;
pub mod pages;
pub mod power_meter;
pub mod power_plan;
pub use cce_ui::widget::scroll_region;
pub mod widgets;
pub mod watchers;

/// Spawn a command and reap it from a background thread, so a finished
/// xdg-open never lingers as a zombie (cce-core's, via cce-ui).
pub(crate) use cce_ui::process::spawn_detached;

/// [`spawn_detached`] for a `tokio::process::Command`: the child is awaited on
/// a runtime task, so it is reaped the moment it exits. A dropped tokio Child
/// was left to the runtime's best-effort orphan queue instead. Needs the
/// runtime `main` enters, as the spawn itself does.
pub(crate) fn spawn_awaited(mut cmd: tokio::process::Command) -> std::io::Result<()> {
    let mut child = cmd.spawn()?;
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}
