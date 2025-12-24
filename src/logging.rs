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
        eprintln!("{}", message);
    }
}

/// Log a debug message (only shown if --debug)
#[allow(dead_code)] // May be used in future
pub fn log_debug(message: &str) {
    if debug() {
        eprintln!("[DEBUG] {}", message);
    }
}

/// Log an HTTP request (only shown if --verbose or --debug)
pub fn log_request(method: &str, url: &str) {
    if verbose() || debug() {
        eprintln!("{} {}", method, url);
    }
}

/// Log an HTTP response (only shown if --verbose or --debug)
pub fn log_response(status: u16, url: &str) {
    if verbose() || debug() {
        eprintln!("{} {}", status, url);
    }
}

/// Redact secrets from debug output
#[allow(dead_code)] // May be used in future
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
