use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::pages::{Page, audio, network, fonts, processes, system_info, storage, packages, accounts};

pub struct Watchers {
    pub rx_audio: Receiver<audio::AudioState>,
    pub rx_network: Receiver<network::NetworkState>,
    pub rx_processes: Receiver<processes::ProcessesState>,
    pub rx_system: Receiver<system_info::SystemState>,
    pub rx_storage: Receiver<storage::StorageState>,
    pub rx_notifications: Receiver<system_info::NotificationsConfig>,
    pub rx_services: Receiver<Vec<processes::ServiceInfo>>,
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
    let rx_network = spawn_bg_active(current_page_shared.clone(), Page::Radios.index() as u8, 5, || network::fetch_network_state());

    let rx_system = spawn_bg_active(current_page_shared.clone(), Page::System.index() as u8, 5, || system_info::fetch_system_state());
    let rx_processes = spawn_bg_active(current_page_shared.clone(), Page::Processes.index() as u8, 3, || processes::fetch_processes_state());
    let rx_storage = spawn_bg_active(current_page_shared.clone(), Page::Storage.index() as u8, 10, || storage::fetch_storage_state());

    let rx_notifications = {
        let (tx, rx) = channel::<system_info::NotificationsConfig>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::System.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| system_info::read_notifications_config()).await;
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
    let rx_services = spawn_bg_active(current_page_shared.clone(), Page::Processes.index() as u8, 3, || processes::fetch_services());
    let rx_accounts = spawn_bg_active(current_page_shared.clone(), Page::Accounts.index() as u8, 3, || accounts::fetch_accounts());

    let (tx_backup, rx_backup) = channel();
    let rx_packages = spawn_bg_active(current_page_shared.clone(), Page::Packages.index() as u8, 30, || packages::fetch_packages_state());
    let (tx_update, rx_update) = channel();

    (
        Watchers {
            rx_audio,
            rx_network,
            rx_processes,
            rx_system,
            rx_storage,
            rx_notifications,
            rx_services,
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
