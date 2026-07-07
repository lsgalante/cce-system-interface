use std::fs;
use std::io::Write;
use cce_ui::widget::input::{Toggle, Dropdown, Spinbox};
use cce_ui::widget::Element;
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use crate::app::{AppAction, PageContent, SectionContextExt};
use crate::pages::AppPage;

const CONFIG_PATH: &str = "/home/lsgalante/.config/cce/config.kdl";

#[derive(Debug, Clone)]
pub struct NotificationsConfig {
    pub enable: bool,
    pub bell: String,
    pub duration: i32,
}

#[derive(Debug, Clone)]
pub struct NotificationsState {
    pub loaded: bool,
    pub enable: bool,
    pub enable_toggle: Toggle,
    pub bell: String,
    pub bell_menu: Dropdown,
    pub duration: i32,
    pub duration_spinbox: Spinbox,
}

impl Default for NotificationsState {
    fn default() -> Self {
        Self {
            loaded: false,
            enable: true,
            enable_toggle: Toggle::new()
                .with_label("Enable Notifications")
                .with_config(&get_config_path(), "enable"),
            bell: "none".to_string(),
            bell_menu: Dropdown::new(
                vec![
                    "None".to_string(),
                    "Bell".to_string(),
                    "Dialog".to_string(),
                    "Message".to_string(),
                ],
                0,
            ).with_label("Notification Sound"),
            duration: 5,
            duration_spinbox: Spinbox::new(5, 1, 60, 1)
                .with_label("Notification Duration")
                .with_unit("s")
                .with_config(&get_config_path(), "duration"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotificationsMessage {
    ToggleNotificationsEnable,
    SetNotificationsBell(String),
    SetNotificationsDuration(i32),
    SendTestNotification,
    Refreshed(NotificationsConfig),
}

pub fn update(state: &mut NotificationsState, msg: NotificationsMessage) {
    match msg {
        NotificationsMessage::ToggleNotificationsEnable => {
            state.enable = !state.enable;
            write_enable_notifications(state.enable);
        }
        NotificationsMessage::SetNotificationsBell(sound) => {
            state.bell = sound.clone();
            write_config_value("bell", &sound);
            send_ipc_command("reload");
        }
        NotificationsMessage::SetNotificationsDuration(d) => {
            state.duration = d;
            write_config_value("duration", &state.duration.to_string());
            send_ipc_command("reload");
        }
        NotificationsMessage::SendTestNotification => {
            tokio::spawn(async move {
                if let Ok(connection) = zbus::Connection::session().await {
                    let _ = connection.call_method(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        Some("org.freedesktop.Notifications"),
                        "Notify",
                        &(
                            "cce-system-settings",
                            0u32,
                            "",
                            "Test Notification",
                            "System notifications are working correctly!",
                            Vec::<&str>::new(),
                            std::collections::HashMap::<&str, zbus::zvariant::Value>::new(),
                            -1i32,
                        )
                    ).await;
                }
            });
        }
        NotificationsMessage::Refreshed(new) => {
            state.loaded = true;
            state.enable = new.enable;
            state.bell = new.bell;
            state.duration = new.duration;
        }
    }
}

fn get_socket_path() -> String {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(display) => format!("/tmp/cce-{}.sock", display),
        Err(_) => "/tmp/cce.sock".to_string(),
    }
}

pub fn read_notifications_config() -> NotificationsConfig {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let val = parse_json(&content);
    let enable = val["notifications"]["enable"].as_bool().unwrap_or(true);
    let bell = val["notifications"]["bell"].as_str().unwrap_or("none").to_string();
    let duration = val["notifications"]["duration"].as_i64().map(|v| v as i32).unwrap_or(5);
    NotificationsConfig {
        enable,
        bell,
        duration,
    }
}

fn parse_json(content: &str) -> serde_json::Value {
    cce_ui::config::parse_kdl_to_json(content)
}

#[cfg(test)]
fn parse_notifications_enable(content: &str) -> bool {
    let val = parse_json(content);
    val["notifications"]["enable"].as_bool().unwrap_or(true)
}

#[cfg(test)]
#[allow(dead_code)]
fn parse_notifications_bell(content: &str) -> String {
    let val = parse_json(content);
    val["notifications"]["bell"].as_str().unwrap_or("none").to_string()
}

#[cfg(test)]
fn parse_notifications_duration(content: &str) -> i32 {
    let val = parse_json(content);
    val["notifications"]["duration"].as_i64().map(|v| v as i32).unwrap_or(5)
}

fn send_ipc_command(cmd: &str) {
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(get_socket_path()) {
        let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
    }
}

fn write_config_value(key: &str, value: &str) {
    cce_ui::config::write_config_value(&get_config_path(), key, value, "notifications");
}

fn write_enable_notifications(enabled: bool) {
    write_config_value("enable", &enabled.to_string());
    send_ipc_command("reload");
}

thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn get_config_path() -> String {
    #[cfg(test)]
    {
        TEST_CONFIG_PATH.with(|p| {
            if let Some(path) = p.borrow().as_ref() {
                return path.clone();
            }
            "/home/lsgalante/.config/cce/config.kdl".to_string()
        })
    }
    #[cfg(not(test))]
    {
        "/home/lsgalante/.config/cce/config.kdl".to_string()
    }
}

impl AppPage for NotificationsState {
    fn clear_children(&mut self, ctx: &mut cce_ui::context::UiContext) {
        self.enable_toggle.clear_children(ctx);
        self.enable_toggle.set_parent(None, ctx);
        self.bell_menu.clear_children(ctx);
        self.bell_menu.set_parent(None, ctx);
        self.duration_spinbox.clear_children(ctx);
        self.duration_spinbox.set_parent(None, ctx);
    }

    fn get_section_containers(&self) -> Vec<cce_ui::widget::SectionContainer> {
        vec![
            cce_ui::widget::SectionContainer::new("Notifications Settings")
                .with_draw_children(false)
                .with_layout(cce_ui::widget::AdaptiveGridLayout {
                    min_col_width: 140.0,
                    gap: 8.0,
                    padding_x: 0.0,
                    padding_y: 0.0,
                    grid: None,
                }),
        ]
    }

    fn link_children(
        &mut self,
        page_root: &mut dyn cce_ui::widget::Element,
        sec_containers: &mut [cce_ui::widget::SectionContainer],
        ctx: &mut cce_ui::context::UiContext,
    ) {
        for sec in sec_containers.iter_mut() {
            cce_ui::widget::link_parent_child(page_root, sec, ctx);
        }
        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.enable_toggle, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.bell_menu, ctx);
        cce_ui::widget::link_parent_child(&mut sec_containers[0], &mut self.duration_spinbox, ctx);
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut dyn LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> PageContent {
        let mut final_pc = PageContent::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

        builder.add_section(&mut final_pc, "Notifications Settings", sec_focused.first().copied().unwrap_or(false), |sec| {
            let mut stack = sec.vstack(8.0);
            let sec_w = stack.context.cw;
            self.enable_toggle.set_toggled(self.enable);
            stack.add_widget(&mut self.enable_toggle, sec_w - 28.0, cce_ui::layout::toggle_height(), ctx);

            let selected_idx = match self.bell.as_str() {
                "none" => 0,
                "bell" => 1,
                "dialog" => 2,
                "message" => 3,
                _ => 0,
            };
            self.bell_menu.selected = selected_idx;
            self.bell_menu.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut self.bell_menu, sec_w - 28.0, 44.0, ctx);

            self.duration_spinbox.value = self.duration;
            self.duration_spinbox.set_label("Notification Duration");
            self.duration_spinbox.set_row_rect(stack.context.left + 14.0, sec_w - 28.0);
            stack.add_widget(&mut self.duration_spinbox, sec_w - 28.0, 44.0, ctx);

            let btn_h = 32.0;
            let white_color = [1.0, 1.0, 1.0, 1.0];
            let btn_bg = [0.20, 0.40, 0.65, 1.0];
            let btn_hover = [0.28, 0.50, 0.78, 1.0];

            stack.add_row(1, 0.0, btn_h, |ctx, _, x, w| {
                ctx.button(
                    "Send Test Notification",
                    x,
                    ctx.ay(),
                    w,
                    btn_h,
                    btn_bg,
                    btn_hover,
                    white_color,
                    AppAction::Notifications(NotificationsMessage::SendTestNotification),
                );
            });
        });

        final_pc
    }

    fn propagate_widget_changes(&mut self, actions: &mut Vec<AppAction>) {
        if self.enable_toggle.take_change() {
            actions.push(AppAction::Notifications(NotificationsMessage::ToggleNotificationsEnable));
        }
        if self.bell_menu.take_change() {
            let sound = match self.bell_menu.selected {
                0 => "none",
                1 => "bell",
                2 => "dialog",
                3 => "message",
                _ => "none",
            }.to_string();
            actions.push(AppAction::Notifications(NotificationsMessage::SetNotificationsBell(sound)));
        }
        if self.duration_spinbox.take_change() {
            actions.push(AppAction::Notifications(NotificationsMessage::SetNotificationsDuration(self.duration_spinbox.value)));
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_view_layout_grid() {
        let mut state = NotificationsState::default();
        let mut layout = cce_ui::layout::ColumnLayout::new(20.0);
        let sec_focused = vec![false];
        let mut ctx = cce_ui::context::UiContext::new();
        let pc = state.view(10.0, 20.0, 800.0, 600.0, false, &sec_focused, &mut layout, &mut ctx);
        assert!(!pc.rects.is_empty() || !pc.texts.is_empty());
    }

    #[test]
    fn test_parse_notifications_enable_default() {
        assert!(parse_notifications_enable(""));
        assert!(parse_notifications_enable("[layout]\ngap = 18\n"));
    }

    #[test]
    fn test_parse_notifications_enable_explicit() {
        let content = "notifications {\n    enable (bool)false\n}\n";
        assert!(!parse_notifications_enable(content));

        let content = "notifications {\n    enable (bool)true\n}\n";
        assert!(parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_enable_other_sections() {
        let content = "layout {\n    enable (bool)false\n}\nnotifications {\n    enable (bool)true\n}\ninput {\n    enable (bool)false\n}\n";
        assert!(parse_notifications_enable(content));

        let content = "layout {\n    enable (bool)true\n}\nnotifications {\n    enable (bool)false\n}\ninput {\n    enable (bool)true\n}\n";
        assert!(!parse_notifications_enable(content));
    }

    #[test]
    fn test_parse_notifications_duration_default() {
        assert_eq!(parse_notifications_duration(""), 5);
        assert_eq!(parse_notifications_duration("notifications {}"), 5);
    }

    #[test]
    fn test_parse_notifications_duration_explicit() {
        let content = "notifications {\n    duration (i64)10\n}\n";
        assert_eq!(parse_notifications_duration(content), 10);
    }
}
