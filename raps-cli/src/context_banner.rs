// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Visual context banner for hub type display.

/// Width of the terminal box borders (total line width).
pub const BOX_WIDTH: usize = 80;
/// Inner content width (BOX_WIDTH minus borders and 2-char padding each side: │  …  │).
pub const INNER_WIDTH: usize = 76;

/// Hub account tier — determines visual treatment.
#[derive(Debug, Clone, PartialEq)]
pub enum HubTier {
    /// A360 personal hub (autodesk.a360)
    Personal,
    /// Autodesk Team Hub (autodesk.core:Hub) — collaboration hub, not ACC/BIM 360
    Team,
    /// ACC or BIM 360 construction hub (autodesk.acc, autodesk.bim360, autodesk.accproject)
    Enterprise,
    Unknown,
}

/// A single hub entry with all display data.
#[derive(Debug, Clone)]
pub struct HubEntry {
    pub id: String,
    pub name: String,
    pub tier: HubTier,
    pub region: Option<String>,
}

/// Derive tier from APS extension_type string.
pub fn tier_from_extension(ext: Option<&str>) -> HubTier {
    match ext {
        Some(e) if e.contains("autodesk.a360") => HubTier::Personal,
        Some(e) if e.contains("autodesk.core:Hub") => HubTier::Team,
        Some(e)
            if e.contains("autodesk.bim360:Account")
                || e.contains("autodesk.acc:Account")
                || e.contains("autodesk.accproject") =>
        {
            HubTier::Enterprise
        }
        _ => HubTier::Unknown,
    }
}

use colored::Colorize;
use raps_dm::types::Hub;

/// Collection of hub entries for display.
pub struct ContextBanner {
    pub hubs: Vec<HubEntry>,
}

impl HubEntry {
    /// Shortened ID for inline display (max 14 chars, matching id_col budget).
    pub fn short_id(&self) -> String {
        let count = self.id.chars().count();
        if count <= 14 {
            self.id.clone()
        } else {
            let prefix: String = self.id.chars().take(8).collect();
            let suffix: String = self.id.chars().rev().take(4).collect::<String>()
                .chars().rev().collect();
            format!("{}…{}", prefix, suffix)
        }
    }

    /// Glyph + label string for this tier (fixed 12 chars wide).
    pub fn tier_label(&self) -> &'static str {
        match self.tier {
            HubTier::Personal   => "○ PERSONAL  ",
            HubTier::Team       => "◇ TEAM      ",
            HubTier::Enterprise => "◆ ENTERPRISE",
            HubTier::Unknown    => "? UNKNOWN   ",
        }
    }
}

impl ContextBanner {
    /// Build banner from a slice of DM Hub objects.
    pub fn from_hubs(hubs: &[Hub]) -> Self {
        let entries = hubs
            .iter()
            .map(|h| {
                let ext = h
                    .attributes
                    .extension
                    .as_ref()
                    .and_then(|e| e.extension_type.as_deref());
                HubEntry {
                    id: h.id.clone(),
                    name: h.attributes.name.clone(),
                    tier: tier_from_extension(ext),
                    region: h.attributes.region.clone(),
                }
            })
            .collect();
        Self { hubs: entries }
    }

    /// Build banner from a single resolved enterprise account (for admin commands).
    pub fn from_account(id: &str, name: &str, region: Option<&str>) -> Self {
        Self {
            hubs: vec![HubEntry {
                id: id.to_string(),
                name: name.to_string(),
                tier: HubTier::Enterprise,
                region: region.map(|s| s.to_string()),
            }],
        }
    }

    /// Print a cyan bordered admin context box to stderr.
    /// Shows the first Enterprise hub entry (the resolved account).
    ///
    /// Example (80 cols):
    /// ┌─ Account Context ────────────────────────────────────────────────────────────┐
    /// │  ◆ ENTERPRISE  Acme Corp                                                     │
    /// │  Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05                         │
    /// │  Region:       US                                                            │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    pub fn print_box(&self) {
        let Some(entry) = self.hubs.iter().find(|h| h.tier == HubTier::Enterprise) else {
            return;
        };
        let region = entry.region.as_deref().unwrap_or("--");
        let top    = box_top("Account Context");
        let line1  = box_line(&format!("◆ ENTERPRISE  {}", entry.name));
        let line2  = box_line(&format!("Account ID:   {}", entry.id));
        let line3  = box_line(&format!("Region:       {}", region));
        let bottom = box_bottom();

        eprintln!("{}", top.cyan());
        eprintln!("{}", line1.cyan().bold());
        eprintln!("{}", line2.cyan());
        eprintln!("{}", line3.cyan());
        eprintln!("{}", bottom.cyan());
    }

    /// Print one inline line per hub to stderr (table/interactive mode only).
    ///
    /// Format (80-col safe):
    ///   ○ PERSONAL    My Projects              a.aBcD…xyz  [US]
    ///   ◇ TEAM        RAPS HUB                 a.YnVz…MTAz [US]
    ///   ◆ ENTERPRISE  Acme Corp                b.01fb…ae05  [US]
    pub fn print_inline(&self) {
        for entry in &self.hubs {
            let region = entry.region.as_deref().unwrap_or("--");
            let name_col = format!("{:<28}", truncate(&entry.name, 28));
            let id_col   = format!("{:<14}", entry.short_id());
            let line = format!(
                "  {}  {}  {}  [{}]",
                entry.tier_label(),
                name_col,
                id_col,
                region
            );
            match entry.tier {
                HubTier::Personal   => eprintln!("{}", line.dimmed()),
                HubTier::Team       => eprintln!("{}", line),
                HubTier::Enterprise => eprintln!("{}", line.cyan().bold()),
                HubTier::Unknown    => eprintln!("{}", line.dimmed()),
            }
        }
    }
}

/// Truncate a string to max_len chars, appending … if cut.
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Render one content line inside the box.
/// Format: │  {content padded to BOX_WIDTH-2}  │  (total BOX_WIDTH visible chars)
pub(crate) fn box_line(content: &str) -> String {
    let inner_content = format!("  {}", truncate(content, INNER_WIDTH));
    format!("│{:<width$}│", inner_content, width = INNER_WIDTH + 2)
}

/// Render the top border: ┌─ {title} ─…─┐  (total BOX_WIDTH visible chars)
pub(crate) fn box_top(title: &str) -> String {
    let prefix = format!("┌─ {} ", title);
    let prefix_chars = prefix.chars().count();
    let dashes = "─".repeat(BOX_WIDTH.saturating_sub(prefix_chars + 1));
    format!("{}{}┐", prefix, dashes)
}

/// Render the bottom border (total BOX_WIDTH visible chars).
pub(crate) fn box_bottom() -> String {
    format!("└{}┘", "─".repeat(BOX_WIDTH - 2))
}

/// Strip ANSI CSI escape sequences (`ESC[...X` where X is any ASCII letter).
/// Handles SGR (colors/bold), cursor movement, and other CSI sequences.
/// Used only for test width assertions.
#[cfg(test)]
pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            // consume all bytes up to and including the final ASCII letter
            for inner in chars.by_ref() {
                if inner.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Print a yellow bordered warning box to stderr when no enterprise hub is found.
/// Called by resolve_account_id when 0 enterprise hubs are available.
pub fn print_warning_no_enterprise() {
    let top    = box_top("⚠  Enterprise Account Required");
    let line1  = box_line("Admin API is not available for personal hubs.");
    let line2  = box_line("Register your app in ACC Custom Integrations to");
    let line3  = box_line("enable admin commands for your enterprise account.");
    let line4  = box_line("Docs: rapscli.xyz/docs/custom-integrations");
    let bottom = box_bottom();

    eprintln!("{}", top.yellow().bold());
    eprintln!("{}", line1.yellow());
    eprintln!("{}", line2.yellow());
    eprintln!("{}", line3.yellow());
    eprintln!("{}", line4.yellow());
    eprintln!("{}", bottom.yellow().bold());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_hub_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.core:Hub")),
            HubTier::Team
        );
    }

    #[test]
    fn test_personal_a360_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.a360:PersonalHub")),
            HubTier::Personal
        );
    }

    #[test]
    fn test_bim360_enterprise_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.bim360:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_acc_enterprise_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.acc:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_accproject_enterprise_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.accproject:Hub")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_unknown_tier() {
        assert_eq!(tier_from_extension(None), HubTier::Unknown);
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.fusion:Hub")),
            HubTier::Unknown
        );
    }

    #[test]
    fn test_from_hubs_classifies_tiers() {
        use raps_dm::types::{Hub, HubAttributes, HubExtension};
        let hubs = vec![
            Hub {
                hub_type: "hubs".into(),
                id: "a.abc123".into(),
                attributes: HubAttributes {
                    name: "My Projects".into(),
                    region: Some("US".into()),
                    extension: Some(HubExtension {
                        extension_type: Some("hubs:autodesk.core:Hub".into()),
                    }),
                },
            },
            Hub {
                hub_type: "hubs".into(),
                id: "b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05".into(),
                attributes: HubAttributes {
                    name: "Acme Corp".into(),
                    region: Some("US".into()),
                    extension: Some(HubExtension {
                        extension_type: Some("hubs:autodesk.bim360:Account".into()),
                    }),
                },
            },
        ];
        let banner = ContextBanner::from_hubs(&hubs);
        assert_eq!(banner.hubs.len(), 2);
        assert_eq!(banner.hubs[0].tier, HubTier::Team);
        assert_eq!(banner.hubs[1].tier, HubTier::Enterprise);
    }

    #[test]
    fn test_short_id_long_truncated() {
        let entry = HubEntry {
            id: "b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05".into(),
            name: "Acme Corp".into(),
            tier: HubTier::Enterprise,
            region: Some("US".into()),
        };
        let short = entry.short_id();
        assert!(short.chars().count() <= 14, "short_id too long: {}", short);
        assert!(short.contains('…'), "should contain ellipsis: {}", short);
    }

    #[test]
    fn test_short_id_short_passthrough() {
        let entry = HubEntry {
            id: "a.abc123".into(),
            name: "Test".into(),
            tier: HubTier::Personal,
            region: None,
        };
        assert_eq!(entry.short_id(), "a.abc123");
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let result = truncate("hello world this is too long", 10);
        assert!(result.chars().count() <= 10);
        assert!(result.ends_with('…'));
    }

    // --- tier_from_extension: Some path with an unrecognised extension string → Unknown ---

    #[test]
    fn tier_unknown_when_some_unrecognised_extension() {
        // Covers the Some(_) arm that falls through to Unknown (not None, not a known vendor string).
        assert_eq!(
            tier_from_extension(Some("autodesk.something:Unknown")),
            HubTier::Unknown
        );
    }

    // --- truncate: exact-limit string must not be truncated ---

    #[test]
    fn truncate_at_exact_limit_unchanged() {
        let s = "a".repeat(10);
        assert_eq!(truncate(&s, 10), s);
    }

    #[test]
    fn truncate_one_under_limit_unchanged() {
        // A string of exactly max_len-1 chars must not be truncated (boundary below the cut).
        let s = "a".repeat(9);
        assert_eq!(truncate(&s, 10), s);
    }

    #[test]
    fn truncate_long_string_ends_with_ellipsis() {
        let result = truncate("abcdefghijklmnopqrstuvwxyz", 10);
        assert!(
            result.ends_with('\u{2026}') || result.ends_with("..."),
            "expected ellipsis at end, got: {result}"
        );
        assert!(result.chars().count() <= 11, "result too long: {result}");
    }

    #[test]
    fn test_box_line_width() {
        let line = box_line("Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05");
        let visible = strip_ansi(&line);
        assert_eq!(
            visible.chars().count(),
            BOX_WIDTH,
            "box_line visible width should be {BOX_WIDTH}, got {}",
            visible.chars().count()
        );
    }

    #[test]
    fn test_box_top_width() {
        let top = box_top("Account Context");
        let visible = strip_ansi(&top);
        assert_eq!(
            visible.chars().count(),
            BOX_WIDTH,
            "box_top visible width should be {BOX_WIDTH}, got {}",
            visible.chars().count()
        );
    }

    #[test]
    fn test_box_bottom_width() {
        let bottom = box_bottom();
        assert_eq!(
            bottom.chars().count(),
            BOX_WIDTH,
            "box_bottom visible width should be {BOX_WIDTH}, got {}",
            bottom.chars().count()
        );
    }

    #[test]
    fn test_warning_box_top_width() {
        let top = box_top("⚠  Enterprise Account Required");
        let visible = strip_ansi(&top);
        assert_eq!(
            visible.chars().count(),
            BOX_WIDTH,
            "warning box_top visible width should be {BOX_WIDTH}, got {}",
            visible.chars().count()
        );
    }
}
