// SPDX-License-Identifier: Apache-2.0
// Key handling for the TUI dashboard

use super::*;
use crate::commands::dashboard::traits::DashboardResource;

pub(super) fn handle_key(
    app: &mut App,
    key: KeyEvent,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return;
    }

    match &app.input_mode {
        InputMode::Filter(text) => {
            let text = text.clone();
            match key.code {
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                    app.filter_text.clear();
                    app.table_state.select(Some(0));
                }
                KeyCode::Enter => {
                    app.filter_text = text;
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    let mut t = text;
                    t.pop();
                    app.filter_text = t.clone();
                    app.input_mode = InputMode::Filter(t);
                    app.table_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    let mut t = text;
                    t.push(c);
                    app.filter_text = t.clone();
                    app.input_mode = InputMode::Filter(t);
                    app.table_state.select(Some(0));
                }
                _ => {}
            }
            return;
        }
        InputMode::Command(text) => {
            let text = text.clone();
            match key.code {
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    execute_command(app, &text, clients, tx);
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    let mut t = text;
                    t.pop();
                    app.input_mode = InputMode::Command(t);
                }
                KeyCode::Char(c) => {
                    let mut t = text;
                    t.push(c);
                    app.input_mode = InputMode::Command(t);
                }
                _ => {}
            }
            return;
        }
        InputMode::Confirm(_) => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.set_status("Confirmed (delete not yet implemented)");
                    app.input_mode = InputMode::Normal;
                }
                _ => {
                    app.set_status("Cancelled");
                    app.input_mode = InputMode::Normal;
                }
            }
            return;
        }
        InputMode::MarkSet => {
            if let KeyCode::Char(c) = key.code {
                let mark = Mark {
                    tab: app.tab,
                    stack: app.nav_stacks[app.tab.index()].clone(),
                    project_context: app.project_context.clone(),
                    hub_context: app.hub_context.clone(),
                };
                app.marks.insert(c, mark);
                app.set_status(format!("Mark '{c}' set"));
            }
            app.input_mode = InputMode::Normal;
            return;
        }
        InputMode::JumpToMark => {
            if let KeyCode::Char(c) = key.code {
                if let Some(mark) = app.marks.get(&c).cloned() {
                    app.tab = mark.tab;
                    app.nav_stacks[mark.tab.index()] = mark.stack;
                    app.project_context = mark.project_context;
                    app.hub_context = mark.hub_context;
                    app.set_status(format!("Jumped to mark '{c}'"));
                    app.data = None;
                    app.table_state.select(Some(0));
                    fetch::load_view(app, clients, tx, false);
                } else {
                    app.set_status(format!("Mark '{c}' not found"));
                }
            }
            app.input_mode = InputMode::Normal;
            return;
        }
        InputMode::Normal => {
}
    }

    // Guard: tabs with empty nav stacks (e.g. F5 before :urn) only allow
    // tab switching, command mode, help, and quit.
    if !app.has_current_view() {
        match key.code {
            KeyCode::Char('q') => {
                app.quit = true;
            }
            KeyCode::Char('?') => {
                show_help(app);
            }
            KeyCode::Char(':') => {
                app.input_mode = InputMode::Command(String::new());
            }
            KeyCode::F(n @ 1..=9) => {
                let tab = ResourceTab::ALL[(n as usize) - 1];
                switch_tab(app, tab, clients, tx);
            }
            KeyCode::Tab => {
                cycle_tab(app, true, clients, tx);
            }
            KeyCode::BackTab => {
                cycle_tab(app, false, clients, tx);
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.quit = true;
        }
        KeyCode::Char('?') => {
            show_help(app);
        }
        KeyCode::Esc => {
            if app.error_msg.is_some() {
                app.error_msg = None;
            } else {
                let went_back = app.pop_view();
                if went_back && app.data.is_none() {
                    fetch::load_view(app, clients, tx, false);
                }
            }
        }
        KeyCode::F(1) => switch_tab(app, ResourceTab::Buckets, clients, tx),
        KeyCode::F(2) => switch_tab(app, ResourceTab::DataManagement, clients, tx),
        KeyCode::F(3) => switch_tab(app, ResourceTab::Issues, clients, tx),
        KeyCode::F(4) => switch_tab(app, ResourceTab::DesignAutomation, clients, tx),
        KeyCode::F(5) => switch_tab(app, ResourceTab::ModelDerivative, clients, tx),
        KeyCode::F(6) => switch_tab(app, ResourceTab::Webhooks, clients, tx),
        KeyCode::F(7) => switch_tab(app, ResourceTab::RealityCapture, clients, tx),
        KeyCode::F(8) => switch_tab(app, ResourceTab::Swarm, clients, tx),
        KeyCode::F(9) => switch_tab(app, ResourceTab::Logs, clients, tx),
        KeyCode::Tab => {
            cycle_tab(app, true, clients, tx);
        }
        KeyCode::BackTab => {
            cycle_tab(app, false, clients, tx);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let count = app.row_count();
            if count > 0 {
                let i = app.table_state.selected().unwrap_or(0);
                let next = if i >= count - 1 { 0 } else { i + 1 };
                app.table_state.select(Some(next));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let count = app.row_count();
            if count > 0 {
                let i = app.table_state.selected().unwrap_or(0);
                let prev = if i == 0 { count - 1 } else { i - 1 };
                app.table_state.select(Some(prev));
            }
        }
        KeyCode::Char('g') => {
            app.table_state.select(Some(0));
        }
        KeyCode::Char('G') => {
            let count = app.row_count();
            if count > 0 {
                app.table_state.select(Some(count - 1));
            }
        }
        KeyCode::Enter => {
            handle_enter(app, clients, tx);
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Filter(String::new());
        }
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command(String::new());
        }
        KeyCode::Char('m') => {
            app.input_mode = InputMode::MarkSet;
        }
        KeyCode::Char('\'') => {
            app.input_mode = InputMode::JumpToMark;
        }
        KeyCode::Char(' ') => {
            if let Some(id) = util::selected_id(app) {
                if app.selected_ids.contains(&id) {
                    app.selected_ids.remove(&id);
                } else {
                    app.selected_ids.insert(id);
                }
            }
        }

        KeyCode::Char('r') => {
            fetch::load_view(app, clients, tx, true);
        }
        KeyCode::Char('y') => {
            if let Some(id) = util::selected_id(app) {
                if util::copy_to_clipboard(&id) {
                    app.set_status(format!("Copied: {id}"));
                } else {
                    app.set_status(format!("ID: {id} (clipboard unavailable)"));
                }
            }
        }
        KeyCode::PageDown => {
            let count = app.row_count();
            if count > 0 {
                let i = app.table_state.selected().unwrap_or(0);
                let next = (i + 20).min(count - 1);
                app.table_state.select(Some(next));
            }
        }
        KeyCode::PageUp => {
            let count = app.row_count();
            if count > 0 {
                let i = app.table_state.selected().unwrap_or(0);
                let prev = i.saturating_sub(20);
                app.table_state.select(Some(prev));
            }
        }
        // F3 ACC tab sub-views
        KeyCode::Char('i') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if stack.last().is_some_and(|v| {
                    matches!(
                        v,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    )
                }) {
                    stack.pop();
                }
                stack.push(ViewKind::IssueList { project_id: pid });
                app.table_state.select(Some(0));
                app.data = None;
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        KeyCode::Char('f') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if stack.last().is_some_and(|v| {
                    matches!(
                        v,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    )
                }) {
                    stack.pop();
                }
                stack.push(ViewKind::RfiList { project_id: pid });
                app.table_state.select(Some(0));
                app.data = None;
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        KeyCode::Char('t') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if stack.last().is_some_and(|v| {
                    matches!(
                        v,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    )
                }) {
                    stack.pop();
                }
                stack.push(ViewKind::AssetList { project_id: pid });
                app.table_state.select(Some(0));
                app.data = None;
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        KeyCode::Char('s') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if stack.last().is_some_and(|v| {
                    matches!(
                        v,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    )
                }) {
                    stack.pop();
                }
                stack.push(ViewKind::SubmittalList { project_id: pid });
                app.table_state.select(Some(0));
                app.data = None;
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        KeyCode::Char('c') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if stack.last().is_some_and(|v| {
                    matches!(
                        v,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    )
                }) {
                    stack.pop();
                }
                stack.push(ViewKind::ChecklistList { project_id: pid });
                app.table_state.select(Some(0));
                app.data = None;
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        // F3 ACC: 'a' on IssueDetail → attachments
        KeyCode::Char('a') if app.tab == ResourceTab::Issues => {
            if let ViewKind::IssueDetail {
                project_id,
                issue_id,
            } = app.current_view().clone()
            {
                app.push_view(ViewKind::IssueAttachmentList {
                    project_id,
                    issue_id,
                });
                fetch::load_view(app, clients, tx, false);
            }
        }
        // F3 ACC: 'T' on project context → issue types
        KeyCode::Char('T') if app.tab == ResourceTab::Issues => {
            if let Some((pid, _)) = &app.project_context {
                let pid = pid.clone();
                app.push_view(ViewKind::IssueTypeList { project_id: pid });
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Select a project first (Enter on a project)");
            }
        }
        // F4 DA tab sub-views
        KeyCode::Char('e') if app.tab == ResourceTab::DesignAutomation => {
            app.nav_stacks[ResourceTab::DesignAutomation.index()] = vec![ViewKind::EngineList];
            app.table_state.select(Some(0));
            app.data = None;
            fetch::load_view(app, clients, tx, false);
        }
        KeyCode::Char('a') if app.tab == ResourceTab::DesignAutomation => {
            app.nav_stacks[ResourceTab::DesignAutomation.index()] = vec![ViewKind::ActivityList];
            app.table_state.select(Some(0));
            app.data = None;
            fetch::load_view(app, clients, tx, false);
        }
        KeyCode::Char('w') if app.tab == ResourceTab::DesignAutomation => {
            app.nav_stacks[ResourceTab::DesignAutomation.index()] = vec![ViewKind::WorkItemList];
            app.table_state.select(Some(0));
            app.data = None;
            fetch::load_view(app, clients, tx, false);
        }
        KeyCode::Char('b') if app.tab == ResourceTab::DesignAutomation => {
            app.nav_stacks[ResourceTab::DesignAutomation.index()] = vec![ViewKind::AppBundleList];
            app.table_state.select(Some(0));
            app.data = None;
            fetch::load_view(app, clients, tx, false);
        }
        _ => {}
    }
}

fn show_help(app: &mut App) {
    let auth_hint = if app.logged_in {
        "3-legged (logged in)"
    } else {
        "2-legged only — run 'raps auth login' for full access"
    };
    app.error_msg = Some(format!(
        "Tabs\n\
         F1 Buckets   F2 Data Mgmt   F3 ACC   F4 Design Auto\n\
         F5 Derivative   F6 Webhooks   F7 Reality   F8 Swarm\n\
         Tab/Shift-Tab  Cycle tabs\n\n\
         Navigation\n\
         j/k        Up / Down          PgUp/PgDn  Page scroll\n\
         g/G        Top / Bottom       Enter      Drill in\n\
         Esc        Go back            y          Copy ID\n\n\
         Tab-Specific\n\
         F3: Hub > Project then  i Issues  f RFIs  t Assets  s Submittals  c Checklists\n\
         F4: e Engines  a Activities  w WorkItems  b AppBundles\n\
         F5: :urn <base64>  Set URN context\n\n\
         Actions\n\
         /  Filter     r  Refresh    :  Command mode    q  Quit\n\n\
         Commands\n\
         :q  Quit         :r  Refresh       :l  Logs\n\
         :p <id>   Set project context\n\
         :urn <b64>  Set Model Derivative URN\n\
         :login    3-legged OAuth       :logout  End session\n\n\
         Auth: {auth_hint}"
    ));
}

fn cycle_tab(
    app: &mut App,
    forward: bool,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
) {
    let tabs = &ResourceTab::ALL;
    let cur = app.tab.index();
    let next_idx = if forward {
        (cur + 1) % tabs.len()
    } else {
        (cur + tabs.len() - 1) % tabs.len()
    };
    switch_tab(app, tabs[next_idx], clients, tx);
}

fn switch_tab(
    app: &mut App,
    tab: ResourceTab,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
) {
    if app.tab != tab {
        // Save cursor position for current view before switching
        app.save_selection_to_cache();
        app.tab = tab;
        app.filter_text.clear();
        app.error_msg = None;
        if app.has_current_view() {
            // Try to restore from cache
            fetch::load_view(app, clients, tx, false);
        } else {
            // Empty stack (e.g. F5 before :urn)
            app.data = None;
            app.set_status("Use :urn <base64> to set context");
        }
    }
}

fn handle_enter(app: &mut App, clients: &Arc<Clients>, tx: &mpsc::UnboundedSender<BackgroundMsg>) {
    let selected = match app.table_state.selected() {
        Some(i) => i,
        None => return,
    };

    let filter = app.filter_text.to_lowercase();

    // 1. Check if it's a trait-based resource
    let mut trait_res: Option<Box<dyn DashboardResource>> = None;
    if let Some(data) = &app.data {
        match data {
            ResourceData::Buckets(b) => trait_res = Some(Box::new(b.clone())),
            ResourceData::Objects(o) => trait_res = Some(Box::new(o.clone())),
            ResourceData::Hubs(h) => trait_res = Some(Box::new(h.clone())),
            ResourceData::Projects(p) => trait_res = Some(Box::new(p.clone())),
            ResourceData::FolderContents(f) => trait_res = Some(Box::new(f.clone())),
            ResourceData::Issues(i) => trait_res = Some(Box::new(i.clone())),
            ResourceData::Rfis(r) => trait_res = Some(Box::new(r.clone())),
            ResourceData::Assets(a) => trait_res = Some(Box::new(a.clone())),
            ResourceData::Submittals(s) => trait_res = Some(Box::new(s.clone())),
            ResourceData::Checklists(c) => trait_res = Some(Box::new(c.clone())),
            ResourceData::Engines(e) => trait_res = Some(Box::new(e.clone())),
            ResourceData::Activities(a) => trait_res = Some(Box::new(a.clone())),
            ResourceData::WorkItems(w) => trait_res = Some(Box::new(w.clone())),
            ResourceData::AppBundles(b) => trait_res = Some(Box::new(b.clone())),
            ResourceData::Derivatives(d) => trait_res = Some(Box::new(d.clone())),
            ResourceData::Webhooks(w) => trait_res = Some(Box::new(w.clone())),
            ResourceData::Photoscenes(p) => trait_res = Some(Box::new(p.clone())),
            _ => {}
        }
    }

    if let Some(res) = trait_res {
        res.handle_enter(selected, &filter, app, clients, tx);
        return;
    }

    // 2. Handle non-trait-based resources (mostly detail views)
    let data = match &app.data {
        Some(d) => d,
        None => return,
    };

    match data {
        ResourceData::BucketDetail(_) => {
            if let ViewKind::BucketDetail { bucket_key } = app.current_view().clone() {
                app.push_view(ViewKind::ObjectList { bucket_key });
                fetch::load_view(app, clients, tx, false);
            }
        }
        ResourceData::IssueDetail(_) => {
            if let ViewKind::IssueDetail { project_id, issue_id } = app.current_view().clone() {
                app.push_view(ViewKind::IssueCommentList { project_id, issue_id });
                fetch::load_view(app, clients, tx, false);
            }
        }
        ResourceData::Manifest(_) => {
            if let ViewKind::ManifestView { urn } = app.current_view().clone() {
                app.push_view(ViewKind::DerivativeList { urn });
                fetch::load_view(app, clients, tx, false);
            }
        }
        _ => {}
    }
}

fn execute_command(
    app: &mut App,
    cmd: &str,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
) {
    let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
    match parts.first().copied() {
        Some("q" | "quit") => app.quit = true,
        Some("r" | "refresh") => {
            if app.has_current_view() {
                fetch::load_view(app, clients, tx, true);
            }
        }
        Some("p" | "project") => {
            if let Some(id) = parts.get(1) {
                let id = id.trim().to_string();
                // Set name to (id) to show it's unresolved
                app.project_context = Some((id.clone(), format!("({id})")));
                // Push IssueList onto Issues tab stack (preserving any hub navigation)
                let stack = &mut app.nav_stacks[ResourceTab::Issues.index()];
                if let Some(last) = stack.last() {
                    if matches!(
                        last,
                        ViewKind::IssueList { .. }
                            | ViewKind::RfiList { .. }
                            | ViewKind::AssetList { .. }
                            | ViewKind::SubmittalList { .. }
                            | ViewKind::ChecklistList { .. }
                    ) {
                        stack.pop();
                    }
                }
                stack.push(ViewKind::IssueList {
                    project_id: id.clone(),
                });
                app.set_status(format!("Project context: {id}"));
                app.push_log(format!("Project set: {id}"));
                if app.tab == ResourceTab::Issues {
                    app.data = None;
                    fetch::load_view(app, clients, tx, false);
                }
            } else {
                app.set_status("Usage: :p <project_id>");
            }
        }
        Some("urn") => {
            if let Some(urn) = parts.get(1) {
                let urn = urn.trim().to_string();
                app.nav_stacks[ResourceTab::ModelDerivative.index()] =
                    vec![ViewKind::ManifestView { urn: urn.clone() }];
                app.set_status(format!(
                    "URN set: {}",
                    if urn.len() > 30 { &urn[..30] } else { &urn }
                ));
                app.push_log(format!("Model Derivative URN: {urn}"));
                // Switch to F5 and load
                app.tab = ResourceTab::ModelDerivative;
                app.filter_text.clear();
                app.error_msg = None;
                app.data = None;
                app.table_state.select(Some(0));
                fetch::load_view(app, clients, tx, false);
            } else {
                app.set_status("Usage: :urn <base64-encoded-urn>");
            }
        }
        Some("l" | "log" | "logs") => {
            if app.logs.is_empty() {
                app.error_msg = Some("No log messages yet.".into());
            } else {
                let recent: Vec<&str> =
                    app.logs.iter().rev().take(20).map(|s| s.as_str()).collect();
                app.error_msg = Some(recent.into_iter().rev().collect::<Vec<_>>().join("\n"));
            }
        }
        Some("login") => {
            if app.logged_in {
                app.set_status("Already logged in (3-legged auth)");
            } else {
                app.pending_action = Some(PendingAction::Login);
            }
        }
        Some("logout") => {
            if !app.logged_in {
                app.set_status("Not logged in");
            } else {
                app.pending_action = Some(PendingAction::Logout);
            }
        }
        _ => {
            app.set_status(format!(
                "Unknown: :{cmd}  (try :q :r :p :l :urn :login :logout)"
            ));
        }
    }
}
