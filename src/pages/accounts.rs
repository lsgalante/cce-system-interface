use crate::app::{AppAction, PageContent};
use clear_ui::layout::{render_widget, Section};
use clear_ui::widget::{TextBox, Widget};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AccountInfo {
    pub email: String,
    pub imap: String,
    pub smtp: String,
    pub is_default: bool,
    pub password: String,
    #[serde(default)]
    pub is_oauth: bool,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_expiry: Option<u64>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountsState {
    pub loaded: bool,
    pub accounts: Vec<AccountInfo>,
    pub selected_idx: Option<usize>,
    pub adding_new: bool,
    pub email_box: TextBox,
    pub password_box: TextBox,
    pub imap_box: TextBox,
    pub smtp_box: TextBox,
    pub status_msg: Option<String>,
    pub status_msg_timer: f32,
    pub oauth_listener_running: bool,
}

impl AccountsState {
    pub fn default_mock() -> Self {
        let mut state = Self::default();
        state.email_box = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Email Address");
        state.password_box = {
            let mut tb = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Password / App Password");
            tb.is_password = true;
            tb
        };
        state.imap_box = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("IMAP Server");
        state.smtp_box = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("SMTP Server");
        state
    }
}

#[derive(Debug, Clone)]
pub enum AccountsMessage {
    Refreshed(Vec<AccountInfo>),
    SelectAccount(usize),
    AddAccountStart,
    AddAccountCancel,
    AddAccountSave,
    DeleteAccount(usize),
    MakeDefault(usize),
    StatusMessage(String),
    GoogleLoginInit,
    GoogleLoginSuccess(AccountInfo),
    ICloudLoginHelp,
}

pub fn get_accounts_path() -> std::path::PathBuf {
    let p = std::path::PathBuf::from("/home/lsgalante/.config/ccec");
    if !p.exists() {
        let _ = std::fs::create_dir_all(&p);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&p) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o700);
                let _ = std::fs::set_permissions(&p, perms);
            }
        }
    }
    p.join("accounts.json")
}

pub fn load_accounts() -> Vec<AccountInfo> {
    let path = get_accounts_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(accounts) = serde_json::from_str(&content) {
                return accounts;
            }
        }
    }
    vec![
        AccountInfo {
            email: "lsgalante@clear-ui.org".to_string(),
            imap: "imap.clear-ui.org:993".to_string(),
            smtp: "smtp.clear-ui.org:465".to_string(),
            is_default: true,
            password: "mock_password".to_string(),
            is_oauth: false,
            access_token: None,
            refresh_token: None,
            token_expiry: None,
            client_id: None,
            client_secret: None,
        },
    ]
}

pub fn save_accounts(accounts: &[AccountInfo]) {
    let path = get_accounts_path();
    if let Ok(content) = serde_json::to_string_pretty(accounts) {
        let _ = std::fs::write(&path, content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(&path, perms);
            }
        }
    }
}

pub async fn fetch_accounts() -> Vec<AccountInfo> {
    load_accounts()
}

const GOOGLE_CLIENT_ID: &str = "REDACTED.apps.googleusercontent.com";
const GOOGLE_CLIENT_SECRET: &str = "GOCSPX-REDACTED";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoogleClientConfig {
    pub client_id: String,
    pub client_secret: String,
}

pub fn load_google_client_config() -> GoogleClientConfig {
    let p = std::path::PathBuf::from("/home/lsgalante/.config/ccec/google_client.json");
    if p.exists() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Ok(config) = serde_json::from_str::<GoogleClientConfig>(&content) {
                return config;
            }
        }
    }
    let default_config = GoogleClientConfig {
        client_id: GOOGLE_CLIENT_ID.to_string(),
        client_secret: GOOGLE_CLIENT_SECRET.to_string(),
    };
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&default_config) {
        let _ = std::fs::write(&p, content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&p) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(&p, perms);
            }
        }
    }
    default_config
}

pub async fn run_google_login(sender: calloop::channel::Sender<AppAction>) {
    let client_config = load_google_client_config();
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
        Ok(l) => l,
        Err(e) => {
            let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(format!("Failed to bind port 8080: {}", e))));
            return;
        }
    };
    
    let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("Waiting for browser login...".to_string())));
    
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri=http%3A%2F%2F127.0.0.1%3A8080&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F&access_type=offline&prompt=consent",
        client_config.client_id
    );
    let _ = std::process::Command::new("xdg-open").arg(&auth_url).spawn();

    if let Ok((mut stream, _)) = listener.accept().await {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buffer = [0; 1024];
        if let Ok(n) = stream.read(&mut buffer).await {
            let req_str = String::from_utf8_lossy(&buffer[..n]);
            if let Some(code_idx) = req_str.find("code=") {
                let rest = &req_str[code_idx + 5..];
                let end_idx = rest.find(|c: char| c == ' ' || c == '&' || c == '\r' || c == '\n').unwrap_or(rest.len());
                let code = rest[..end_idx].to_string();
                
                let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("Exchanging code for token...".to_string())));
                
                // Perform token exchange
                exchange_code_for_tokens(code, sender.clone()).await;
                
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                                <html><head><style>body { font-family: sans-serif; background-color: #08080c; color: #fff; text-align: center; padding-top: 50px; }</style></head><body><h2>Clear System Settings Authentication Successful!</h2><p>You can close this tab and return to the application.</p></body></html>";
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            } else {
                let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("OAuth Error: No code received".to_string())));
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                                <html><head><style>body { font-family: sans-serif; background-color: #08080c; color: #ff6060; text-align: center; padding-top: 50px; }</style></head><body><h2>Clear System Settings Authentication Failed</h2><p>No authorization code was found.</p></body></html>";
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            }
        }
    }
}

pub async fn exchange_code_for_tokens(code: String, sender: calloop::channel::Sender<AppAction>) {
    let client_config = load_google_client_config();
    let client = reqwest::Client::new();
    let params = [
        ("code", code.as_str()),
        ("client_id", client_config.client_id.as_str()),
        ("client_secret", client_config.client_secret.as_str()),
        ("redirect_uri", "http://127.0.0.1:8080"),
        ("grant_type", "authorization_code"),
    ];
    
    match client.post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await 
    {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    let access_token = json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let refresh_token = json.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let expires_in = json.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
                    
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let expiry = now + expires_in;
 
                    // Request user profile info to get the email address
                    if let Ok(email_resp) = client.get("https://www.googleapis.com/oauth2/v2/userinfo")
                        .bearer_auth(&access_token)
                        .send()
                        .await 
                    {
                        if let Ok(email_json) = email_resp.json::<serde_json::Value>().await {
                            if let Some(email) = email_json.get("email").and_then(|v| v.as_str()) {
                                let new_acc = AccountInfo {
                                    email: email.to_string(),
                                    imap: "imap.gmail.com:993".to_string(),
                                    smtp: "smtp.gmail.com:465".to_string(),
                                    is_default: false,
                                    password: String::new(),
                                    is_oauth: true,
                                    access_token: Some(access_token),
                                    refresh_token: Some(refresh_token),
                                    token_expiry: Some(expiry),
                                    client_id: Some(client_config.client_id),
                                    client_secret: Some(client_config.client_secret),
                                };
                                let _ = sender.send(AppAction::Accounts(AccountsMessage::GoogleLoginSuccess(new_acc)));
                                return;
                            }
                        }
                    }
                }
                let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("Failed to parse Google profile".to_string())));
            } else {
                let err_text = resp.text().await.unwrap_or_default();
                let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(format!("Token exchange failed: {}", err_text))));
            }
        }
        Err(e) => {
            let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(format!("Token request failed: {}", e))));
        }
    }
}

const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

pub fn view(state: &mut AccountsState, cx: f32, cy: f32, cw: f32, _ch: f32) -> PageContent {
    let mut pc = PageContent::new();
    let y = cy + 12.0;

    let mut sec = Section::new(&mut pc, cx, y, cw, "Online Accounts");

    if !state.loaded {
        sec.text(&mut pc, "Loading online accounts...", 12.0, 0.0, 12.0, TEXT_DIM);
        sec.spacing(18.0);
    } else {
        let usable_w = cw - 24.0;
        let gap = 24.0;
        let left_w = (usable_w - gap) * 0.40;
        let right_w = (usable_w - gap) * 0.60;
        let left_x = cx + 12.0;
        let right_x = left_x + left_w + gap;

        let mut left_y = sec.ay();
        let row_h = 28.0;
        let row_gap = 8.0;

        if state.accounts.is_empty() {
            pc.text("No accounts configured.", left_x + 8.0, left_y, 12.0, TEXT_DIM);
            left_y += 20.0;
        } else {
            for (idx, acc) in state.accounts.iter().enumerate() {
                let label = if acc.is_default {
                    format!("{} [Default]", acc.email)
                } else {
                    acc.email.clone()
                };
                let is_selected = state.selected_idx == Some(idx) && !state.adding_new;
                let bg_col = if is_selected { [0.20, 0.40, 0.65, 0.4] } else { [0.10, 0.10, 0.16, 0.3] };
                pc.button(
                    &label,
                    left_x,
                    left_y,
                    left_w,
                    row_h,
                    bg_col,
                    [0.20, 0.20, 0.25, 0.15],
                    [0.90, 0.90, 0.95, 1.0],
                    AppAction::Accounts(AccountsMessage::SelectAccount(idx)),
                );
                left_y += row_h + row_gap;
            }
        }

        left_y += 12.0;
        
        let add_bg = if state.adding_new { [0.20, 0.40, 0.65, 0.4] } else { [0.13, 0.18, 0.14, 1.0] };
        pc.button(
            "Add Account",
            left_x,
            left_y,
            left_w,
            row_h,
            add_bg,
            [0.25, 0.30, 0.26, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            AppAction::Accounts(AccountsMessage::AddAccountStart),
        );
        left_y += row_h + row_gap;

        if let Some(selected_idx) = state.selected_idx {
            if selected_idx < state.accounts.len() && !state.adding_new {
                let acc = &state.accounts[selected_idx];
                if !acc.is_default {
                    pc.button(
                        "Make Default",
                        left_x,
                        left_y,
                        left_w,
                        row_h,
                        [0.15, 0.15, 0.25, 1.0],
                        [0.25, 0.25, 0.35, 1.0],
                        [1.0, 1.0, 1.0, 1.0],
                        AppAction::Accounts(AccountsMessage::MakeDefault(selected_idx)),
                    );
                    left_y += row_h + row_gap;
                }
                pc.button(
                    "Delete Account",
                    left_x,
                    left_y,
                    left_w,
                    row_h,
                    [0.33, 0.20, 0.20, 1.0],
                    [0.45, 0.25, 0.25, 1.0],
                    [1.0, 0.33, 0.33, 1.0],
                    AppAction::Accounts(AccountsMessage::DeleteAccount(selected_idx)),
                );
                left_y += row_h + row_gap;
            }
        }

        let mut right_y = sec.ay();

        if state.adding_new {
            pc.text("Add New Account", right_x, right_y, 14.0, [0.35, 0.65, 0.90, 1.0]);
            right_y += 24.0;

            pc.text("Note: Gmail uses Google Login. iCloud requires App PW.", right_x, right_y, 11.0, TEXT_DIM);
            right_y += 18.0;

            let widget_h = 26.0;
            let field_gap = 14.0;

            // Email Address textbox
            let email_top = state.email_box.top_room();
            state.email_box.set_row_rect(right_x, right_w);
            clear_ui::layout::render_widget(&mut pc, &mut state.email_box, right_x, right_y + email_top, right_w, widget_h);
            right_y += widget_h + email_top + field_gap;

            // Password textbox
            let password_top = state.password_box.top_room();
            state.password_box.set_row_rect(right_x, right_w);
            clear_ui::layout::render_widget(&mut pc, &mut state.password_box, right_x, right_y + password_top, right_w, widget_h);
            right_y += widget_h + password_top + field_gap;

            // IMAP Server textbox
            let imap_top = state.imap_box.top_room();
            state.imap_box.set_row_rect(right_x, right_w);
            clear_ui::layout::render_widget(&mut pc, &mut state.imap_box, right_x, right_y + imap_top, right_w, widget_h);
            right_y += widget_h + imap_top + field_gap;

            // SMTP Server textbox
            let smtp_top = state.smtp_box.top_room();
            state.smtp_box.set_row_rect(right_x, right_w);
            clear_ui::layout::render_widget(&mut pc, &mut state.smtp_box, right_x, right_y + smtp_top, right_w, widget_h);
            right_y += widget_h + smtp_top + field_gap;

            let helper_w = (right_w - 8.0) / 2.0;
            pc.button(
                "Login (Google)",
                right_x,
                right_y,
                helper_w,
                row_h,
                [0.15, 0.15, 0.25, 1.0],
                [0.25, 0.25, 0.35, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Accounts(AccountsMessage::GoogleLoginInit),
            );
            pc.button(
                "Login (iCloud)",
                right_x + helper_w + 8.0,
                right_y,
                helper_w,
                row_h,
                [0.15, 0.15, 0.25, 1.0],
                [0.25, 0.25, 0.35, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Accounts(AccountsMessage::ICloudLoginHelp),
            );
            right_y += row_h + 16.0;

            pc.button(
                "Save Account",
                right_x,
                right_y,
                helper_w,
                row_h,
                [0.13, 0.18, 0.14, 1.0],
                [0.25, 0.30, 0.26, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Accounts(AccountsMessage::AddAccountSave),
            );
            pc.button(
                "Cancel",
                right_x + helper_w + 8.0,
                right_y,
                helper_w,
                row_h,
                [0.33, 0.20, 0.20, 1.0],
                [0.45, 0.25, 0.25, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                AppAction::Accounts(AccountsMessage::AddAccountCancel),
            );
            right_y += row_h + 12.0;
        } else if let Some(selected_idx) = state.selected_idx {
            if selected_idx < state.accounts.len() {
                let acc = &state.accounts[selected_idx];

                pc.text("Account Details", right_x, right_y, 14.0, [0.35, 0.65, 0.90, 1.0]);
                right_y += 28.0;

                pc.text(&format!("Email Address:   {}", acc.email), right_x + 8.0, right_y, 12.0, [0.90, 0.90, 0.95, 1.0]);
                right_y += 18.0;

                let auth_type = if acc.is_oauth { "OAuth2 (Google)" } else { "Password-based" };
                pc.text(&format!("Authentication:  {}", auth_type), right_x + 8.0, right_y, 12.0, [0.83, 0.83, 0.83, 1.0]);
                right_y += 18.0;

                pc.text(&format!("IMAP Server:     {}", acc.imap), right_x + 8.0, right_y, 12.0, [0.83, 0.83, 0.83, 1.0]);
                right_y += 18.0;

                pc.text(&format!("SMTP Server:     {}", acc.smtp), right_x + 8.0, right_y, 12.0, [0.83, 0.83, 0.83, 1.0]);
                right_y += 24.0;

                if acc.is_oauth {
                    pc.button(
                        "Click to Login (Browser)",
                        right_x + 8.0,
                        right_y,
                        200.0,
                        row_h,
                        [0.15, 0.15, 0.25, 1.0],
                        [0.25, 0.25, 0.35, 1.0],
                        [1.0, 1.0, 1.0, 1.0],
                        AppAction::Accounts(AccountsMessage::GoogleLoginInit),
                    );
                    right_y += row_h + 12.0;
                }
            }
        } else {
            pc.text("Select an account to view details, or click Add Account.", right_x, right_y, 12.0, TEXT_DIM);
            right_y += 20.0;
        }

        if let Some(ref msg) = state.status_msg {
            pc.text(msg, right_x, right_y + 12.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
            right_y += 24.0;
        }

        sec.content_y = left_y.max(right_y) + 24.0;
    }

    sec.finish(&mut pc);
    pc
}

pub fn update(state: &mut AccountsState, msg: AccountsMessage) {
    match msg {
        AccountsMessage::Refreshed(accs) => {
            state.loaded = true;
            state.accounts = accs;
            if state.selected_idx.is_none() && !state.accounts.is_empty() {
                state.selected_idx = Some(0);
            } else if let Some(idx) = state.selected_idx {
                if idx >= state.accounts.len() {
                    state.selected_idx = if state.accounts.is_empty() { None } else { Some(0) };
                }
            }
        }
        AccountsMessage::SelectAccount(idx) => {
            state.selected_idx = Some(idx);
            state.adding_new = false;
        }
        AccountsMessage::AddAccountStart => {
            state.adding_new = true;
            state.email_box.text = String::new();
            state.email_box.edit_buffer = String::new();
            state.password_box.text = String::new();
            state.password_box.edit_buffer = String::new();
            state.imap_box.text = String::new();
            state.imap_box.edit_buffer = String::new();
            state.smtp_box.text = String::new();
            state.smtp_box.edit_buffer = String::new();
        }
        AccountsMessage::AddAccountCancel => {
            state.adding_new = false;
            state.selected_idx = if state.accounts.is_empty() { None } else { Some(0) };
        }
        AccountsMessage::AddAccountSave => {
            let email = state.email_box.text.trim().to_string();
            let password = state.password_box.text.trim().to_string();
            let imap = state.imap_box.text.trim().to_string();
            let smtp = state.smtp_box.text.trim().to_string();

            if email.is_empty() || password.is_empty() || imap.is_empty() || smtp.is_empty() {
                state.status_msg = Some("All fields must be filled!".to_string());
                return;
            }

            let new_acc = AccountInfo {
                email: email.clone(),
                imap,
                smtp,
                is_default: state.accounts.is_empty(),
                password,
                is_oauth: false,
                access_token: None,
                refresh_token: None,
                token_expiry: None,
                client_id: None,
                client_secret: None,
            };

            if let Some(pos) = state.accounts.iter().position(|a| a.email == email) {
                state.accounts[pos] = new_acc;
            } else {
                state.accounts.push(new_acc);
            }

            save_accounts(&state.accounts);
            state.adding_new = false;
            state.selected_idx = state.accounts.iter().position(|a| a.email == email);
            state.status_msg = Some("Account saved successfully!".to_string());
        }
        AccountsMessage::DeleteAccount(idx) => {
            if idx < state.accounts.len() {
                let deleted = state.accounts.remove(idx);
                if deleted.is_default && !state.accounts.is_empty() {
                    state.accounts[0].is_default = true;
                }
                save_accounts(&state.accounts);
                state.selected_idx = if state.accounts.is_empty() { None } else { Some(0) };
                state.status_msg = Some("Account deleted successfully!".to_string());
            }
        }
        AccountsMessage::MakeDefault(idx) => {
            if idx < state.accounts.len() {
                for (i, acc) in state.accounts.iter_mut().enumerate() {
                    acc.is_default = i == idx;
                }
                save_accounts(&state.accounts);
                state.status_msg = Some("Default account updated!".to_string());
            }
        }
        AccountsMessage::StatusMessage(msg) => {
            state.status_msg = Some(msg);
        }
        AccountsMessage::GoogleLoginInit => {
            state.status_msg = Some("Starting Google Sign-In...".to_string());
        }
        AccountsMessage::GoogleLoginSuccess(new_acc) => {
            let email = new_acc.email.clone();
            if let Some(pos) = state.accounts.iter().position(|a| a.email == email) {
                state.accounts[pos] = new_acc;
            } else {
                state.accounts.push(new_acc);
            }
            save_accounts(&state.accounts);
            state.adding_new = false;
            state.selected_idx = state.accounts.iter().position(|a| a.email == email);
            state.status_msg = Some("Google account authenticated!".to_string());
        }
        AccountsMessage::ICloudLoginHelp => {
            let _ = std::process::Command::new("xdg-open").arg("https://appleid.apple.com/").spawn();
            state.status_msg = Some("Generate iCloud App Password...".to_string());
        }
    }
}
