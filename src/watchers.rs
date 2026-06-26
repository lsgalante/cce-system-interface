use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::pages::{Page, audio, display, network, interface, input, processes, system_info, storage, packages, accounts};
use cce_ui::widget::Finger;

pub struct Watchers {
    pub rx_audio: Receiver<audio::AudioState>,
    pub rx_display: Receiver<display::DisplayState>,
    pub rx_network: Receiver<network::NetworkState>,
    pub rx_layout: Receiver<interface::WindowsState>,
    pub rx_input: Receiver<input::InputState>,
    pub rx_fingers: Receiver<Vec<Finger>>,
    pub rx_processes: Receiver<processes::ProcessesState>,
    pub rx_system: Receiver<system_info::SystemState>,
    pub rx_status: Receiver<interface::StatusData>,
    pub rx_storage: Receiver<storage::StorageState>,
    pub rx_notifications: Receiver<system_info::NotificationsConfig>,
    pub rx_typeface: Receiver<interface::InterfaceState>,
    pub rx_services: Receiver<Vec<processes::ServiceInfo>>,
    pub rx_interface: Receiver<interface::InterfaceState>,
    pub rx_accounts: Receiver<Vec<accounts::AccountInfo>>,
    pub rx_packages: Receiver<packages::PackagesState>,
    pub rx_layout_status: Receiver<interface::LayoutStatusInfo>,
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
    let rx_display = spawn_bg_active(current_page_shared.clone(), Page::Display.index() as u8, 10, || display::fetch_display_state());
    let rx_network = spawn_bg_active(current_page_shared.clone(), Page::Radios.index() as u8, 5, || network::fetch_network_state());

    let rx_layout = {
        let (tx, rx) = channel::<interface::WindowsState>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Interface.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| interface::read_windows_config()).await;
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


    let rx_input = {
        let (tx, rx) = channel::<input::InputState>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Input.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| input::read_input_config()).await;
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

    let rx_fingers = {
        let (tx, rx) = channel::<Vec<Finger>>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let socket_path = match std::env::var("WAYLAND_DISPLAY") {
                Ok(display) => format!("/tmp/cce-input-coords-{}.sock", display),
                Err(_) => "/tmp/cce-input-coords.sock".to_string(),
            };
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Input.index() as u8 {
                    if let Ok(stream) = tokio::net::UnixStream::connect(&socket_path).await {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stream);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if current_page_shared.load(Ordering::SeqCst) != Page::Input.index() as u8 {
                                break;
                            }
                            if let Ok(fingers) = serde_json::from_str::<Vec<Finger>>(&line) {
                                if tx.send(fingers).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });
        rx
    };

    let rx_system = spawn_bg_active(current_page_shared.clone(), Page::System.index() as u8, 5, || system_info::fetch_system_state());
    let rx_processes = spawn_bg_active(current_page_shared.clone(), Page::Processes.index() as u8, 3, || processes::fetch_processes_state());
    let rx_status = spawn_bg_active(current_page_shared.clone(), Page::Interface.index() as u8, 10, || interface::fetch_status_state());
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

    let rx_typeface = spawn_bg_active(current_page_shared.clone(), Page::Interface.index() as u8, 30, || interface::fetch_typeface_state());
    let rx_services = spawn_bg_active(current_page_shared.clone(), Page::Processes.index() as u8, 3, || processes::fetch_services());
    let rx_accounts = spawn_bg_active(current_page_shared.clone(), Page::Accounts.index() as u8, 3, || accounts::fetch_accounts());

    let rx_interface = {
        let (tx, rx) = channel::<interface::InterfaceState>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Interface.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_secs(30),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| interface::read_interface_config()).await;
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

    let rx_layout_status = {
        let (tx, rx) = channel::<interface::LayoutStatusInfo>();
        let current_page_shared = current_page_shared.clone();
        tokio::spawn(async move {
            let mut last_fetch: Option<std::time::Instant> = None;
            loop {
                let current_page = current_page_shared.load(Ordering::SeqCst);
                if current_page == Page::Interface.index() as u8 {
                    let should_fetch = match last_fetch {
                        None => true,
                        Some(t) => t.elapsed() >= std::time::Duration::from_millis(250),
                    };
                    if should_fetch {
                        let val = tokio::task::spawn_blocking(|| interface::read_current_layout_status()).await;
                        if let Ok(val) = val {
                            if tx.send(val).is_err() { break; }
                        }
                        last_fetch = Some(std::time::Instant::now());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
        rx
    };

    let (tx_backup, rx_backup) = channel();
    let rx_packages = spawn_bg_active(current_page_shared.clone(), Page::Packages.index() as u8, 30, || packages::fetch_packages_state());
    let (tx_update, rx_update) = channel();

    (
        Watchers {
            rx_audio,
            rx_display,
            rx_network,
            rx_layout,
            rx_input,
            rx_fingers,
            rx_processes,
            rx_system,
            rx_status,
            rx_storage,
            rx_notifications,
            rx_typeface,
            rx_services,
            rx_interface,
            rx_accounts,
            rx_packages,
            rx_layout_status,
        },
        tx_backup,
        rx_backup,
        tx_update,
        rx_update,
    )
}
