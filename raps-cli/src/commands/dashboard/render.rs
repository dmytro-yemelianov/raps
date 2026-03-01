// SPDX-License-Identifier: Apache-2.0
// UI rendering for the TUI dashboard

use super::*;

/// Maximum content width — prevents rendering artifacts on ultra-wide terminals
const MAX_UI_WIDTH: u16 = 220;

pub(super) fn ui(f: &mut Frame, app: &mut App) {
    let full = f.area();

    // Clear the entire frame to prevent artifacts from previous renders
    f.render_widget(Clear, full);

    // Constrain width on very wide terminals; center the content area
    let size = if full.width > MAX_UI_WIDTH {
        let pad = (full.width - MAX_UI_WIDTH) / 2;
        Rect::new(pad, full.y, MAX_UI_WIDTH, full.height)
    } else {
        full
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar (single line)
            Constraint::Length(1), // Breadcrumb
            Constraint::Min(5),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    // Tab bar — render manually to avoid Tabs+Block rendering conflicts
    let mut tab_spans = vec![Span::styled(" ", Style::default())];
    for (i, t) in ResourceTab::ALL.iter().enumerate() {
        if i > 0 {
            tab_spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
        let style = if *t == app.tab {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        tab_spans.push(Span::styled(t.label(), style));
    }
    let tab_line = Paragraph::new(Line::from(tab_spans)).style(Style::default().bg(Color::Black));
    f.render_widget(tab_line, chunks[0]);

    // Breadcrumb
    let breadcrumb_text = app.breadcrumb();
    let breadcrumb = Paragraph::new(Line::from(vec![
        Span::styled(" > ", Style::default().fg(Color::DarkGray)),
        Span::styled(breadcrumb_text, Style::default().fg(Color::Cyan)),
    ]));
    f.render_widget(breadcrumb, chunks[1]);

    // Main content — single outer block, content rendered inside
    render_main_content(f, &mut *app, chunks[2]);

    // Status bar
    render_status_bar(f, app, chunks[3]);

    // Error/help overlay
    if let Some(ref msg) = app.error_msg {
        render_overlay(f, msg, size);
    }
}

fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.has_current_view() {
        app.current_view().title()
    } else {
        "No context — use :urn <base64>".into()
    };
    // Single outer block for the content area — prevents double-border artifacts
    let outer_block = Block::default().borders(Borders::ALL).title(title);
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    if !app.has_current_view() {
        let hint = Paragraph::new(" Press :urn <base64-encoded-urn> to view a manifest")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint, inner);
        return;
    }

    if app.data.is_none() {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame = SPINNER[(app.tick as usize / 2) % SPINNER.len()];
        let loading = Paragraph::new(format!(" {frame} Loading..."))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(loading, inner);
        return;
    }

    let Some(data) = app.data.as_ref() else {
        return;
    };
    let filter = app.filter_text.to_lowercase();

    match data {
        ResourceData::Buckets(rows) => {
            let filtered: Vec<&BucketRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.key.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Key", "Policy", "Created"],
                &[
                    Constraint::Percentage(50),
                    Constraint::Percentage(20),
                    Constraint::Percentage(30),
                ],
                filtered
                    .iter()
                    .map(|r| vec![r.key.as_str(), r.policy.as_str(), r.created.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Objects(rows) => {
            let filtered: Vec<&ObjectRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.key.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "Size", "SHA1"],
                &[
                    Constraint::Percentage(50),
                    Constraint::Percentage(20),
                    Constraint::Percentage(30),
                ],
                filtered
                    .iter()
                    .map(|r| vec![r.key.as_str(), r.size.as_str(), r.sha1.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Hubs(rows) => {
            let filtered: Vec<&HubRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "ID", "Region"],
                &[
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| vec![r.name.as_str(), r.id.as_str(), r.region.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Projects(rows) => {
            let filtered: Vec<&ProjectRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "ID"],
                &[Constraint::Percentage(50), Constraint::Percentage(50)],
                filtered
                    .iter()
                    .map(|r| vec![r.name.as_str(), r.id.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::FolderContents(rows) => {
            let filtered: Vec<&FolderContentRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    let type_style = if r.content_type == "folder" {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let icon = if r.content_type == "folder" { "/" } else { " " };
                    Row::new(vec![
                        Cell::from(Span::styled(format!("{icon}{}", r.name), type_style)),
                        Cell::from(r.content_type.as_str()),
                        Cell::from(r.modified.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(50),
                    Constraint::Percentage(15),
                    Constraint::Percentage(35),
                ],
            )
            .header(
                Row::new(vec!["Name", "Type", "Modified"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::Issues(rows) => {
            let filtered: Vec<&IssueRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.title.as_str()),
                        Cell::from(Span::styled(
                            r.status.as_str(),
                            util::status_color(&r.status),
                        )),
                        Cell::from(r.assigned_to.as_str()),
                        Cell::from(r.created_at.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(40),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(25),
                ],
            )
            .header(
                Row::new(vec!["Title", "Status", "Assigned", "Created"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::Rfis(rows) => {
            let filtered: Vec<&RfiRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.title.as_str()),
                        Cell::from(Span::styled(
                            r.status.as_str(),
                            util::status_color(&r.status),
                        )),
                        Cell::from(r.priority.as_str()),
                        Cell::from(r.created_at.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(40),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(30),
                ],
            )
            .header(
                Row::new(vec!["Title", "Status", "Priority", "Created"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::Assets(rows) => {
            let filtered: Vec<&AssetRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| {
                        r.id.to_lowercase().contains(&filter)
                            || r.description.to_lowercase().contains(&filter)
                    })
                    .collect()
            };
            render_table(
                f,
                inner,
                &["ID", "ClientAssetId", "Description", "Status"],
                &[
                    Constraint::Percentage(25),
                    Constraint::Percentage(20),
                    Constraint::Percentage(35),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| {
                        vec![
                            r.id.as_str(),
                            r.client_asset_id.as_str(),
                            r.description.as_str(),
                            r.status.as_str(),
                        ]
                    })
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Submittals(rows) => {
            let filtered: Vec<&SubmittalRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.title.as_str()),
                        Cell::from(r.number.as_str()),
                        Cell::from(Span::styled(
                            r.status.as_str(),
                            util::status_color(&r.status),
                        )),
                        Cell::from(r.due_date.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(35),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(30),
                ],
            )
            .header(
                Row::new(vec!["Title", "Number", "Status", "Due Date"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::Checklists(rows) => {
            let filtered: Vec<&ChecklistRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.title.as_str()),
                        Cell::from(Span::styled(
                            r.status.as_str(),
                            util::status_color(&r.status),
                        )),
                        Cell::from(r.location.as_str()),
                        Cell::from(r.due_date.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(35),
                    Constraint::Percentage(15),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ],
            )
            .header(
                Row::new(vec!["Title", "Status", "Location", "Due Date"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::IssueComments(rows) => {
            let filtered: Vec<&IssueCommentRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.body.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Body", "Created By", "Created At"],
                &[
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ],
                filtered
                    .iter()
                    .map(|r| {
                        vec![
                            r.body.as_str(),
                            r.created_by.as_str(),
                            r.created_at.as_str(),
                        ]
                    })
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::IssueAttachments(rows) => {
            let filtered: Vec<&IssueAttachmentRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "URN", "ID"],
                &[
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| vec![r.name.as_str(), r.urn.as_str(), r.id.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::IssueTypes(rows) => {
            let filtered: Vec<&IssueTypeRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Title", "Active", "ID"],
                &[
                    Constraint::Percentage(50),
                    Constraint::Percentage(15),
                    Constraint::Percentage(35),
                ],
                filtered
                    .iter()
                    .map(|r| vec![r.title.as_str(), r.is_active.as_str(), r.id.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Engines(rows) => {
            let filtered: Vec<&EngineRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["ID", "Description"],
                &[Constraint::Percentage(60), Constraint::Percentage(40)],
                filtered
                    .iter()
                    .map(|r| vec![r.id.as_str(), r.description.as_str()])
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Activities(rows) => {
            let filtered: Vec<&ActivityRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["ID"],
                &[Constraint::Percentage(100)],
                filtered.iter().map(|r| vec![r.id.as_str()]).collect(),
                &mut app.table_state,
            );
        }
        ResourceData::WorkItems(rows) => {
            let filtered: Vec<&WorkItemRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            let table_rows: Vec<Row> = filtered
                .iter()
                .map(|r| {
                    Row::new(vec![
                        Cell::from(r.id.as_str()),
                        Cell::from(Span::styled(
                            r.status.as_str(),
                            util::da_status_color(&r.status),
                        )),
                        Cell::from(r.progress.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ],
            )
            .header(
                Row::new(vec!["ID", "Status", "Progress"])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .bottom_margin(1),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
        ResourceData::AppBundles(rows) => {
            let filtered: Vec<&AppBundleRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["ID"],
                &[Constraint::Percentage(100)],
                filtered.iter().map(|r| vec![r.id.as_str()]).collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Derivatives(rows) => {
            let filtered: Vec<&DerivativeRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "OutputType", "Role", "MIME", "Size"],
                &[
                    Constraint::Percentage(30),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| {
                        vec![
                            r.name.as_str(),
                            r.output_type.as_str(),
                            r.role.as_str(),
                            r.mime.as_str(),
                            r.size.as_str(),
                        ]
                    })
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Webhooks(rows) => {
            let filtered: Vec<&WebhookRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| {
                        r.event.to_lowercase().contains(&filter)
                            || r.callback_url.to_lowercase().contains(&filter)
                    })
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Event", "Callback URL", "Status", "System", "Created"],
                &[
                    Constraint::Percentage(20),
                    Constraint::Percentage(30),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| {
                        vec![
                            r.event.as_str(),
                            r.callback_url.as_str(),
                            r.status.as_str(),
                            r.system.as_str(),
                            r.created.as_str(),
                        ]
                    })
                    .collect(),
                &mut app.table_state,
            );
        }
        ResourceData::Photoscenes(rows) => {
            let filtered: Vec<&PhotosceneRow> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            render_table(
                f,
                inner,
                &["Name", "ID", "Type", "Format", "Status"],
                &[
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                ],
                filtered
                    .iter()
                    .map(|r| {
                        vec![
                            r.name.as_str(),
                            r.id.as_str(),
                            r.scene_type.as_str(),
                            r.format.as_str(),
                            r.status.as_str(),
                        ]
                    })
                    .collect(),
                &mut app.table_state,
            );
        }
        // Detail views (key-value)
        ResourceData::BucketDetail(fields)
        | ResourceData::ObjectDetail(fields)
        | ResourceData::ItemDetail(fields)
        | ResourceData::IssueDetail(fields)
        | ResourceData::RfiDetail(fields)
        | ResourceData::AssetDetail(fields)
        | ResourceData::SubmittalDetail(fields)
        | ResourceData::ChecklistDetail(fields)
        | ResourceData::WorkItemDetail(fields)
        | ResourceData::Manifest(fields)
        | ResourceData::DerivativeDetail(fields)
        | ResourceData::WebhookDetail(fields)
        | ResourceData::PhotosceneDetail(fields)
        | ResourceData::SwarmStatus(fields) => {
            let table_rows: Vec<Row> = fields
                .iter()
                .map(|field| {
                    Row::new(vec![
                        Cell::from(Span::styled(
                            field.label.as_str(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Cell::from(field.value.as_str()),
                    ])
                })
                .collect();
            let table = Table::new(
                table_rows,
                [Constraint::Percentage(25), Constraint::Percentage(75)],
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
            f.render_stateful_widget(table, inner, &mut app.table_state);
        }
    }
}

/// Generic table renderer for simple string-cell tables
fn render_table(
    f: &mut Frame,
    area: Rect,
    headers: &[&str],
    widths: &[Constraint],
    rows_data: Vec<Vec<&str>>,
    table_state: &mut TableState,
) {
    let table_rows: Vec<Row> = rows_data
        .iter()
        .map(|cells| Row::new(cells.iter().map(|c| Cell::from(*c)).collect::<Vec<_>>()))
        .collect();
    let header = Row::new(headers.iter().map(|h| Cell::from(*h)).collect::<Vec<_>>())
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);
    let table = Table::new(table_rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(table, area, table_state);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![];

    match &app.input_mode {
        InputMode::Filter(text) => {
            spans.push(Span::styled(
                format!(" /{text}"),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(
                "_",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
        InputMode::Command(text) => {
            spans.push(Span::styled(
                format!(" :{text}"),
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::styled(
                "_",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
        InputMode::Confirm(msg) => {
            spans.push(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
        InputMode::Normal => {
            if !app.filter_text.is_empty() {
                spans.push(Span::styled(
                    format!(" [filter: {}]", app.filter_text),
                    Style::default().fg(Color::Yellow),
                ));
            }
            // Show status message while fresh, otherwise show shortcut hints
            if !app.status_msg.is_empty() && app.status_is_active() {
                spans.push(Span::styled(
                    format!(" {}", app.status_msg),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" {}", shortcut_hints(app)),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
    }

    let right_parts = {
        let mut parts = vec![];
        // Auth state
        if app.logged_in {
            parts.push("3leg".to_string());
        } else {
            parts.push("2leg".to_string());
        }
        // Context
        if let Some(hub) = &app.hub_context {
            let short = if hub.len() > 12 { &hub[..12] } else { hub };
            parts.push(format!("hub:{short}"));
        }
        if let Some((_, name)) = &app.project_context {
            let short = if name.len() > 16 { &name[..16] } else { name };
            parts.push(format!("proj:{short}"));
        }
        parts.push(format!("api:{}", app.api_calls));
        if let Some(t) = app.last_refresh {
            let elapsed = t.elapsed().as_secs();
            parts.push(format!("{elapsed}s ago"));
        }
        parts.join(" | ")
    };

    if !right_parts.is_empty() {
        let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_len = right_parts.len();
        let total = area.width as usize;
        if left_len + right_len < total {
            let padding = total - left_len - right_len;
            spans.push(Span::raw(" ".repeat(padding.saturating_sub(1))));
        }
        spans.push(Span::styled(
            right_parts,
            Style::default().fg(Color::DarkGray),
        ));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black));
    f.render_widget(bar, area);
}

fn render_overlay(f: &mut Frame, msg: &str, area: Rect) {
    if area.width < 10 || area.height < 6 {
        return; // terminal too small for overlay
    }
    let width = (area.width * 80 / 100).clamp(40, 120).min(area.width);
    // Wrap long lines manually to estimate real line count
    let inner_w = width.saturating_sub(2).max(1) as usize; // account for borders
    let mut wrapped_count: u16 = 0;
    for line in msg.lines() {
        if line.is_empty() {
            wrapped_count += 1;
        } else {
            wrapped_count += (line.len() as u16).max(1).div_ceil(inner_w as u16);
        }
    }
    let height = (wrapped_count + 4).min(area.height.saturating_sub(2));
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let overlay_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, overlay_area);

    let text: Vec<Line> = msg.lines().map(|l| Line::from(Span::raw(l))).collect();
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Info (Esc to close) ")
                .style(Style::default().bg(Color::Black).fg(Color::White)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, overlay_area);
}

/// Get context-sensitive shortcut hints for the status bar.
fn shortcut_hints(app: &App) -> String {
    if !app.has_current_view() {
        return ":urn <b64>  ?:Help  q:Quit".into();
    }

    let view = app.current_view();

    // Detail views
    if matches!(
        view,
        ViewKind::BucketDetail { .. }
            | ViewKind::ObjectDetail { .. }
            | ViewKind::ItemDetail { .. }
            | ViewKind::IssueDetail { .. }
            | ViewKind::RfiDetail { .. }
            | ViewKind::AssetDetail { .. }
            | ViewKind::SubmittalDetail { .. }
            | ViewKind::ChecklistDetail { .. }
            | ViewKind::WorkItemDetail { .. }
            | ViewKind::ManifestView { .. }
            | ViewKind::DerivativeDetail { .. }
            | ViewKind::WebhookDetail { .. }
            | ViewKind::PhotosceneDetail { .. }
            | ViewKind::SwarmOverview
    ) {
        return "Esc:Back  y:Copy  r:Refresh  Enter:Drill  ?:Help".into();
    }

    // Tab-specific list hints
    let tab_hint = match app.tab {
        ResourceTab::Issues => {
            if matches!(
                view,
                ViewKind::IssueList { .. }
                    | ViewKind::RfiList { .. }
                    | ViewKind::AssetList { .. }
                    | ViewKind::SubmittalList { .. }
                    | ViewKind::ChecklistList { .. }
            ) {
                "i:Issues f:RFIs t:Assets s:Submittals c:Checklists  "
            } else {
                ""
            }
        }
        ResourceTab::DesignAutomation => "e:Eng a:Act w:WI b:Bundles  ",
        _ => "",
    };

    format!("{tab_hint}Enter:Open  Esc:Back  /:Filter  y:Copy  ?:Help")
}
