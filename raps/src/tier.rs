// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Product tier gating for feature access control
//!
//! RAPS is available in three tiers:
//! - **Core**: Essential APS functionality (OSS, Derivative, Data Management, Auth)
//! - **Community**: Extended features (ACC, Design Automation, Reality, Webhooks, Pipelines, Plugins)
//! - **Pro**: Enterprise features (Analytics, Audit, Compliance, SSO)
//!
//! This module provides runtime checks for feature availability.

use anyhow::{Result, bail};

/// Product tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Core tier: Essential APS functionality
    Core,
    /// Community tier: Extended features
    Community,
    /// Pro tier: Enterprise features
    Pro,
}

impl Tier {
    /// Get the current tier based on enabled features
    pub const fn current() -> Self {
        #[cfg(feature = "pro")]
        { Self::Pro }
        #[cfg(all(feature = "community", not(feature = "pro")))]
        { Self::Community }
        #[cfg(not(any(feature = "community", feature = "pro")))]
        { Self::Core }
    }

    /// Check if this tier includes the specified tier
    pub const fn includes(self, required: Tier) -> bool {
        match (self, required) {
            // Pro includes everything
            (Tier::Pro, _) => true,
            // Community includes Core and Community
            (Tier::Community, Tier::Core) => true,
            (Tier::Community, Tier::Community) => true,
            (Tier::Community, Tier::Pro) => false,
            // Core only includes Core
            (Tier::Core, Tier::Core) => true,
            (Tier::Core, _) => false,
        }
    }

    /// Get the tier name for display
    pub const fn name(self) -> &'static str {
        match self {
            Tier::Core => "Core",
            Tier::Community => "Community",
            Tier::Pro => "Pro",
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Check if a feature is available in the current tier.
/// Returns an error if the feature requires a higher tier.
pub fn require_tier(feature: &str, required_tier: Tier) -> Result<()> {
    let current = Tier::current();
    if current.includes(required_tier) {
        Ok(())
    } else {
        bail!(
            "Feature '{}' requires {} tier or higher.\n\
             Current tier: {}\n\n\
             Upgrade instructions:\n\
             - Community: cargo install raps --features community\n\
             - Pro: Contact sales@rapscli.xyz for licensing",
            feature,
            required_tier,
            current
        )
    }
}

/// Check if community tier features are available
pub fn require_community(feature: &str) -> Result<()> {
    require_tier(feature, Tier::Community)
}

/// Check if pro tier features are available
pub fn require_pro(feature: &str) -> Result<()> {
    require_tier(feature, Tier::Pro)
}

/// Macro to conditionally compile commands based on tier
#[macro_export]
macro_rules! community_only {
    ($feature:expr, $code:expr) => {{
        #[cfg(feature = "community")]
        { $code }
        #[cfg(not(feature = "community"))]
        {
            anyhow::bail!(
                "Feature '{}' requires Community tier or higher.\n\
                 Current tier: Core\n\n\
                 To use this feature, reinstall with community features:\n\
                 cargo install raps --features community",
                $feature
            )
        }
    }};
}

#[macro_export]
macro_rules! pro_only {
    ($feature:expr, $code:expr) => {{
        #[cfg(feature = "pro")]
        { $code }
        #[cfg(not(feature = "pro"))]
        {
            anyhow::bail!(
                "Feature '{}' requires Pro tier.\n\
                 Current tier: {}\n\n\
                 Contact sales@rapscli.xyz for Pro licensing.",
                $feature,
                $crate::tier::Tier::current()
            )
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_includes() {
        assert!(Tier::Pro.includes(Tier::Core));
        assert!(Tier::Pro.includes(Tier::Community));
        assert!(Tier::Pro.includes(Tier::Pro));

        assert!(Tier::Community.includes(Tier::Core));
        assert!(Tier::Community.includes(Tier::Community));
        assert!(!Tier::Community.includes(Tier::Pro));

        assert!(Tier::Core.includes(Tier::Core));
        assert!(!Tier::Core.includes(Tier::Community));
        assert!(!Tier::Core.includes(Tier::Pro));
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(Tier::Core.name(), "Core");
        assert_eq!(Tier::Community.name(), "Community");
        assert_eq!(Tier::Pro.name(), "Pro");
    }

    #[test]
    fn test_current_tier() {
        // This test depends on compile-time features
        let current = Tier::current();
        #[cfg(feature = "pro")]
        assert_eq!(current, Tier::Pro);
        #[cfg(all(feature = "community", not(feature = "pro")))]
        assert_eq!(current, Tier::Community);
        #[cfg(not(any(feature = "community", feature = "pro")))]
        assert_eq!(current, Tier::Core);
    }

    #[test]
    fn test_require_tier_success() {
        // Should always succeed for Core (always available)
        assert!(require_tier("test", Tier::Core).is_ok());
    }
}
