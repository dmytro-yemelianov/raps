// SPDX-License-Identifier: Apache-2.0
// Utility functions for the TUI dashboard

use super::*;

pub(super) fn format_timestamp(epoch_ms: u64) -> String {
    let secs = (epoch_ms / 1000) as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| epoch_ms.to_string())
}

pub(super) fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub(super) fn status_color(status: &str) -> Style {
    let lower = status.to_lowercase();
    if lower.contains("open") || lower.contains("active") {
        Style::default().fg(Color::Green)
    } else if lower.contains("closed") || lower.contains("resolved") {
        Style::default().fg(Color::DarkGray)
    } else if lower.contains("draft") || lower.contains("pending") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    }
}

pub(super) fn da_status_color(status: &str) -> Style {
    let lower = status.to_lowercase();
    if lower.contains("success") {
        Style::default().fg(Color::Green)
    } else if lower.contains("fail") {
        Style::default().fg(Color::Red)
    } else if lower.contains("pending") || lower.contains("inprogress") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    }
}

/// Copy text to the system clipboard using platform-specific tools.
pub(super) fn copy_to_clipboard(text: &str) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[cfg(target_os = "windows")]
    let result = Command::new("clip")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });

    #[cfg(target_os = "macos")]
    let result = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Try wl-copy (Wayland), xclip, xsel in order
        let tools: &[(&str, &[&str])] = &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ];
        for (cmd, args) in tools {
            if let Ok(mut child) = Command::new(cmd)
                .args(*args)
                .stdin(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if child.wait().is_ok() {
                    return true;
                }
            }
        }
        return false;
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    result.is_ok()
}

/// Get the copyable ID from the selected row in the current data.
pub(super) fn selected_id(app: &App) -> Option<String> {
    let sel = app.table_state.selected()?;
    let data = app.data.as_ref()?;
    let filter = app.filter_text.to_lowercase();

    match data {
        ResourceData::Buckets(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.key.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.key.clone())
        }
        ResourceData::Objects(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.key.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.key.clone())
        }
        ResourceData::Hubs(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Projects(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::FolderContents(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Issues(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Rfis(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Assets(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| {
                        r.id.to_lowercase().contains(&filter)
                            || r.description.to_lowercase().contains(&filter)
                    })
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Submittals(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Checklists(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::IssueComments(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.body.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::IssueAttachments(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::IssueTypes(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.title.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Engines(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Activities(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::WorkItems(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::AppBundles(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.id.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Derivatives(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.urn.clone())
        }
        ResourceData::Webhooks(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| {
                        r.event.to_lowercase().contains(&filter)
                            || r.callback_url.to_lowercase().contains(&filter)
                    })
                    .collect()
            };
            filtered.get(sel).map(|r| r.hook_id.clone())
        }
        ResourceData::Photoscenes(rows) => {
            let filtered: Vec<_> = if filter.is_empty() {
                rows.iter().collect()
            } else {
                rows.iter()
                    .filter(|r| r.name.to_lowercase().contains(&filter))
                    .collect()
            };
            filtered.get(sel).map(|r| r.id.clone())
        }
        // Detail views — copy the first field's value (usually the ID)
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
        | ResourceData::PhotosceneDetail(fields) => fields.first().map(|f| f.value.clone()),
    }
}
