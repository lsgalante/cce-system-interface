use crate::app::{form_button, form_divider, form_pairs, AppAction, PageContent};
use cce_ui::context::UiContext;
use cce_ui::widget::Handle;
use cce_ui::layout::{lay_row, Cell, PageLayoutBuilder, PageFlow, RenderTarget};
use cce_ui::scene::layout::Rect;
use cce_ui::widget::ScrollRegion;
use cce_ui::widget::TextBox;

/// Secret Service entries are keyed by (service, address) — the same pair
/// cce-mail resolves passwords through. `KEYRING_SERVICE_LEGACY` is the
/// pre-rename name (the app was `cce-email`); it is only ever deleted here,
/// never written, since cce-mail adopts those entries on its next start.
const KEYRING_SERVICE: &str = "cce-mail";
const KEYRING_SERVICE_LEGACY: &str = "cce-email";

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

/// Where an account's password actually lives — the fact the page could not
/// show when the 2026-08-29 keyring migration stranded every entry in the
/// retired KeePassXC vault: accounts.json looked perfectly healthy while
/// cce-mail ran cache-only for two days. Probed off the main thread by
/// [`fetch_accounts`]; never derived in the render path, where a wedged
/// Secret Service would freeze the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringStatus {
    /// The Secret Service answered with a password for this address.
    InKeyring,
    /// No keyring entry, but accounts.json still holds a plaintext password
    /// (the pre-migration fallback; cce-mail adopts it on its next start).
    OnDiskPlaintext,
    /// Nowhere: the keyring has no entry and the file field is blank.
    /// Mail cannot sign in — the stranded-vault failure mode.
    Missing,
}

/// The status for one account given whether the keyring answered. `None`
/// for accounts the question does not apply to (OAuth signs in with
/// refreshed tokens; the mock account never touches the keyring).
pub fn status_from(acc: &AccountInfo, keyring_has_entry: bool) -> Option<KeyringStatus> {
    if acc.is_oauth || acc.password == "mock_password" || acc.email == "lsgalante@cce-ui.org" {
        return None;
    }
    Some(if keyring_has_entry {
        KeyringStatus::InKeyring
    } else if !acc.password.is_empty() {
        KeyringStatus::OnDiskPlaintext
    } else {
        KeyringStatus::Missing
    })
}

/// What the accounts watcher delivers: the file contents plus, for each
/// password account, where its credential actually lives.
#[derive(Debug, Clone)]
pub struct AccountsSnapshot {
    pub accounts: Vec<AccountInfo>,
    pub keyring: Vec<(String, KeyringStatus)>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountsState {
    pub loaded: bool,
    pub accounts: Vec<AccountInfo>,
    pub selected_idx: Option<usize>,
    pub adding_new: bool,
    pub email_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub password_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub imap_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub smtp_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub status_msg: Option<String>,
    pub status_msg_timer: f32,
    pub oauth_listener_running: bool,
    /// The account being edited, keyed by address rather than row index: the
    /// background refresh replaces `accounts` wholesale, and an index would
    /// quietly re-point the open form at a different account.
    pub editing_email: Option<String>,
    /// Per-account OAuth credentials — the copy in `accounts.json` that
    /// cce-mail actually refreshes with, not the global template.
    pub oauth_client_id_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    pub oauth_client_secret_box: Handle<cce_ui::widget::Adapted<TextBox>>,
    /// Per-address keyring status from the last snapshot, plus optimistic
    /// updates from Save/Delete (the 3s watcher pass corrects them).
    pub keyring: std::collections::HashMap<String, KeyringStatus>,
    /// The account rows scroll independently of the page. Rows stay plain
    /// `PageContent` buttons (network's list, not services'), so they dispatch
    /// through `page_buttons` and this page still needs no dispatch-root
    /// bookkeeping — the clip rect is what keeps a scrolled-out row from
    /// drawing, and `renderer.rs` clamps each button to its emission-time clip.
    pub list: ScrollRegion,
}

impl AccountsState {
    /// The page's state, its form fields inserted into `ctx`.
    pub fn new(ctx: &mut UiContext) -> Self {
        let mut state = Self::default();
        state.email_box = ctx.insert(TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Email Address"));
        state.password_box = {
            let mut tb = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Password / App Password");
            tb.is_password = true;
            ctx.insert(tb)
        };
        state.imap_box = ctx.insert(TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("IMAP Server"));
        state.smtp_box = ctx.insert(TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("SMTP Server"));
        state.oauth_client_id_box = ctx.insert(TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Google Client ID"));
        state.oauth_client_secret_box = {
            let mut tb = TextBox::new(String::new()).with_multiline(false).with_draw_bg_border(true).with_label("Google Client Secret");
            tb.is_password = true;
            ctx.insert(tb)
        };
        state.list = ScrollRegion::new(cce_ui::layout::spinbox_height(), LIST_GAP).with_sink_behind(true);
        state
    }
}

#[derive(Debug, Clone)]
pub enum AccountsMessage {
    Refreshed(AccountsSnapshot),
    SelectAccount(usize),
    AddAccountStart,
    AddAccountCancel,
    AddAccountSave,
    DeleteAccount(usize),
    MakeDefault(usize),
    StatusMessage(String),
    GoogleLoginInit,
    GoogleLoginSuccess(AccountInfo),
    /// The browser flow ended — successfully, in error, or by timing out. Sent
    /// from `run_google_login` on every exit path so the port-36137 listener is
    /// never believed to be alive after its task is gone.
    GoogleLoginFinished,
    ICloudLoginHelp,
    EditAccountStart(usize),
    EditAccountSave,
    EditAccountCancel,
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
        // A file that cannot be read is shown as no accounts, never as the
        // mock: the mock, saved back, is what used to replace real accounts.
        // Nothing overwrites it either (`accounts_file::update` refuses).
        return match std::fs::read_to_string(&path).map(|c| serde_json::from_str(&c)) {
            Ok(Ok(accounts)) => accounts,
            Ok(Err(e)) => {
                eprintln!("accounts: {} does not parse: {e}", path.display());
                Vec::new()
            }
            Err(e) => {
                eprintln!("accounts: cannot read {}: {e}", path.display());
                Vec::new()
            }
        };
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

/// Apply one change to accounts.json AS IT IS ON DISK, not to this page's
/// copy, and adopt what was written. cce-mail writes the file too (refreshed
/// tokens, passwords moved into the keyring), so saving the page's list
/// wholesale would undo whatever it wrote since the last refresh. See
/// `accounts_file` for the lock and the atomic replace.
fn commit(state: &mut AccountsState, change: impl FnOnce(&mut Vec<AccountInfo>)) -> bool {
    match crate::accounts_file::update(&get_accounts_path(), change) {
        Ok(written) => {
            state.accounts = written;
            true
        }
        Err(e) => {
            state.status_msg = Some(format!("Could not save accounts: {e}"));
            false
        }
    }
}

/// The last keyring probe: which accounts it covered (their Debug form,
/// hashed), when, and what it found.
static KEYRING_PROBE: std::sync::Mutex<Option<(u64, std::time::Instant, Vec<(String, KeyringStatus)>)>> =
    std::sync::Mutex::new(None);

/// How long a keyring probe stands while the account list is unchanged.
const KEYRING_PROBE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(30);

/// Forget the last keyring probe, so the next refresh asks again — after
/// anything this page does to an account or its secret.
pub fn invalidate_keyring_probe() {
    *KEYRING_PROBE.lock().unwrap() = None;
}

pub async fn fetch_accounts() -> AccountsSnapshot {
    let accounts = load_accounts();
    // The probe fetches each account's secret over D-Bus to learn whether it
    // exists, waking the keyring. This page refreshes every 3 s — it is the
    // app's first page — so the probe is reused while the account list is
    // the same, for up to KEYRING_PROBE_MAX_AGE; this page's own actions
    // invalidate it (`invalidate_keyring_probe`).
    let accounts_key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{accounts:?}").hash(&mut h);
        h.finish()
    };
    if let Some((key, at, keyring)) = KEYRING_PROBE.lock().unwrap().as_ref() {
        if *key == accounts_key && at.elapsed() < KEYRING_PROBE_MAX_AGE {
            return AccountsSnapshot { accounts, keyring: keyring.clone() };
        }
    }
    // Secret Service lookups are synchronous DBus; keep them off the async
    // workers (a wedged provider used to block for 12s at a time).
    let probe = accounts.clone();
    let keyring: Vec<(String, KeyringStatus)> = tokio::task::spawn_blocking(move || {
        probe
            .iter()
            .filter_map(|acc| {
                let has_entry = keyring::Entry::new(KEYRING_SERVICE, &acc.email)
                    .and_then(|e| e.get_password())
                    .is_ok();
                status_from(acc, has_entry).map(|s| (acc.email.clone(), s))
            })
            .collect()
    })
    .await
    .unwrap_or_default();
    *KEYRING_PROBE.lock().unwrap() = Some((accounts_key, std::time::Instant::now(), keyring.clone()));
    AccountsSnapshot { accounts, keyring }
}

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

/// The Google OAuth client this desktop uses. There is no built-in default:
/// the ID and secret used to be compiled in as constants, which put a live
/// client secret into a public repository. They now come only from
/// google_client.json, written by the Accounts page when the user pastes
/// their own client's values. An empty config means "not set up yet"; the
/// page shows the boxes to fill in.
pub fn load_google_client_config() -> GoogleClientConfig {
    let p = cce_ui::config::cce_config_dir().join("google_client.json");
    if let Ok(content) = std::fs::read_to_string(&p) {
        if let Ok(config) = serde_json::from_str::<GoogleClientConfig>(&content) {
            return config;
        }
    }
    let empty = GoogleClientConfig { client_id: String::new(), client_secret: String::new() };
    let _ = write_google_client_config(&p, &empty);
    empty
}

/// How long the loopback listener waits for the browser redirect before giving
/// up. Without a bound, abandoning the consent screen would hold port 36137 —
/// and `oauth_listener_running` with it — for the life of the process.
const OAUTH_WAIT: std::time::Duration = std::time::Duration::from_secs(300);

pub async fn run_google_login(sender: calloop::channel::Sender<AppAction>) {
    google_login_flow(&sender).await;
    // The listener is dropped by now, so the button is live again whether the
    // flow succeeded, failed to bind, or timed out.
    let _ = sender.send(AppAction::Accounts(AccountsMessage::GoogleLoginFinished));
}

async fn google_login_flow(sender: &calloop::channel::Sender<AppAction>) {
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
    let state = random_token();
    
    // Mail scopes: these accounts feed cce-mail's IMAP/SMTP (XOAUTH2 needs
    // https://mail.google.com/). The old request asked for cloud-platform/
    // cclog/aicode scopes — tokens Gmail rejects with AUTHENTICATIONFAILED.
    // calendar.readonly and calendar.events feed cce-calendar-sync, which
    // reads the tokens this flow stores in accounts.json (events is the
    // write half of the calendar mirror). No tasks scope: cce-list's lists
    // are vault notes now, and its Google Tasks sync is gone.
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri=http%3A%2F%2Flocalhost%3A36137%2Fauth%2Fcallback&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fuserinfo.email+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.events&access_type=offline&prompt=consent&code_challenge={}&code_challenge_method=S256&state={}",
        client_config.client_id,
        challenge,
        state
    );
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(&auth_url);
    let _ = crate::spawn_detached(cmd);

    let deadline = tokio::time::Instant::now() + OAUTH_WAIT;
    let Some((mut stream, outcome)) = await_oauth_callback(&listener, &state, deadline).await else {
        let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(
            "Google sign-in timed out — start the sign-in again to retry.".to_string(),
        )));
        return;
    };
    match outcome {
        Err(error) => {
            let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage(
                format!("Google sign-in was not completed ({error})."),
            )));
            respond(&mut stream, "200 OK", false, "Sign-in cancelled",
                "Nothing was saved. You can close this tab.").await;
        }
        Ok(code) => {
            let _ = sender.send(AppAction::Accounts(AccountsMessage::StatusMessage("Exchanging code for token...".to_string())));
            if exchange_code_for_tokens(code, verifier, sender.clone()).await {
                respond(&mut stream, "200 OK", true, "Clear System Settings Authentication Successful!",
                    "You can close this tab and return to the application.").await;
            } else {
                respond(&mut stream, "200 OK", false, "Clear System Settings Authentication Failed",
                    "Google accepted the sign-in but the token exchange failed; System Settings shows why.").await;
            }
        }
    }
}

/// Wait for the redirect that belongs to this flow: the connection it came
/// on (still to be answered) and its code, or Google's `error=`. `None` when
/// `deadline` passes first.
///
/// This used to take the FIRST connection and end the flow with it, so a
/// favicon fetch, a browser preconnect, or any local process touching the
/// port lost the sign-in. Everything that is not the callback carrying this
/// flow's `state` is answered and the wait goes on.
async fn await_oauth_callback(
    listener: &tokio::net::TcpListener,
    state: &str,
    deadline: tokio::time::Instant,
) -> Option<(tokio::net::TcpStream, Result<String, String>)> {
    loop {
        let mut stream = match tokio::time::timeout_at(deadline, listener.accept()).await {
            Ok(Ok((stream, _))) => stream,
            Ok(Err(_)) => continue,
            Err(_) => return None,
        };
        let Some(head) = read_request_head(&mut stream).await else { continue };
        match parse_oauth_callback(&head, state) {
            OAuthCallback::NotCallback => {
                respond(&mut stream, "404 Not Found", false, "Not found", "").await;
            }
            OAuthCallback::Unrecognised => {
                // A stale tab from an earlier attempt, or a request this flow
                // did not start. Say so, and keep waiting for the real one.
                respond(&mut stream, "400 Bad Request", false, "Not this sign-in",
                    "This page is from another sign-in attempt. Finish the one System Settings just opened.").await;
            }
            OAuthCallback::Denied(error) => return Some((stream, Err(error))),
            OAuthCallback::Code(code) => return Some((stream, Ok(code))),
        }
    }
}

/// 16 random bytes, base64url: the OAuth `state` that ties the redirect to the
/// flow that asked for it.
fn random_token() -> String {
    use base64::Engine;
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 16];
    ring::rand::SystemRandom::new().fill(&mut bytes).unwrap();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// What one request to the loopback listener turned out to be.
#[derive(Debug, PartialEq)]
enum OAuthCallback {
    /// Not a GET of /auth/callback at all (a favicon, a probe).
    NotCallback,
    /// The callback path, but without this flow's `state`, or with neither a
    /// code nor an error.
    Unrecognised,
    /// Google redirected with `error=` (the user declined, or the request was
    /// refused).
    Denied(String),
    /// The authorization code, URL-decoded.
    Code(String),
}

/// Classify a request head. Only the request line's own query is read — the
/// old `find("code=")` over the whole request matched a header (a Referer
/// carrying `code=`) as readily as the query — and its values are decoded.
fn parse_oauth_callback(head: &str, state: &str) -> OAuthCallback {
    let mut parts = head.lines().next().unwrap_or("").split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return OAuthCallback::NotCallback;
    };
    if !target.starts_with('/') {
        return OAuthCallback::NotCallback;
    }
    let Ok(url) = reqwest::Url::parse(&format!("http://localhost{target}")) else {
        return OAuthCallback::NotCallback;
    };
    if url.path() != "/auth/callback" {
        return OAuthCallback::NotCallback;
    }
    let param = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());
    if param("state").as_deref() != Some(state) {
        return OAuthCallback::Unrecognised;
    }
    if let Some(error) = param("error") {
        return OAuthCallback::Denied(error);
    }
    match param("code") {
        Some(code) if !code.is_empty() => OAuthCallback::Code(code),
        _ => OAuthCallback::Unrecognised,
    }
}

/// Read up to the end of the request head (8 KiB at most), giving up after a
/// few seconds so a connection that never speaks cannot stall the listener.
async fn read_request_head(stream: &mut tokio::net::TcpStream) -> Option<String> {
    use tokio::io::AsyncReadExt;
    let read = async {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        while buf.len() < 8192 && !buf.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut chunk).await.ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        Some(String::from_utf8_lossy(&buf).into_owned())
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), read).await.ok().flatten()
}

async fn respond(stream: &mut tokio::net::TcpStream, status: &str, ok: bool, title: &str, text: &str) {
    use tokio::io::AsyncWriteExt;
    let color = if ok { "#fff" } else { "#ff6060" };
    let body = format!(
        "<html><head><style>body {{ font-family: sans-serif; background-color: #08080c; color: {color}; text-align: center; padding-top: 50px; }}</style></head><body><h2>{title}</h2><p>{text}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

/// Trade the code for tokens and report the new account; true when the
/// account was signed in (every failure has already been reported as status).
pub async fn exchange_code_for_tokens(code: String, verifier: String, sender: calloop::channel::Sender<AppAction>) -> bool {
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
                                return true;
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
    false
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
/// The amber the keyring line warns in — the list's `warning` glyph too.
const TEXT_WARN: [f32; 4] = [0.90, 0.75, 0.40, 1.0];

/// Gap between account rows. style: deliberate — list rows pack tighter
/// than the pane gap, like every list on this app's pages (what stands
/// inside a row is `list_gap()` apart and in from the region's edges).
const LIST_GAP: f32 = 4.0;
/// Rows shown before the region starts scrolling. The list sits ABOVE the
/// actions and the edit form, so it cannot fill the page the way services'
/// does; it grows with the account count up to here and scrolls past it,
/// rather than reserving a fixed well that is mostly empty on the
/// one-or-two-account host this page usually runs on.
const LIST_MAX_ROWS: usize = 8;

pub fn view(state: &mut AccountsState, cx: f32, cy: f32, cw: f32, ch: f32, sec_focused: &[bool], layout: &mut PageFlow, ctx: &mut cce_ui::context::UiContext) -> PageContent {
    let mut final_pc = PageContent::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(1);

    let widget_h = cce_ui::layout::spinbox_height();
    let btn_h = cce_ui::layout::button_height();

    builder.add_section_spanned(&mut final_pc, "", 1, sec_focused.first().copied().unwrap_or(false), |sec| {
        let mut form = sec.form();
        if !state.loaded {
            form.column().text("Loading online accounts...", 12.0, TEXT_DIM);
            sec.place(form, ctx);
            return;
        }
        let narrow = form.width() < 520.0;
        // A login in flight is an active mode too — tint whichever button could
        // have started it, so the "already waiting on the browser" reply is not
        // the only clue.
        let login_bg = if state.oauth_listener_running { (ACCENT_BG, ACCENT_BG) } else { BTN_NEUTRAL };
        let add_bg = if state.adding_new { (ACCENT_BG, ACCENT_BG) } else { BTN_PRIMARY };
        let sel = state.selected_idx.filter(|&i| i < state.accounts.len());
        let mut col = form.column();

        // ── Account list: a scroll region, selection tinted, default marked ──
        // It grows with the account count up to LIST_MAX_ROWS and scrolls past it.
        if state.accounts.is_empty() {
            col.text("No accounts configured.", 12.0, TEXT_DIM);
        } else {
            // Row height comes from the region, not from spinbox_height():
            // ScrollRegion floors item_height at the list font's line box, and
            // drawing at a different height than it virtualizes on would drift
            // the rows out from under their own hit boxes.
            let item_h = state.list.item_height;
            let rows_shown = state.accounts.len().min(LIST_MAX_ROWS);
            let list_h = rows_shown as f32 * (item_h + LIST_GAP) + 8.0;
            let list = &mut state.list;
            let accounts = &state.accounts;
            let keyring = &state.keyring;
            let (selected_idx, adding_new) = (state.selected_idx, state.adding_new);
            col.draw(0.0, list_h, false, move |pc, r, _| {
                let (list_x, list_y, list_w, list_h) = (r.x, r.y, r.width, r.height);
                // Dissolved List (Phase 6v): scroll state + frame prims are app-owned.
                list.set_rect(list_x, list_y, list_w, list_h);
                list.update_bounds(accounts.len(), list_y, list_h);
                list.push_prims(pc);
                pc.push_clip_rect(list_x, list_y, list_w, list_h);
                for (idx, acc) in accounts.iter().enumerate() {
                    // Same predicate the region virtualizes on — a row scrolled out
                    // of the box is not emitted at all.
                    let Some(draw_y) = list.get_item_draw_y(idx, 4.0) else {
                        continue;
                    };
                    // The stranded-vault tell, visible without selecting the row.
                    let missing = keyring.get(&acc.email) == Some(&KeyringStatus::Missing);
                    let is_selected = selected_idx == Some(idx) && !adding_new;
                    let (bg, hover) = if is_selected {
                        (ACCENT_BG, [0.22, 0.44, 0.70, 0.45])
                    } else {
                        ([1.0, 1.0, 1.0, 0.04], [1.0, 1.0, 1.0, 0.10])
                    };
                    let cell = lay_row(Rect { x: list_x, y: draw_y, width: list_w, height: item_h }, &[Cell::grow(item_h)])[0];
                    pc.button_left(
                        &acc.email,
                        cell.x,
                        cell.y,
                        cell.width,
                        cell.height,
                        bg,
                        hover,
                        TEXT_BTN,
                        AppAction::Accounts(AccountsMessage::SelectAccount(idx)),
                    );
                    // The row's marks run on after the address: a `star` glyph
                    // and "default", a `warning` glyph and "no password". Each
                    // is its glyph and its word — the word alone when the icon
                    // set is missing — placed past the address as the renderer
                    // shapes it, in the button's own face.
                    let marks: &[(&str, &str, [f32; 4])] = match (acc.is_default, missing) {
                        (true, true) => &[("star", "default", TEXT_BTN), ("warning", "no password", TEXT_WARN)],
                        (true, false) => &[("star", "default", TEXT_BTN)],
                        (false, true) => &[("warning", "no password", TEXT_WARN)],
                        (false, false) => &[],
                    };
                    if !marks.is_empty() {
                        let font = cce_ui::layout::button_font();
                        let size = 12.0;
                        let ty = crate::app::label_y_in(draw_y, item_h, size, Some(&font));
                        let g = 11.0;
                        // A list gap before each mark; a glyph and its word, half that.
                        let gap = cce_ui::layout::list_gap();
                        let mut mx = cell.x + cce_ui::layout::CONTROL_TEXT_INSET
                            + crate::app::text_width(&acc.email, size, Some(&font));
                        for (icon, word, color) in marks {
                            mx += gap;
                            if pc.icon(icon, mx, draw_y + (item_h - g) / 2.0, g, g, *color) {
                                mx += g + gap / 2.0;
                            }
                            pc.text_with_font(word, mx, ty, size, TEXT_BTN, &font);
                            mx += crate::app::text_width(word, size, Some(&font));
                        }
                    }
                }
                pc.pop_clip_rect();
                // The scrollbar's fore copy, over the rows at the raise's fade.
                list.push_scrollbar_fore(pc);
            });
        }

        // ── Global actions ──
        // Add, Edit, Delete in one row of squares. Edit and Delete act on the
        // account LIST, so they sit beside Add; everything below the divider is
        // about one account's fields. Icon faces, so each is a square the height
        // of a button. Edit and Delete need a selection, so they appear only with
        // one: OMITTED rather than dimmed, since an icon's only disabled state is
        // opacity, and a faint square that still takes the click reads as a
        // control that ignored you. Without an icon set they are word buttons,
        // and `narrow` picks their width. Google sign-in lives inside the add form.
        let icons_ok = cce_ui::upload_icon("plus", 32).is_some();
        let sq = if icons_ok { btn_h } else if narrow { 86.0 } else { 110.0 };
        col.row(|r| {
            let icon_button = |r: &mut cce_ui::layout::FormGroup<'_, '_, PageContent>, icon: &'static str, word: &'static str,
                               colors: ([f32; 4], [f32; 4]), text: [f32; 4], action: AccountsMessage| {
                r.draw(sq, btn_h, false, move |pc, c, _| {
                    pc.button_icon(icon, word, c.x, c.y, c.width, c.height, colors.0, colors.1, text, 1.0, AppAction::Accounts(action));
                });
            };
            icon_button(r, "plus", "Add Account", add_bg, TEXT_BTN, AccountsMessage::AddAccountStart);
            if let Some(i) = sel {
                icon_button(r, "pencil", "Edit", BTN_NEUTRAL, TEXT_BTN, AccountsMessage::EditAccountStart(i));
                icon_button(r, "trash", "Delete", BTN_DANGER, TEXT_DANGER, AccountsMessage::DeleteAccount(i));
            }
        });

        form_divider(&mut col);

        // ── Context zone: add form / edit form / selected details ──
        let heading = [0.35, 0.65, 0.90, 1.0];
        let kv = |label: &str, value: &str, color: [f32; 4]| (label.to_string(), TEXT_DIM, value.to_string(), color);
        if state.adding_new {
            col.block(|b| {
                b.text("Add New Account", 14.0, heading);
                b.text("Gmail signs in with Google below; iCloud requires an App Password.", 11.0, TEXT_DIM);
            });
            col.widget_h(ctx, state.email_box, widget_h)
                .widget_h(ctx, state.password_box, widget_h)
                .widget_h(ctx, state.imap_box, widget_h)
                .widget_h(ctx, state.smtp_box, widget_h);
            col.row(|r| {
                form_button(r, "Save", 0.0, (BTN_PRIMARY.0, BTN_PRIMARY.1, TEXT_BTN), AppAction::Accounts(AccountsMessage::AddAccountSave));
                form_button(r, "Cancel", 0.0, (BTN_NEUTRAL.0, BTN_NEUTRAL.1, TEXT_BTN), AppAction::Accounts(AccountsMessage::AddAccountCancel));
                // Four buttons in one row is the tightest cell on the page — the full
                // labels clip below ~440px of section width, so they ride `narrow`.
                form_button(r, if narrow { "Google" } else { "Login (Google)" }, 0.0, (login_bg.0, login_bg.1, TEXT_BTN),
                    AppAction::Accounts(AccountsMessage::GoogleLoginInit));
                form_button(r, if narrow { "iCloud" } else { "Login (iCloud)" }, 0.0, (BTN_NEUTRAL.0, BTN_NEUTRAL.1, TEXT_BTN),
                    AppAction::Accounts(AccountsMessage::ICloudLoginHelp));
            });
        } else if let Some(acc) = state
            .editing_email
            .as_ref()
            .and_then(|e| state.accounts.iter().find(|a| a.email == *e))
            .cloned()
        {
            col.text("Edit Account", 14.0, heading);
            form_pairs(&mut col, 12.0, vec![kv("Email", &acc.email, TEXT_BTN)]);
            col.text("The address identifies the account \u{2014} delete and re-add to change it.", 11.0, TEXT_DIM);

            // An OAuth account has no password to edit; a password one has no
            // client credentials. Neither ever shows the other's fields.
            if acc.is_oauth {
                col.widget_h(ctx, state.imap_box, widget_h).widget_h(ctx, state.smtp_box, widget_h);
                col.block(|b| {
                    b.text("Credentials this account refreshes tokens with, taking effect", 11.0, TEXT_DIM);
                    b.text("on the next refresh \u{2014} Re-login to re-issue the tokens now.", 11.0, TEXT_DIM);
                });
                col.widget_h(ctx, state.oauth_client_id_box, widget_h).widget_h(ctx, state.oauth_client_secret_box, widget_h);
            } else {
                col.widget_h(ctx, state.password_box, widget_h)
                    .widget_h(ctx, state.imap_box, widget_h)
                    .widget_h(ctx, state.smtp_box, widget_h);
            }
            col.row(|r| {
                form_button(r, "Save", 0.0, (BTN_PRIMARY.0, BTN_PRIMARY.1, TEXT_BTN), AppAction::Accounts(AccountsMessage::EditAccountSave));
                form_button(r, "Cancel", 0.0, (BTN_NEUTRAL.0, BTN_NEUTRAL.1, TEXT_BTN), AppAction::Accounts(AccountsMessage::EditAccountCancel));
            });
        } else if let Some(selected_idx) = sel {
            let acc = state.accounts[selected_idx].clone();
            let auth_type = if acc.is_oauth { "OAuth2 (Google)" } else { "Password" };
            let mut pairs = vec![kv("Email", &acc.email, TEXT_BTN), kv("Authentication", auth_type, TEXT_BTN)];
            // Where the password actually lives — the row that would have
            // shown the 08-29 vault stranding at a glance. Only password
            // accounts carry it; the probe skips OAuth and mock.
            if let Some(status) = state.keyring.get(&acc.email) {
                let (text, color) = match status {
                    KeyringStatus::InKeyring => ("in keyring", TEXT_BTN),
                    KeyringStatus::OnDiskPlaintext => ("on disk (plaintext) \u{2014} migrates to keyring", TEXT_WARN),
                    KeyringStatus::Missing => ("MISSING \u{2014} mail cannot sign in; Edit to set it", TEXT_DANGER),
                };
                pairs.push(kv("Password", text, color));
            }
            pairs.push(kv("IMAP", &acc.imap, TEXT_BTN));
            pairs.push(kv("SMTP", &acc.smtp, TEXT_BTN));
            form_pairs(&mut col, 12.0, pairs);

            // What is left of the per-account actions once Edit and Delete moved
            // up beside Add. The row holds only what applies, and for a default
            // password account that is nothing, so it is skipped.
            let mut actions: Vec<(&str, ([f32; 4], [f32; 4]), AccountsMessage)> = Vec::new();
            if !acc.is_default {
                actions.push(("Make Default", BTN_NEUTRAL, AccountsMessage::MakeDefault(selected_idx)));
            }
            if acc.is_oauth {
                let relogin = if narrow { "Re-login" } else { "Re-login (Browser)" };
                actions.push((relogin, login_bg, AccountsMessage::GoogleLoginInit));
            }
            if !actions.is_empty() {
                col.row(|r| {
                    for (label, colors, action) in actions {
                        form_button(r, label, 0.0, (colors.0, colors.1, TEXT_BTN), AppAction::Accounts(action));
                    }
                });
            }
        } else {
            col.text("Select an account to view details, or add one.", 12.0, TEXT_DIM);
        }

        if let Some(ref msg) = state.status_msg {
            col.text(msg.clone(), 12.0, [0.56, 0.83, 0.56, 1.0]);
        }
        sec.place(form, ctx);
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

/// Seed a box with a value. Both halves, for the same reason `live_text` reads
/// both: `text` is what paints, `edit_buffer` is what a focused box reads back.
fn fill_box(tb: &mut cce_ui::widget::Adapted<TextBox>, value: &str) {
    tb.text = value.to_string();
    tb.edit_buffer = value.to_string();
}

pub fn update(state: &mut AccountsState, msg: AccountsMessage, ctx: &mut UiContext) {
    match msg {
        AccountsMessage::Refreshed(snap) => {
            state.loaded = true;
            state.accounts = snap.accounts;
            state.keyring = snap.keyring.into_iter().collect();
            // An account deleted out from under an open edit form leaves it
            // editing nothing; close it rather than render a blank zone.
            if let Some(ref e) = state.editing_email {
                if !state.accounts.iter().any(|a| a.email == *e) {
                    state.editing_email = None;
                    ctx[state.password_box].placeholder = None;
                }
            }
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
            state.editing_email = None;
        }
        AccountsMessage::AddAccountStart => {
            state.adding_new = true;
            state.editing_email = None;
            fill_box(&mut ctx[state.email_box], "");
            fill_box(&mut ctx[state.password_box], "");
            fill_box(&mut ctx[state.imap_box], "");
            fill_box(&mut ctx[state.smtp_box], "");
            // Adding needs a real password; only editing may leave it blank.
            ctx[state.password_box].placeholder = None;
        }
        AccountsMessage::AddAccountCancel => {
            state.adding_new = false;
            state.selected_idx = if state.accounts.is_empty() { None } else { Some(0) };
        }
        AccountsMessage::AddAccountSave => {
            let email = live_text(&ctx[state.email_box]);
            let password = live_text(&ctx[state.password_box]);
            let imap = live_text(&ctx[state.imap_box]);
            let smtp = live_text(&ctx[state.smtp_box]);

            if email.is_empty() || password.is_empty() || imap.is_empty() || smtp.is_empty() {
                state.status_msg = Some("All fields must be filled!".to_string());
                return;
            }

            // The password goes to the Secret Service under the SAME entry
            // cce-mail resolves (service "cce-mail", account = address) and
            // the on-disk field stays blank; plaintext-on-disk only as the
            // fallback when no keyring answers (cce-mail migrates it later).
            let mut stored_password = password.clone();
            let mut in_keyring = false;
            if password != "mock_password" {
                if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &email) {
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
                is_default: false,
                password: stored_password,
                is_oauth: false,
                access_token: None,
                refresh_token: None,
                token_expiry: None,
                client_id: None,
                client_secret: None,
            };

            let saved = commit(state, |accounts| {
                let mut new_acc = new_acc;
                if let Some(pos) = accounts.iter().position(|a| a.email == new_acc.email) {
                    new_acc.is_default = accounts[pos].is_default;
                    accounts[pos] = new_acc;
                } else {
                    new_acc.is_default = accounts.is_empty();
                    accounts.push(new_acc);
                }
            });
            if !saved {
                return;
            }
            state.adding_new = false;
            state.selected_idx = state.accounts.iter().position(|a| a.email == email);
            // Optimistic: the watcher's next probe confirms it.
            state.keyring.insert(
                email,
                if in_keyring { KeyringStatus::InKeyring } else { KeyringStatus::OnDiskPlaintext },
            );
            state.status_msg = Some(if in_keyring {
                "Account saved (password in keyring)".to_string()
            } else {
                "Account saved (keyring unavailable — password stored in file)".to_string()
            });
        }
        AccountsMessage::DeleteAccount(idx) => {
            if idx < state.accounts.len() {
                let deleted = state.accounts[idx].clone();
                let saved = commit(state, |accounts| {
                    let was_default = accounts.iter().any(|a| a.email == deleted.email && a.is_default);
                    accounts.retain(|a| a.email != deleted.email);
                    if was_default && !accounts.iter().any(|a| a.is_default) {
                        if let Some(first) = accounts.first_mut() {
                            first.is_default = true;
                        }
                    }
                });
                if !saved {
                    return;
                }
                state.keyring.remove(&deleted.email);
                // Drop the keyring password and cce-mail's cached mail too.
                // The pre-rename service is cleared as well, so an account
                // deleted before cce-mail ever adopted it leaves nothing behind.
                for service in [KEYRING_SERVICE, KEYRING_SERVICE_LEGACY] {
                    if let Ok(entry) = keyring::Entry::new(service, &deleted.email) {
                        let _ = entry.delete_credential();
                    }
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
                let email = state.accounts[idx].email.clone();
                if !commit(state, |accounts| {
                    for acc in accounts.iter_mut() {
                        acc.is_default = acc.email == email;
                    }
                }) {
                    return;
                }
                state.status_msg = Some("Default account updated!".to_string());
            }
        }
        AccountsMessage::StatusMessage(msg) => {
            state.status_msg = Some(msg);
        }
        AccountsMessage::GoogleLoginInit => {
            state.oauth_listener_running = true;
            state.status_msg = Some("Starting Google Sign-In...".to_string());
        }
        AccountsMessage::GoogleLoginFinished => {
            state.oauth_listener_running = false;
        }
        AccountsMessage::GoogleLoginSuccess(mut new_acc) => {
            let email = new_acc.email.clone();
            if !commit(state, |accounts| {
                if let Some(pos) = accounts.iter().position(|a| a.email == new_acc.email) {
                    // A re-login refreshes credentials; it must not silently
                    // un-default the account it replaces.
                    new_acc.is_default = accounts[pos].is_default;
                    accounts[pos] = new_acc;
                } else {
                    accounts.push(new_acc);
                }
            }) {
                return;
            }
            state.adding_new = false;
            state.selected_idx = state.accounts.iter().position(|a| a.email == email);
            state.status_msg = Some("Google account authenticated!".to_string());
        }
        AccountsMessage::EditAccountStart(idx) => {
            let Some(acc) = state.accounts.get(idx).cloned() else { return };
            state.editing_email = Some(acc.email.clone());
            state.adding_new = false;
            state.selected_idx = Some(idx);

            fill_box(&mut ctx[state.imap_box], &acc.imap);
            fill_box(&mut ctx[state.smtp_box], &acc.smtp);
            if acc.is_oauth {
                // Show what this account actually authenticates with: its own
                // pinned copy, or the global template it would fall back to.
                // Only read the template when something is missing — loading it
                // writes the file when absent, which a full account never needs.
                let (id, secret) = match (acc.client_id.clone(), acc.client_secret.clone()) {
                    (Some(id), Some(secret)) => (id, secret),
                    (id, secret) => {
                        let fallback = load_google_client_config();
                        (id.unwrap_or(fallback.client_id), secret.unwrap_or(fallback.client_secret))
                    }
                };
                fill_box(&mut ctx[state.oauth_client_id_box], &id);
                fill_box(&mut ctx[state.oauth_client_secret_box], &secret);
            } else {
                // The password lives in the keyring. Never read a secret back
                // just to prefill a field — blank means "keep what is stored".
                fill_box(&mut ctx[state.password_box], "");
                ctx[state.password_box].set_placeholder("unchanged \u{2014} type to replace");
            }
        }
        AccountsMessage::EditAccountSave => {
            let Some(email) = state.editing_email.clone() else { return };
            let Some(idx) = state.accounts.iter().position(|a| a.email == email) else {
                state.editing_email = None;
                state.status_msg = Some("That account no longer exists.".to_string());
                return;
            };

            // Validate everything BEFORE touching state.accounts: a mid-way
            // bail would otherwise leave memory disagreeing with the file.
            let imap = live_text(&ctx[state.imap_box]);
            let smtp = live_text(&ctx[state.smtp_box]);
            if imap.is_empty() || smtp.is_empty() {
                state.status_msg = Some("IMAP and SMTP must be filled!".to_string());
                return;
            }
            let is_oauth = state.accounts[idx].is_oauth;
            let creds = if is_oauth {
                let id = live_text(&ctx[state.oauth_client_id_box]);
                let secret = live_text(&ctx[state.oauth_client_secret_box]);
                if id.is_empty() || secret.is_empty() {
                    state.status_msg = Some("Both Client ID and Client Secret are required!".to_string());
                    return;
                }
                Some((id, secret))
            } else {
                None
            };
            let password = if is_oauth { String::new() } else { live_text(&ctx[state.password_box]) };

            let mut msg = "Account updated".to_string();
            let mut new_password = None;
            if !password.is_empty() {
                // Same Secret Service entry cce-mail resolves; the on-disk
                // field stays blank whenever the keyring accepted it.
                let mut stored = password.clone();
                if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &email) {
                    if entry.set_password(&password).is_ok() {
                        stored = String::new();
                        msg = "Account updated (password in keyring)".to_string();
                        state.keyring.insert(email.clone(), KeyringStatus::InKeyring);
                    } else {
                        state.keyring.insert(email.clone(), KeyringStatus::OnDiskPlaintext);
                    }
                }
                new_password = Some(stored);
            }

            let mut found = false;
            if !commit(state, |accounts| {
                let Some(acc) = accounts.iter_mut().find(|a| a.email == email) else { return };
                found = true;
                if let Some(stored) = new_password {
                    acc.password = stored;
                }
                acc.imap = imap;
                acc.smtp = smtp;
                if let Some((id, secret)) = creds {
                    acc.client_id = Some(id);
                    acc.client_secret = Some(secret);
                }
            }) {
                return;
            }
            if !found {
                state.editing_email = None;
                state.status_msg = Some("That account no longer exists.".to_string());
                return;
            }
            state.editing_email = None;
            ctx[state.password_box].placeholder = None;
            state.status_msg = Some(msg);
        }
        AccountsMessage::EditAccountCancel => {
            state.editing_email = None;
            ctx[state.password_box].placeholder = None;
        }
        AccountsMessage::ICloudLoginHelp => {
            let mut cmd = std::process::Command::new("xdg-open");
            cmd.arg("https://appleid.apple.com/");
            let _ = crate::spawn_detached(cmd);
            state.status_msg = Some("Generate iCloud App Password...".to_string());
        }
    }
}

impl AccountsState {
    /// The list is only laid out (and its rect refreshed) when this holds — gate
    /// the region's input on it so a stale rect can't eat events on the loading
    /// screen or the empty-state text. Network's `wifi_list_visible` precedent.
    fn list_visible(&self) -> bool {
        self.loaded && !self.accounts.is_empty()
    }
}

impl crate::pages::AppPage for AccountsState {
    // Sections: [the one well] — the group depends on the mode.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        // Must mirror the view's field order exactly — this is what ctrl-nav
        // walks, and the edit form shows a different set per auth type.
        let editing_oauth = self
            .editing_email
            .as_ref()
            .and_then(|e| self.accounts.iter().find(|a| a.email == *e))
            .map(|a| a.is_oauth);
        let modify: Vec<cce_ui::widget::WidgetId> = if self.adding_new {
            vec![
                self.email_box.id(),
                self.password_box.id(),
                self.imap_box.id(),
                self.smtp_box.id(),
            ]
        } else {
            match editing_oauth {
                Some(true) => vec![
                    self.imap_box.id(),
                    self.smtp_box.id(),
                    self.oauth_client_id_box.id(),
                    self.oauth_client_secret_box.id(),
                ],
                Some(false) => vec![
                    self.password_box.id(),
                    self.imap_box.id(),
                    self.smtp_box.id(),
                ],
                None => Vec::new(),
            }
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
        sec_focused: &[bool],
        layout: &mut cce_ui::layout::PageFlow,
        ctx: &mut cce_ui::context::UiContext,
    ) -> crate::app::PageContent {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(&mut self, _actions: &mut Vec<crate::app::AppAction>, ctx: &mut UiContext) {
        if self.adding_new && ctx[self.email_box].take_change() {
            let email_val = ctx[self.email_box].text.trim().to_lowercase();
            if email_val.ends_with("@gmail.com") {
                ctx[self.imap_box].text = "imap.gmail.com:993".to_string();
                ctx[self.imap_box].edit_buffer = "imap.gmail.com:993".to_string();
                ctx[self.smtp_box].text = "smtp.gmail.com:465".to_string();
                ctx[self.smtp_box].edit_buffer = "smtp.gmail.com:465".to_string();
            } else if email_val.ends_with("@icloud.com") {
                ctx[self.imap_box].text = "imap.mail.me.com:993".to_string();
                ctx[self.imap_box].edit_buffer = "imap.mail.me.com:993".to_string();
                ctx[self.smtp_box].text = "smtp.mail.me.com:587".to_string();
                ctx[self.smtp_box].edit_buffer = "smtp.mail.me.com:587".to_string();
            } else if email_val.ends_with("@outlook.com") || email_val.ends_with("@hotmail.com") {
                ctx[self.imap_box].text = "outlook.office365.com:993".to_string();
                ctx[self.imap_box].edit_buffer = "outlook.office365.com:993".to_string();
                ctx[self.smtp_box].text = "smtp.office365.com:587".to_string();
                ctx[self.smtp_box].edit_buffer = "smtp.office365.com:587".to_string();
            }
        }
    }

    // The dissolved list's own input, all gated on the region actually having
    // been laid out this frame. Row CLICKS are not here: the rows are
    // PageContent buttons, so their AppAction still travels the page_buttons
    // path — these hooks only carry the region's hover, drag and scrolling.
    fn handle_pointer_move(
        &mut self,
        lx: f32,
        ly: f32,
        _actions: &mut Vec<crate::app::AppAction>,
        _ctx: &mut cce_ui::context::UiContext,
    ) -> bool {
        self.list_visible() && self.list.cursor_moved(lx, ly)
    }

    fn handle_pointer_down(&mut self, lx: f32, ly: f32, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.list_visible() && self.list.press(lx, ly)
    }

    fn handle_pointer_up(&mut self, _ctx: &mut cce_ui::context::UiContext) -> bool {
        self.list.release()
    }

    fn handle_mouse_wheel(&mut self, delta: &cce_ui::widget::MouseScrollDelta, lx: f32, ly: f32) -> bool {
        self.list_visible() && self.list.wheel(delta, lx, ly)
    }

    fn handle_key_input(&mut self, event: &cce_ui::widget::KeyEvent) -> bool {
        self.list_visible() && self.list.keyboard(event)
    }

    fn tick(&mut self, dt: f32) -> bool {
        self.list.tick(dt)
    }
}

#[cfg(test)]
mod tests {
    use cce_ui::widget::WidgetHost;
    use super::*;
    use cce_ui::layout::PageFlow;

    #[test]
    fn test_accounts_page_view() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        state.loaded = true;
        let mut layout = PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ui);
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

    #[test]
    fn list_reserves_its_height_so_later_rows_clear_it() {
        let mut ui = cce_ui::context::UiContext::new();
        // The scroll region advances the section by hand. SectionContext keeps a
        // parallel per-column Grid and its `spacing` recomputes
        // `content_y = grid.max_height()`, so reserving the height by bumping
        // `content_y` alone is silently discarded and every following row draws
        // back on top of the list. Caught live: "Add Account" and the detail
        // rows were painted over the account rows.
        let mut state = AccountsState::new(&mut ui);
        state.loaded = true;
        state.accounts = (0..12).map(|i| acct(&format!("a{i}@example.org"), false)).collect();
        let mut layout = PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ui);

        let list_bottom = state.list.y + state.list.h;
        let add = pc
            .buttons
            .iter()
            .map(|(b, _, _)| b.base())
            .find(|b| b.label.as_deref() == Some("Add Account"))
            .expect("Add Account button is painted");
        assert!(
            add.y >= list_bottom,
            "Add Account (y={}) must clear the list (bottom={})",
            add.y,
            list_bottom
        );
    }

    #[test]
    fn list_emits_only_the_rows_the_region_virtualizes_on() {
        let mut ui = cce_ui::context::UiContext::new();
        // Row buttons are emitted under the same `get_item_draw_y` predicate the
        // region scrolls by, so a list longer than the cap paints the visible
        // window rather than all of its rows.
        let mut state = AccountsState::new(&mut ui);
        state.loaded = true;
        state.accounts = (0..40).map(|i| acct(&format!("a{i}@example.org"), false)).collect();
        let mut layout = PageFlow::new();
        let pc = view(&mut state, 10.0, 20.0, 800.0, 600.0, &[false], &mut layout, &mut ui);

        let rows = pc
            .buttons
            .iter()
            .filter(|(b, _, _)| b.base().label.as_deref().is_some_and(|l| l.starts_with("a")))
            .count();
        assert!(
            rows > 0 && rows <= LIST_MAX_ROWS + 2,
            "expected at most the visible window of rows, got {rows} of 40"
        );
    }

    fn acct(email: &str, is_oauth: bool) -> AccountInfo {
        AccountInfo {
            email: email.to_string(),
            imap: "imap.example.org:993".to_string(),
            smtp: "smtp.example.org:465".to_string(),
            is_default: false,
            password: String::new(),
            is_oauth,
            access_token: None,
            refresh_token: None,
            token_expiry: None,
            client_id: is_oauth.then(|| "pinned-id".to_string()),
            client_secret: is_oauth.then(|| "pinned-secret".to_string()),
        }
    }

    #[test]
    fn keyring_status_maps_every_account_kind() {
        let pw = acct("pw@example.org", false);
        // The probe's answer decides between the two clean states.
        assert_eq!(status_from(&pw, true), Some(KeyringStatus::InKeyring));
        assert_eq!(status_from(&pw, false), Some(KeyringStatus::Missing));
        // A plaintext file field is the pre-migration fallback, not missing.
        let mut on_disk = acct("file@example.org", false);
        on_disk.password = "hunter2".to_string();
        assert_eq!(status_from(&on_disk, false), Some(KeyringStatus::OnDiskPlaintext));
        // ...unless the keyring also has it, which reads as migrated.
        assert_eq!(status_from(&on_disk, true), Some(KeyringStatus::InKeyring));
        // OAuth and the mock account get no indicator at all.
        assert_eq!(status_from(&acct("oauth@example.org", true), false), None);
        let mut mock = acct("lsgalante@cce-ui.org", false);
        mock.password = "mock_password".to_string();
        assert_eq!(status_from(&mock, false), None);
    }

    /// These assertions deliberately stop short of EditAccountSave's success
    /// path: it calls commit, which writes the real accounts.json under
    /// XDG_CONFIG_HOME. Only the early-return paths are exercised here.
    #[test]
    fn edit_prefills_the_account_but_never_the_password() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        state.accounts = vec![acct("a@example.org", false)];

        update(&mut state, AccountsMessage::EditAccountStart(0), &mut ui);

        assert_eq!(state.editing_email.as_deref(), Some("a@example.org"));
        assert_eq!(ui[state.imap_box].text, "imap.example.org:993");
        assert_eq!(ui[state.smtp_box].text, "smtp.example.org:465");
        // The secret is in the keyring; a blank box plus a placeholder is how
        // "keep the stored one" is expressed.
        assert!(ui[state.password_box].text.is_empty());
        assert!(ui[state.password_box].placeholder.is_some());
    }

    #[test]
    fn editing_an_oauth_account_shows_its_pinned_credentials() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        state.accounts = vec![acct("g@gmail.com", true)];

        update(&mut state, AccountsMessage::EditAccountStart(0), &mut ui);

        assert_eq!(ui[state.oauth_client_id_box].text, "pinned-id");
        assert_eq!(ui[state.oauth_client_secret_box].text, "pinned-secret");
        assert!(ui[state.oauth_client_secret_box].is_password, "the secret stays masked");
    }

    /// The form keys on the address, so a background refresh that reorders the
    /// list cannot silently re-point it at a different account. Proven via a
    /// validation bounce: reaching the IMAP check at all means the lookup found
    /// the right row after the reorder.
    #[test]
    fn edit_follows_the_account_across_a_reorder() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        state.accounts = vec![acct("first@example.org", false), acct("second@example.org", false)];

        update(&mut state, AccountsMessage::EditAccountStart(1), &mut ui);
        assert_eq!(state.editing_email.as_deref(), Some("second@example.org"));

        update(
            &mut state,
            AccountsMessage::Refreshed(AccountsSnapshot { accounts: vec![acct("second@example.org", false), acct("first@example.org", false)], keyring: Vec::new() }),&mut ui);
        assert_eq!(state.editing_email.as_deref(), Some("second@example.org"), "the refresh keeps the form open");

        fill_box(&mut ui[state.imap_box], "");
        update(&mut state, AccountsMessage::EditAccountSave, &mut ui);
        assert_eq!(state.status_msg.as_deref(), Some("IMAP and SMTP must be filled!"));
        assert!(state.editing_email.is_some(), "a failed save keeps the form open");
    }

    #[test]
    fn a_vanished_account_closes_the_edit_form() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        state.accounts = vec![acct("gone@example.org", false)];
        update(&mut state, AccountsMessage::EditAccountStart(0), &mut ui);

        update(&mut state, AccountsMessage::Refreshed(AccountsSnapshot { accounts: vec![acct("other@example.org", false)], keyring: Vec::new() }), &mut ui);

        assert!(state.editing_email.is_none());
        assert!(ui[state.password_box].placeholder.is_none(), "the placeholder does not leak into the add form");
    }

    fn get(target: &str) -> String {
        format!("GET {target} HTTP/1.1\r\nHost: localhost:36137\r\n\r\n")
    }

    #[test]
    fn the_callback_is_recognised_by_path_state_and_query_alone() {
        let st = "s3cr3t";
        // Google's code carries a slash, sometimes percent-encoded: decode it.
        assert_eq!(
            parse_oauth_callback(&get("/auth/callback?state=s3cr3t&code=4%2F0AbC&scope=x"), st),
            OAuthCallback::Code("4/0AbC".into())
        );
        assert_eq!(parse_oauth_callback(&get("/auth/callback?code=4/0AbC&state=s3cr3t"), st), OAuthCallback::Code("4/0AbC".into()));
        assert_eq!(
            parse_oauth_callback(&get("/auth/callback?error=access_denied&state=s3cr3t"), st),
            OAuthCallback::Denied("access_denied".into())
        );
        // Strays: they must not end the flow.
        assert_eq!(parse_oauth_callback(&get("/favicon.ico"), st), OAuthCallback::NotCallback);
        assert_eq!(parse_oauth_callback("", st), OAuthCallback::NotCallback);
        assert_eq!(parse_oauth_callback("POST /auth/callback?code=x&state=s3cr3t HTTP/1.1\r\n\r\n", st), OAuthCallback::NotCallback);
        // The callback, but not this flow's: a stale tab or a forged redirect.
        assert_eq!(parse_oauth_callback(&get("/auth/callback?code=x&state=old"), st), OAuthCallback::Unrecognised);
        assert_eq!(parse_oauth_callback(&get("/auth/callback?code=x"), st), OAuthCallback::Unrecognised);
        assert_eq!(parse_oauth_callback(&get("/auth/callback?state=s3cr3t"), st), OAuthCallback::Unrecognised);
        // A `code=` anywhere but the query is not a code.
        let referer = "GET /auth/callback?state=s3cr3t HTTP/1.1\r\nReferer: https://x/?code=leak\r\n\r\n";
        assert_eq!(parse_oauth_callback(referer, st), OAuthCallback::Unrecognised);
    }

    /// The bug: the listener took the first connection, whatever it was, and
    /// ended the sign-in with it. Strays now get an answer and the wait goes on.
    #[tokio::test]
    async fn strays_are_answered_and_the_wait_goes_on() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        let waiter = tokio::spawn(async move {
            let (_stream, outcome) = await_oauth_callback(&listener, "flow", deadline).await.unwrap();
            outcome
        });
        async fn send(addr: std::net::SocketAddr, req: &str) -> String {
            let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
            s.write_all(req.as_bytes()).await.unwrap();
            let mut reply = String::new();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), s.read_to_string(&mut reply)).await;
            reply
        }
        // A connection that opens and says nothing (a preconnect)...
        drop(tokio::net::TcpStream::connect(addr).await.unwrap());
        // ...the favicon, and a stale tab from an earlier attempt.
        assert!(send(addr, &get("/favicon.ico")).await.starts_with("HTTP/1.1 404"));
        assert!(send(addr, &get("/auth/callback?code=old&state=earlier")).await.starts_with("HTTP/1.1 400"));
        assert!(!waiter.is_finished(), "a stray ended the flow");
        // The real redirect still lands. Its reply is the caller's to send.
        let mut real = tokio::net::TcpStream::connect(addr).await.unwrap();
        real.write_all(get("/auth/callback?code=4%2Freal&state=flow").as_bytes()).await.unwrap();
        assert_eq!(waiter.await.unwrap(), Ok("4/real".to_string()));
    }

    #[tokio::test]
    async fn the_wait_ends_at_the_deadline() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(50);
        assert!(await_oauth_callback(&listener, "flow", deadline).await.is_none());
    }

    /// The listener flag has to come back down on EVERY exit path, not just the
    /// happy one — a stuck `true` would disable the button for the life of the
    /// process, which is worse than the double-bind it prevents.
    #[test]
    fn oauth_listener_flag_tracks_the_flow() {
        let mut ui = cce_ui::context::UiContext::new();
        let mut state = AccountsState::new(&mut ui);
        assert!(!state.oauth_listener_running);

        update(&mut state, AccountsMessage::GoogleLoginInit, &mut ui);
        assert!(state.oauth_listener_running, "starting a login marks the port busy");

        update(&mut state, AccountsMessage::GoogleLoginFinished, &mut ui);
        assert!(!state.oauth_listener_running, "a finished flow frees the button");
    }
}
