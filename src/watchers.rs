use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::pages::{Page, audio, bluetooth, default_apps, network, fonts, processes, services, system_info, storage, packages, accounts, notifications, timers};

pub struct Watchers {
    pub rx_audio: Receiver<audio::AudioState>,
    pub rx_network: Receiver<network::NetworkState>,
    pub rx_bluetooth: Receiver<bluetooth::BluetoothState>,
    pub rx_processes: Receiver<processes::ProcessesState>,
    pub rx_system: Receiver<system_info::SystemInfo>,
    pub rx_storage: Receiver<storage::StorageState>,
    pub rx_notifications: Receiver<notifications::NotificationsConfig>,
    pub rx_services: Receiver<Vec<services::ServiceInfo>>,
    pub rx_default_apps: Receiver<default_apps::DefaultAppsInfo>,
    pub rx_timers: Receiver<Vec<timers::TimerInfo>>,
    pub rx_fonts: Receiver<fonts::FontsState>,
    pub rx_accounts: Receiver<Vec<accounts::AccountInfo>>,
    pub rx_packages: Receiver<packages::PackagesState>,
}

fn spawn_bg_active<T, F>(
    current_page_shared: Arc<AtomicU8>,
    target_page_idx: u8,
    period_secs: u64,
    f: fn() -> F,
) -> Receiver<T>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    let (tx, rx) = channel::<T>();
    tokio::spawn(async move {
        let mut last_fetch: Option<std::time::Instant> = None;
        loop {
            let current_page = current_page_shared.load(Ordering::SeqCst);
            if current_page == target_page_idx {
                let should_fetch = match last_fetch {
                    None => true,
                    Some(t) => t.elapsed() >= std::time::Duration::from_secs(period_secs),
                };
                if should_fetch {
                    let val = f().await;
                    if tx.send(val).is_err() { break; }
                    last_fetch = Some(std::time::Instant::now());
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    });
    rx
}

pub fn spawn_all(
    current_page_shared: Arc<AtomicU8>,
) -> (
    Watchers,
    Sender<storage::StorageMessage>,
    Receiver<storage::StorageMessage>,
    Sender<packages::PackagesMessage>,
    Receiver<packages::PackagesMessage>,
) {
    let rx_audio = spawn_bg_active(current_page_shared.clone(), Page::Audio.index() as u8, 3, || audio::fetch_audio_state());
    let rx_network = spawn_bg_active(current_page_shared.clone(), Page::Network.index() as u8, 5, || network::fetch_network_state());
    let rx_bluetooth = spawn_bg_active(current_page_shared.clone(), Page::Bluetooth.index() as u8, 5, || bluetooth::fetch_bluetooth_page_state());

    let rx_system = spawn_bg_active(current_page_shared.clone(), Page::System.index() as u8, 5, || system_info::fetch_system_state());
    let rx_processes = spawn_bg_active(current_page_shared.clone(), Page::Processes.index() as u8, 3, || processes::fetch_processes_state());
    let rx_storage = spawn_bg_active(current_page_shared.clone(), Page::Storage.index() as u8, 10, || storage::fetch_storage_state());

    let rx_notifications = {
        let (tx, rx) = channel::<notifications::NotificationsConfig>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Notifications.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| notifications::read_notifications_config()).await;
                        if let Ok(val) = val {
                            if tx.send(val).is_err() { break; }
                        }
                        last_fetch = Some(std::time::Instant::now());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });
        rx
    };

    let rx_fonts = spawn_bg_active(current_page_shared.clone(), Page::Fonts.index() as u8, 30, || fonts::fetch_typeface_state());
    let rx_services = spawn_bg_active(current_page_shared.clone(), Page::Services.index() as u8, 3, || services::fetch_services());
    let rx_default_apps = spawn_bg_active(current_page_shared.clone(), Page::DefaultApps.index() as u8, 10, || default_apps::fetch_default_apps());
    let rx_timers = spawn_bg_active(current_page_shared.clone(), Page::Timers.index() as u8, 5, || timers::fetch_timers());
    let rx_accounts = spawn_bg_active(current_page_shared.clone(), Page::Accounts.index() as u8, 3, || accounts::fetch_accounts());

    let (tx_backup, rx_backup) = channel();
    let rx_packages = spawn_bg_active(current_page_shared.clone(), Page::Packages.index() as u8, 30, || packages::fetch_packages_state());
    let (tx_update, rx_update) = channel();

    (
        Watchers {
            rx_audio,
            rx_network,
            rx_bluetooth,
            rx_processes,
            rx_system,
            rx_storage,
            rx_notifications,
            rx_services,
            rx_default_apps,
            rx_timers,
            rx_fonts,
            rx_accounts,
            rx_packages,
        },
        tx_backup,
        rx_backup,
        tx_update,
        rx_update,
    )
}
