use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Tabs, Wrap},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::shell::{CommandInfo, RapsHelper};
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::{Config, load_profiles};
use raps_kernel::http::HttpClientConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DashboardSettings {
    lazy_load_data: bool,
}

impl Default for DashboardSettings {
    fn default() -> Self {
        Self {
            lazy_load_data: true,
        }
    }
}

fn get_settings_path() -> PathBuf {
    let mut path = directories::ProjectDirs::from("com", "autodesk", "raps")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    path.push("dashboard-settings.toml");
    path
}

fn load_settings() -> DashboardSettings {
    let path = get_settings_path();
    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(settings) = toml::from_str(&contents) {
            return settings;
        }
    }
    DashboardSettings::default()
}

fn save_settings(settings: &DashboardSettings) {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(contents) = toml::to_string_pretty(settings) {
        let _ = fs::write(&path, contents);
    }
}

struct DashboardCache {
    hubs: Option<Vec<(String, String)>>,
    projects: HashMap<String, Vec<(String, String)>>,
}

#[derive(Clone, Debug)]
enum FormField {
    Positional { name: String, value: String },
    FlagValue { name: String, value: String },
    FlagToggle { name: String, enabled: bool },
}

#[derive(Clone, Debug)]
struct FormState {
    fields: Vec<FormField>,
    focused_index: usize,
}

impl FormState {
    fn new(params: &[&str], flags: &[&str], include_optional: bool) -> Self {
        let mut fields = Vec::new();

        for p in params {
            fields.push(FormField::Positional {
                name: p.to_string(),
                value: String::new(),
            });
        }

        if include_optional {
            for f in flags {
                if f.contains("<") && f.contains(">") {
                    // e.g. "--name <NAME>" -> clean up to just display the flag
                    let name = f.split_whitespace().next().unwrap_or(f).to_string();
                    fields.push(FormField::FlagValue {
                        name,
                        value: String::new(),
                    });
                } else {
                    fields.push(FormField::FlagToggle {
                        name: f.to_string(),
                        enabled: false,
                    });
                }
            }
        }

        Self {
            fields,
            focused_index: 0,
        }
    }

    fn generate_args(&self) -> String {
        let mut parts = Vec::new();
        for field in &self.fields {
            match field {
                FormField::Positional { value, .. } => {
                    if !value.is_empty() {
                        parts.push(value.clone());
                    }
                }
                FormField::FlagValue { name, value } => {
                    if !value.is_empty() {
                        parts.push(format!("{} \"{}\"", name, value.replace('"', "\\\"")));
                    }
                }
                FormField::FlagToggle { name, enabled } => {
                    if *enabled {
                        parts.push(name.clone());
                    }
                }
            }
        }
        parts.join(" ")
    }
}

#[derive(Default)]
enum PromptPhase {
    #[default]
    None,
    HubSelection {
        state: ListState,
        hub_list: Vec<(String, String)>,
        needs_project: bool,
        include_optional: bool,
    },
    ProjectSelection {
        state: ListState,
        hub_id: String,
        project_list: Vec<(String, String)>,
        include_optional: bool,
    },
    Form(FormState),
}
#[derive(Default, PartialEq, Clone)]
enum FocusPane {
    #[default]
    CommandList,
    OutputTable,
}

enum ExecEvent {
    Line(String),
    JsonData(serde_json::Value),
}

struct App {
    #[allow(dead_code)]
    commands: Vec<CommandInfo>,

    // Focus state
    focus: FocusPane,

    // Tabs state
    tabs: Vec<String>,
    tab_index: usize,

    // Command lists per tab: tab_index -> flat_list
    grouped_lists: Vec<Vec<(String, String, CommandInfo)>>,

    // Selection state per tab: tab_index -> list_state
    grouped_states: Vec<ListState>,

    // Prompt state
    prompt_phase: PromptPhase,
    prompt_text: String,
    prompt_cmd_path: String,

    // Auth state for display
    auth_status: String,
    cache: Arc<RwLock<DashboardCache>>,

    // Execution state
    execution_log: Vec<String>,
    execution_json_data: Option<serde_json::Value>,
    is_executing: bool,
    output_scroll: u16,

    // Overlays
    settings: DashboardSettings,
    settings_active: bool,
    help_active: bool,

    // Debug Output Toggle
    show_debug: bool,
}

impl App {
    async fn new(mut commands: Vec<CommandInfo>) -> Self {
        // Fetch auth status asynchronously
        let config = Config::from_env_lenient().unwrap_or(Config {
            client_id: String::new(),
            client_secret: String::new(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: String::new(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        });
        let http_config = HttpClientConfig::from_cli_and_env(None);
        let auth_client = AuthClient::new_with_http_config(config.clone(), http_config);

        let auth_status = if auth_client.get_token().await.is_ok() {
            let profile_name = load_profiles()
                .ok()
                .and_then(|data| data.active_profile)
                .unwrap_or_else(|| "default".to_string());
            format!("Logged in [{}]", profile_name)
        } else {
            "Not logged in".to_string()
        };

        let settings = load_settings();
        let cache = Arc::new(RwLock::new(DashboardCache {
            hubs: None,
            projects: HashMap::new(),
        }));

        if auth_client.get_token().await.is_ok() && settings.lazy_load_data {
            let config_clone = config.clone();
            let auth_clone = auth_client.clone();
            let http_clone = HttpClientConfig::from_cli_and_env(None);
            let dm_client =
                DataManagementClient::new_with_http_config(config_clone, auth_clone, http_clone);
            let cache_clone = cache.clone();

            tokio::spawn(async move {
                if let Ok(hubs) = dm_client.list_hubs().await {
                    let hub_items: Vec<(String, String)> = hubs
                        .into_iter()
                        .map(|h| (h.id, h.attributes.name))
                        .collect();

                    {
                        let mut w = cache_clone.write().await;
                        w.hubs = Some(hub_items.clone());
                    }

                    for (hub_id, _) in hub_items.into_iter().take(4) {
                        if let Ok(projs) = dm_client.list_projects(&hub_id).await {
                            let proj_items: Vec<(String, String)> = projs
                                .into_iter()
                                .map(|p| (p.id, p.attributes.name))
                                .collect();
                            let mut w = cache_clone.write().await;
                            w.projects.insert(hub_id, proj_items);
                        }
                    }
                }
            });
        }

        // Sort commands alphabetically by name for consistent tabs
        commands.sort_by(|a, b| a.name.cmp(&b.name));

        // Remove "help" and "completions" as they aren't useful in dashboard
        let filtered_commands: Vec<CommandInfo> = commands
            .into_iter()
            .filter(|c| {
                c.name != "help"
                    && c.name != "completions"
                    && c.name != "shell"
                    && c.name != "dashboard"
                    && c.name != "serve"
            })
            .collect();

        let mut tabs = Vec::new();
        let mut grouped_lists = Vec::new();
        let mut grouped_states = Vec::new();

        for cmd in &filtered_commands {
            tabs.push(cmd.name.to_lowercase());

            let mut flat_list = Vec::new();
            // We start depth at 0, but since the tab IS the root command,
            // we really just want the subcommands flattened.
            if cmd.subcommands.is_empty() {
                // If a command has no subcommands (e.g. auth login, but imagine a root command with no subs)
                // just push itself so it's selectable.
                flatten_commands(cmd, &mut flat_list, 0, "");
            } else {
                for sub in cmd.subcommands {
                    flatten_commands(sub, &mut flat_list, 0, &cmd.name);
                }
            }

            grouped_lists.push(flat_list);

            let mut state = ListState::default();
            state.select(Some(0));
            grouped_states.push(state);
        }

        Self {
            commands: filtered_commands,
            tabs,
            tab_index: 0,
            grouped_lists,
            grouped_states,

            focus: FocusPane::CommandList,

            prompt_phase: PromptPhase::None,
            prompt_text: String::new(),
            prompt_cmd_path: String::new(),

            auth_status,
            cache,

            execution_log: Vec::new(),
            execution_json_data: None,
            is_executing: false,
            output_scroll: 0,

            settings,
            settings_active: false,
            help_active: false,
            show_debug: false,
        }
    }

    fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tab_index = (self.tab_index + 1) % self.tabs.len();
    }

    fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if self.tab_index == 0 {
            self.tab_index = self.tabs.len() - 1;
        } else {
            self.tab_index -= 1;
        }
    }

    fn next_item(&mut self) {
        if self.grouped_lists.is_empty() {
            return;
        }

        let list = &self.grouped_lists[self.tab_index];
        if list.is_empty() {
            return;
        }

        let state = &mut self.grouped_states[self.tab_index];
        let i = match state.selected() {
            Some(i) => {
                if i >= list.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn prev_item(&mut self) {
        if self.grouped_lists.is_empty() {
            return;
        }

        let list = &self.grouped_lists[self.tab_index];
        if list.is_empty() {
            return;
        }

        let state = &mut self.grouped_states[self.tab_index];
        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    list.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }
}

fn flatten_commands(
    cmd: &CommandInfo,
    flat: &mut Vec<(String, String, CommandInfo)>,
    depth: usize,
    prefix: &str,
) {
    let indent = "  ".repeat(depth);
    let display_name = format!("{}{}", indent, cmd.name);

    let full_path = if prefix.is_empty() {
        cmd.name.to_string()
    } else {
        format!("{} {}", prefix, cmd.name)
    };

    flat.push((display_name, full_path.clone(), cmd.clone()));

    for sub in cmd.subcommands {
        flatten_commands(sub, flat, depth + 1, &full_path);
    }
}

pub async fn execute() -> Result<()> {
    // 1. Fetch commands directly from memory
    let helper = RapsHelper::new();
    let commands = helper.commands().to_vec();
    let mut app = App::new(commands).await;

    // 2. Setup terminal (do not enable raw mode yet, run_app handles it)
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // 3. Run app
    let res = run_app(&mut terminal, &mut app).await;

    // 4. Any final cleanup if needed (run_app guarantees original state)
    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend + io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    let (tx, mut rx) = mpsc::channel::<ExecEvent>(100);
    let mut tick_rate = tokio::time::interval(Duration::from_millis(50));

    loop {
        terminal.draw(|f| ui(f, app))?;

        tokio::select! {
            _ = tick_rate.tick() => {
                // Periodically check if there are new logs from the executing command
                while let Ok(event) = rx.try_recv() {
                    match event {
                        ExecEvent::Line(line) => app.execution_log.push(line),
                        ExecEvent::JsonData(val) => {
                            app.execution_json_data = Some(val);
                        }
                    }
                }
            }
            crossterm_event = tokio::task::spawn_blocking(move || {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    event::read().ok()
                } else {
                    None
                }
            }) => {
                if let Ok(Some(Event::Key(key))) = crossterm_event {
                    if key.kind == event::KeyEventKind::Press {
                        if app.is_executing {
                            // If user hits Escape, we could implement a cancellation mechanism here.
                            // For now, we wait or just let them return to dashboard when done.
                            if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                                app.is_executing = false;
                            }
                        } else if app.settings_active {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('s') => {
                                    app.settings_active = false;
                                }
                                KeyCode::Enter | KeyCode::Char(' ') => {
                                    app.settings.lazy_load_data = !app.settings.lazy_load_data;
                                    save_settings(&app.settings);
                                }
                                _ => {}
                            }
                        } else if app.help_active {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('?') => {
                                    app.help_active = false;
                                }
                                _ => {}
                            }
                        } else if !matches!(app.prompt_phase, PromptPhase::None) {
                            let phase = std::mem::replace(&mut app.prompt_phase, PromptPhase::None);
                            match phase {
                                PromptPhase::HubSelection { mut state, hub_list, needs_project, include_optional } => match key.code {
                                    KeyCode::Esc => {}, // remains None
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        let i = match state.selected() {
                                            Some(i) => if i >= hub_list.len().saturating_sub(1) { 0 } else { i + 1 },
                                            None => 0,
                                        };
                                        state.select(Some(i));
                                        app.prompt_phase = PromptPhase::HubSelection { state, hub_list, needs_project, include_optional };
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        let i = match state.selected() {
                                            Some(i) => if i == 0 { hub_list.len().saturating_sub(1) } else { i - 1 },
                                            None => 0,
                                        };
                                        state.select(Some(i));
                                        app.prompt_phase = PromptPhase::HubSelection { state, hub_list, needs_project, include_optional };
                                    }
                                    KeyCode::Enter => {
                                        if let Some(i) = state.selected() {
                                            let hub_id = hub_list[i].0.clone();
                                            app.prompt_text.push_str(&hub_id);
                                            app.prompt_text.push(' ');

                                            if needs_project {
                                                        let projs = app.cache.read().await.projects.get(&hub_id).cloned();
                                                if let Some(p) = projs {
                                                    if !p.is_empty() {
                                                        let mut new_state = ratatui::widgets::ListState::default();
                                                        new_state.select(Some(0));
                                                        app.prompt_phase = PromptPhase::ProjectSelection {
                                                            state: new_state,
                                                            hub_id,
                                                            project_list: p,
                                                            include_optional,
                                                        };
                                                    } else {
                                                        app.prompt_phase = PromptPhase::Form(FormState::new(
                                                            &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                            &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                            include_optional
                                                        ));
                                                    }
                                                } else {
                                                    app.prompt_phase = PromptPhase::Form(FormState::new(
                                                        &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                        &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                            include_optional
                                                    ));
                                                }
                                            } else {
                                                app.prompt_phase = PromptPhase::Form(FormState::new(
                                                    &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                    &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                            include_optional
                                                ));
                                            }
                                        } else {
                                            app.prompt_phase = PromptPhase::HubSelection { state, hub_list, needs_project, include_optional };
                                        }
                                    }
                                    _ => { app.prompt_phase = PromptPhase::HubSelection { state, hub_list, needs_project, include_optional }; }
                                },
                                PromptPhase::ProjectSelection { mut state, hub_id, project_list, include_optional } => match key.code {
                                    KeyCode::Esc => {},
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        let i = match state.selected() {
                                            Some(i) => if i >= project_list.len().saturating_sub(1) { 0 } else { i + 1 },
                                            None => 0,
                                        };
                                        state.select(Some(i));
                                        app.prompt_phase = PromptPhase::ProjectSelection { state, hub_id, project_list, include_optional };
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        let i = match state.selected() {
                                            Some(i) => if i == 0 { project_list.len().saturating_sub(1) } else { i - 1 },
                                            None => 0,
                                        };
                                        state.select(Some(i));
                                        app.prompt_phase = PromptPhase::ProjectSelection { state, hub_id, project_list, include_optional };
                                    }
                                    KeyCode::Enter => {
                                        if let Some(i) = state.selected() {
                                            let proj_id = project_list[i].0.clone();
                                            app.prompt_text.push_str(&proj_id);
                                            app.prompt_text.push(' ');
                                        }
                                        app.prompt_phase = PromptPhase::Form(FormState::new(
                                            &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                            &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                            include_optional
                                        ));
                                    }
                                    _ => { app.prompt_phase = PromptPhase::ProjectSelection { state, hub_id, project_list, include_optional }; }
                                },
                                PromptPhase::Form(mut form) => match key.code {
                                    KeyCode::Esc => {
                                        app.prompt_phase = PromptPhase::None;
                                    }
                                    KeyCode::Down => {
                                        if !form.fields.is_empty() {
                                            form.focused_index = (form.focused_index + 1) % form.fields.len();
                                        }
                                        app.prompt_phase = PromptPhase::Form(form);
                                    }
                                    KeyCode::Up => {
                                        if !form.fields.is_empty() {
                                            form.focused_index = if form.focused_index == 0 {
                                                form.fields.len() - 1
                                            } else {
                                                form.focused_index - 1
                                            };
                                        }
                                        app.prompt_phase = PromptPhase::Form(form);
                                    }
                                    KeyCode::Enter => {
                                        let base_args = app.prompt_text.clone();
                                        let form_args = form.generate_args();

                                        let final_args = if base_args.is_empty() {
                                            form_args
                                        } else if form_args.is_empty() {
                                            base_args
                                        } else {
                                            format!("{} {}", base_args.trim_end(), form_args)
                                        };

                                        app.prompt_phase = PromptPhase::None;
                                        app.is_executing = true;
                                        app.execution_log.clear();
                                        app.execution_json_data = None;
                                        app.output_scroll = 0;

                                        let tx_clone = tx.clone();
                                        let full_path = app.prompt_cmd_path.clone();
                                        let args_str = final_args;

                                        tokio::spawn(async move {
                                            let _ = spawn_and_stream(full_path, args_str, tx_clone).await;
                                        });
                                    }
                                    KeyCode::Char(' ') => {
                                        if let Some(field) = form.fields.get_mut(form.focused_index) {
                                            if let FormField::FlagToggle { enabled, .. } = field {
                                                *enabled = !*enabled;
                                                app.prompt_phase = PromptPhase::Form(form);
                                                continue;
                                            }
                                        }
                                        // Fall back to typing a space if it wasn't a checkbox
                                        if let Some(field) = form.fields.get_mut(form.focused_index) {
                                            match field {
                                                FormField::Positional { value, .. } | FormField::FlagValue { value, .. } => {
                                                    value.push(' ');
                                                }
                                                _ => {}
                                            }
                                        }
                                        app.prompt_phase = PromptPhase::Form(form);
                                    }
                                    KeyCode::Backspace => {
                                        if let Some(field) = form.fields.get_mut(form.focused_index) {
                                            match field {
                                                FormField::Positional { value, .. } | FormField::FlagValue { value, .. } => {
                                                    value.pop();
                                                }
                                                _ => {}
                                            }
                                        }
                                        app.prompt_phase = PromptPhase::Form(form);
                                    }
                                    KeyCode::Char(c) => {
                                        if let Some(field) = form.fields.get_mut(form.focused_index) {
                                            match field {
                                                FormField::Positional { value, .. } | FormField::FlagValue { value, .. } => {
                                                    value.push(c);
                                                }
                                                _ => {}
                                            }
                                        }
                                        app.prompt_phase = PromptPhase::Form(form);
                                    }
                                    _ => { app.prompt_phase = PromptPhase::Form(form); }
                                },
                                PromptPhase::None => unreachable!(),
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('s') => app.settings_active = true,
                                KeyCode::Char('?') => app.help_active = true,
                                KeyCode::Char('d') => app.show_debug = !app.show_debug,
                                KeyCode::Tab => {
                                    app.focus = match app.focus {
                                        FocusPane::CommandList => FocusPane::OutputTable,
                                        FocusPane::OutputTable => FocusPane::CommandList,
                                    };
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if app.focus == FocusPane::CommandList {
                                        app.next_item();
                                    } else {
                                        app.output_scroll = app.output_scroll.saturating_add(1);
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if app.focus == FocusPane::CommandList {
                                        app.prev_item();
                                    } else {
                                        app.output_scroll = app.output_scroll.saturating_sub(1);
                                    }
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    if app.focus == FocusPane::CommandList {
                                        app.next_tab();
                                    }
                                }
                                KeyCode::Left | KeyCode::Char('h') => {
                                    if app.focus == FocusPane::CommandList {
                                        app.prev_tab();
                                    }
                                }
                                KeyCode::Enter => {
                                    let has_shift = key.modifiers.contains(event::KeyModifiers::SHIFT);

                                    if app.grouped_lists.is_empty() {
                                        continue;
                                    }
                                    let list = &app.grouped_lists[app.tab_index];
                                    let state = &app.grouped_states[app.tab_index];

                                    if let Some(i) = state.selected() {
                                        let (_, ref full_path, ref cmd) = list[i];
                                        app.prompt_cmd_path = full_path.clone();
                                        app.prompt_text.clear();

                                        let needs_hub = cmd.params.iter().any(|p| p.to_lowercase().contains("hub_id"));
                                        let needs_project = cmd.params.iter().any(|p| p.to_lowercase().contains("project_id"));

                                        let mut execute_directly = false;

                                        let new_phase = if needs_hub && app.settings.lazy_load_data {
                                            let hubs = app.cache.read().await.hubs.clone();
                                            if let Some(h) = hubs {
                                                if !h.is_empty() {
                                                    let mut state = ratatui::widgets::ListState::default();
                                                    state.select(Some(0));
                                                    PromptPhase::HubSelection {
                                                        state,
                                                        hub_list: h,
                                                        needs_project,
                                                        include_optional: has_shift,
                                                    }
                                                } else {
                                                    PromptPhase::Form(FormState::new(
                                                        &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                        &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                        has_shift
                                                    ))
                                                }
                                            } else {
                                                PromptPhase::Form(FormState::new(
                                                    &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                    &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                    has_shift
                                                ))
                                            }
                                        } else {
                                            PromptPhase::Form(FormState::new(
                                                &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.params.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                &app.grouped_lists[app.tab_index][app.grouped_states[app.tab_index].selected().unwrap_or(0)].2.flags.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
                                                has_shift
                                            ))
                                        };

                                        // If the resulting phase is a Form and it has no fields, skip straight to execution
                                        if let PromptPhase::Form(ref form) = new_phase {
                                            if form.fields.is_empty() {
                                                execute_directly = true;
                                            }
                                        }

                                        if execute_directly {
                                            app.prompt_phase = PromptPhase::None;
                                            app.is_executing = true;
                                            app.execution_log.clear();
                                            app.execution_json_data = None;
                                            app.output_scroll = 0;

                                            let tx_clone = tx.clone();
                                            let full_path = app.prompt_cmd_path.clone();
                                            let args_str = app.prompt_text.clone(); // No form args

                                            tokio::spawn(async move {
                                                let _ = spawn_and_stream(full_path, args_str, tx_clone).await;
                                            });
                                        } else {
                                            app.prompt_phase = new_phase;
                                        }
                                    }
                                }
                                _ => {
                                    // Check if the pressed key matches the first letter of any tab
                                    if let KeyCode::Char(c) = key.code {
                                        let c_lower = c.to_ascii_lowercase();
                                        for (i, tab) in app.tabs.iter().enumerate() {
                                            if tab.starts_with(c_lower) {
                                                app.tab_index = i;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn format_json_value(val: &serde_json::Value, indent: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let pad = " ".repeat(indent);
    match val {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                lines.push(format!("{{}}"));
            } else {
                for (k, v) in map {
                    match v {
                        serde_json::Value::Object(o) if !o.is_empty() => {
                            lines.push(format!("{}{}:", pad, k));
                            lines.extend(format_json_value(v, indent + 2));
                        }
                        serde_json::Value::Array(a) if !a.is_empty() => {
                            lines.push(format!("{}{}:", pad, k));
                            lines.extend(format_json_value(v, indent + 2));
                        }
                        serde_json::Value::String(s) => {
                            lines.push(format!("{}{}: {}", pad, k, s));
                        }
                        _ => {
                            let val_str = format_json_value(v, 0).join("");
                            lines.push(format!("{}{}: {}", pad, k, val_str.trim()));
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                lines.push(format!("[]"));
            } else {
                for item in arr {
                    match item {
                        serde_json::Value::Object(m) if !m.is_empty() => {
                            let mut inner = format_json_value(item, indent + 2);
                            if let Some(first) = inner.first_mut() {
                                let old = " ".repeat(indent + 2);
                                let new_prefix = format!("{}- ", " ".repeat(indent));
                                if first.starts_with(&old) {
                                    *first = first.replacen(&old, &new_prefix, 1);
                                } else {
                                    *first =
                                        format!("{}- {}", " ".repeat(indent), first.trim_start());
                                }
                            }
                            lines.extend(inner);
                        }
                        serde_json::Value::Array(a) if !a.is_empty() => {
                            lines.push(format!("{}-", pad));
                            lines.extend(format_json_value(item, indent + 2));
                        }
                        serde_json::Value::String(s) => {
                            lines.push(format!("{}- {}", pad, s));
                        }
                        _ => {
                            let val_str = format_json_value(item, 0).join("");
                            lines.push(format!("{}- {}", pad, val_str.trim()));
                        }
                    }
                }
            }
        }
        serde_json::Value::String(s) => lines.push(format!("{}{}", pad, s)),
        serde_json::Value::Bool(b) => lines.push(format!("{}{}", pad, b)),
        serde_json::Value::Number(n) => lines.push(format!("{}{}", pad, n)),
        serde_json::Value::Null => lines.push(format!("{}null", pad)),
    }
    lines
}

async fn spawn_and_stream(
    full_path: String,
    args_str: String,
    tx: mpsc::Sender<ExecEvent>,
) -> Result<()> {
    let _ = tx
        .send(ExecEvent::Line(format!(
            "Running: raps {} {}",
            full_path, args_str
        )))
        .await;

    let exe = std::env::current_exe()?;
    let mut cmd = tokio::process::Command::new(exe);

    for part in full_path.split_whitespace() {
        cmd.arg(part);
    }

    if let Some(parsed_args) = shlex::split(&args_str) {
        for arg in parsed_args {
            cmd.arg(arg);
        }
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx
                .send(ExecEvent::Line(format!("Failed to spawn process: {}", e)))
                .await;
            return Ok(());
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let tx_out = tx.clone();
    let tx_err = tx.clone();

    // Spawn a task to read stdout
    if let Some(out) = stdout {
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut reader = BufReader::new(out).lines();
            let mut full_output = String::new();
            while let Ok(Some(line)) = reader.next_line().await {
                full_output.push_str(&line);
                full_output.push('\n');
                let display_lines = match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(val) => {
                        // If it's a JSON array containing objects (like a list of Hubs),
                        // emit a JsonData event so the UI can draw a Table instead of flat text logs
                        if let serde_json::Value::Array(ref arr) = val {
                            if !arr.is_empty() && arr.iter().all(|i| i.is_object()) {
                                let _ = tx_out.send(ExecEvent::JsonData(val.clone())).await;
                                // We also format it conventionally just in case the UI fails
                                format_json_value(&val, 0)
                            } else {
                                format_json_value(&val, 0)
                            }
                        } else {
                            format_json_value(&val, 0)
                        }
                    }
                    Err(_) => vec![line.clone()],
                };
                for l in display_lines {
                    let _ = tx_out.send(ExecEvent::Line(l)).await;
                }
            }

            // On completion, check if the full payload parses as JSON (if it wasn't intercepted line-by-line).
            // Sometimes CLI commands output a multiline json, which wouldn't be caught by the per-line checker above.
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&full_output) {
                if let serde_json::Value::Array(ref arr) = val {
                    if !arr.is_empty() && arr.iter().all(|i| i.is_object()) {
                        let _ = tx_out.send(ExecEvent::JsonData(val.clone())).await;
                    }
                }
            }
        });
    }

    // Spawn a task to read stderr
    if let Some(err) = stderr {
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx_err
                    .send(ExecEvent::Line(format!("[ERROR]: {}", line)))
                    .await;
            }
        });
    }

    let status = child.wait().await?;
    let _ = tx
        .send(ExecEvent::Line(format!(
            "Command exited with status: {}",
            status.code().unwrap_or(1)
        )))
        .await;
    let _ = tx
        .send(ExecEvent::Line(
            "Press [Esc] or [Enter] to return to Dashboard...".to_string(),
        ))
        .await;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    let mut constraint_list = vec![
        Constraint::Length(3), // Tabs
        Constraint::Min(0),    // Content
    ];

    // If we have executed something, show an execution panel above the status bar
    let show_exec_panel = app.is_executing || !app.execution_log.is_empty();
    if show_exec_panel {
        // Expand to percentage value to handle verbose JSON
        constraint_list.push(Constraint::Percentage(60)); // Execution Panel
    }
    constraint_list.push(Constraint::Length(1)); // Status Bar

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraint_list)
        .split(size);

    // Render Tabs with Scrolling Logic to prevent Horizontal Overflow
    let available_width = chunks[0].width.saturating_sub(4) as usize; // account for borders

    let mut start_idx = 0;
    let mut current_width = 0;

    // Calculate how many tabs fit backwards from the selected tab to keep it in view
    for (i, title) in app.tabs.iter().enumerate() {
        let title_len = title.len() + 3; // +3 for separators & padding
        if i < app.tab_index {
            current_width += title_len;
        }
    }

    // Attempt to keep selected tab on screen by incrementing start_idx if we exceed available width
    while current_width > available_width && start_idx < app.tab_index {
        let title_len = app.tabs[start_idx].len() + 3;
        current_width = current_width.saturating_sub(title_len);
        start_idx += 1;
    }

    let tab_titles: Vec<Line> = app
        .tabs
        .iter()
        .skip(start_idx)
        .map(|t| Line::from(Span::styled(t, Style::default().fg(Color::Green))))
        .collect();

    let title_prefix = if start_idx > 0 { "< " } else { "" };
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {}RAPS Commands ", title_prefix)),
        )
        .select(app.tab_index - start_idx)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .divider(Span::raw("|"));

    f.render_widget(tabs, chunks[0]);

    // Split Content Horizontal for List vs Details
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(chunks[1]);

    if app.grouped_lists.is_empty() {
        return;
    }

    let list_items = &app.grouped_lists[app.tab_index];
    let state = &mut app.grouped_states[app.tab_index];

    // Left pane: Command list
    let items: Vec<ListItem> = list_items
        .iter()
        .map(|(display_name, _, _)| ListItem::new(display_name.clone()))
        .collect();

    let list_border_style = if app.focus == FocusPane::CommandList {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Subcommands ")
                .borders(Borders::ALL)
                .border_style(list_border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, content_chunks[0], state);

    // Right pane: Detail
    let detail_text = match state.selected() {
        Some(i) => {
            if i < list_items.len() {
                let (_, ref full_path, ref cmd) = list_items[i];
                let mut lines = Vec::new();
                lines.push(Line::from(format!("Command: raps {}", full_path)));
                lines.push(Line::from(""));
                lines.push(Line::from(format!("Description: {}", cmd.description)));
                lines.push(Line::from(""));

                if !cmd.params.is_empty() {
                    lines.push(Line::from("Required Parameters:"));
                    for p in cmd.params {
                        lines.push(Line::from(format!("  <{}>", p)));
                    }
                    lines.push(Line::from(""));
                }

                if !cmd.flags.is_empty() {
                    lines.push(Line::from("Flags:"));
                    for flag in cmd.flags {
                        lines.push(Line::from(format!("  {}", flag)));
                    }
                }
                lines
            } else {
                vec![Line::from("Empty")]
            }
        }
        None => vec![Line::from("Select a command from the left pane")],
    };

    let detail_block = Block::default()
        .title(format!(" Details (Auth: {}) ", app.auth_status))
        .borders(Borders::ALL);

    let detail = Paragraph::new(detail_text)
        .block(detail_block)
        .wrap(Wrap { trim: true });

    f.render_widget(detail, content_chunks[1]);

    // Render Execution Panel if active
    let mut current_chunk_idx = 2;

    let has_table = if let Some(serde_json::Value::Array(ref arr)) = app.execution_json_data {
        !arr.is_empty() && arr.iter().all(|i| i.is_object())
    } else {
        false
    };

    let show_exec_panel =
        app.is_executing || has_table || (app.show_debug && !app.execution_log.is_empty());

    if show_exec_panel {
        let log_lines: Vec<ListItem> = app
            .execution_log
            .iter()
            .map(|line| ListItem::new(Line::from(line.as_str())))
            .collect();

        // Auto-scroll logic vs manual scroll offset
        let visible_height = chunks[current_chunk_idx].height.saturating_sub(2) as usize; // account for borders
        let list_len = log_lines.len();

        let max_scroll = list_len.saturating_sub(visible_height);

        // Prevent manual scroll from going out of bounds
        if app.output_scroll as usize > max_scroll {
            app.output_scroll = max_scroll as u16;
        }

        // If scrolling 0, it means we are at the bottom (auto-follow).
        // Since `output_scroll` increments on 'Up', we subtract it from max_scroll.
        let scroll_offset = max_scroll.saturating_sub(app.output_scroll as usize);

        // We use a List without a ListState so it's statelessly rendered,
        // using skip() for naive "auto scrolling" or manual scrolling.
        let visible_items = log_lines
            .into_iter()
            .skip(scroll_offset)
            .collect::<Vec<_>>();

        let exec_border_style = if app.focus == FocusPane::OutputTable {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title_str = format!(
            " {} ",
            if app.is_executing {
                "Execution Output (Running...)"
            } else {
                "Execution Output (Done)"
            }
        );

        // Check if we have structured JSON array data
        let mut rendered_table = false;
        if !app.is_executing {
            if let Some(serde_json::Value::Array(ref arr)) = app.execution_json_data {
                if !arr.is_empty() && arr.iter().all(|i| i.is_object()) {
                    let mut headers = Vec::new();
                    for item in arr.iter().filter_map(|i| i.as_object()) {
                        for key in item.keys() {
                            if !headers.contains(key) {
                                headers.push(key.clone());
                            }
                        }
                    }

                    if !headers.is_empty() {
                        let mut table_rows = Vec::new();
                        for item in arr.iter().filter_map(|i| i.as_object()) {
                            let mut cells = Vec::new();
                            for h in &headers {
                                let val_str = match item.get(h) {
                                    Some(serde_json::Value::String(s)) => s.clone(),
                                    Some(serde_json::Value::Number(n)) => n.to_string(),
                                    Some(serde_json::Value::Bool(b)) => b.to_string(),
                                    Some(serde_json::Value::Null) => "null".to_string(),
                                    Some(v) => format!("{}", v),
                                    None => "".to_string(),
                                };
                                cells.push(Cell::from(val_str));
                            }
                            table_rows.push(Row::new(cells));
                        }

                        let header_row = Row::new(
                            headers
                                .iter()
                                .map(|h| Cell::from(h.as_str()))
                                .collect::<Vec<_>>(),
                        )
                        .style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )
                        .bottom_margin(1);

                        let max_scroll = table_rows
                            .len()
                            .saturating_sub(visible_height.saturating_sub(2));
                        if app.output_scroll as usize > max_scroll {
                            app.output_scroll = max_scroll as u16;
                        }
                        let scroll_offset = max_scroll.saturating_sub(app.output_scroll as usize);

                        let visible_rows = table_rows
                            .into_iter()
                            .skip(scroll_offset)
                            .collect::<Vec<_>>();

                        // Set equal widths for now
                        let constraints: Vec<Constraint> = headers
                            .iter()
                            .map(|_| Constraint::Percentage(100 / headers.len() as u16))
                            .collect();

                        let table = ratatui::widgets::Table::new(visible_rows, constraints)
                            .header(header_row)
                            .block(
                                Block::default()
                                    .title(title_str.clone())
                                    .borders(Borders::ALL)
                                    .border_style(exec_border_style),
                            )
                            .column_spacing(2);

                        f.render_widget(table, chunks[current_chunk_idx]);
                        rendered_table = true;
                    }
                }
            }
        }

        if !rendered_table && app.show_debug {
            let exec_panel = List::new(visible_items).block(
                Block::default()
                    .title(title_str)
                    .borders(Borders::ALL)
                    .border_style(exec_border_style),
            );
            f.render_widget(exec_panel, chunks[current_chunk_idx]);
        }
        current_chunk_idx += 1;
    }

    // Render Status Bar at the bottom
    let status_bar = Paragraph::new(Span::styled(
        " Navigation: [\u{2190}\u{2193}\u{2191}\u{2192}/hjkl], [Tab], Alpha Hotkey | Action: [Enter] Execute, [d] Debug Output, [Esc] Cancel Prompts, [q/Esc] Quit ",
        Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD),
    ));
    f.render_widget(status_bar, chunks[current_chunk_idx]);

    // Render Overlays
    if app.settings_active {
        let area = centered_rect(50, 40, size);
        let settings_text = vec![
            Line::from("Dashboard Settings"),
            Line::from(""),
            Line::from(format!(
                " [{}] Lazy Load Hubs/Projects (faster prompts)",
                if app.settings.lazy_load_data {
                    "x"
                } else {
                    " "
                }
            )),
            Line::from(""),
            Line::from("Press [Space] or [Enter] to toggle."),
            Line::from("Press [Esc] or [s] to close."),
        ];
        let settings_block = Paragraph::new(settings_text)
            .block(
                Block::default()
                    .title(" Settings ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(settings_block, area);
    } else if app.help_active {
        let area = centered_rect(60, 50, size);
        let help_text = vec![
            Line::from("Keyboard Shortcuts"),
            Line::from(""),
            Line::from(" Global:"),
            Line::from("   [q] or [Esc]      Quit dashboard"),
            Line::from("   [s]               Toggle Settings"),
            Line::from("   [d]               Toggle Debug/JSON Output"),
            Line::from("   [?]               Toggle Help"),
            Line::from(""),
            Line::from(" Navigation:"),
            Line::from("   [\u{2190}\u{2192}] or [h][l]   Change Tab"),
            Line::from("   [Tab]             Next Tab"),
            Line::from("   [\u{2191}\u{2193}] or [k][j]   Select Command"),
            Line::from("   [a-z]             Jump to Tab (first letter)"),
            Line::from(""),
            Line::from(" Action:"),
            Line::from("   [Enter]           Execute Command / Submit Prompt"),
            Line::from(""),
            Line::from("Press [Esc] or [?] to close."),
        ];
        let help_block = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));
        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(help_block, area);
    } else if !matches!(app.prompt_phase, PromptPhase::None) {
        match &mut app.prompt_phase {
            PromptPhase::None => {}
            PromptPhase::HubSelection {
                state, hub_list, ..
            } => {
                let area = centered_rect(60, 50, size);
                let items: Vec<ListItem> = hub_list
                    .iter()
                    .map(|(id, name)| ListItem::new(format!("{} ({})", name, id)))
                    .collect();
                let block = Block::default()
                    .title(" Select Hub (Auto-Cached) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let list = List::new(items)
                    .block(block)
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                f.render_widget(ratatui::widgets::Clear, area);
                f.render_stateful_widget(list, area, state);
            }
            PromptPhase::ProjectSelection {
                state,
                project_list,
                ..
            } => {
                let area = centered_rect(60, 50, size);
                let items: Vec<ListItem> = project_list
                    .iter()
                    .map(|(id, name)| ListItem::new(format!("{} ({})", name, id)))
                    .collect();
                let block = Block::default()
                    .title(" Select Project (Auto-Cached) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let list = List::new(items)
                    .block(block)
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                f.render_widget(ratatui::widgets::Clear, area);
                f.render_stateful_widget(list, area, state);
            }
            PromptPhase::Form(form) => {
                if form.fields.is_empty() {
                    // This is an argument-less command (e.g., auth test), it should execute natively
                    // without a form showing up. In theory we could fast-track this earlier, but we'll
                    // gracefully handle it here by showing a subtle prompt or executing directly (handled earlier)
                    let area = centered_rect(60, 20, size);
                    let prompt_block =
                        Paragraph::new("Press Enter to execute command with no arguments.")
                            .block(
                                Block::default()
                                    .title(format!(" Execute: raps {} ", app.prompt_cmd_path))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Yellow)),
                            )
                            .style(Style::default().fg(Color::White).bg(Color::Black));
                    f.render_widget(ratatui::widgets::Clear, area);
                    f.render_widget(prompt_block, area);
                } else {
                    let area = centered_rect(60, 50, size);

                    let mut items = Vec::new();
                    for (i, field) in form.fields.iter().enumerate() {
                        let is_focused = i == form.focused_index;

                        let cursor_span = if is_focused {
                            Span::styled(
                                ">> ",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::raw("   ")
                        };

                        let content = match field {
                            FormField::Positional { name, value }
                            | FormField::FlagValue { name, value } => {
                                let val_disp = if value.is_empty() {
                                    Span::styled("_", Style::default().fg(Color::DarkGray))
                                } else {
                                    Span::styled(value.as_str(), Style::default().fg(Color::Cyan))
                                };

                                Line::from(vec![
                                    cursor_span,
                                    Span::styled(
                                        format!("{}: ", name),
                                        Style::default().add_modifier(Modifier::BOLD),
                                    ),
                                    val_disp,
                                ])
                            }
                            FormField::FlagToggle { name, enabled } => {
                                let checkbox = if *enabled { "[x]" } else { "[ ]" };
                                Line::from(vec![
                                    cursor_span,
                                    Span::styled(
                                        format!("{} ", checkbox),
                                        Style::default().fg(if *enabled {
                                            Color::Green
                                        } else {
                                            Color::DarkGray
                                        }),
                                    ),
                                    Span::raw(name.as_str()),
                                ])
                            }
                        };
                        items.push(ListItem::new(content));
                    }

                    let block = Block::default()
                        .title(format!(" Parameters (raps {}) ", app.prompt_cmd_path))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow));

                    let list = List::new(items)
                        .block(block)
                        .style(Style::default().fg(Color::White).bg(Color::Black));

                    f.render_widget(ratatui::widgets::Clear, area);
                    f.render_widget(list, area);
                }
            }
        }
    }
}

// Helper function to create a centered rectangle for popups
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
