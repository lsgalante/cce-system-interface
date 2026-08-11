use crate::app::{AppAction, PageContent, SectionContextExt, section_divider, section_kv_row};
use cce_ui::layout::{PageLayoutBuilder, LayoutStrategy};
use cce_ui::widget::{TextBox, WidgetHost};

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
    pub email_box: cce_ui::widget::Adapted<TextBox>,
    pub password_box: cce_ui::widget::Adapted<TextBox>,
    pub imap_box: cce_ui::widget::Adapted<TextBox>,
    pub smtp_box: cce_ui::widget::Adapted<TextBox>,
    pub status_msg: Option<String>,
    pub status_msg_timer: f32,
    pub oauth_listener_running: bool,
    pub editing_oauth_creds: bool,
    pub oauth_client_id_box: cce_ui::widget::Adapted<TextBox>,
    pub oauth_client_secret_box: cce_ui::widget::Adapted<TextBox>,
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
        
        let client_config = load_google_client_config();
        state.oauth_client_id_box = TextBox::new(client_config.client_id).with_multiline(false).with_draw_bg_border(true).with_label("Google Client ID");
        state.oauth_client_secret_box = {
            let mut tb = TextBox::new(client_config.client_secret).with_multiline(false).with_draw_bg_border(true).with_label("Google Client Secret");
            tb.is_password = true;
            tb
        };
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
    EditOAuthCredsStart,
    EditOAuthCredsSave,
    EditOAuthCredsCancel,
}

pub fn get_accounts_path() -> std::path::PathBuf {
    let p = cce_ui::config::cce_config_dir();
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
            email: "lsgalante@cce-ui.org".to_string(),
            imap: "imap.cce-ui.org:993".to_string(),
            smtp: "smtp.cce-ui.org:465".to_string(),
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

fn generate_pkce() -> (String, String) {
    use ring::rand::SecureRandom;
    use base64::Engine;
    let rand = ring::rand::SystemRandom::new();
    let mut bytes = [0u8; 32];
    rand.fill(&mut bytes).unwrap();
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    
    let hash = ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash.as_ref());
    
    (verifier, challenge)
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoogleClientConfig {
    pub client_id: String,
    pub client_secret: String,
}

fn write_google_client_config(p: &std::path::Path, config: &GoogleClientConfig) -> std::io::Result<()> {
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(config) {
        std::fs::write(p, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(p) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(p, perms);
            }
        }
    }
    Ok(())
}

pub fn load_google_client_config() -> GoogleClientConfig {
    let p = cce_ui::config::cce_config_dir().join("google_client.json");
    let default_config = GoogleClientConfig {
        client_id: GOOGLE_CLIENT_ID.to_string(),
        client_secret: GOOGLE_CLIENT_SECRET.to_string(),
    };
    if p.exists() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Ok(config) = serde_json::from_str::<GoogleClientConfig>(&content) {
                // If it is the old dummy client ID, or if it is the new client ID but the secret is empty, overwrite/migrate it
                if config.client_id == "REDACTED.apps.googleusercontent.com"
                    || (config.client_id == GOOGLE_CLIENT_ID && config.client_secret.is_empty())
                {
                    let _ = write_google_client_config(&p, &default_config);
                    return default_config;
                }
                return config;
            }
        }
    }
    let _ = write_google_client_config(&p, &default_config);
    default_config
}

pub async fn run_google_login(sender: calloop::channel::Sender<AppAction>) {
    let client_config = load_google_client_config();
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:36137").await {
        Ok(l) => l,
        Err(e) => {
            let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(format!("Failed to bind port 36137: {}", e))));
            return;
        }
    };
    
    let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("Waiting for browser login...".to_string())));
    
    let (verifier, challenge) = generate_pkce();
    
    // Mail scopes: these accounts feed cce-email's IMAP/SMTP (XOAUTH2 needs
    // https://mail.google.com/). The old request asked for cloud-platform/
    // cclog/aicode scopes — tokens Gmail rejects with AUTHENTICATIONFAILED.
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri=http%3A%2F%2Flocalhost%3A36137%2Fauth%2Fcallback&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fuserinfo.email&access_type=offline&prompt=consent&code_challenge={}&code_challenge_method=S256",
        client_config.client_id,
        challenge
    );
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(&auth_url);
    let _ = cce_ui::process::spawn_detached(cmd);

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
                exchange_code_for_tokens(code, verifier, sender.clone()).await;
                
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

pub async fn exchange_code_for_tokens(code: String, verifier: String, sender: calloop::channel::Sender<AppAction>) {
    let client_config = load_google_client_config();
    let client = reqwest::Client::new();
    let mut params = vec![
        ("code", code.as_str()),
        ("client_id", client_config.client_id.as_str()),
        ("redirect_uri", "http://localhost:36137/auth/callback"),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier.as_str()),
    ];
    if !client_config.client_secret.is_empty() {
        params.push(("client_secret", client_config.client_secret.as_str()));
    }

    
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
                                    refresh_token: Some(refresh_token.clone()),
                                    token_expiry: Some(expiry),
                                    client_id: Some(client_config.client_id.clone()),
                                    client_secret: Some(client_config.client_secret.clone()),
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

// The calm palette: neutral chrome, one green primary, quiet red danger, and
// the accent tint marking both the selected row and an active mode button.
const BTN_NEUTRAL: ([f32; 4], [f32; 4]) = ([0.15, 0.15, 0.20, 1.0], [0.22, 0.22, 0.28, 1.0]);
const BTN_PRIMARY: ([f32; 4], [f32; 4]) = ([0.13, 0.18, 0.14, 1.0], [0.25, 0.30, 0.26, 1.0]);
const BTN_DANGER: ([f32; 4], [f32; 4]) = ([0.25, 0.14, 0.14, 1.0], [0.40, 0.20, 0.20, 1.0]);
const ACCENT_BG: [f32; 4] = [0.20, 0.40, 0.65, 0.35];
const TEXT_BTN: [f32; 4] = [0.90, 0.90, 0.95, 1.0];
const TEXT_DANGER: [f32; 4] = [0.95, 0.55, 0.55, 1.0];

pub fn view(state: &mut AccountsState, cx: f32, cy: f32, cw: f32, ch: f32, layout: &mut dyn LayoutStrategy, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    let row_h = cce_ui::layout::spinbox_height();
    let widget_h = cce_ui::layout::spinbox_height();
    let btn_h = 26.0;

    builder.add_section_spanned(&mut final_pc, "", 1, false, |sec| {
        if !state.loaded {
            sec.text("Loading online accounts...", 12.0, 0.0, 12.0, TEXT_DIM);
            return;
        }
        let item_w = sec.cw - 2.0 * (sec.padding() + 12.0);
        let rx = sec.left;
        let mut stack = sec.vstack(8.0);

        // ── Account list: frameless rows, selection tinted, default marked ──
        if state.accounts.is_empty() {
            stack.context.text("No accounts configured.", 12.0, 0.0, 12.0, TEXT_DIM);
        } else {
            for (idx, acc) in state.accounts.iter().enumerate() {
                let label = if acc.is_default {
                    format!("{}   \u{2022} default", acc.email)
                } else {
                    acc.email.clone()
                };
                let is_selected = state.selected_idx == Some(idx) && !state.adding_new && !state.editing_oauth_creds;
                let (bg, hover) = if is_selected {
                    (ACCENT_BG, [0.22, 0.44, 0.70, 0.45])
                } else {
                    ([1.0, 1.0, 1.0, 0.04], [1.0, 1.0, 1.0, 0.10])
                };
                stack.add_row(1, 0.0, row_h, |c, _, x, w| {
                    c.button_left(
                        &label,
                        x,
                        c.ay(),
                        w,
                        row_h,
                        bg,
                        hover,
                        TEXT_BTN,
                        AppAction::Accounts(AccountsMessage::SelectAccount(idx)),
                    );
                });
            }
        }

        stack.context.spacing(6.0);

        // ── Global actions: one compact row ──
        let add_bg = if state.adding_new { (ACCENT_BG, ACCENT_BG) } else { BTN_PRIMARY };
        let oauth_bg = if state.editing_oauth_creds { (ACCENT_BG, ACCENT_BG) } else { BTN_NEUTRAL };
        let narrow = item_w < 520.0;
        stack.add_row(3, 8.0, btn_h, |c, i, x, w| {
            let (label, colors, action) = match i {
                0 => (if narrow { "Add" } else { "Add Account" }, add_bg, AccountsMessage::AddAccountStart),
                1 => (if narrow { "Google Login" } else { "Sign in with Google" }, BTN_NEUTRAL, AccountsMessage::GoogleLoginInit),
                _ => (if narrow { "Google API" } else { "Google API Settings" }, oauth_bg, AccountsMessage::EditOAuthCredsStart),
            };
            c.button(label, x, c.ay(), w, btn_h, colors.0, colors.1, TEXT_BTN, AppAction::Accounts(action));
        });

        section_divider(stack.context);

        // ── Context zone: add form / OAuth form / selected details ──
        if state.adding_new {
            stack.context.text("Add New Account", 12.0, 0.0, 14.0, [0.35, 0.65, 0.90, 1.0]);
            stack.context.text("Gmail uses Google Login; iCloud requires an App Password.", 12.0, 0.0, 11.0, TEXT_DIM);

            state.email_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.email_box, item_w, widget_h, ctx);
            state.password_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.password_box, item_w, widget_h, ctx);
            state.imap_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.imap_box, item_w, widget_h, ctx);
            state.smtp_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.smtp_box, item_w, widget_h, ctx);

            stack.context.spacing(4.0);
            stack.add_row(4, 8.0, btn_h, |c, i, x, w| {
                let (label, colors, text_col, action) = match i {
                    0 => ("Save", BTN_PRIMARY, TEXT_BTN, AccountsMessage::AddAccountSave),
                    1 => ("Cancel", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::AddAccountCancel),
                    2 => ("Login (Google)", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::GoogleLoginInit),
                    _ => ("Login (iCloud)", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::ICloudLoginHelp),
                };
                c.button(label, x, c.ay(), w, btn_h, colors.0, colors.1, text_col, AppAction::Accounts(action));
            });
        } else if state.editing_oauth_creds {
            stack.context.text("Google OAuth Credentials", 12.0, 0.0, 14.0, [0.35, 0.65, 0.90, 1.0]);
            stack.context.text("Client ID & secret from your Google Cloud Console.", 12.0, 0.0, 11.0, TEXT_DIM);
            stack.context.text("Use a 'Desktop app' OAuth client with the Gmail API enabled \u{2014}", 12.0, 0.0, 11.0, TEXT_DIM);
            stack.context.text("loopback redirects (http://localhost:*) are then allowed automatically.", 12.0, 0.0, 11.0, TEXT_DIM);

            state.oauth_client_id_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.oauth_client_id_box, item_w, widget_h, ctx);
            state.oauth_client_secret_box.set_row_rect(rx + 12.0, item_w);
            stack.add_widget(&mut state.oauth_client_secret_box, item_w, widget_h, ctx);

            stack.context.spacing(4.0);
            stack.add_row(2, 8.0, btn_h, |c, i, x, w| {
                let (label, colors, action) = match i {
                    0 => ("Save Credentials", BTN_PRIMARY, AccountsMessage::EditOAuthCredsSave),
                    _ => ("Cancel", BTN_NEUTRAL, AccountsMessage::EditOAuthCredsCancel),
                };
                c.button(label, x, c.ay(), w, btn_h, colors.0, colors.1, TEXT_BTN, AppAction::Accounts(action));
            });
        } else if let Some(selected_idx) = state.selected_idx {
            if selected_idx < state.accounts.len() {
                let acc = state.accounts[selected_idx].clone();

                section_kv_row(stack.context, "Email", &acc.email, TEXT_BTN);
                let auth_type = if acc.is_oauth { "OAuth2 (Google)" } else { "Password" };
                section_kv_row(stack.context, "Authentication", auth_type, TEXT_BTN);
                section_kv_row(stack.context, "IMAP", &acc.imap, TEXT_BTN);
                section_kv_row(stack.context, "SMTP", &acc.smtp, TEXT_BTN);

                stack.context.spacing(6.0);

                // Per-account actions, compact; Delete quiet-red, at the end.
                let mut actions: Vec<(&str, ([f32; 4], [f32; 4]), [f32; 4], AccountsMessage)> = Vec::new();
                if !acc.is_default {
                    actions.push(("Make Default", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::MakeDefault(selected_idx)));
                }
                if acc.is_oauth {
                    actions.push(("Re-login (Browser)", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::GoogleLoginInit));
                }
                actions.push(("Delete", BTN_DANGER, TEXT_DANGER, AccountsMessage::DeleteAccount(selected_idx)));
                stack.add_row(3, 8.0, btn_h, |c, i, x, w| {
                    if let Some((label, colors, text_col, action)) = actions.get(i).cloned() {
                        c.button(label, x, c.ay(), w, btn_h, colors.0, colors.1, text_col, AppAction::Accounts(action));
                    }
                });
            }
        } else {
            stack.context.text("Select an account to view details, or add one.", 12.0, 0.0, 12.0, TEXT_DIM);
        }

        if let Some(ref msg) = state.status_msg {
            stack.context.spacing(6.0);
            stack.context.text(msg, 12.0, 0.0, 12.0, [0.56, 0.83, 0.56, 1.0]);
        }
    });
    final_pc
}

/// A TextBox's live contents: the in-progress edit buffer while the box is
/// still focused, the committed text otherwise. Reading `.text` alone drops
/// whatever was typed into the last-focused field (its buffer only commits on
/// FocusOut), which made Save fail with "All fields must be filled!" unless
/// the user happened to click elsewhere first.
fn live_text(tb: &cce_ui::widget::Adapted<TextBox>) -> String {
    if tb.editing {
        tb.edit_buffer.trim().to_string()
    } else {
        tb.text.trim().to_string()
    }
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
            state.editing_oauth_creds = false;
        }
        AccountsMessage::AddAccountStart => {
            state.adding_new = true;
            state.editing_oauth_creds = false;
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
            let email = live_text(&state.email_box);
            let password = live_text(&state.password_box);
            let imap = live_text(&state.imap_box);
            let smtp = live_text(&state.smtp_box);

            if email.is_empty() || password.is_empty() || imap.is_empty() || smtp.is_empty() {
                state.status_msg = Some("All fields must be filled!".to_string());
                return;
            }

            // The password goes to the Secret Service under the SAME entry
            // cce-email resolves (service "cce-email", account = address) and
            // the on-disk field stays blank; plaintext-on-disk only as the
            // fallback when no keyring answers (cce-email migrates it later).
            let mut stored_password = password.clone();
            let mut in_keyring = false;
            if password != "mock_password" {
                if let Ok(entry) = keyring::Entry::new("cce-email", &email) {
                    if entry.set_password(&password).is_ok() {
                        stored_password = String::new();
                        in_keyring = true;
                    }
                }
            }

            let new_acc = AccountInfo {
                email: email.clone(),
                imap,
                smtp,
                is_default: state.accounts.is_empty(),
                password: stored_password,
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
            state.status_msg = Some(if in_keyring {
                "Account saved (password in keyring)".to_string()
            } else {
                "Account saved (keyring unavailable — password stored in file)".to_string()
            });
        }
        AccountsMessage::DeleteAccount(idx) => {
            if idx < state.accounts.len() {
                let deleted = state.accounts.remove(idx);
                if deleted.is_default && !state.accounts.is_empty() {
                    state.accounts[0].is_default = true;
                }
                save_accounts(&state.accounts);
                // Drop the keyring password and cce-email's cached mail too.
                if let Ok(entry) = keyring::Entry::new("cce-email", &deleted.email) {
                    let _ = entry.delete_credential();
                }
                let safe_email = deleted.email.replace('@', "_").replace('.', "_");
                let cache = cce_ui::config::cce_config_dir().join(format!("emails_{}.json", safe_email));
                let _ = std::fs::remove_file(cache);
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
            let mut cmd = std::process::Command::new("xdg-open");
            cmd.arg("https://appleid.apple.com/");
            let _ = cce_ui::process::spawn_detached(cmd);
            state.status_msg = Some("Generate iCloud App Password...".to_string());
        }
        AccountsMessage::EditOAuthCredsStart => {
            state.editing_oauth_creds = true;
            state.adding_new = false;
            let config = load_google_client_config();
            state.oauth_client_id_box.text = config.client_id.clone();
            state.oauth_client_id_box.edit_buffer = config.client_id;
            state.oauth_client_secret_box.text = config.client_secret.clone();
            state.oauth_client_secret_box.edit_buffer = config.client_secret;
        }
        AccountsMessage::EditOAuthCredsSave => {
            let client_id = live_text(&state.oauth_client_id_box);
            let client_secret = live_text(&state.oauth_client_secret_box);
            if client_id.is_empty() || client_secret.is_empty() {
                state.status_msg = Some("Both Client ID and Client Secret are required!".to_string());
                return;
            }
            let config = GoogleClientConfig {
                client_id,
                client_secret,
            };
            let p = cce_ui::config::cce_config_dir().join("google_client.json");
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(&config) {
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
            state.editing_oauth_creds = false;
            state.status_msg = Some("Google API credentials updated successfully!".to_string());
        }
        AccountsMessage::EditOAuthCredsCancel => {
            state.editing_oauth_creds = false;
        }
    }
}

impl crate::pages::AppPage for AccountsState {
    // Sections: [the one well] — the group depends on the mode.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        let modify: Vec<cce_ui::widget::WidgetId> = if self.editing_oauth_creds {
            vec![
                self.oauth_client_id_box.id(),
                self.oauth_client_secret_box.id(),
            ]
        } else if self.adding_new {
            vec![
                self.email_box.id(),
                self.password_box.id(),
                self.imap_box.id(),
                self.smtp_box.id(),
            ]
        } else {
            Vec::new()
        };
        vec![modify]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        _sec_focused: &[bool],
        layout: &mut dyn cce_ui::layout::LayoutStrategy,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>) {
        if self.adding_new && self.email_box.take_change() {
            let email_val = self.email_box.text.trim().to_lowercase();
            if email_val.ends_with("@gmail.com") {
                self.imap_box.text = "imap.gmail.com:993".to_string();
                self.imap_box.edit_buffer = "imap.gmail.com:993".to_string();
                self.smtp_box.text = "smtp.gmail.com:465".to_string();
                self.smtp_box.edit_buffer = "smtp.gmail.com:465".to_string();
            } else if email_val.ends_with("@icloud.com") {
                self.imap_box.text = "imap.mail.me.com:993".to_string();
                self.imap_box.edit_buffer = "imap.mail.me.com:993".to_string();
                self.smtp_box.text = "smtp.mail.me.com:587".to_string();
                self.smtp_box.edit_buffer = "smtp.mail.me.com:587".to_string();
            } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                self.imap_box.text = "outlook.office365.com:993".to_string();
                self.imap_box.edit_buffer = "outlook.office365.com:993".to_string();
                self.smtp_box.text = "smtp.office365.com:587".to_string();
                self.smtp_box.edit_buffer = "smtp.office365.com:587".to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cce_ui::layout::AdaptiveGrid;

    #[test]
    fn test_accounts_page_view() {
        let mut state = AccountsState::default_mock();
        state.loaded = true;
        let mut layout = AdaptiveGrid::new(260.0, 20.0);
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &mut layout, &mut cce_ui::context::UiContext::new());
        println!("PC BUTTONS COUNT: {}", pc.buttons.len());
        for (i, (btn, _, _)) in pc.buttons.iter().enumerate() {
            let base = btn.base();
            println!(
                "Button {}: label={:?}, x={}, y={}, w={}, h={}, bg={:?}, hover_bg={:?}, label_color={:?}",
                i, base.label, base.x, base.y, base.w, base.h, btn.bg, btn.hover_bg, btn.label_color
            );
        }
        assert!(!pc.buttons.is_empty(), "Accounts page should have buttons");
    }
}

