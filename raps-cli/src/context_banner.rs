// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Visual context banner for Personal vs Enterprise hub display.

/// Width of the terminal box borders (total line width).
pub const BOX_WIDTH: usize = 80;
/// Inner content width (BOX_WIDTH minus borders and 2-char padding each side: │  …  │).
pub const INNER_WIDTH: usize = 76;

/// Hub account tier — determines visual treatment.
#[derive(Debug, Clone, PartialEq)]
pub enum HubTier {
    Personal,
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
        Some(e) if e.contains("autodesk.core:Hub") => HubTier::Personal,
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
    /// Shortened ID for inline display (max 16 chars).
    pub fn short_id(&self) -> String {
        if self.id.len() <= 16 {
            self.id.clone()
        } else {
            let prefix = &self.id[..8.min(self.id.len())];
            let suffix = &self.id[self.id.len().saturating_sub(4)..];
            format!("{}…{}", prefix, suffix)
        }
    }

    /// Glyph + label string for this tier (fixed 12 chars wide).
    pub fn tier_label(&self) -> &'static str {
        match self.tier {
            HubTier::Personal   => "○ PERSONAL  ",
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

    /// Print one inline line per hub to stderr (table/interactive mode only).
    ///
    /// Format (80-col safe):
    ///   ○ PERSONAL    My Projects              a.aBcD…xyz  [US]
    ///   ◆ ENTERPRISE  Acme Corp                01fb…ae05   [US]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_hub_tier() {
        assert_eq!(
            tier_from_extension(Some("hubs:autodesk.core:Hub")),
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
        assert_eq!(banner.hubs[0].tier, HubTier::Personal);
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
        assert!(short.len() <= 16, "short_id too long: {}", short);
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
}
