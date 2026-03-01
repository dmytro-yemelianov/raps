// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive mode control
//!
//! Provides functions to check if interactive mode is enabled and handle
//! prompts appropriately based on the --non-interactive flag.

#[cfg(not(test))]
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static NON_INTERACTIVE: AtomicBool = AtomicBool::new(false);
static YES: AtomicBool = AtomicBool::new(false);
static STRICT: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(crate) static MOCK_IS_TERMINAL: AtomicBool = AtomicBool::new(true);

/// Initialize interactive mode flags.
///
/// Also checks environment variables as fallback:
/// - `RAPS_NON_INTERACTIVE=1` — equivalent to `--non-interactive`
/// - `RAPS_YES=1` — equivalent to `--yes`
/// - `RAPS_STRICT=1` — equivalent to `--strict`
pub fn init(non_interactive: bool, yes: bool) {
    init_full(non_interactive, yes, false);
}

/// Initialize all interactive mode flags including strict mode.
pub fn init_full(non_interactive: bool, yes: bool, strict: bool) {
    let s = strict || env_is_truthy("RAPS_STRICT");
    // --strict implies --non-interactive
    let ni = non_interactive || s || env_is_truthy("RAPS_NON_INTERACTIVE");
    let y = yes || env_is_truthy("RAPS_YES");
    NON_INTERACTIVE.store(ni, Ordering::Relaxed);
    YES.store(y, Ordering::Relaxed);
    STRICT.store(s, Ordering::Relaxed);
}

/// Test-only init that sets flags directly without reading env vars.
/// Prevents env var pollution from leaking between parallel tests.
#[cfg(test)]
pub fn init_exact(non_interactive: bool, yes: bool, strict: bool) {
    let ni = non_interactive || strict; // strict implies non-interactive
    NON_INTERACTIVE.store(ni, Ordering::Relaxed);
    YES.store(yes, Ordering::Relaxed);
    STRICT.store(strict, Ordering::Relaxed);
}

fn env_is_truthy(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
}

/// Check if non-interactive mode is enabled (explicit flag or no TTY detected)
pub fn is_non_interactive() -> bool {
    #[cfg(test)]
    let is_term = MOCK_IS_TERMINAL.load(Ordering::Relaxed);
    #[cfg(not(test))]
    let is_term = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

    NON_INTERACTIVE.load(Ordering::Relaxed) || !is_term
}

/// Check if --yes flag is set (auto-confirm)
#[allow(dead_code)] // May be used in future
pub fn is_yes() -> bool {
    YES.load(Ordering::Relaxed)
}

/// Check if --strict mode is enabled.
///
/// Strict mode is designed for CI/CD: it implies non-interactive and also
/// rejects silent defaults — every ambiguous parameter must be explicit.
pub fn is_strict() -> bool {
    STRICT.load(Ordering::Relaxed)
}

/// Detect if the environment is headless (no display server / browser available).
///
/// Returns `true` when browser-based OAuth is unlikely to work:
/// - SSH sessions (`SSH_CONNECTION` or `SSH_TTY` set)
/// - No display server on Linux (`DISPLAY` and `WAYLAND_DISPLAY` both unset)
/// - Docker / CI containers (`container` env var or `/.dockerenv` exists)
/// - Explicit non-interactive flag
pub fn is_headless() -> bool {
    if is_non_interactive() {
        return true;
    }

    // SSH session
    if std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some() {
        return true;
    }

    // Linux without a display server
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return true;
    }

    // Container environment
    if std::env::var_os("container").is_some()
        || std::path::Path::new("/.dockerenv").exists()
        || std::env::var_os("CI").is_some()
    {
        return true;
    }

    false
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
/// Returns true if --yes is set or if the user confirms interactively.
/// Returns false in non-interactive mode without --yes.
pub fn should_proceed_destructive(action: &str) -> bool {
    if is_yes() {
        return true;
    }

    if is_non_interactive() {
        return false; // Fail in non-interactive mode without --yes
    }

    // In interactive mode, prompt the user for confirmation
    dialoguer::Confirm::new()
        .with_prompt(format!("Are you sure you want to {}?", action))
        .default(false)
        .interact()
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serializes tests that manipulate environment variables to prevent leaking
    // env state to other parallel tests in the same process.
    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn reset_state() {
        init_exact(false, false, false);
        MOCK_IS_TERMINAL.store(true, Ordering::Relaxed);
    }

    #[test]
    fn test_init_non_interactive() {
        reset_state();
        init_exact(true, false, false);
        assert!(is_non_interactive());
        assert!(!is_yes());
        reset_state();
    }

    #[test]
    fn test_init_yes() {
        reset_state();
        init_exact(false, true, false);
        assert!(!is_non_interactive());
        assert!(is_yes());
        reset_state();
    }

    #[test]
    fn test_init_both() {
        reset_state();
        init_exact(true, true, false);
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
        init_exact(true, false, false);
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
        init_exact(false, true, false); // --yes flag set
        assert!(should_proceed_destructive("delete bucket"));
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_non_interactive_no_yes() {
        reset_state();
        init_exact(true, false, false); // non-interactive but no --yes
        assert!(!should_proceed_destructive("delete bucket"));
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_interactive() {
        reset_state();
        init_exact(false, false, false); // interactive mode
        assert!(!should_proceed_destructive("delete bucket")); // Should prompt
        reset_state();
    }

    #[test]
    fn test_should_proceed_destructive_non_interactive_with_yes() {
        reset_state();
        init_exact(true, true, false); // non-interactive with --yes
        assert!(should_proceed_destructive("delete bucket"));
        reset_state();
    }

    #[test]
    fn test_strict_implies_non_interactive() {
        reset_state();
        init_exact(false, false, true);
        assert!(is_strict());
        assert!(is_non_interactive()); // strict implies non-interactive
        reset_state();
    }

    #[test]
    fn test_strict_env_var() {
        // Acquire env mutex to prevent env var leaking to other tests.
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        reset_state();
        // SAFETY: protected by ENV_TEST_LOCK mutex; no other test reads
        // this env var concurrently.
        unsafe { std::env::set_var("RAPS_STRICT", "1") };
        // Use a scope + cleanup pattern so remove_var runs even on panic.
        let result = std::panic::catch_unwind(|| {
            init_full(false, false, false);
            assert!(is_strict());
            assert!(is_non_interactive());
        });
        unsafe { std::env::remove_var("RAPS_STRICT") };
        reset_state();
        result.unwrap();
    }

    #[test]
    fn test_not_strict_by_default() {
        reset_state();
        assert!(!is_strict());
    }
}
