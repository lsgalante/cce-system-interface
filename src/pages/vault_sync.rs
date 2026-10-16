//! Vault Sync, a section of the Accounts page: cce-sync's server, where its
//! token is kept, and the apps the vault server has granted access (OAuth
//! grants: Claude's connector, the phone's web client), each revocable.
//!
//! Driven through the `cce-sync` CLI (`login`, `grants`, `revoke`), so the
//! page shares its token lookup, its client and its server check instead of
//! copying them. Until 2026-10-16 all of this was CLI-only.
//!
//! The grants are a network request, so they are read when the Accounts page
//! first loads, after an action here, and on Check, not on the page's 3 s
//! refresh. The token itself is never read into the page except to move it,
//! and never shown.

use std::sync::OnceLock;
use std::time::Duration;

use cce_ui::context::UiContext;
use cce_ui::widget::input::TextBox;
use cce_ui::widget::{Adapted, Handle};

use crate::app::{button_need, form_button, form_divider, wrap_to_width, AppAction, PageCanvas};
use crate::pages::accounts::AccountsMessage;

/// cce-sync's keyring service; the account is the server URL.
const KEYRING_SERVICE: &str = "cce-sync";
/// A grants or revoke round trip to the vault server.
const NET_TIMEOUT: Duration = Duration::from_secs(15);

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
const WARN: [f32; 4] = [0.95, 0.75, 0.55, 1.0];
const RED: [f32; 4] = [0.95, 0.55, 0.55, 1.0];
const NEUTRAL: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.16, 0.24, 1.0], [0.25, 0.30, 0.26, 1.0], TEXT_FG);
const GO: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.29, 0.18, 1.0], [0.25, 0.30, 0.26, 1.0], [1.0, 1.0, 1.0, 1.0]);
const DANGER: ([f32; 4], [f32; 4], [f32; 4]) = ([0.30, 0.14, 0.14, 1.0], [0.45, 0.20, 0.20, 1.0], [0.98, 0.65, 0.65, 1.0]);

/// Set by the host: how results come back from the spawned commands.
static SENDER: OnceLock<calloop::channel::Sender<AppAction>> = OnceLock::new();

pub fn set_sender(tx: calloop::channel::Sender<AppAction>) {
    let _ = SENDER.set(tx);
}

fn send(msg: SyncMessage) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(AppAction::Accounts(AccountsMessage::Sync(msg)));
    }
}

/// Where cce-sync finds its token, in its own order of lookup.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenPlace {
    Env,
    Keyring,
    /// `~/.config/cce/cce-sync.token`: on disk in plain text.
    File,
    Missing,
    /// The keyring did not answer in time.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grant {
    pub id: String,
    pub client: String,
    pub email: String,
    pub since: String,
    pub last_used: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncSnapshot {
    /// None: no `sync { server }` in config.kdl (nor CCE_SYNC_SERVER).
    pub server: Option<String>,
    pub token: TokenPlace,
    pub grants: Result<Vec<Grant>, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confirm {
    Revoke { id: String, client: String },
    MoveToKeyring,
}

#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub snap: Option<SyncSnapshot>,
    pub checking: bool,
    /// What is running ("Saving the token..."), while it runs.
    pub busy: Option<String>,
    pub result: Option<Result<String, String>>,
    pub confirm: Option<Confirm>,
    pub token_box: Handle<Adapted<TextBox>>,
}

impl SyncState {
    pub fn new(ctx: &mut UiContext) -> Self {
        let tb = TextBox::new(String::new())
            .with_multiline(false)
            .with_draw_bg_border(true)
            .with_password(true)
            .with_label("New token");
        Self { token_box: ctx.insert(tb), ..Self::default() }
    }

    /// The token box is laid out: a server is set and nothing is running.
    pub fn token_box_visible(&self) -> bool {
        self.snap.as_ref().is_some_and(|s| s.server.is_some()) && self.busy.is_none() && self.confirm.is_none()
    }
}

#[derive(Debug, Clone)]
pub enum SyncMessage {
    Check,
    Checked(Box<SyncSnapshot>),
    SetToken,
    MoveToKeyring,
    Revoke(String, String),
    ConfirmYes,
    ConfirmNo,
    Done(Result<String, String>),
    Dismiss,
}

// ── Reading ──

fn token_file() -> std::path::PathBuf {
    cce_ui::config::cce_config_dir().join("cce-sync.token")
}

/// `sync { server "…" }` (a child) or `sync server="…"` (a property), as
/// cce-sync reads it, after CCE_SYNC_SERVER.
pub fn server_from(env: Option<String>, config_kdl: &str) -> Option<String> {
    if let Some(s) = env.filter(|s| !s.is_empty()) {
        return Some(s);
    }
    let doc: kdl::KdlDocument = config_kdl.parse().ok()?;
    let node = doc.get("sync")?;
    let child = node.children().and_then(|c| c.get_arg("server")).and_then(|v| v.as_string());
    let prop = node.get("server").and_then(|e| e.value().as_string());
    child.or(prop).map(str::to_string)
}

/// `cce-sync grants`: `<id>  <client padded to 24> <email>  since <t>  last
/// used <t>[  (revoked)]`. The client name may be longer than its column,
/// so the email is found as the last word, not by position.
pub fn parse_grants(out: &str) -> Vec<Grant> {
    out.lines()
        .filter_map(|line| {
            let (id, rest) = line.split_once("  ")?;
            let (left, right) = rest.split_once("  since ")?;
            let left = left.trim_end();
            let (client, email) = match left.rsplit_once(' ') {
                Some((c, e)) if e.contains('@') => (c.trim(), e),
                _ => (left, ""),
            };
            let revoked = right.trim_end().ends_with("(revoked)");
            let right = right.trim_end().trim_end_matches("(revoked)").trim_end();
            let (since, last_used) = right.split_once("  last used ").unwrap_or((right, ""));
            Some(Grant {
                id: id.trim().to_string(),
                client: client.to_string(),
                email: email.to_string(),
                since: since.trim().to_string(),
                last_used: last_used.trim().to_string(),
                revoked,
            })
        })
        .collect()
}

async fn read_snapshot() -> SyncSnapshot {
    let config = std::fs::read_to_string(cce_ui::config::get_config_path()).unwrap_or_default();
    let server = server_from(std::env::var("CCE_SYNC_SERVER").ok(), &config);
    let token = match &server {
        _ if std::env::var("CCE_SYNC_TOKEN").is_ok_and(|t| !t.trim().is_empty()) => TokenPlace::Env,
        None => TokenPlace::Missing,
        Some(server) => {
            // The keyring is synchronous D-Bus, and a wedged one does not
            // answer: off the runtime, and not waited on for long.
            let server = server.clone();
            let probe = tokio::task::spawn_blocking(move || {
                keyring::Entry::new(KEYRING_SERVICE, &server).and_then(|e| e.get_password()).is_ok()
            });
            match tokio::time::timeout(crate::FETCH_TIMEOUT, probe).await {
                Ok(Ok(true)) => TokenPlace::Keyring,
                Ok(_) if token_file().exists() => TokenPlace::File,
                Ok(_) => TokenPlace::Missing,
                Err(_) => TokenPlace::Unknown,
            }
        }
    };
    let grants = if server.is_none() || token == TokenPlace::Missing {
        Ok(Vec::new())
    } else {
        cce_sync(&["grants"], None).await.map(|out| parse_grants(&out))
    };
    SyncSnapshot { server, token, grants }
}

/// Run `cce-sync <args>`, `stdin` written to it if given: its stdout, or its
/// error in its own words.
async fn cce_sync(args: &[&str], stdin: Option<String>) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new("cce-sync");
    cmd.args(args)
        .stdin(if stdin.is_some() { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| format!("Could not run cce-sync: {e}"))?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let _ = pipe.write_all(format!("{text}\n").as_bytes()).await;
    }
    let out = tokio::time::timeout(NET_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "The vault server did not answer in 15 s.".to_string())?
        .map_err(|e| format!("cce-sync failed: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim().trim_start_matches("error: ").trim();
        Err(if err.is_empty() { "cce-sync failed without saying why.".to_string() } else { err.to_string() })
    }
}

// ── Changing ──

fn run(state: &mut SyncState, busy: &str, job: impl std::future::Future<Output = Result<String, String>> + Send + 'static) {
    // No runtime (a unit test driving `update`): nothing to run it on.
    let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
    state.busy = Some(busy.to_string());
    state.result = None;
    rt.spawn(async move {
        let res = job.await;
        send(SyncMessage::Done(res));
    });
}

pub fn update(state: &mut SyncState, msg: SyncMessage, ctx: &mut UiContext) {
    match msg {
        SyncMessage::Check => {
            if state.checking {
                return;
            }
            let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
            state.checking = true;
            rt.spawn(async {
                let snap = read_snapshot().await;
                send(SyncMessage::Checked(Box::new(snap)));
            });
        }
        SyncMessage::Checked(snap) => {
            state.snap = Some(*snap);
            state.checking = false;
        }
        SyncMessage::SetToken => {
            let tb = &mut ctx[state.token_box];
            let token = if tb.editing { tb.edit_buffer.trim().to_string() } else { tb.text.trim().to_string() };
            tb.text.clear();
            tb.edit_buffer.clear();
            if token.is_empty() || state.busy.is_some() {
                return;
            }
            // `login` checks it with the server before storing it in the
            // keyring, so a mistyped token is refused, not kept.
            run(state, "Checking the token with the server...", async move {
                cce_sync(&["login"], Some(token)).await.map(|_| "The token is stored in the keyring.".to_string())
            });
        }
        SyncMessage::MoveToKeyring => state.confirm = Some(Confirm::MoveToKeyring),
        SyncMessage::Revoke(id, client) => state.confirm = Some(Confirm::Revoke { id, client }),
        SyncMessage::ConfirmNo => state.confirm = None,
        SyncMessage::ConfirmYes => match state.confirm.take() {
            Some(Confirm::Revoke { id, client }) => {
                run(state, "Revoking...", async move {
                    cce_sync(&["revoke", &id], None).await.map(|_| format!("{client} can no longer reach the vault."))
                });
            }
            Some(Confirm::MoveToKeyring) => {
                run(state, "Moving the token to the keyring...", async move {
                    let path = token_file();
                    let token = std::fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
                    cce_sync(&["login"], Some(token.trim().to_string())).await?;
                    // Stored and checked: only now is the plain copy removed.
                    std::fs::remove_file(&path).map_err(|e| format!("Stored in the keyring, but {} could not be removed: {e}", path.display()))?;
                    Ok("The token is in the keyring, and the plain file is gone.".to_string())
                });
            }
            None => {}
        },
        SyncMessage::Done(res) => {
            state.busy = None;
            state.result = Some(res);
            update(state, SyncMessage::Check, ctx);
        }
        SyncMessage::Dismiss => state.result = None,
    }
}

// ── View ──

fn wrapped(col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, text: &str, size: f32, color: [f32; 4]) {
    let lines = wrap_to_width(text, col.form_width(), size);
    col.lines(lines, size, color);
}

fn action(msg: SyncMessage) -> AppAction {
    AppAction::Accounts(AccountsMessage::Sync(msg))
}

/// The section's contents, in a column of the Accounts page's second well.
pub fn view(state: &SyncState, col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, ctx: &mut UiContext) {
    let Some(snap) = &state.snap else {
        col.text(if state.checking { "Checking cce-sync..." } else { "Not checked yet." }, 12.0, TEXT_DIM);
        return;
    };
    let Some(server) = &snap.server else {
        wrapped(col, "Not set up: config.kdl has no sync { server \"…\" } block.", 12.0, TEXT_FG);
        return;
    };
    let (token, color) = match snap.token {
        TokenPlace::Keyring => ("in the keyring", TEXT_FG),
        TokenPlace::Env => ("from CCE_SYNC_TOKEN", TEXT_FG),
        TokenPlace::File => ("in a plain file (cce-sync.token)", WARN),
        TokenPlace::Missing => ("none: sync is signed out", WARN),
        TokenPlace::Unknown => ("unknown: the keyring did not answer", WARN),
    };
    crate::app::form_pairs(
        col,
        12.0,
        vec![("Server".into(), TEXT_DIM, server.clone(), TEXT_FG), ("Token".into(), TEXT_DIM, token.to_string(), color)],
    );
    if snap.token == TokenPlace::File && state.busy.is_none() && state.confirm.is_none() {
        col.row(|r| {
            form_button(r, "Move to keyring", button_need("Move to keyring"), GO, action(SyncMessage::MoveToKeyring));
            r.space(0.0, true);
        });
    }

    if state.token_box_visible() {
        col.widget_h(ctx, state.token_box, cce_ui::layout::spinbox_height());
        wrapped(col, "Checked with the server, then kept in the keyring.", 11.0, TEXT_DIM);
        col.row(|r| {
            form_button(r, "Save token", button_need("Save token"), NEUTRAL, action(SyncMessage::SetToken));
            r.space(0.0, true);
        });
    }

    form_divider(col);
    col.text("Connected apps", 12.0, TEXT_FG);
    match &snap.grants {
        Err(e) => wrapped(col, &format!("Could not list them: {e}"), 11.0, RED),
        Ok(grants) => {
            let live: Vec<&Grant> = grants.iter().filter(|g| !g.revoked).collect();
            if live.is_empty() {
                wrapped(col, "No app can reach the vault.", 11.0, TEXT_DIM);
            }
            for g in live {
                let detail = [
                    (!g.email.is_empty()).then(|| g.email.clone()),
                    Some(format!("since {}", g.since)),
                    (!g.last_used.is_empty()).then(|| format!("last used {}", g.last_used)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                col.row(|r| {
                    r.text(g.client.clone(), 12.0, TEXT_FG);
                    r.space(0.0, true);
                    if state.busy.is_none() && state.confirm.is_none() {
                        form_button(r, "Revoke", button_need("Revoke"), DANGER, action(SyncMessage::Revoke(g.id.clone(), g.client.clone())));
                    }
                });
                wrapped(col, &detail, 11.0, TEXT_DIM);
            }
            let revoked = grants.len() - grants.iter().filter(|g| !g.revoked).count();
            if revoked > 0 {
                wrapped(col, &format!("{revoked} revoked, not shown."), 11.0, TEXT_DIM);
            }
        }
    }

    if let Some(c) = &state.confirm {
        form_divider(col);
        let (question, go) = match c {
            Confirm::Revoke { client, .. } => (format!("Revoke {client}? It loses access to the vault until it signs in again."), "Revoke"),
            Confirm::MoveToKeyring => ("Store the token in the keyring and delete the plain file?".to_string(), "Move"),
        };
        wrapped(col, &question, 12.0, WARN);
        col.row(|r| {
            form_button(r, go, button_need(go), DANGER, action(SyncMessage::ConfirmYes));
            form_button(r, "Cancel", button_need("Cancel"), NEUTRAL, action(SyncMessage::ConfirmNo));
            r.space(0.0, true);
        });
    }
    if let Some(b) = &state.busy {
        wrapped(col, b, 11.0, TEXT_DIM);
    }
    if let Some(res) = &state.result {
        let (text, color) = match res {
            Ok(t) => (t.as_str(), TEXT_FG),
            Err(e) => (e.as_str(), RED),
        };
        wrapped(col, text, 11.0, color);
        col.row(|r| {
            form_button(r, "Dismiss", button_need("Dismiss"), NEUTRAL, action(SyncMessage::Dismiss));
            r.space(0.0, true);
        });
    }
    if state.busy.is_none() {
        col.row(|r| {
            let label = if state.checking { "Checking..." } else { "Check again" };
            form_button(r, label, button_need("Check again"), NEUTRAL, action(SyncMessage::Check));
            r.space(0.0, true);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real `cce-sync grants` output (2026-10-16, ids shortened).
    #[test]
    fn reads_the_grants() {
        let out = "\
g_IUvV  cce vault (the phone app) lsgalante12@gmail.com  since 2026-10-14 20:05  last used 2026-10-14 22:40
g_a3lv  cce vault (the phone app) lsgalante12@gmail.com  since 2026-10-13 17:04  last used 2026-10-14 17:06  (revoked)
g_Jph5  Claude                   lsgalante12@gmail.com  since 2026-10-13 15:58  last used 2026-10-16 08:47
g_noem  Bare                       since 2026-10-13 15:58  last used
";
        let g = parse_grants(out);
        assert_eq!(g.len(), 4);
        assert_eq!((g[0].id.as_str(), g[0].client.as_str()), ("g_IUvV", "cce vault (the phone app)"));
        assert_eq!(g[0].email, "lsgalante12@gmail.com");
        assert_eq!((g[0].since.as_str(), g[0].last_used.as_str()), ("2026-10-14 20:05", "2026-10-14 22:40"));
        assert!(!g[0].revoked && g[1].revoked);
        assert_eq!((g[2].client.as_str(), g[2].last_used.as_str()), ("Claude", "2026-10-16 08:47"));
        assert_eq!((g[3].client.as_str(), g[3].email.as_str()), ("Bare", ""));
    }

    #[test]
    fn finds_the_server_as_cce_sync_does() {
        assert_eq!(server_from(None, "sync {\n    server \"https://vault.example\"\n}\n").as_deref(), Some("https://vault.example"));
        assert_eq!(server_from(None, "sync server=\"https://p.example\"\n").as_deref(), Some("https://p.example"));
        assert_eq!(server_from(Some("https://env.example".into()), "sync { server \"x\" }").as_deref(), Some("https://env.example"));
        assert_eq!(server_from(None, "style { }"), None);
        assert_eq!(server_from(Some(String::new()), "nope {"), None);
    }

    #[test]
    fn revoking_asks_first() {
        let mut ui = UiContext::new();
        let mut st = SyncState::new(&mut ui);
        st.snap = Some(SyncSnapshot { server: Some("s".into()), token: TokenPlace::Keyring, grants: Ok(Vec::new()) });
        update(&mut st, SyncMessage::Revoke("g_1".into(), "Claude".into()), &mut ui);
        assert!(st.busy.is_none(), "nothing runs before the confirmation");
        assert_eq!(st.confirm, Some(Confirm::Revoke { id: "g_1".into(), client: "Claude".into() }));
        assert!(!st.token_box_visible(), "the token box steps aside for the question");
        update(&mut st, SyncMessage::ConfirmNo, &mut ui);
        assert!(st.confirm.is_none() && st.busy.is_none());
    }
}
