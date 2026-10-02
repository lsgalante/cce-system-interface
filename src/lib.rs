mod accounts_file;
pub mod app;
pub mod pages;
pub mod power_meter;
pub mod power_plan;
pub use cce_ui::widget::scroll_region;
pub mod widgets;
pub mod watchers;

/// Spawn a command and reap it from a background thread, so a finished
/// xdg-open never lingers as a zombie. cce-ui's `process::spawn_detached`
/// did exactly this until cce-ui@4e94236 removed the module; the accounts and
/// system-info pages were its remaining callers.
pub(crate) fn spawn_detached(mut cmd: std::process::Command) -> std::io::Result<()> {
    let mut child = cmd.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
