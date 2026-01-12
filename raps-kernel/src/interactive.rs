// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive mode control
//!
//! Provides functions to check if interactive mode is enabled and handle
//! prompts appropriately based on the --non-interactive flag.

use std::sync::atomic::{AtomicBool, Ordering};

static NON_INTERACTIVE: AtomicBool = AtomicBool::new(false);
static YES: AtomicBool = AtomicBool::new(false);

/// Initialize interactive mode flags
pub fn init(non_interactive: bool, yes: bool) {
    NON_INTERACTIVE.store(non_interactive, Ordering::Relaxed);
    YES.store(yes, Ordering::Relaxed);
}

/// Check if non-interactive mode is enabled
pub fn is_non_interactive() -> bool {
    NON_INTERACTIVE.load(Ordering::Relaxed)
}

/// Check if --yes flag is set (auto-confirm)
#[allow(dead_code)] // May be used in future
pub fn is_yes() -> bool {
    YES.load(Ordering::Relaxed)
}

/// Require a value in non-interactive mode
///
/// Returns an error if non-interactive mode is enabled and the value is None
#[allow(dead_code)] // May be used in future
pub fn require_value<T>(value: Option<T>, name: &str) -> Result<T, anyhow::Error> {
    match value {
        Some(v) => Ok(v),
        None => {
            if is_non_interactive() {
                anyhow::bail!(
                    "{} is required in non-interactive mode. Use --{} flag or set environment variable.",
                    name,
                    name.replace('_', "-")
                );
            }
            // In interactive mode, return None wrapped in error to trigger prompt
            anyhow::bail!("{name} is required");
        }
    }
}

/// Check if a destructive action should proceed
///
/// Returns true if --yes is set or if interactive mode allows confirmation
#[allow(dead_code)] // May be used in future
pub fn should_proceed_destructive(_action: &str) -> bool {
    if is_yes() {
        return true;
    }

    if is_non_interactive() {
        return false; // Fail in non-interactive mode without --yes
    }

    // In interactive mode, return false to trigger confirmation prompt
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests manipulate global state, so they should be run with --test-threads=1
    // or they may interfere with each other

    fn reset_state() {
        init(false, false);
    }

    #[test]
    fn test_init_non_interactive() {
        reset_state();
        init(true, false);
        assert!(is_non_interactive());
        assert!(!is_yes());
        reset_state();
    }

    #[test]
    fn test_init_yes() {
        reset_state();
        init(false, true);
        assert!(!is_non_interactive());
        assert!(is_yes());
        reset_state();
    }

    #[test]
    fn test_init_both() {
        reset_state();
        init(true, true);
        assert!(is_non_interactive());
        assert!(is_yes());
        reset_state();
    }

    #[test]
    fn test_default_state() {
        reset_state();
        assert!(!is_non_interactive());
        assert!(!is_yes());
    }

    #[test]
    fn test_require_value_some() {
        reset_state();
        let result = require_value(Some("test"), "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_require_value_none_interactive() {
        reset_state();
        let result = require_value::<String>(None, "name");
        assert!(result.is_err());
        // In interactive mode, should just say it's required (to trigger prompt)
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("required"));
    }

    #[test]
    fn test_require_value_none_non_interactive() {
        reset_state();
        init(true, false);
        let result = require_value::<String>(None, "name");
        assert!(result.is_err());
        // In non-interactive mode, should mention the flag
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("non-interactive"));
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_yes() {
        reset_state();
        init(false, true); // --yes flag set
        assert!(should_proceed_destructive("delete bucket"));
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_non_interactive_no_yes() {
        reset_state();
        init(true, false); // non-interactive but no --yes
        assert!(!should_proceed_destructive("delete bucket"));
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_interactive() {
        reset_state();
        init(false, false); // interactive mode
        assert!(!should_proceed_destructive("delete bucket")); // Should prompt
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_non_interactive_with_yes() {
        reset_state();
        init(true, true); // non-interactive with --yes
        assert!(should_proceed_destructive("delete bucket"));
        reset_state();
    }
}
