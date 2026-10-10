use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::pages::{Page, audio, bluetooth, browser, default_apps, network, processes, services, system_info, storage, packages, accounts, notifications, timers, power};

/// The page on screen, published by the UI (`watch::Sender::send_replace`)
/// to every page worker.
pub type PageWatch = tokio::sync::watch::Receiver<u8>;

/// Wakes the UI loop after a worker sends: the runner cannot see the std
/// channels below, and until 2026-10-05 the app made it poll them every
/// 250 ms (`idle_poll_interval`) whatever page was open.
pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub struct Watchers {
    pub rx_audio: Receiver<audio::AudioState>,
    pub rx_network: Receiver<network::NetworkState>,
    pub rx_bluetooth: Receiver<bluetooth::BluetoothState>,
    pub rx_power: Receiver<power::PowerFacts>,
    pub rx_processes: Receiver<processes::ProcessesState>,
    pub rx_system: Receiver<system_info::SystemInfo>,
    pub rx_storage: Receiver<storage::StorageInfo>,
    pub rx_notifications: Receiver<notifications::NotificationsConfig>,
    pub rx_browser: Receiver<browser::BrowserConfig>,
    pub rx_services: Receiver<Vec<services::ServiceInfo>>,
    pub rx_default_apps: Receiver<default_apps::DefaultAppsInfo>,
    pub rx_timers: Receiver<Vec<timers::TimerInfo>>,
    pub rx_accounts: Receiver<accounts::AccountsSnapshot>,
    pub rx_packages: Receiver<packages::PackagesState>,
}

/// One page's worker: while `target_page` is on screen, fetch every
/// `period_secs` (and at once on arriving, if the last fetch is that old),
/// send the result, wake the UI. While another page is up it sleeps until the
/// page changes — until 2026-10-05 each of the fourteen workers woke every
/// 250 ms to look, whatever was open. `None` from `f` sends nothing.
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
    Fut: std::future::Future<Output = Option<T>> + Send + 'static,
{
    let target = target_page.index() as u8;
    let period = std::time::Duration::from_secs(period_secs);
    let (tx, rx) = channel::<T>();
    tokio::spawn(async move {
        let mut last_fetch: Option<std::time::Instant> = None;
        loop {
            if *page.borrow_and_update() != target {
                if page.changed().await.is_err() {
                    break;
                }
                continue;
            }
            if last_fetch.is_none_or(|t| t.elapsed() >= period) {
                if let Some(val) = f().await {
                    if tx.send(val).is_err() {
                        break;
                    }
                    wake();
                }
                last_fetch = Some(std::time::Instant::now());
            }
            let due = period.saturating_sub(last_fetch.map_or(period, |t| t.elapsed()));
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
        tokio::task::spawn_blocking(f).await.ok()
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
    let (p, k) = w(); let rx_processes = spawn_bg_active(p, Page::Processes, 3, k, processes::fetch_processes_state);
    let (p, k) = w(); let rx_storage = spawn_bg_active(p, Page::Storage, 10, k, storage::fetch_storage_state);

    let (p, k) = w(); let rx_notifications = spawn_bg_blocking(p, Page::Notifications, 30, k, notifications::read_notifications_config);
    // Config-file poll while the Browser page is open: catches edits made
    // outside this app (the browser itself, cce-data-editor).
    let (p, k) = w(); let rx_browser = spawn_bg_blocking(p, Page::Browser, 5, k, browser::read_browser_config);

    let (p, k) = w(); let rx_services = spawn_bg_active(p, Page::Services, 3, k, services::fetch_services);
    let (p, k) = w(); let rx_default_apps = spawn_bg_active(p, Page::DefaultApps, 10, k, default_apps::fetch_default_apps);
    let (p, k) = w(); let rx_timers = spawn_bg_active(p, Page::Timers, 5, k, timers::fetch_timers);
    let (p, k) = w(); let rx_accounts = spawn_bg_active(p, Page::Accounts, 3, k, accounts::fetch_accounts);

    let (tx_backup, rx_backup) = channel();
    let (p, k) = w(); let rx_packages = spawn_bg_active(p, Page::Packages, 30, k, packages::fetch_packages_state);
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
            rx_services,
            rx_default_apps,
            rx_timers,
            rx_accounts,
            rx_packages,
        },
        tx_backup,
        rx_backup,
        tx_update,
        rx_update,
    )
}
