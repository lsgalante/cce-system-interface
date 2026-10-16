//! Display: night light, backlight brightness, and what each screen is.
//!
//! Night light is the compositor's (`crate::night_light` in cce-compositor,
//! a gamma table per output). Its mode and night temperature change live
//! over the control socket (`night-light auto|on|off`, `night-light
//! temperature <K>`), which lasts until the next config load, and are also
//! written to `config.kdl`'s `night_light { }` block so a reload keeps them.
//! The schedule (a location's sunset and sunrise, or fixed hours) and the
//! fade are read here but set in that block.
//!
//! Brightness is the kernel backlight (`/sys/class/backlight`), set through
//! `brightnessctl`, the same tool the brightness keys run.
//!
//! A slider reports every step of a drag, so both are debounced: the value
//! goes out once the drag pauses, and the config write once it stops.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cce_ui::compose::{PageFlow, PageLayoutBuilder};
use cce_ui::context::UiContext;
use cce_ui::widget::input::Dropdown;
use cce_ui::widget::{Adapted, Handle, Slider};

use crate::app::{button_need, form_button, form_divider, form_pairs, wrap_to_width, AppAction, PageCanvas};
use crate::pages::AppPage;

const TEXT_FG: [f32; 4] = [0.83, 0.83, 0.83, 1.0];
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];
/// Error text: light enough for small type on the dark plate (7.6:1; the
/// old [0.85, 0.25, 0.25] was 4.1:1, under AA for text this size).
const RED: [f32; 4] = [0.95, 0.55, 0.55, 1.0];
const WARN: [f32; 4] = [0.95, 0.75, 0.55, 1.0];
const NEUTRAL: ([f32; 4], [f32; 4], [f32; 4]) = ([0.16, 0.16, 0.24, 1.0], [0.25, 0.30, 0.26, 1.0], TEXT_FG);

/// The night temperatures the slider spans: the compositor's floor up to
/// neutral (it takes up to 12000 K, but a night light cooler than daylight
/// is not one).
pub const NIGHT_MIN_K: u32 = 1700;
pub const NIGHT_MAX_K: u32 = 6500;
/// Brightness never goes below this from here: some panels switch the
/// backlight off at 0, and a black screen hides the slider that would fix it.
pub const MIN_BRIGHTNESS_PCT: u32 = 5;

/// config value, menu label.
pub const MODES: [(&str, &str); 3] = [("auto", "Follow schedule"), ("on", "Always on"), ("off", "Off")];

#[derive(Debug, Clone, PartialEq)]
pub enum Schedule {
    None,
    Fixed,
    Sun { lat: f64, lon: f64 },
}

/// `ccectl night-light status`, parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct NightLight {
    pub mode: String,
    /// The temperature on screen now, and where it is heading.
    pub shown_k: u32,
    pub target_k: u32,
    pub night_k: u32,
    pub day_k: u32,
    pub schedule: Schedule,
    /// When tonight's night starts and ends ("18:16"); None with no schedule
    /// or in a polar day or night.
    pub night_from: Option<String>,
    pub night_to: Option<String>,
    pub transition_min: f64,
    /// (output, state): applied, neutral, idle, unsupported, client, refused.
    pub outputs: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Backlight {
    pub device: String,
    pub percent: u32,
}

/// One line of `ccectl outputs --json`.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputInfo {
    pub name: String,
    pub enabled: bool,
    pub mode_w: u32,
    pub mode_h: u32,
    pub refresh_hz: f64,
    pub scale: f64,
    pub logical_w: u32,
    pub logical_h: u32,
    pub mm_w: u32,
    pub mm_h: u32,
    pub ppi: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplaySnapshot {
    /// The compositor answered on its control socket.
    pub compositor: bool,
    /// None: config.kdl has no `night_light { }` block.
    pub night: Option<NightLight>,
    pub outputs: Vec<OutputInfo>,
    pub backlights: Vec<Backlight>,
}

pub struct DisplayState {
    pub loaded: bool,
    pub snap: DisplaySnapshot,
    pub mode_menu: Handle<Adapted<Dropdown>>,
    pub night_slider: Handle<Adapted<Slider>>,
    /// One per backlight, index-aligned with `snap.backlights`.
    pub brightness_sliders: Vec<Handle<Adapted<Slider>>>,
    /// The last slider change, so a refresh landing mid-drag does not pull
    /// the knob back to what the system said a moment before.
    pub last_user_change: Option<Instant>,
    /// The last failed change, shown until dismissed or the next one.
    pub error: Option<String>,
    /// Generations for the debounced sends: a send goes out only if no newer
    /// change came after it.
    night_gen: Arc<AtomicU64>,
    persist_gen: Arc<AtomicU64>,
    brightness_gen: Arc<AtomicU64>,
    /// How results come back from the spawned commands (set by the host).
    pub sender: Option<calloop::channel::Sender<AppAction>>,
}

impl DisplayState {
    pub fn new(ctx: &mut UiContext) -> Self {
        Self {
            loaded: false,
            snap: DisplaySnapshot::default(),
            mode_menu: ctx.insert(Dropdown::new(MODES.iter().map(|(_, l)| l.to_string()).collect(), 0).with_label("Mode")),
            night_slider: ctx.insert(named_slider(NIGHT_MIN_K as f32, NIGHT_MAX_K as f32, "Night temperature")),
            brightness_sliders: Vec::new(),
            last_user_change: None,
            error: None,
            night_gen: Arc::new(AtomicU64::new(0)),
            persist_gen: Arc::new(AtomicU64::new(0)),
            brightness_gen: Arc::new(AtomicU64::new(0)),
            sender: None,
        }
    }

    fn night_visible(&self) -> bool {
        self.loaded && self.snap.compositor && self.snap.night.is_some()
    }

    /// A slider was moved within the last two seconds.
    fn user_is_adjusting(&self) -> bool {
        self.last_user_change.is_some_and(|t| t.elapsed() < Duration::from_secs(2))
    }
}

/// A slider named for screen readers but drawing no label of its own: the
/// row above it carries the name and the value, which a slider label
/// would only repeat.
fn named_slider(min: f32, max: f32, name: &str) -> Adapted<Slider> {
    let mut s = Slider::new().with_range(min, max).with_scroll(true);
    s.set_accessible_name(Some(name));
    s
}

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    Refreshed(Box<DisplaySnapshot>),
    SetMode(String),
    /// The night slider moved to this many kelvin.
    SetNightTemperature(u32),
    /// Backlight `i` moved to this percent.
    SetBrightness(usize, u32),
    Failed(String),
    DismissError,
}

// ── Reading ──

/// The `key=value` fields of a status line; a bare word (the "(polar)" note)
/// is kept as a key with an empty value.
fn fields(line: &str) -> Vec<(&str, &str)> {
    line.split_whitespace().map(|t| t.split_once('=').unwrap_or((t, ""))).collect()
}

pub fn parse_night_status(line: &str) -> Option<NightLight> {
    let f = fields(line.trim());
    let get = |k: &str| f.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
    if get("configured") != Some("yes") {
        return None;
    }
    let k = |key: &str| get(key).and_then(|v| v.parse::<f64>().ok()).map_or(0, |v| v.round() as u32);
    let schedule = match get("schedule") {
        Some(s) if s.starts_with("sun(") => {
            let inner = s.trim_start_matches("sun(").trim_end_matches(')');
            let mut it = inner.split(',').map(|v| v.parse::<f64>().unwrap_or(0.0));
            Schedule::Sun { lat: it.next().unwrap_or(0.0), lon: it.next().unwrap_or(0.0) }
        }
        Some("fixed") => Schedule::Fixed,
        _ => Schedule::None,
    };
    let clock = |key: &str| get(key).filter(|v| !v.is_empty() && *v != "-").map(str::to_string);
    let outputs = get("outputs")
        .filter(|v| *v != "-")
        .map(|v| v.split(',').filter_map(|o| o.split_once(':')).map(|(n, s)| (n.to_string(), s.to_string())).collect())
        .unwrap_or_default();
    Some(NightLight {
        mode: get("mode").unwrap_or("auto").to_string(),
        shown_k: k("temperature"),
        target_k: k("target"),
        night_k: k("night_temperature"),
        day_k: k("day_temperature"),
        schedule,
        night_from: clock("night_from"),
        night_to: clock("night_to"),
        transition_min: get("transition").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        outputs,
    })
}

pub fn parse_outputs(json_lines: &str) -> Vec<OutputInfo> {
    json_lines
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| {
            let u = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let f = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
            OutputInfo {
                name: v.get("name").and_then(|x| x.as_str()).unwrap_or("?").to_string(),
                enabled: v.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
                mode_w: u("mode_w"),
                mode_h: u("mode_h"),
                refresh_hz: f("refresh_mhz") / 1000.0,
                scale: f("scale"),
                logical_w: u("logical_w"),
                logical_h: u("logical_h"),
                mm_w: u("mm_w"),
                mm_h: u("mm_h"),
                ppi: f("ppi"),
            }
        })
        .collect()
}

fn read_backlights() -> Vec<Backlight> {
    let Ok(dir) = std::fs::read_dir("/sys/class/backlight") else { return Vec::new() };
    let mut out: Vec<Backlight> = dir
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let read = |f: &str| std::fs::read_to_string(p.join(f)).ok()?.trim().parse::<u64>().ok();
            let (cur, max) = (read("brightness")?, read("max_brightness")?);
            (max > 0).then(|| Backlight {
                device: e.file_name().to_string_lossy().into_owned(),
                percent: ((cur * 100 + max / 2) / max) as u32,
            })
        })
        .collect();
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}

fn ctl(line: &str) -> Result<String, String> {
    match cce_ui::ipc::ctl::send(line, Some(Duration::from_secs(2))) {
        Ok(reply) if reply.starts_with("error") => Err(reply.trim().trim_start_matches("error: ").to_string()),
        Ok(reply) => Ok(reply),
        Err(e) => Err(format!("The compositor did not answer: {e}")),
    }
}

/// The page worker's read: a socket round trip or two and a few sysfs files.
pub fn read_display_snapshot() -> DisplaySnapshot {
    let status = ctl("night-light status");
    let compositor = status.is_ok();
    DisplaySnapshot {
        compositor,
        night: status.ok().and_then(|s| parse_night_status(&s)),
        outputs: if compositor { ctl("outputs --json").map(|s| parse_outputs(&s)).unwrap_or_default() } else { Vec::new() },
        backlights: read_backlights(),
    }
}

// ── Changing ──

/// Run `job` off the UI thread after `delay`, unless a newer call on the
/// same `gen` came first. A failure comes back as `Failed`.
fn debounced(
    gen: &Arc<AtomicU64>,
    delay: Duration,
    sender: Option<calloop::channel::Sender<AppAction>>,
    job: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    let mine = gen.fetch_add(1, Ordering::SeqCst) + 1;
    let gen = gen.clone();
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        if gen.load(Ordering::SeqCst) != mine {
            return;
        }
        if let Ok(Err(e)) = tokio::task::spawn_blocking(job).await {
            if let Some(tx) = sender {
                let _ = tx.send(AppAction::Display(DisplayMessage::Failed(e)));
            }
        }
    });
}

/// Write one key of `night_light { }`. An unparseable config.kdl is left
/// alone — `write_config_value` would start from an empty document and
/// write back a file holding only this key.
fn persist_night(key: &str, value: &str) -> Result<(), String> {
    let path = cce_ui::config::get_config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if content.parse::<kdl::KdlDocument>().is_err() {
        return Err("config.kdl does not parse, so the change was not saved (it holds until the next reload).".to_string());
    }
    if cce_ui::config::write_config_value(&path.to_string_lossy(), &format!("night_light.{key}"), value, "night_light") {
        Ok(())
    } else {
        Err("Could not write config.kdl; the change holds until the next reload.".to_string())
    }
}

fn set_brightness(device: &str, pct: u32) -> Result<(), String> {
    let out = std::process::Command::new("brightnessctl")
        .args(["-q", "-d", device, "set", &format!("{pct}%")])
        .output()
        .map_err(|e| format!("Could not run brightnessctl: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("brightnessctl: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

fn mode_index(mode: &str) -> usize {
    MODES.iter().position(|(m, _)| *m == mode).unwrap_or(0)
}

/// Kelvin ↔ the slider's 0–1.
fn k_to_norm(k: u32) -> f32 {
    (k.clamp(NIGHT_MIN_K, NIGHT_MAX_K) - NIGHT_MIN_K) as f32 / (NIGHT_MAX_K - NIGHT_MIN_K) as f32
}

/// The slider's kelvin (`get_scaled_value`), in 50 K steps.
fn round_k(k: f32) -> u32 {
    (((k / 50.0).round() * 50.0) as u32).clamp(NIGHT_MIN_K, NIGHT_MAX_K)
}

fn pct_to_norm(p: u32) -> f32 {
    p.min(100) as f32 / 100.0
}

/// The slider's percent (`get_scaled_value`), never below the floor.
fn clamp_pct(p: f32) -> u32 {
    (p.round().clamp(0.0, 100.0) as u32).max(MIN_BRIGHTNESS_PCT)
}

pub fn update(state: &mut DisplayState, msg: DisplayMessage, ctx: &mut UiContext) {
    match msg {
        DisplayMessage::Refreshed(new) => {
            let mut new = *new;
            let adjusting = state.user_is_adjusting();
            // Mid-drag, what the user set stands over what the system said.
            if adjusting {
                if let (Some(n), Some(old)) = (new.night.as_mut(), state.snap.night.as_ref()) {
                    n.night_k = old.night_k;
                }
                for b in new.backlights.iter_mut() {
                    if let Some(old) = state.snap.backlights.iter().find(|o| o.device == b.device) {
                        b.percent = old.percent;
                    }
                }
            }
            // One slider per backlight.
            while state.brightness_sliders.len() < new.backlights.len() {
                let h = ctx.insert(named_slider(0.0, 100.0, "Brightness"));
                state.brightness_sliders.push(h);
            }
            for h in state.brightness_sliders.drain(new.backlights.len()..) {
                ctx.remove(h);
            }
            if !adjusting {
                if let Some(n) = &new.night {
                    ctx[state.night_slider].set_value(k_to_norm(n.night_k));
                }
                for (b, &h) in new.backlights.iter().zip(&state.brightness_sliders) {
                    ctx[h].set_value(pct_to_norm(b.percent));
                }
            }
            if let Some(n) = &new.night {
                ctx[state.mode_menu].selected = mode_index(&n.mode);
            }
            state.snap = new;
            state.loaded = true;
        }
        DisplayMessage::SetMode(mode) => {
            state.error = None;
            if let Some(n) = state.snap.night.as_mut() {
                n.mode = mode.clone();
            }
            // Live first; saved only if the compositor took it.
            let sender = state.sender.clone();
            tokio::task::spawn_blocking(move || {
                let res = ctl(&format!("night-light {mode}")).and_then(|_| persist_night("mode", &mode));
                if let (Err(e), Some(tx)) = (res, sender) {
                    let _ = tx.send(AppAction::Display(DisplayMessage::Failed(e)));
                }
            });
        }
        DisplayMessage::SetNightTemperature(k) => {
            state.error = None;
            state.last_user_change = Some(Instant::now());
            if let Some(n) = state.snap.night.as_mut() {
                n.night_k = k;
            }
            debounced(&state.night_gen, Duration::from_millis(80), state.sender.clone(), move || {
                ctl(&format!("night-light temperature {k}")).map(|_| ())
            });
            debounced(&state.persist_gen, Duration::from_millis(900), state.sender.clone(), move || {
                persist_night("temperature", &k.to_string())
            });
        }
        DisplayMessage::SetBrightness(i, pct) => {
            state.error = None;
            state.last_user_change = Some(Instant::now());
            let Some(b) = state.snap.backlights.get_mut(i) else { return };
            b.percent = pct;
            let device = b.device.clone();
            debounced(&state.brightness_gen, Duration::from_millis(60), state.sender.clone(), move || set_brightness(&device, pct));
        }
        DisplayMessage::Failed(e) => state.error = Some(e),
        DisplayMessage::DismissError => state.error = None,
    }
}

// ── View ──

fn wrapped(col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, text: &str, size: f32, color: [f32; 4]) {
    let lines = wrap_to_width(text, col.form_width(), size);
    col.lines(lines, size, color);
}

/// Several sentences as one run of lines: one paragraph, a line apart, not
/// a control gap apart.
fn paragraph(col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, texts: &[String], size: f32, color: [f32; 4]) {
    let w = col.form_width();
    let lines: Vec<String> = texts.iter().flat_map(|t| wrap_to_width(t, w, size)).collect();
    if !lines.is_empty() {
        col.lines(lines, size, color);
    }
}

/// What a night-light output state means for the person looking at it.
fn output_state_note(state: &str) -> Option<&'static str> {
    match state {
        "unsupported" => Some("this display cannot take a colour table, so night light cannot tint it"),
        "client" => Some("another program (a gamma tool such as gammastep or wlsunset) holds it"),
        "refused" => Some("the display refused the colour table"),
        _ => None,
    }
}

fn schedule_lines(n: &NightLight) -> Vec<String> {
    let mut lines = Vec::new();
    match (&n.schedule, &n.night_from, &n.night_to) {
        (Schedule::Sun { .. }, Some(from), Some(to)) => lines.push(format!("Tonight: sunset {from} to sunrise {to}")),
        (Schedule::Sun { .. }, _, _) => lines.push("No sunset or sunrise here today (polar day or night).".to_string()),
        (Schedule::Fixed, Some(from), Some(to)) => lines.push(format!("Night: {from} to {to}")),
        (Schedule::Fixed, _, _) => lines.push("Fixed hours.".to_string()),
        (Schedule::None, _, _) => {
            lines.push("No schedule is set, so Follow schedule stays at the daytime temperature.".to_string());
        }
    }
    if n.schedule != Schedule::None && n.transition_min > 0.0 {
        lines.push(format!("Fades over {} minutes.", n.transition_min.round()));
    }
    lines
}

pub fn view(state: &mut DisplayState, cx: f32, cy: f32, cw: f32, ch: f32, focused: &[bool], layout: &mut PageFlow, ctx: &mut UiContext) -> PageCanvas {
    let mut final_pc = PageCanvas::new();
    let sec_w = 320.0f32;
    let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(3);
    let focus = |i: usize| focused.get(i).copied().unwrap_or(false);
    let snap = state.snap.clone();

    builder.add_section(&mut final_pc, "Night Light", focus(0), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        if !state.loaded {
            col.text("Asking the compositor...", 12.0, TEXT_DIM);
        } else if !snap.compositor {
            wrapped(&mut col, "The compositor did not answer, so night light cannot be read or changed.", 12.0, WARN);
        } else if let Some(n) = &snap.night {
            col.widget_h(ctx, state.mode_menu, cce_ui::layout::dropdown_height());
            let now = if n.shown_k >= n.day_k && n.target_k >= n.day_k {
                format!("Now {} K, the daytime temperature", n.shown_k)
            } else if n.shown_k != n.target_k {
                format!("Now {} K, fading to {} K", n.shown_k, n.target_k)
            } else {
                format!("Now {} K", n.shown_k)
            };
            wrapped(&mut col, &now, 12.0, TEXT_FG);

            form_divider(&mut col);
            col.row(|r| {
                r.text("Night temperature", 12.0, TEXT_FG);
                r.space(0.0, true);
                r.text(format!("{} K", n.night_k), 12.0, TEXT_FG);
            });
            col.widget_h(ctx, state.night_slider, cce_ui::layout::slider_height());
            wrapped(&mut col, "Lower is warmer. 6500 K is no tint.", 11.0, TEXT_DIM);

            form_divider(&mut col);
            let mut notes = schedule_lines(n);
            notes.push("The location or hours and the fade are set in config.kdl's night_light block.".to_string());
            paragraph(&mut col, &notes, 11.0, TEXT_DIM);
            let warnings: Vec<String> = n
                .outputs
                .iter()
                .filter_map(|(output, st)| output_state_note(st).map(|note| format!("{output}: {note}.")))
                .collect();
            paragraph(&mut col, &warnings, 11.0, WARN);
        } else {
            wrapped(&mut col, "Night light is not set up: config.kdl has no night_light block.", 12.0, TEXT_FG);
            wrapped(
                &mut col,
                "Add one with a temperature and a location (night_light { temperature 3500; location latitude=… longitude=… }), then reload.",
                11.0,
                TEXT_DIM,
            );
        }
        if let Some(e) = state.error.clone() {
            form_divider(&mut col);
            wrapped(&mut col, &e, 11.0, RED);
            col.row(|r| {
                form_button(r, "Dismiss", button_need("Dismiss"), NEUTRAL, AppAction::Display(DisplayMessage::DismissError));
                r.space(0.0, true);
            });
        }
        sec.place(form, ctx);
    });

    builder.add_section(&mut final_pc, "Brightness", focus(1), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        if state.loaded {
            if snap.backlights.is_empty() {
                wrapped(&mut col, "No adjustable backlight. An external monitor's brightness is set on the monitor.", 11.0, TEXT_DIM);
            }
            let several = snap.backlights.len() > 1;
            for (b, &h) in snap.backlights.iter().zip(&state.brightness_sliders) {
                col.row(|r| {
                    r.text(if several { b.device.clone() } else { "Screen".to_string() }, 12.0, TEXT_FG);
                    r.space(0.0, true);
                    r.text(format!("{}%", b.percent), 12.0, TEXT_FG);
                });
                col.widget_h(ctx, h, cce_ui::layout::slider_height());
            }
        }
        sec.place(form, ctx);
    });

    builder.add_section(&mut final_pc, "Displays", focus(2), |sec| {
        let mut form = sec.form();
        let mut col = form.column();
        if state.loaded && snap.compositor {
            for (i, o) in snap.outputs.iter().enumerate() {
                if i > 0 {
                    form_divider(&mut col);
                }
                col.text(o.name.clone(), 12.0, TEXT_FG);
                // A headless or virtual output reports no refresh rate.
                let rate = if o.refresh_hz > 0.0 { format!(" at {:.0} Hz", o.refresh_hz) } else { String::new() };
                let mut pairs = vec![("Resolution".to_string(), TEXT_DIM, format!("{} × {}{rate}", o.mode_w, o.mode_h), TEXT_FG)];
                pairs.push(("Scale".to_string(), TEXT_DIM, format!("{}× ({} × {} logical)", o.scale, o.logical_w, o.logical_h), TEXT_FG));
                if o.mm_w > 0 && o.mm_h > 0 {
                    pairs.push(("Size".to_string(), TEXT_DIM, format!("{} × {} mm, {:.0} ppi", o.mm_w, o.mm_h, o.ppi), TEXT_FG));
                }
                if !o.enabled {
                    pairs.push(("State".to_string(), TEXT_DIM, "Off".to_string(), WARN));
                }
                form_pairs(&mut col, 12.0, pairs);
            }
            if snap.outputs.is_empty() {
                col.text("No displays reported.", 11.0, TEXT_DIM);
            }
            form_divider(&mut col);
            wrapped(&mut col, "Scale is set in config.kdl's output block. When the screen turns off is on the Power page.", 11.0, TEXT_DIM);
        }
        sec.place(form, ctx);
    });

    final_pc
}

impl AppPage for DisplayState {
    // Sections: [Night Light, Brightness, Displays]. Mirrors the view's
    // gates: the night controls only once a night_light block answered.
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        let night = if self.night_visible() { vec![self.mode_menu.id(), self.night_slider.id()] } else { Vec::new() };
        let bright = if self.loaded { self.brightness_sliders.iter().take(self.snap.backlights.len()).map(|h| h.id()).collect() } else { Vec::new() };
        vec![night, bright, Vec::new()]
    }

    fn view(
        &mut self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        _root_focused: bool,
        sec_focused: &[bool],
        layout: &mut PageFlow,
        ctx: &mut UiContext,
    ) -> PageCanvas {
        view(self, cx, cy, cw, ch, sec_focused, layout, ctx)
    }

    fn propagate_widget_changes(
        &mut self,
        events: &[(cce_ui::widget::WidgetId, cce_ui::widget::WidgetEvent)],
        actions: &mut Vec<AppAction>,
        ctx: &mut UiContext,
    ) {
        for &(id, ref ev) in events {
            if *ev != cce_ui::widget::WidgetEvent::Changed {
                continue;
            }
            if id == self.mode_menu.id() {
                let mode = MODES.get(ctx[self.mode_menu].selected).map_or("auto", |(m, _)| m);
                if self.snap.night.as_ref().is_some_and(|n| n.mode != mode) {
                    actions.push(AppAction::Display(DisplayMessage::SetMode(mode.to_string())));
                }
            } else if id == self.night_slider.id() {
                // `get_scaled_value`, not `value()`: the adapter's `value()` is the
                // widget's integer reading, not the slider's position.
                let k = round_k(ctx[self.night_slider].get_scaled_value());
                if self.snap.night.as_ref().is_some_and(|n| n.night_k != k) {
                    actions.push(AppAction::Display(DisplayMessage::SetNightTemperature(k)));
                }
            } else if let Some(i) = self.brightness_sliders.iter().position(|h| h.id() == id) {
                let pct = clamp_pct(ctx[self.brightness_sliders[i]].get_scaled_value());
                if self.snap.backlights.get(i).is_some_and(|b| b.percent != pct) {
                    actions.push(AppAction::Display(DisplayMessage::SetBrightness(i, pct)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real `ccectl night-light status` (2026-10-16).
    const STATUS: &str = "configured=yes mode=auto temperature=6500 target=6500 night_temperature=3500 day_temperature=6500 schedule=sun(40.710,-74.010) night_from=18:16 night_to=07:07 night=0.00 transition=45 outputs=eDP-1:neutral\n";

    #[test]
    fn reads_the_night_light_status() {
        let n = parse_night_status(STATUS).unwrap();
        assert_eq!(n.mode, "auto");
        assert_eq!((n.shown_k, n.target_k, n.night_k, n.day_k), (6500, 6500, 3500, 6500));
        assert_eq!(n.schedule, Schedule::Sun { lat: 40.71, lon: -74.01 });
        assert_eq!((n.night_from.as_deref(), n.night_to.as_deref()), (Some("18:16"), Some("07:07")));
        assert_eq!(n.transition_min, 45.0);
        assert_eq!(n.outputs, [("eDP-1".to_string(), "neutral".to_string())]);
        assert_eq!(schedule_lines(&n), ["Tonight: sunset 18:16 to sunrise 07:07", "Fades over 45 minutes."]);

        assert!(parse_night_status("configured=no\n").is_none());
        let polar = parse_night_status(
            "configured=yes mode=on temperature=3500 target=3500 night_temperature=3500 day_temperature=6500 schedule=sun(78.000,15.000) night_from=- night_to=- (polar) night=1.00 transition=0 outputs=-",
        )
        .unwrap();
        assert!(polar.night_from.is_none() && polar.outputs.is_empty());
        assert!(schedule_lines(&polar)[0].contains("polar"));
    }

    /// Real `ccectl outputs --json` (2026-10-16).
    #[test]
    fn reads_the_outputs() {
        let o = parse_outputs(r#"{"enabled":true,"logical_h":1200,"logical_w":1920,"mm_h":220,"mm_w":340,"mode_h":2400,"mode_w":3840,"name":"eDP-1","ppi":140.99037433155078,"px_per_mm":5.550802139037433,"refresh_mhz":60000,"scale":2.0,"source":"measured","x":0,"y":0}"#);
        assert_eq!(o.len(), 1);
        assert_eq!((o[0].mode_w, o[0].mode_h, o[0].refresh_hz, o[0].scale), (3840, 2400, 60.0, 2.0));
        assert_eq!((o[0].logical_w, o[0].mm_w), (1920, 340));
    }

    #[test]
    fn slider_values_round_trip() {
        let mut ui = UiContext::new();
        let st = DisplayState::new(&mut ui);
        ui[st.night_slider].set_value(k_to_norm(3500));
        assert_eq!(round_k(ui[st.night_slider].get_scaled_value()), 3500);
        ui[st.night_slider].set_value(0.0);
        assert_eq!(round_k(ui[st.night_slider].get_scaled_value()), NIGHT_MIN_K);
        assert_eq!(round_k(3512.0), 3500, "50 K steps");
        assert_eq!(clamp_pct(0.0), MIN_BRIGHTNESS_PCT, "never black");
        assert_eq!(clamp_pct(100.0 * pct_to_norm(60)), 60);
    }

    fn loaded(ui: &mut UiContext) -> DisplayState {
        let mut st = DisplayState::new(ui);
        let snap = DisplaySnapshot {
            compositor: true,
            night: parse_night_status(STATUS),
            outputs: Vec::new(),
            backlights: vec![Backlight { device: "intel_backlight".into(), percent: 40 }],
        };
        update(&mut st, DisplayMessage::Refreshed(Box::new(snap)), ui);
        st
    }

    /// A refresh during a drag keeps what was dragged to.
    #[test]
    fn a_refresh_mid_drag_does_not_pull_the_slider_back() {
        let mut ui = UiContext::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _g = rt.enter();
        let mut st = loaded(&mut ui);
        assert_eq!(st.brightness_sliders.len(), 1);
        update(&mut st, DisplayMessage::SetBrightness(0, 80), &mut ui);
        update(&mut st, DisplayMessage::SetNightTemperature(2700), &mut ui);
        let mut stale = st.snap.clone();
        stale.backlights[0].percent = 40;
        stale.night.as_mut().unwrap().night_k = 3500;
        update(&mut st, DisplayMessage::Refreshed(Box::new(stale)), &mut ui);
        assert_eq!(st.snap.backlights[0].percent, 80);
        assert_eq!(st.snap.night.as_ref().unwrap().night_k, 2700);
    }

    #[test]
    fn section_widgets_mirror_the_view() {
        let mut ui = UiContext::new();
        let mut st = DisplayState::new(&mut ui);
        assert_eq!(st.section_widgets(), vec![Vec::new(), Vec::new(), Vec::new()]);
        let mut st2 = loaded(&mut ui);
        let w = st2.section_widgets();
        assert_eq!(w[0].len(), 2);
        assert_eq!(w[1].len(), 1);
        st.loaded = true;
        st.snap.compositor = true;
        assert!(st.section_widgets()[0].is_empty(), "no night_light block: no night controls");
    }

    #[test]
    fn every_state_renders() {
        let render = |st: &mut DisplayState, ui: &mut UiContext| {
            let mut layout = PageFlow::new();
            view(st, 0.0, 0.0, 800.0, 600.0, &[false; 3], &mut layout, ui).finish()
        };
        let mut ui = UiContext::new();
        let mut st = DisplayState::new(&mut ui);
        render(&mut st, &mut ui);
        st.loaded = true;
        let pc = render(&mut st, &mut ui);
        assert!(pc.texts().iter().any(|t| t.text.contains("did not answer")));
        st.snap.compositor = true;
        let pc = render(&mut st, &mut ui);
        assert!(pc.texts().iter().any(|t| t.text.contains("not set up")));
        let mut st = loaded(&mut ui);
        st.error = Some("brightnessctl: no permission".into());
        let pc = render(&mut st, &mut ui);
        let texts: Vec<String> = pc.texts().into_iter().map(|t| t.text).collect();
        assert!(texts.iter().any(|t| t == "3500 K"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "40%"));
        assert!(texts.iter().any(|t| t.contains("no permission")));
    }
}
