//! Which backend answers each XDG portal interface, the way the portal
//! frontend (`xdg-desktop-portal`) decides it — for the System page's
//! Portals section.
//!
//! The routing lives in an unversioned user file,
//! `~/.config/xdg-desktop-portal/cce-portals.conf`, and fails quietly: an
//! interface with no line falls to `default=gtk`, and a line naming a
//! backend that is not installed is skipped with an info-level log nobody
//! reads. Either way apps still get an answer, just not cce's. So this reads
//! the same files the frontend reads and resolves them with its rules
//! (portals.conf(5) and xdg-desktop-portal 1.22's `xdp-portal-impl.c`):
//!
//! - **Config file:** the first of `<desktop>-portals.conf` (each
//!   `XDG_CURRENT_DESKTOP` entry, lowercased) then `portals.conf`, in
//!   `$XDG_CONFIG_HOME`, `$XDG_CONFIG_DIRS`, `/etc`, `$XDG_DATA_HOME`,
//!   `$XDG_DATA_DIRS`, `/usr/share`, each under `xdg-desktop-portal/`.
//! - **Backends:** `*.portal` files under `xdg-desktop-portal/portals/` in
//!   the data dirs; the file's stem is the name the config uses.
//! - **An interface:** its own key, else `default`; each is a `;` list tried
//!   in order, skipping backends that are missing or do not declare the
//!   interface. `*` is the first that does, by name; `none` (first) is no
//!   backend at all. Nothing found falls back to `UseIn=`, then to gtk.
//!   Settings is the exception: the frontend merges EVERY listed backend.
//!
//! The frontend's own environment is used when it is running (read from
//! `/proc/<pid>/environ`): it is a systemd user service, and its
//! `XDG_CURRENT_DESKTOP` is the one that picks the file, not this app's.
//! It also reads all of this once, at startup, so a file edited since then
//! is flagged — it is not in effect until the next login.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IMPL_PREFIX: &str = "org.freedesktop.impl.portal.";
const SETTINGS: &str = "org.freedesktop.impl.portal.Settings";
const FRONTEND: &str = "org.freedesktop.portal.Desktop";

/// The variables the frontend resolves with.
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub current_desktop: Option<String>,
    pub home: Option<String>,
    pub config_home: Option<String>,
    pub config_dirs: Option<String>,
    pub data_home: Option<String>,
    pub data_dirs: Option<String>,
}

impl Env {
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Self {
        let get = |k: &str| get(k).filter(|v| !v.is_empty());
        Env {
            current_desktop: get("XDG_CURRENT_DESKTOP"),
            home: get("HOME"),
            config_home: get("XDG_CONFIG_HOME"),
            config_dirs: get("XDG_CONFIG_DIRS"),
            data_home: get("XDG_DATA_HOME"),
            data_dirs: get("XDG_DATA_DIRS"),
        }
    }

    /// A process's environment, from its NUL-separated `environ`.
    pub fn from_environ(raw: &[u8]) -> Self {
        let vars: BTreeMap<String, String> = raw
            .split(|&b| b == 0)
            .filter_map(|kv| {
                let kv = String::from_utf8_lossy(kv);
                let (k, v) = kv.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
        Env::from_vars(|k| vars.get(k).cloned())
    }

    fn home_join(&self, rel: &str) -> PathBuf {
        PathBuf::from(self.home.clone().unwrap_or_default()).join(rel)
    }

    fn desktops(&self) -> Vec<String> {
        self.current_desktop
            .as_deref()
            .unwrap_or("")
            .split(':')
            .filter(|d| !d.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    }

    fn data_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.data_home.as_ref().map_or_else(|| self.home_join(".local/share"), PathBuf::from)];
        let system = self.data_dirs.as_deref().unwrap_or("/usr/local/share:/usr/share");
        dirs.extend(system.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
        dirs.push(PathBuf::from("/usr/share"));
        dedup(dirs)
    }

    /// Where `portals.conf` files are looked for, highest precedence first.
    fn config_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.config_home.as_ref().map_or_else(|| self.home_join(".config"), PathBuf::from)];
        dirs.extend(self.config_dirs.as_deref().unwrap_or("/etc/xdg").split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
        dirs.push(PathBuf::from("/etc"));
        dirs.extend(self.data_dirs());
        dedup(dirs)
    }

    /// Every candidate config file, in the order the frontend tries them.
    pub fn config_candidates(&self) -> Vec<PathBuf> {
        let desktops = self.desktops();
        self.config_dirs()
            .into_iter()
            .flat_map(|dir| {
                let dir = dir.join("xdg-desktop-portal");
                desktops
                    .iter()
                    .map(|d| dir.join(format!("{d}-portals.conf")))
                    .chain(std::iter::once(dir.join("portals.conf")))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn portal_dirs(&self) -> Vec<PathBuf> {
        self.data_dirs().into_iter().map(|d| d.join("xdg-desktop-portal").join("portals")).collect()
    }
}

fn dedup(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

/// One installed `*.portal` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Backend {
    pub name: String,
    pub path: PathBuf,
    pub dbus_name: String,
    pub interfaces: Vec<String>,
    pub use_in: Vec<String>,
}

impl Backend {
    fn implements(&self, iface: &str) -> bool {
        self.interfaces.iter().any(|i| i == iface)
    }

    /// cce's own backends: cce-desktop, cce (the file chooser), cce-shortcuts.
    pub fn is_cce(name: &str) -> bool {
        name == "cce" || name.starts_with("cce-")
    }
}

/// `[group] key=value` lines, as the frontend's GKeyFile reads them.
fn ini_group(text: &str, group: &str) -> Vec<(String, String)> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            inside = line == format!("[{group}]");
            continue;
        }
        if inside {
            if let Some((k, v)) = line.split_once('=') {
                out.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }
    out
}

fn list(value: &str) -> Vec<String> {
    value.split(';').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
}

pub fn parse_portal_file(name: &str, path: &Path, text: &str) -> Option<Backend> {
    let kv = ini_group(text, "portal");
    let get = |k: &str| kv.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    Some(Backend {
        name: name.to_string(),
        path: path.to_path_buf(),
        dbus_name: get("DBusName")?.to_string(),
        interfaces: list(get("Interfaces").unwrap_or("")),
        use_in: list(get("UseIn").unwrap_or("")),
    })
}

/// Every installed backend, by name; the first directory to define a name
/// wins, as with the frontend.
pub fn load_backends(env: &Env) -> BTreeMap<String, Backend> {
    let mut out = BTreeMap::new();
    for dir in env.portal_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "portal")).collect();
        files.sort();
        for path in files {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
            if out.contains_key(&name) {
                continue;
            }
            if let Some(b) = std::fs::read_to_string(&path).ok().and_then(|t| parse_portal_file(&name, &path, &t)) {
                out.insert(name, b);
            }
        }
    }
    out
}

/// The `[preferred]` group of a portals.conf.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conf {
    pub default: Option<Vec<String>>,
    pub ifaces: BTreeMap<String, Vec<String>>,
}

pub fn parse_conf(text: &str) -> Conf {
    let mut conf = Conf::default();
    for (k, v) in ini_group(text, "preferred") {
        if k == "default" {
            conf.default = Some(list(&v));
        } else if k.starts_with(IMPL_PREFIX) {
            conf.ifaces.insert(k, list(&v));
        }
    }
    conf
}

/// How an interface was routed.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The backends that answer; more than one only for Settings.
    pub backends: Vec<String>,
    /// Entries in the interface's own line that the frontend skips.
    pub skipped: Vec<String>,
    /// The answer came from the `UseIn=`/gtk fallback, not the config.
    pub fallback: bool,
}

/// Resolve `iface` with the frontend's rules (see the module docs).
pub fn resolve(conf: Option<&Conf>, backends: &BTreeMap<String, Backend>, desktops: &[String], iface: &str) -> Route {
    let merge = iface == SETTINGS;
    let own = conf.and_then(|c| c.ifaces.get(iface));
    let default = conf.and_then(|c| c.default.as_ref());
    let mut skipped = Vec::new();

    // `none` heading the line in force means no backend at all.
    if own.or(default).and_then(|l| l.first()).is_some_and(|f| f == "none") {
        return Route { backends: Vec::new(), skipped, fallback: false };
    }

    let pick = |entries: &Vec<String>, skipped: &mut Vec<String>, report: bool| -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for entry in entries {
            let candidates: Vec<String> = match entry.as_str() {
                "*" => backends.values().filter(|b| b.implements(iface)).map(|b| b.name.clone()).collect(),
                "none" => break,
                name => match backends.get(name) {
                    Some(b) if b.implements(iface) => vec![name.to_string()],
                    _ => {
                        if report {
                            skipped.push(name.to_string());
                        }
                        Vec::new()
                    }
                },
            };
            for c in candidates {
                if !found.contains(&c) {
                    found.push(c);
                }
                if !merge {
                    return found;
                }
            }
        }
        found
    };

    let mut found = own.map(|l| pick(l, &mut skipped, true)).unwrap_or_default();
    if found.is_empty() {
        found = default.map(|l| pick(l, &mut skipped, false)).unwrap_or_default();
    }
    if !found.is_empty() {
        return Route { backends: found, skipped, fallback: false };
    }
    let use_in: Vec<String> = backends
        .values()
        .filter(|b| b.implements(iface) && b.use_in.iter().any(|u| desktops.iter().any(|d| d.eq_ignore_ascii_case(u))))
        .map(|b| b.name.clone())
        .collect();
    let found = if !use_in.is_empty() {
        if merge { use_in } else { use_in.into_iter().take(1).collect() }
    } else if backends.get("gtk").is_some_and(|b| b.implements(iface)) {
        vec!["gtk".to_string()]
    } else {
        Vec::new()
    };
    Route { backends: found, skipped, fallback: true }
}

/// One line of the Portals section.
#[derive(Debug, Clone, PartialEq)]
pub struct PortalRow {
    /// Without the `org.freedesktop.impl.portal.` prefix.
    pub interface: String,
    /// The backend(s) that answer, or "none".
    pub answer: String,
    /// Why this row needs attention; None = routed as intended.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PortalReport {
    /// The portals.conf in force, `~`-abbreviated; None = no file found.
    pub config: Option<String>,
    pub rows: Vec<PortalRow>,
    /// Problems with the setup as a whole (no config, stale frontend).
    pub notes: Vec<String>,
}

impl PortalReport {
    pub fn problem_count(&self) -> usize {
        self.rows.iter().filter(|r| r.problem.is_some()).count() + self.notes.len()
    }
}

/// What the bus says about the backends: who is running, and who it could start.
#[derive(Debug, Clone, Default)]
pub struct BusFacts {
    pub owned: Vec<String>,
    pub activatable: Vec<String>,
}

impl BusFacts {
    fn can_answer(&self, dbus_name: &str) -> bool {
        self.owned.iter().chain(&self.activatable).any(|n| n == dbus_name)
    }
}

/// The report, from files already found. `stale_since` is the frontend's
/// start time: files modified after it are not in effect yet.
pub fn report(
    env: &Env,
    config: Option<(&Path, &str)>,
    backends: &BTreeMap<String, Backend>,
    bus: Option<&BusFacts>,
    stale_since: Option<std::time::SystemTime>,
) -> PortalReport {
    let conf = config.map(|(_, text)| parse_conf(text));
    let desktops = env.desktops();

    // The interfaces worth a row: what cce's backends implement, and what
    // the config names explicitly.
    let mut ifaces: Vec<String> = backends
        .values()
        .filter(|b| Backend::is_cce(&b.name))
        .flat_map(|b| b.interfaces.iter().cloned())
        .chain(conf.iter().flat_map(|c| c.ifaces.keys().cloned()))
        .collect();
    ifaces.sort();
    ifaces.dedup();

    let rows = ifaces
        .iter()
        .map(|iface| {
            let route = resolve(conf.as_ref(), backends, &desktops, iface);
            let cce_impl: Vec<&str> =
                backends.values().filter(|b| Backend::is_cce(&b.name) && b.implements(iface)).map(|b| b.name.as_str()).collect();
            let mut problems = Vec::new();
            if !cce_impl.is_empty() && !route.backends.iter().any(|b| Backend::is_cce(b)) {
                problems.push(format!("{} implements it but is not routed", cce_impl.join(", ")));
            }
            for name in &route.skipped {
                problems.push(if backends.contains_key(name) {
                    format!("{name} does not implement it")
                } else {
                    format!("{name} is not installed")
                });
            }
            if let Some(bus) = bus {
                for name in &route.backends {
                    if let Some(b) = backends.get(name) {
                        if !bus.can_answer(&b.dbus_name) {
                            problems.push(format!("{name} cannot start ({} has no service file)", b.dbus_name));
                        }
                    }
                }
            }
            let mut answer = if route.backends.is_empty() { "none".to_string() } else { route.backends.join(", ") };
            if route.fallback && !route.backends.is_empty() {
                answer.push_str(" (fallback)");
            }
            PortalRow {
                interface: iface.trim_start_matches(IMPL_PREFIX).to_string(),
                answer,
                problem: (!problems.is_empty()).then(|| problems.join("; ")),
            }
        })
        .collect();

    let mut notes = Vec::new();
    if config.is_none() {
        notes.push(format!(
            "No portals.conf for {}: every interface falls back to UseIn= and gtk",
            env.current_desktop.as_deref().unwrap_or("an unset XDG_CURRENT_DESKTOP")
        ));
    }
    if let Some(started) = stale_since {
        let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        let mut changed: Vec<String> = config
            .map(|(p, _)| p)
            .into_iter()
            .chain(backends.values().filter(|b| Backend::is_cce(&b.name)).map(|b| b.path.as_path()))
            .filter(|p| modified(p).is_some_and(|m| m > started))
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        changed.sort();
        if !changed.is_empty() {
            notes.push(format!(
                "{} changed after the portal started; in effect at the next login",
                changed.join(", ")
            ));
        }
    }

    PortalReport { config: config.map(|(p, _)| tilde(env, p)), rows, notes }
}

fn tilde(env: &Env, path: &Path) -> String {
    match env.home.as_deref().and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Clock ticks of `/proc/<pid>/stat`'s starttime: USER_HZ, which Linux
/// fixes at 100 on every architecture it exposes it on.
const USER_HZ: u64 = 100;

/// When a process started, from its stat line and the boot time.
fn process_start(pid: u32) -> Option<std::time::SystemTime> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Fields after the parenthesised comm, which may itself hold spaces.
    let after = &stat[stat.rfind(')')? + 2..];
    let ticks: u64 = after.split_whitespace().nth(19)?.parse().ok()?;
    let btime: u64 = std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(btime) + std::time::Duration::from_millis(ticks * 1000 / USER_HZ))
}

async fn bus() -> Option<&'static zbus::Connection> {
    static CONN: tokio::sync::OnceCell<Option<zbus::Connection>> = tokio::sync::OnceCell::const_new();
    CONN.get_or_init(|| async { zbus::Connection::session().await.ok() }).await.as_ref()
}

/// Gather the files and bus facts and build the report.
pub async fn fetch() -> PortalReport {
    let mut frontend_pid = None;
    let mut facts = None;
    if let Some(conn) = bus().await {
        if let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await {
            if let Ok(name) = zbus::names::BusName::try_from(FRONTEND) {
                frontend_pid = dbus.get_connection_unix_process_id(name).await.ok();
            }
            let owned = dbus.list_names().await.unwrap_or_default();
            let activatable = dbus.list_activatable_names().await.unwrap_or_default();
            facts = Some(BusFacts {
                owned: owned.iter().map(|n| n.to_string()).collect(),
                activatable: activatable.iter().map(|n| n.to_string()).collect(),
            });
        }
    }
    let frontend_env = frontend_pid.and_then(|pid| std::fs::read(format!("/proc/{pid}/environ")).ok());
    let env = match &frontend_env {
        Some(raw) => Env::from_environ(raw),
        None => Env::from_vars(|k| std::env::var(k).ok()),
    };
    let started = frontend_pid.and_then(process_start);
    tokio::task::spawn_blocking(move || {
        let config = env.config_candidates().into_iter().find_map(|p| std::fs::read_to_string(&p).ok().map(|t| (p, t)));
        let backends = load_backends(&env);
        report(&env, config.as_ref().map(|(p, t)| (p.as_path(), t.as_str())), &backends, facts.as_ref(), started)
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(name: &str, ifaces: &[&str], use_in: &[&str]) -> (String, Backend) {
        (
            name.to_string(),
            Backend {
                name: name.to_string(),
                path: PathBuf::from(format!("/x/{name}.portal")),
                dbus_name: format!("org.freedesktop.impl.portal.desktop.{name}"),
                interfaces: ifaces.iter().map(|i| format!("{IMPL_PREFIX}{i}")).collect(),
                use_in: use_in.iter().map(|s| s.to_string()).collect(),
            },
        )
    }

    fn installed() -> BTreeMap<String, Backend> {
        [
            backend("cce-desktop", &["Settings", "Inhibit", "Print"], &["cce"]),
            backend("cce", &["FileChooser"], &["cce"]),
            backend("gtk", &["Settings", "Inhibit", "Print", "FileChooser", "AppChooser"], &["gnome"]),
            backend("wlr", &["ScreenCast"], &["wlroots"]),
        ]
        .into_iter()
        .collect()
    }

    fn iface(short: &str) -> String {
        format!("{IMPL_PREFIX}{short}")
    }

    #[test]
    fn config_search_order_is_the_man_pages() {
        let env = Env {
            current_desktop: Some("CCE:wlroots".into()),
            home: Some("/h".into()),
            config_dirs: Some("/etc/xdg".into()),
            data_dirs: Some("/usr/share".into()),
            ..Default::default()
        };
        let c = env.config_candidates();
        let s = |p: &str| PathBuf::from(p);
        assert_eq!(c[0], s("/h/.config/xdg-desktop-portal/cce-portals.conf"));
        assert_eq!(c[1], s("/h/.config/xdg-desktop-portal/wlroots-portals.conf"));
        assert_eq!(c[2], s("/h/.config/xdg-desktop-portal/portals.conf"));
        assert_eq!(c[3], s("/etc/xdg/xdg-desktop-portal/cce-portals.conf"));
        let pos = |p: &str| c.iter().position(|x| x == &s(p)).unwrap();
        assert!(pos("/etc/xdg-desktop-portal/portals.conf") < pos("/h/.local/share/xdg-desktop-portal/cce-portals.conf"));
        assert_eq!(c.last(), Some(&s("/usr/share/xdg-desktop-portal/portals.conf")));
    }

    #[test]
    fn an_interface_line_beats_the_default_and_settings_merges() {
        let conf = parse_conf(
            "[preferred]\n# comment\norg.freedesktop.impl.portal.Settings=cce-desktop;gtk\norg.freedesktop.impl.portal.FileChooser=cce\ndefault=gtk\n",
        );
        let b = installed();
        let r = |i: &str| resolve(Some(&conf), &b, &["cce".into()], &iface(i)).backends;
        assert_eq!(r("Settings"), vec!["cce-desktop", "gtk"]);
        assert_eq!(r("FileChooser"), vec!["cce"]);
        assert_eq!(r("Inhibit"), vec!["gtk"], "no line: the default answers");
    }

    #[test]
    fn missing_and_wrong_backends_are_skipped_and_reported() {
        let conf = parse_conf("[preferred]\norg.freedesktop.impl.portal.Inhibit=ghost;cce;cce-desktop\ndefault=gtk\n");
        let route = resolve(Some(&conf), &installed(), &[], &iface("Inhibit"));
        assert_eq!(route.backends, vec!["cce-desktop"]);
        assert_eq!(route.skipped, vec!["ghost", "cce"]);
    }

    #[test]
    fn none_and_star() {
        let b = installed();
        let conf = parse_conf("[preferred]\norg.freedesktop.impl.portal.Print=none;cce-desktop\norg.freedesktop.impl.portal.Inhibit=*\n");
        assert!(resolve(Some(&conf), &b, &[], &iface("Print")).backends.is_empty());
        assert_eq!(resolve(Some(&conf), &b, &[], &iface("Inhibit")).backends, vec!["cce-desktop"], "first by name");
    }

    #[test]
    fn no_answer_in_the_config_falls_back_to_use_in_then_gtk() {
        let b = installed();
        let conf = parse_conf("[preferred]\norg.freedesktop.impl.portal.ScreenCast=wlr\n");
        let r = resolve(Some(&conf), &b, &["cce".into()], &iface("Print"));
        assert_eq!((r.backends, r.fallback), (vec!["cce-desktop".to_string()], true));
        let r = resolve(None, &b, &["kde".into()], &iface("AppChooser"));
        assert_eq!((r.backends, r.fallback), (vec!["gtk".to_string()], true));
    }

    /// The failure this section exists for: an interface cce implements,
    /// whose line went missing, quietly answered by gtk.
    #[test]
    fn a_cce_interface_routed_elsewhere_is_a_problem() {
        let env = Env { current_desktop: Some("cce".into()), home: Some("/h".into()), ..Default::default() };
        let text = "[preferred]\norg.freedesktop.impl.portal.Settings=cce-desktop;gtk\norg.freedesktop.impl.portal.FileChooser=cce\norg.freedesktop.impl.portal.ScreenCast=wlr\ndefault=gtk\n";
        let path = PathBuf::from("/h/.config/xdg-desktop-portal/cce-portals.conf");
        let b = installed();
        let rep = report(&env, Some((&path, text)), &b, None, None);
        assert_eq!(rep.config.as_deref(), Some("~/.config/xdg-desktop-portal/cce-portals.conf"));
        let row = |i: &str| rep.rows.iter().find(|r| r.interface == i).unwrap_or_else(|| panic!("no {i} row"));
        assert_eq!(row("Settings").problem, None);
        assert_eq!(row("Settings").answer, "cce-desktop, gtk");
        assert_eq!(row("ScreenCast").problem, None, "wlr is the intended answer; cce has none");
        assert_eq!(row("Inhibit").answer, "gtk");
        assert!(row("Inhibit").problem.as_deref().is_some_and(|p| p.contains("cce-desktop implements it")));
        assert!(row("Print").problem.is_some());
        assert!(rep.rows.iter().all(|r| r.interface != "AppChooser"), "only cce's interfaces and the config's lines");
        assert_eq!(rep.problem_count(), 2);
    }

    #[test]
    fn a_backend_the_bus_cannot_start_is_a_problem() {
        let env = Env { current_desktop: Some("cce".into()), ..Default::default() };
        let text = "[preferred]\norg.freedesktop.impl.portal.FileChooser=cce\n";
        let path = PathBuf::from("/c/cce-portals.conf");
        let bus = BusFacts { owned: vec![], activatable: vec!["org.freedesktop.impl.portal.desktop.cce-desktop".into()] };
        let rep = report(&env, Some((&path, text)), &installed(), Some(&bus), None);
        let fc = rep.rows.iter().find(|r| r.interface == "FileChooser").unwrap();
        assert!(fc.problem.as_deref().is_some_and(|p| p.contains("cce cannot start")), "{fc:?}");
    }

    #[test]
    fn no_config_is_a_note() {
        let env = Env { current_desktop: Some("cce".into()), ..Default::default() };
        let rep = report(&env, None, &installed(), None, None);
        assert_eq!(rep.notes.len(), 1);
        assert!(rep.config.is_none());
    }

    #[test]
    fn the_frontends_environ_is_parsed() {
        let env = Env::from_environ(b"HOME=/h\0XDG_CURRENT_DESKTOP=cce\0XDG_DATA_DIRS=\0JUNK\0");
        assert_eq!(env.current_desktop.as_deref(), Some("cce"));
        assert_eq!(env.home.as_deref(), Some("/h"));
        assert_eq!(env.data_dirs, None, "empty means unset");
    }

    #[test]
    fn portal_files_parse() {
        let b = parse_portal_file(
            "cce-desktop",
            Path::new("/p"),
            "[portal]\nDBusName=org.freedesktop.impl.portal.desktop.cce-desktop\nInterfaces=org.freedesktop.impl.portal.Settings;org.freedesktop.impl.portal.Inhibit;\nUseIn=cce;\n",
        )
        .unwrap();
        assert_eq!(b.interfaces.len(), 2);
        assert_eq!(b.use_in, vec!["cce"]);
        assert!(parse_portal_file("x", Path::new("/p"), "[portal]\nInterfaces=a;\n").is_none(), "no DBusName");
    }
}
