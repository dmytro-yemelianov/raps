// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Logging and verbosity control
//!
//! Provides global flags for controlling output verbosity and formatting:
//! - --no-color: Disable ANSI colors
//! - --quiet: Print only result payload
//! - --verbose: Show request summaries
//! - --debug: Include full trace (redacts secrets)

use std::sync::atomic::{AtomicBool, Ordering};

/// Global logging state
static NO_COLOR: AtomicBool = AtomicBool::new(false);
static QUIET: AtomicBool = AtomicBool::new(false);
static VERBOSE: AtomicBool = AtomicBool::new(false);
static DEBUG: AtomicBool = AtomicBool::new(false);

/// Initialize logging flags
pub fn init(no_color: bool, quiet: bool, verbose: bool, debug: bool) {
    NO_COLOR.store(no_color, Ordering::Relaxed);
    QUIET.store(quiet, Ordering::Relaxed);
    VERBOSE.store(verbose, Ordering::Relaxed);
    DEBUG.store(debug, Ordering::Relaxed);

    // Disable colored output globally if --no-color is set
    if no_color {
        colored::control::set_override(false);
    }
}

/// Check if colors should be disabled
#[allow(dead_code)] // May be used in future
pub fn no_color() -> bool {
    NO_COLOR.load(Ordering::Relaxed)
}

/// Check if quiet mode is enabled
pub fn quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// Check if verbose mode is enabled
pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Check if debug mode is enabled
pub fn debug() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

/// Log a verbose message (only shown if --verbose or --debug)
pub fn log_verbose(message: &str) {
    if verbose() || debug() {
        eprintln!("{}", redact_secrets(message));
    }
}

/// Log a debug message (only shown if --debug)
pub fn log_debug(message: &str) {
    if debug() {
        eprintln!("[DEBUG] {}", redact_secrets(message));
    }
}

/// Log an HTTP request (only shown if --verbose or --debug)
pub fn log_request(method: &str, url: &str) {
    if verbose() || debug() {
        eprintln!("{} {}", method, redact_secrets(url));
    }
}

/// Log an HTTP response (only shown if --verbose or --debug)
pub fn log_response(status: u16, url: &str) {
    if verbose() || debug() {
        eprintln!("{} {}", status, redact_secrets(url));
    }
}

/// Redact secrets from debug output
pub fn redact_secrets(text: &str) -> String {
    // Redact common secret patterns
    let mut redacted = text.to_string();

    // Redact client secrets - match patterns like "client_secret: value" or "api-key=value"
    let secret_pattern =
        regex::Regex::new(r"(?i)(client[_-]?secret|secret[_-]?key|api[_-]?key)\s*[:=]\s*[^\s]+")
            .unwrap();
    redacted = secret_pattern
        .replace_all(&redacted, "$1: [REDACTED]")
        .to_string();

    // Redact tokens (JWT-like strings) - match patterns like "token: abc123..." or "bearer=xyz..."
    let token_pattern = regex::Regex::new(
        r"(?i)(token|access[_-]?token|refresh[_-]?token|bearer)\s*[:=]\s*([A-Za-z0-9_-]{20,})",
    )
    .unwrap();
    redacted = token_pattern
        .replace_all(&redacted, "$1: [REDACTED]")
        .to_string();

    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Flag tests are not reliable due to global state and parallel test execution.
    // The init() function modifies global AtomicBool values which can race with other tests.
    // Testing redact_secrets is more valuable and deterministic.

    // ==================== Redact Secrets Tests ====================

    #[test]
    fn test_redact_client_secret() {
        let text = "client_secret: abc123xyz";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abc123xyz"));
    }

    #[test]
    fn test_redact_client_secret_underscore() {
        let text = "client_secret=my_super_secret_value";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("my_super_secret_value"));
    }

    #[test]
    fn test_redact_api_key() {
        let text = "api_key: supersecretapikey123";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("supersecretapikey123"));
    }

    #[test]
    fn test_redact_api_key_dash() {
        let text = "api-key=myapikey456";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("myapikey456"));
    }

    #[test]
    fn test_redact_secret_key() {
        let text = "secret_key: topsecret";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("topsecret"));
    }

    #[test]
    fn test_redact_access_token() {
        let text = "access_token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_redact_refresh_token() {
        let text = "refresh_token=abcdefghijklmnopqrstuvwxyz";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let text = "bearer: ABCDEFGHIJKLMNOPQRSTUVWXYZ123456";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ123456"));
    }

    #[test]
    fn test_redact_case_insensitive() {
        let text1 = "CLIENT_SECRET: secret1";
        let text2 = "Client_Secret: secret2";
        let text3 = "client_SECRET: secret3";

        assert!(redact_secrets(text1).contains("[REDACTED]"));
        assert!(redact_secrets(text2).contains("[REDACTED]"));
        assert!(redact_secrets(text3).contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_preserves_non_secret_text() {
        let text = "This is a normal message without secrets";
        let redacted = redact_secrets(text);
        assert_eq!(text, redacted);
    }

    #[test]
    fn test_redact_multiple_secrets() {
        let text = "client_secret: secret1 api_key: key123";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("secret1"));
        assert!(!redacted.contains("key123"));
        assert!(redacted.matches("[REDACTED]").count() >= 2);
    }

    #[test]
    fn test_redact_mixed_content() {
        let text = "Logging in with client_secret: mysecret for user john";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("Logging in"));
        assert!(redacted.contains("for user john"));
        assert!(!redacted.contains("mysecret"));
    }

    #[test]
    fn test_redact_short_token_not_redacted() {
        // Tokens shorter than 20 chars should not be redacted (not a real token)
        let text = "token: short";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("short"));
    }

    #[test]
    fn test_redact_empty_string() {
        let text = "";
        let redacted = redact_secrets(text);
        assert_eq!(redacted, "");
    }
}
