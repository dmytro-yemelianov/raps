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
pub fn is_yes() -> bool {
    YES.load(Ordering::Relaxed)
}

/// Require a value in non-interactive mode
///
/// Returns an error if non-interactive mode is enabled and the value is None
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
            anyhow::bail!("{} is required", name);
        }
    }
}

/// Check if a destructive action should proceed
///
/// Returns true if --yes is set or if interactive mode allows confirmation
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
