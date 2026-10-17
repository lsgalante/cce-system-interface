//! Code Agent (cce-code-agent) settings: what a new session starts on
//! (model, effort, permission mode), the features the app turns on in the
//! CLI (prompt suggestions, suggested tasks, Claude's own next steps), desktop notifications, the sidebar at startup and
//! the `claude` binary to run. Edits the app's own config
//! (`~/.config/cce/cce-code-agent/config.kdl`, section "code-agent"), which
//! cce-code-agent reads at every CLI launch (its `src/settings.rs`), so a
//! change counts from the next session or restart; nothing reloads.
//!
//! The pickers here offer the CLI's documented values only: model aliases
//! (`--model`), effort levels (`--effort`) and permission modes. A model's
//! own list comes from the CLI at runtime, so a specific version is picked
//! in the agent's header, not here. The first row of each, "default",
//! leaves the choice to the CLI.

use std::fs;

use cce_ui::compose::{PageFlow, PageLayoutBuilder};
use cce_ui::context::UiContext;
use cce_ui::widget::input::{Dropdown, Toggle};
use cce_ui::widget::{Handle, TextBox};

use crate::app::{wrap_to_width, AppAction, PageCanvas};
use crate::pages::AppPage;

const APP: &str = "cce-code-agent";
const SECTION: &str = "code-agent";
const TEXT_DIM: [f32; 4] = [0.53, 0.53, 0.60, 1.0];

/// config value, menu label — `--model` aliases.
pub const MODELS: [(&str, &str); 4] =
    [("default", "Claude Code's Default"), ("opus", "Opus"), ("sonnet", "Sonnet"), ("haiku", "Haiku")];

/// config value, menu label — `--effort` levels. A model that takes no
/// effort (Haiku) ignores it.
pub const EFFORTS: [(&str, &str); 6] = [
    ("default", "Auto (the model's own)"),
    ("low", "Low"),
    ("medium", "Medium"),
    ("high", "High"),
    ("xhigh", "Extra High"),
    ("max", "Max"),
];

/// config value, menu label — the permission modes, worded as the agent's
/// own picker words them. Bypass is left out: the CLI refuses to switch
/// into it unless launched with `--dangerously-skip-permissions`.
pub const MODES: [(&str, &str); 5] = [
    ("default", "Ask before acting"),
    ("acceptEdits", "Accept edits, ask for the rest"),
    ("plan", "Plan only, change nothing"),
    ("auto", "Auto: a classifier decides"),
    ("dontAsk", "Don't ask: deny what would ask"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAgentConfig {
    pub model: String,
    pub effort: String,
    pub mode: String,
    pub prompt_suggestions: bool,
    pub suggested_tasks: bool,
    /// Claude suggests the next prompt itself (`suggest_next_prompt`).
    pub next_steps: bool,
    /// Desktop notifications when a turn ends, or a permission prompt
    /// waits, where the user is not looking.
    pub notifications: bool,
    pub sidebar: bool,
    /// The `claude` binary; empty is `claude` on PATH.
    pub cli: String,
}

impl Default for CodeAgentConfig {
    /// The agent's own defaults, as its `settings::Settings` has them.
    fn default() -> Self {
        Self {
            model: "default".into(),
            effort: "default".into(),
            mode: "default".into(),
            prompt_suggestions: true,
            suggested_tasks: true,
            next_steps: true,
            notifications: true,
            sidebar: true,
            cli: String::new(),
        }
    }
}

pub struct CodeAgentState {
    pub loaded: bool,
    pub config: CodeAgentConfig,
    pub model_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub effort_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub mode_menu: Handle<cce_ui::widget::Adapted<Dropdown>>,
    pub suggestions_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub tasks_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub next_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub notify_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub sidebar_toggle: Handle<cce_ui::widget::Adapted<Toggle>>,
    pub cli_box: Handle<cce_ui::widget::Adapted<TextBox>>,
}

impl CodeAgentState {
    /// The page's state, its widgets inserted into `ctx` and filled from the config.
    pub fn new(ctx: &mut UiContext) -> Self {
        let config = read_code_agent_config();
        let labels = |rows: &[(&str, &str)]| rows.iter().map(|(_, l)| l.to_string()).collect::<Vec<_>>();
        let mut cli_box = TextBox::new(config.cli.clone())
            .with_draw_bg_border(true)
            .with_placeholder("claude (on PATH)")
            .with_label("Claude Code CLI");
        cli_box.edit_buffer = config.cli.clone();
        Self {
            loaded: true,
            model_menu: ctx.insert(Dropdown::new(labels(&MODELS), index(&MODELS, &config.model)).with_label("Model")),
            effort_menu: ctx.insert(Dropdown::new(labels(&EFFORTS), index(&EFFORTS, &config.effort)).with_label("Effort")),
            mode_menu: ctx.insert(Dropdown::new(labels(&MODES), index(&MODES, &config.mode)).with_label("Permission Mode")),
            // Left-aligned, as the Browser page's toggles: centred, the
            // label straddled the seam at the toggle's midpoint.
            suggestions_toggle: ctx.insert(Toggle::new().with_label("Prompt Suggestions").with_left_align(true)),
            tasks_toggle: ctx.insert(Toggle::new().with_label("Suggested Tasks").with_left_align(true)),
            next_toggle: ctx.insert(Toggle::new().with_label("Suggest Next Steps").with_left_align(true)),
            notify_toggle: ctx.insert(Toggle::new().with_label("Notifications").with_left_align(true)),
            sidebar_toggle: ctx.insert(Toggle::new().with_label("Sidebar at Start").with_left_align(true)),
            cli_box: ctx.insert(cli_box),
            config,
        }
    }
}

/// A value's row; one the page does not offer (a full model id written
/// by hand, say) is the first, and `show` names it on the closed menu.
fn index(rows: &[(&str, &str)], value: &str) -> usize {
    rows.iter().position(|(v, _)| *v == value).unwrap_or(0)
}

/// Point a menu at `value`: its row, or the value itself on the trigger
/// when no row is it (the agent passes such a value through as it is).
fn show(dd: &mut cce_ui::widget::Adapted<Dropdown>, rows: &[(&str, &str)], value: &str) {
    let known = rows.iter().any(|(v, _)| *v == value);
    dd.selected = if known { index(rows, value) } else { usize::MAX };
    dd.custom_display_text = (!known).then(|| value.to_string());
}

fn value(rows: &[(&str, &str)], i: usize) -> String {
    rows.get(i).map_or("default", |(v, _)| v).to_string()
}

#[derive(Debug, Clone)]
pub enum CodeAgentMessage {
    SetModel(String),
    SetEffort(String),
    SetMode(String),
    ToggleSuggestions,
    ToggleTasks,
    ToggleNextSteps,
    ToggleNotifications,
    ToggleSidebar,
    /// Commit the CLI field.
    Apply,
    Refreshed(CodeAgentConfig),
}

/// A TextBox's live contents: the edit buffer while focused, the
/// committed text otherwise.
fn live_text(tb: &cce_ui::widget::Adapted<TextBox>) -> String {
    if tb.editing { tb.edit_buffer.trim().to_string() } else { tb.text.trim().to_string() }
}

pub fn update(state: &mut CodeAgentState, msg: CodeAgentMessage, ctx: &mut UiContext) {
    let c = &mut state.config;
    match msg {
        CodeAgentMessage::SetModel(v) => {
            write_config_value("model", &v);
            c.model = v;
        }
        CodeAgentMessage::SetEffort(v) => {
            write_config_value("effort", &v);
            c.effort = v;
        }
        CodeAgentMessage::SetMode(v) => {
            write_config_value("permission-mode", &v);
            c.mode = v;
        }
        CodeAgentMessage::ToggleSuggestions => {
            c.prompt_suggestions = !c.prompt_suggestions;
            write_config_value("prompt-suggestions", &c.prompt_suggestions.to_string());
        }
        CodeAgentMessage::ToggleTasks => {
            c.suggested_tasks = !c.suggested_tasks;
            write_config_value("suggested-tasks", &c.suggested_tasks.to_string());
        }
        CodeAgentMessage::ToggleNextSteps => {
            c.next_steps = !c.next_steps;
            write_config_value("next-steps", &c.next_steps.to_string());
        }
        CodeAgentMessage::ToggleNotifications => {
            c.notifications = !c.notifications;
            write_config_value("notifications", &c.notifications.to_string());
        }
        CodeAgentMessage::ToggleSidebar => {
            c.sidebar = !c.sidebar;
            write_config_value("sidebar", &c.sidebar.to_string());
        }
        CodeAgentMessage::Apply => {
            c.cli = live_text(&ctx[state.cli_box]);
            if c.cli.is_empty() {
                cce_ui::config::remove_config_value(&config_path(), &format!("{SECTION}.cli"));
            } else {
                write_config_value("cli", &c.cli);
            }
        }
        CodeAgentMessage::Refreshed(new) => {
            state.loaded = true;
            // Don't clobber the field mid-edit with a watcher refresh.
            let editing = ctx[state.cli_box].editing;
            if !editing && state.config.cli != new.cli {
                ctx[state.cli_box].text = new.cli.clone();
                ctx[state.cli_box].edit_buffer = new.cli.clone();
            }
            let cli = if editing { state.config.cli.clone() } else { new.cli.clone() };
            state.config = CodeAgentConfig { cli, ..new };
        }
    }
}

fn config_path() -> String {
    cce_ui::config::get_app_config_path(APP).to_string_lossy().into_owned()
}

pub fn read_code_agent_config() -> CodeAgentConfig {
    parse(&fs::read_to_string(config_path()).unwrap_or_default())
}

fn parse(content: &str) -> CodeAgentConfig {
    let v = cce_ui::config::parse_kdl_to_json(content);
    let s = &v[SECTION];
    let d = CodeAgentConfig::default();
    let text = |k: &str, or: &str| s[k].as_str().map(str::trim).filter(|t| !t.is_empty()).unwrap_or(or).to_string();
    let flag = |k: &str, or: bool| s[k].as_bool().unwrap_or(or);
    CodeAgentConfig {
        model: text("model", &d.model),
        effort: text("effort", &d.effort),
        mode: text("permission-mode", &d.mode),
        prompt_suggestions: flag("prompt-suggestions", d.prompt_suggestions),
        suggested_tasks: flag("suggested-tasks", d.suggested_tasks),
        next_steps: flag("next-steps", d.next_steps),
        notifications: flag("notifications", d.notifications),
        sidebar: flag("sidebar", d.sidebar),
        cli: text("cli", ""),
    }
}

fn write_config_value(key: &str, value: &str) {
    let path = config_path();
    // The per-app config dir may not exist yet.
    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = fs::create_dir_all(dir);
    }
    // Nesting comes from the dotted key path.
    cce_ui::config::write_config_value(&path, &format!("{SECTION}.{key}"), value, SECTION);
}

fn wrapped(col: &mut cce_ui::compose::FormGroup<'_, '_, PageCanvas>, text: &str) {
    let size = crate::app::text_small();
    let lines = wrap_to_width(text, col.form_width(), size);
    col.lines(lines, size, TEXT_DIM);
}

impl AppPage for CodeAgentState {
    // Sections: [New Sessions] [Code Agent]
    fn section_widgets(&mut self) -> Vec<Vec<cce_ui::widget::WidgetId>> {
        vec![
            vec![self.model_menu.id(), self.effort_menu.id(), self.mode_menu.id()],
            vec![self.suggestions_toggle.id(), self.tasks_toggle.id(), self.next_toggle.id(), self.notify_toggle.id(), self.sidebar_toggle.id(), self.cli_box.id()],
        ]
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
        let mut final_pc = PageCanvas::new();
        let sec_w = 320.0f32;
        let mut builder = PageLayoutBuilder::new(layout, cx, cy, cw, ch, sec_w).with_section_count(2);
        let focus = |i: usize| sec_focused.get(i).copied().unwrap_or(false);
        let (menu_h, toggle_h, field_h) =
            (cce_ui::layout::dropdown_height(), cce_ui::layout::toggle_height(), cce_ui::layout::textbox_height());
        let c = self.config.clone();

        builder.add_section(&mut final_pc, "New Sessions", focus(0), |sec| {
            show(&mut ctx[self.model_menu], &MODELS, &c.model);
            show(&mut ctx[self.effort_menu], &EFFORTS, &c.effort);
            show(&mut ctx[self.mode_menu], &MODES, &c.mode);
            let mut form = sec.form();
            let mut col = form.column();
            col.widget_h(ctx, self.model_menu, menu_h)
                .widget_h(ctx, self.effort_menu, menu_h)
                .widget_h(ctx, self.mode_menu, menu_h);
            wrapped(
                &mut col,
                "What a session starts on when it has no picks of its own: a new one, \
                 or one opened from disk that was never set. The header's pickers \
                 still change them per session.",
            );
            sec.place(form, ctx);
        });

        builder.add_section(&mut final_pc, "Code Agent", focus(1), |sec| {
            ctx[self.suggestions_toggle].set_toggled(c.prompt_suggestions);
            ctx[self.tasks_toggle].set_toggled(c.suggested_tasks);
            ctx[self.next_toggle].set_toggled(c.next_steps);
            ctx[self.notify_toggle].set_toggled(c.notifications);
            ctx[self.sidebar_toggle].set_toggled(c.sidebar);
            let mut form = sec.form();
            let mut col = form.column();
            col.widget_h(ctx, self.suggestions_toggle, toggle_h)
                .widget_h(ctx, self.tasks_toggle, toggle_h)
                .widget_h(ctx, self.next_toggle, toggle_h)
                .widget_h(ctx, self.notify_toggle, toggle_h)
                .widget_h(ctx, self.sidebar_toggle, toggle_h)
                .widget_h(ctx, self.cli_box, field_h)
                .draw(0.0, cce_ui::layout::button_height(), false, |pc, r, _| {
                    pc.button(
                        "Apply",
                        r.x,
                        r.y,
                        r.width,
                        r.height,
                        [0.20, 0.40, 0.65, 1.0],
                        [0.28, 0.50, 0.78, 1.0],
                        [1.0, 1.0, 1.0, 1.0],
                        AppAction::CodeAgent(CodeAgentMessage::Apply),
                    );
                });
            wrapped(&mut col, "Changes count from the next session, or the next restart of the CLI.");
            sec.place(form, ctx);
        });

        final_pc
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
            let msg = if id == self.model_menu.id() {
                CodeAgentMessage::SetModel(value(&MODELS, ctx[self.model_menu].selected))
            } else if id == self.effort_menu.id() {
                CodeAgentMessage::SetEffort(value(&EFFORTS, ctx[self.effort_menu].selected))
            } else if id == self.mode_menu.id() {
                CodeAgentMessage::SetMode(value(&MODES, ctx[self.mode_menu].selected))
            } else if id == self.suggestions_toggle.id() {
                CodeAgentMessage::ToggleSuggestions
            } else if id == self.tasks_toggle.id() {
                CodeAgentMessage::ToggleTasks
            } else if id == self.next_toggle.id() {
                CodeAgentMessage::ToggleNextSteps
            } else if id == self.notify_toggle.id() {
                CodeAgentMessage::ToggleNotifications
            } else if id == self.sidebar_toggle.id() {
                CodeAgentMessage::ToggleSidebar
            } else if id == self.cli_box.id() {
                CodeAgentMessage::Apply
            } else {
                continue;
            };
            actions.push(AppAction::CodeAgent(msg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_set_is_the_agents_defaults() {
        assert_eq!(parse(""), CodeAgentConfig::default());
        assert_eq!(parse("browser { model \"opus\" }"), CodeAgentConfig::default());
    }

    /// The keys are the ones cce-code-agent's `settings::parse` reads.
    #[test]
    fn reads_what_the_agent_reads() {
        let c = parse(
            "code-agent {\n  model \"opus\"\n  effort \"xhigh\"\n  permission-mode \"plan\"\n  prompt-suggestions (bool)false\n  suggested-tasks (bool)false\n  next-steps (bool)false\n  notifications (bool)false\n  sidebar (bool)false\n  cli \"/opt/claude\"\n}\n",
        );
        assert_eq!(
            c,
            CodeAgentConfig {
                model: "opus".into(),
                effort: "xhigh".into(),
                mode: "plan".into(),
                prompt_suggestions: false,
                suggested_tasks: false,
                next_steps: false,
                notifications: false,
                sidebar: false,
                cli: "/opt/claude".into(),
            }
        );
    }

    /// What the page writes reads back as written, flags as flags: the
    /// agent's reader takes only a real bool.
    #[test]
    fn written_values_read_back() {
        let dir = std::env::temp_dir().join(format!("cce-code-agent-page-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.kdl").to_string_lossy().into_owned();
        for (k, v) in [("model", "sonnet"), ("effort", "max"), ("permission-mode", "acceptEdits"), ("sidebar", "false"), ("suggested-tasks", "false"), ("notifications", "false")] {
            assert!(cce_ui::config::write_config_value(&path, &format!("{SECTION}.{k}"), v, SECTION));
        }
        let c = parse(&fs::read_to_string(&path).unwrap());
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            c,
            CodeAgentConfig {
                model: "sonnet".into(),
                effort: "max".into(),
                mode: "acceptEdits".into(),
                sidebar: false,
                suggested_tasks: false,
                notifications: false,
                ..CodeAgentConfig::default()
            }
        );
    }

    #[test]
    fn an_unknown_value_shows_the_first_row() {
        assert_eq!(index(&MODELS, "claude-opus-5-5"), 0);
        assert_eq!(index(&MODES, "bypassPermissions"), 0);
        assert_eq!(value(&EFFORTS, 4), "xhigh");
        assert_eq!(value(&EFFORTS, 99), "default");
    }

    /// A pick on a menu becomes its message, and a committed CLI field Apply.
    #[test]
    fn swept_changes_become_actions() {
        let mut ui = UiContext::new();
        let mut st = CodeAgentState::new(&mut ui);
        let _ = ui.take_events();
        // Made off (the view sets it from the config).
        assert!(ui[st.sidebar_toggle].set_value_string("true"));
        ui[st.cli_box].set_value("/opt/claude");
        let events = ui.take_events();
        let mut actions = Vec::new();
        st.propagate_widget_changes(&events, &mut actions, &mut ui);
        assert!(
            matches!(
                actions.as_slice(),
                [AppAction::CodeAgent(CodeAgentMessage::ToggleSidebar), AppAction::CodeAgent(CodeAgentMessage::Apply)]
            ),
            "{actions:?}"
        );
    }

    #[test]
    fn view_draws_both_sections() {
        let mut ui = UiContext::new();
        let mut st = CodeAgentState::new(&mut ui);
        let mut layout = PageFlow::new();
        let pc = st.view(10.0, 20.0, 800.0, 600.0, false, &[false, false], &mut layout, &mut ui).finish();
        assert!(!pc.texts().is_empty());
    }
}
