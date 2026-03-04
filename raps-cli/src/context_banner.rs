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

impl HubEntry {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_hub_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.core:Hub")),
            HubTier::Personal
        );
    }

    #[test]
    fn test_bim360_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.bim360:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_acc_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.acc:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_accproject_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.accproject:Hub")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_unknown_tier() {
        assert_eq!(HubEntry::tier_from_extension(None), HubTier::Unknown);
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.fusion:Hub")),
            HubTier::Unknown
        );
    }
}
