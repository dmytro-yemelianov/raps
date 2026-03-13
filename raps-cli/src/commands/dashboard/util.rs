// SPDX-License-Identifier: Apache-2.0
// Utility functions for the TUI dashboard

use super::*;
use crate::commands::dashboard::traits::DashboardResource;

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
        false
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
        ResourceData::Buckets(b) => b.get_id(sel, &filter),
        ResourceData::Objects(o) => o.get_id(sel, &filter),
        ResourceData::Hubs(h) => h.get_id(sel, &filter),
        ResourceData::Projects(p) => p.get_id(sel, &filter),
        ResourceData::FolderContents(f) => f.get_id(sel, &filter),
        ResourceData::Issues(i) => i.get_id(sel, &filter),
        ResourceData::Rfis(r) => r.get_id(sel, &filter),
        ResourceData::Assets(a) => a.get_id(sel, &filter),
        ResourceData::Submittals(s) => s.get_id(sel, &filter),
        ResourceData::Checklists(c) => c.get_id(sel, &filter),
        ResourceData::IssueComments(rows) => {
            let filtered: Vec<_> = rows
                .iter()
                .filter(|r| filter.is_empty() || r.body.to_lowercase().contains(&filter))
                .collect();
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::IssueAttachments(rows) => {
            let filtered: Vec<_> = rows
                .iter()
                .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
                .collect();
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::IssueTypes(rows) => {
            let filtered: Vec<_> = rows
                .iter()
                .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
                .collect();
            filtered.get(sel).map(|r| r.id.clone())
        }
        ResourceData::Engines(e) => e.get_id(sel, &filter),
        ResourceData::Activities(a) => a.get_id(sel, &filter),
        ResourceData::WorkItems(w) => w.get_id(sel, &filter),
        ResourceData::AppBundles(b) => b.get_id(sel, &filter),
        ResourceData::Derivatives(d) => d.get_id(sel, &filter),
        ResourceData::Webhooks(w) => w.get_id(sel, &filter),
        ResourceData::Photoscenes(p) => p.get_id(sel, &filter),
        ResourceData::Logs(l) => l.get_id(sel, &filter),
        // Detail views — copy the value of the selected field
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
            let filtered: Vec<_> = fields
                .iter()
                .filter(|f| {
                    filter.is_empty()
                        || f.label.to_lowercase().contains(&filter)
                        || f.value.to_lowercase().contains(&filter)
                })
                .collect();
            filtered.get(sel).map(|f| f.value.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp_valid() {
        // 2024-01-01 00:00:00 UTC = 1704067200 seconds = 1704067200000 ms
        let result = format_timestamp(1_704_067_200_000);
        assert_eq!(result, "2024-01-01 00:00");
    }

    #[test]
    fn test_format_timestamp_zero() {
        let result = format_timestamp(0);
        assert_eq!(result, "1970-01-01 00:00");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_status_color_open() {
        let style = status_color("open");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_status_color_closed() {
        let style = status_color("closed");
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_status_color_draft() {
        let style = status_color("draft");
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_status_color_unknown() {
        let style = status_color("something");
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn test_da_status_color_success() {
        let style = da_status_color("success");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_da_status_color_failed() {
        let style = da_status_color("failed");
        assert_eq!(style.fg, Some(Color::Red));
    }

    #[test]
    fn test_da_status_color_pending() {
        let style = da_status_color("pending");
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_da_status_color_unknown() {
        let style = da_status_color("something");
        assert_eq!(style.fg, Some(Color::White));
    }
}
