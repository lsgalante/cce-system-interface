use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::pages::{Page, appearance, audio, bluetooth, browser, default_apps, display, keyboard, network, processes, services, system_info, storage, packages, accounts, notifications, timers, power};

/// The page on screen, published by the UI (`watch::Sender::send_replace`)
/// to every page worker.
pub type PageWatch = tokio::sync::watch::Receiver<u8>;

/// Wakes the UI loop after a worker sends: the runner cannot see the std
/// channels below, and until 2026-10-05 the app made it poll them every
/// 250 ms (`idle_poll_interval`) whatever page was open.
pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub struct Watchers {
    pub rx_audio: Receiver<audio::AudioState>,
    pub rx_network: Receiver<network::NetworkSnapshot>,
    pub rx_bluetooth: Receiver<bluetooth::BluetoothState>,
    pub rx_power: Receiver<power::PowerFacts>,
    pub rx_processes: Receiver<processes::ProcessesState>,
    pub rx_system: Receiver<system_info::SystemInfo>,
    pub rx_storage: Receiver<storage::StorageInfo>,
    pub rx_notifications: Receiver<notifications::NotificationsConfig>,
    pub rx_browser: Receiver<browser::BrowserConfig>,
    pub rx_appearance: Receiver<appearance::AppearanceConfig>,
    pub rx_keyboard: Receiver<keyboard::KeyboardSnapshot>,
    pub rx_display: Receiver<display::DisplaySnapshot>,
    pub rx_services: Receiver<Vec<services::ServiceInfo>>,
    pub rx_default_apps: Receiver<default_apps::DefaultAppsInfo>,
    pub rx_timers: Receiver<Vec<timers::TimerInfo>>,
    pub rx_accounts: Receiver<accounts::AccountsSnapshot>,
    pub rx_packages: Receiver<packages::PackagesState>,
    /// `checkupdates`, on a slower clock than the installed list.
    pub rx_updates_check: Receiver<packages::PackagesMessage>,
}

/// One page's worker: while `target_page` is on screen, fetch every
/// `period_secs` (and at once on arriving, if the last fetch is that old),
/// send the result, wake the UI. While another page is up it sleeps until the
/// page changes — until 2026-10-05 each of the fourteen workers woke every
/// 250 ms to look, whatever was open. `None` from `f` sends nothing.
///
/// `f` may also name when it wants its next turn, sooner than the period: a
/// fetch with a fast half and a slow half sends the fast half and asks to
/// come back for the rest (Processes: rows at once, CPU % from a baseline
/// half a second later).
fn spawn_page_worker<T, F, Fut>(
    mut page: PageWatch,
    target_page: Page,
    period_secs: u64,
    wake: Wake,
    f: F,
) -> Receiver<T>
where
    T: Send + 'static,
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Option<(T, Option<std::time::Duration>)>> + Send + 'static,
{
    let target = target_page.index() as u8;
    let period = std::time::Duration::from_secs(period_secs);
    let (tx, rx) = channel::<T>();
    tokio::spawn(async move {
        // When the next fetch is due; None = at once.
        let mut due_at: Option<std::time::Instant> = None;
        loop {
            if *page.borrow_and_update() != target {
                if page.changed().await.is_err() {
                    break;
                }
                continue;
            }
            if due_at.is_none_or(|t| std::time::Instant::now() >= t) {
                let mut next_in = period;
                if let Some((val, sooner)) = f().await {
                    if tx.send(val).is_err() {
                        break;
                    }
                    wake();
                    next_in = sooner.map_or(period, |d| d.min(period));
                }
                due_at = Some(std::time::Instant::now() + next_in);
            }
            let due = due_at.map_or(period, |t| t.saturating_duration_since(std::time::Instant::now()));
            tokio::select! {
                _ = tokio::time::sleep(due) => {}
                changed = page.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}

/// [`spawn_page_worker`] over an async fetch that always answers.
fn spawn_bg_active<T, F>(
    page: PageWatch,
    target_page: Page,
    period_secs: u64,
    wake: Wake,
    f: fn() -> F,
) -> Receiver<T>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    spawn_page_worker(page, target_page, period_secs, wake, move || {
        let fut = f();
        async move { Some((fut.await, None)) }
    })
}

/// [`spawn_bg_active`] for a fetch that may ask for its next turn early.
fn spawn_bg_paced<T, F>(
    page: PageWatch,
    target_page: Page,
    period_secs: u64,
    wake: Wake,
    f: fn() -> F,
) -> Receiver<T>
where
    T: Send + 'static,
    F: std::future::Future<Output = (T, Option<std::time::Duration>)> + Send + 'static,
{
    spawn_page_worker(page, target_page, period_secs, wake, move || {
        let fut = f();
        async move { Some(fut.await) }
    })
}

/// [`spawn_page_worker`] over a blocking read, run off the runtime's workers.
fn spawn_bg_blocking<T>(
    page: PageWatch,
    target_page: Page,
    period_secs: u64,
    wake: Wake,
    f: fn() -> T,
) -> Receiver<T>
where
    T: Send + 'static,
{
    spawn_page_worker(page, target_page, period_secs, wake, move || async move {
        tokio::task::spawn_blocking(f).await.ok().map(|v| (v, None))
    })
}

pub fn spawn_all(
    page: PageWatch,
    wake: Wake,
) -> (
    Watchers,
    Sender<storage::StorageMessage>,
    Receiver<storage::StorageMessage>,
    Sender<packages::PackagesMessage>,
    Receiver<packages::PackagesMessage>,
) {
    let w = || (page.clone(), wake.clone());
    let (p, k) = w(); let rx_audio = spawn_bg_active(p, Page::Audio, 3, k, audio::fetch_audio_state);
    let (p, k) = w(); let rx_network = spawn_bg_active(p, Page::Network, 5, k, network::fetch_network_state);
    let (p, k) = w(); let rx_bluetooth = spawn_bg_active(p, Page::Bluetooth, 5, k, bluetooth::fetch_bluetooth_page_state);
    let (p, k) = w(); let rx_power = spawn_bg_active(p, Page::Power, 5, k, power::fetch_power_state);

    let (p, k) = w(); let rx_system = spawn_bg_active(p, Page::System, 5, k, system_info::fetch_system_state);
    let (p, k) = w(); let rx_processes = spawn_bg_paced(p, Page::Processes, 3, k, processes::fetch_processes_state);
    let (p, k) = w(); let rx_storage = spawn_bg_active(p, Page::Storage, 10, k, storage::fetch_storage_state);

    let (p, k) = w(); let rx_notifications = spawn_bg_blocking(p, Page::Notifications, 30, k, notifications::read_notifications_config);
    // Config-file poll while the Browser page is open: catches edits made
    // outside this app (the browser itself, cce-data-editor).
    let (p, k) = w(); let rx_browser = spawn_bg_blocking(p, Page::Browser, 5, k, browser::read_browser_config);
    // The shared config's style block, for edits made elsewhere (cce-data-editor, by hand).
    let (p, k) = w(); let rx_appearance = spawn_bg_blocking(p, Page::Appearance, 5, k, appearance::read_appearance_config);
    // input.kdl, for edits made in cce-data-editor or by hand.
    let (p, k) = w(); let rx_keyboard = spawn_bg_blocking(p, Page::Keyboard, 5, k, keyboard::read_keyboard_snapshot);
    // Night light's status moves (a fade, the schedule), as do the backlight and outputs.
    let (p, k) = w(); let rx_display = spawn_bg_blocking(p, Page::Display, 5, k, display::read_display_snapshot);

    let (p, k) = w(); let rx_services = spawn_bg_active(p, Page::Services, 3, k, services::fetch_services);
    let (p, k) = w(); let rx_default_apps = spawn_bg_active(p, Page::DefaultApps, 10, k, default_apps::fetch_default_apps);
    let (p, k) = w(); let rx_timers = spawn_bg_active(p, Page::Timers, 5, k, timers::fetch_timers);
    let (p, k) = w(); let rx_accounts = spawn_bg_active(p, Page::Accounts, 3, k, accounts::fetch_accounts);

    let (tx_backup, rx_backup) = channel();
    let (p, k) = w(); let rx_packages = spawn_bg_active(p, Page::Packages, 30, k, packages::fetch_packages_state);
    let (p, k) = w(); let rx_updates_check = spawn_bg_active(p, Page::Packages, packages::UPDATE_CHECK_SECS, k, packages::check_updates);
    let (tx_update, rx_update) = channel();

    (
        Watchers {
            rx_audio,
            rx_network,
            rx_bluetooth,
            rx_power,
            rx_processes,
            rx_system,
            rx_storage,
            rx_notifications,
            rx_browser,
            rx_appearance,
            rx_keyboard,
            rx_display,
            rx_services,
            rx_default_apps,
            rx_timers,
            rx_accounts,
            rx_packages,
            rx_updates_check,
        },
        tx_backup,
        rx_backup,
        tx_update,
        rx_update,
    )
}
