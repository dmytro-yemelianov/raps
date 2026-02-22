// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Logging and verbosity control
//!
//! Provides global flags for controlling output verbosity and formatting:
//! - --no-color: Disable ANSI colors
//! - --quiet: Print only result payload
//! - --verbose: Show request summaries
//! - --debug: Include full trace (redacts secrets)

use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use regex::Regex;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Global logging state
static NO_COLOR: AtomicBool = AtomicBool::new(false);
static QUIET: AtomicBool = AtomicBool::new(false);
static VERBOSE: AtomicBool = AtomicBool::new(false);
static DEBUG: AtomicBool = AtomicBool::new(false);

static WORKER_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

/// Flush background logs by dropping the WorkerGuard
pub fn flush() {
    if let Ok(mut guard) = WORKER_GUARD.lock() {
        let _ = guard.take(); // dropping the guard flushes the async logger
    }
}

/// Initialize logging flags and tracing
pub fn init(no_color: bool, quiet: bool, verbose: bool, debug: bool) {
    NO_COLOR.store(no_color, Ordering::Relaxed);
    QUIET.store(quiet, Ordering::Relaxed);
    VERBOSE.store(verbose, Ordering::Relaxed);
    DEBUG.store(debug, Ordering::Relaxed);

    // Disable colored output globally if --no-color is set
    if no_color {
        colored::control::set_override(false);
    }

    // Allow RAPS_LOG or RUST_LOG env vars to override CLI flags
    let console_filter = std::env::var("RAPS_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .map(EnvFilter::new)
        .unwrap_or_else(|_| {
            if debug {
                EnvFilter::new("debug")
            } else if verbose {
                EnvFilter::new("info")
            } else if quiet {
                EnvFilter::new("error")
            } else {
                EnvFilter::new("warn")
            }
        });

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(!no_color)
        .with_target(debug)
        .without_time()
        .with_filter(console_filter);

    let log_dir = directories::ProjectDirs::from("com", "autodesk", "raps")
        .map(|dirs| dirs.data_local_dir().join("logs"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(".raps-logs")
        });

    let _ = std::fs::create_dir_all(&log_dir);
    cleanup_old_logs(&log_dir, 7);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "raps.log");
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);
    if let Ok(mut lock) = WORKER_GUARD.lock() {
        *lock = Some(guard);
    }

    // File log filter: configurable via RAPS_FILE_LOG env var
    let file_filter = std::env::var("RAPS_FILE_LOG")
        .map(EnvFilter::new)
        .unwrap_or_else(|_| EnvFilter::new("raps=debug,info"));

    // File log format: JSON if RAPS_FILE_FORMAT=json, plain text otherwise
    let use_json = std::env::var("RAPS_FILE_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let file_layer: Box<dyn Layer<_> + Send + Sync> = if use_json {
        Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking_appender)
                .with_current_span(true)
                .with_filter(file_filter),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_appender)
                .with_ansi(false)
                .with_filter(file_filter),
        )
    };

    let _ = tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .try_init();
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

/// Redact secrets from debug output
pub fn redact_secrets(text: &str) -> String {
    fn secret_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(
                r"(?i)(client[_-]?secret|secret[_-]?key|api[_-]?key)\s*[:=]\s*[^\s]+",
            )
            .expect("secret_pattern regex is valid")
        })
    }

    fn token_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(
                r#"(?i)(token|access[_-]?token|refresh[_-]?token|bearer)\s*"?\s*[:=]\s*"?\s*([A-Za-z0-9_\-\.]{20,})"#,
            )
            .expect("token_pattern regex is valid")
        })
    }

    let redacted = secret_pattern()
        .replace_all(text, "$1: [REDACTED]");
    token_pattern()
        .replace_all(&redacted, "$1: [REDACTED]")
        .into_owned()
}

/// Maximum total log size in bytes (50 MB).
const MAX_LOG_BYTES: u64 = 50 * 1024 * 1024;

/// Remove old log files, keeping at most `max_files` and staying under `MAX_LOG_BYTES`.
fn cleanup_old_logs(log_dir: &std::path::Path, max_files: usize) {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("raps.log")
        })
        .collect();
    // Most recent first
    files.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
    let mut total_size = 0u64;
    for (i, file) in files.iter().enumerate() {
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        total_size += size;
        if i >= max_files || total_size > MAX_LOG_BYTES {
            let _ = std::fs::remove_file(file.path());
        }
    }
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

    #[test]
    fn test_redact_json_access_token() {
        let text = r#""access_token":"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.abc123""#;
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
}
