// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Logging and verbosity control
//!
//! Provides global flags for controlling output verbosity and formatting:
//! - --no-color: Disable ANSI colors
//! - --quiet: Print only result payload
//! - --verbose: Show request summaries
//! - --debug: Include full trace (redacts secrets)

use regex::Regex;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Return the log directory used by default (when no `--log-file` override is given).
pub fn log_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "autodesk", "raps")
        .map(|dirs| dirs.data_local_dir().join("logs"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".raps-logs")
        })
}

/// Initialize logging flags and tracing.
///
/// `log_file` – when `Some`, write the file log to that exact path instead of
/// the default daily-rolling file in the platform data directory.
pub fn init(no_color: bool, quiet: bool, verbose: bool, debug: bool, log_file: Option<&Path>) {
    // Resolve whether colors should be used, checking all standard signals:
    //   1. --no-color flag
    //   2. NO_COLOR env var (https://no-color.org) — any value disables color
    //   3. TERM=dumb — terminal that cannot render escape sequences
    //   4. CLICOLOR=0 — caller explicitly opted out
    //   5. stdout is not a TTY (piped, redirected, CI without TTY allocation)
    // CLICOLOR_FORCE=1 / CLICOLOR=1 can re-enable colors even when stdout is
    // not a TTY (e.g., CI systems that do support ANSI via pseudo-TTY).
    let clicolor_force = std::env::var("CLICOLOR_FORCE")
        .ok()
        .is_some_and(|v| v != "0");
    let clicolor_off = std::env::var("CLICOLOR")
        .ok()
        .is_some_and(|v| v == "0");
    let no_color_env = std::env::var("NO_COLOR").is_ok();
    let dumb_term = std::env::var("TERM").as_deref() == Ok("dumb");
    let is_tty = std::io::stdout().is_terminal();

    let should_color = !no_color
        && !no_color_env
        && !dumb_term
        && !clicolor_off
        && (clicolor_force || is_tty);

    NO_COLOR.store(!should_color, Ordering::Relaxed);
    QUIET.store(quiet, Ordering::Relaxed);
    VERBOSE.store(verbose, Ordering::Relaxed);
    DEBUG.store(debug, Ordering::Relaxed);

    // Always set an explicit override so the colored crate doesn't rely on
    // its own lazy-initialized env snapshot, which may race or miss signals.
    colored::control::set_override(should_color);

    // Allow RAPS_LOG or RUST_LOG env vars to override CLI flags
    let console_filter = std::env::var("RAPS_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .map(EnvFilter::new)
        .unwrap_or_else(|_| {
            if debug {
                EnvFilter::new("debug")
            } else if verbose {
                EnvFilter::new("info,raps=debug")
            } else if quiet {
                EnvFilter::new("error")
            } else {
                EnvFilter::new("warn")
            }
        });

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(RedactingMakeWriter::new(std::io::stderr))
        .with_ansi(should_color)
        .with_target(debug)
        .without_time()
        .with_filter(console_filter);

    // Determine the file appender: custom path overrides the default daily-rolling log.
    let (non_blocking_appender, guard) = if let Some(path) = log_file {
        // Ensure the parent directory exists.
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            let _ = crate::security::create_dir_restricted(parent);
        }
        let appender = tracing_appender::rolling::never(
            path.parent().unwrap_or_else(|| std::path::Path::new(".")),
            path.file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("raps.log")),
        );
        tracing_appender::non_blocking(appender)
    } else {
        let log_dir = log_dir();
        let _ = crate::security::create_dir_restricted(&log_dir);
        cleanup_old_logs(&log_dir, 7);
        let appender = tracing_appender::rolling::daily(&log_dir, "raps.log");
        tracing_appender::non_blocking(appender)
    };

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

    let redacting_appender = RedactingMakeWriter::new(non_blocking_appender);

    let file_layer: Box<dyn Layer<_> + Send + Sync> = if use_json {
        Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(redacting_appender)
                .with_current_span(true)
                .with_filter(file_filter),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(redacting_appender)
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
            Regex::new(r"(?i)(client[_-]?secret|secret[_-]?key|api[_-]?key)\s*[:=]\s*[^\s]+")
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

    fn auth_header_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(r"(?i)(Authorization:\s*(?:Bearer|Basic))\s+[^\s,;]+")
                .expect("auth_header_pattern regex is valid")
        })
    }

    fn cookie_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(r"(?i)((?:Set-)?Cookie:)\s*[^\r\n]+").expect("cookie_pattern regex is valid")
        })
    }

    fn x_api_key_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(r"(?i)(X-API-Key:)\s*[^\s,;]+").expect("x_api_key_pattern regex is valid")
        })
    }

    fn url_token_pattern() -> &'static Regex {
        static PAT: OnceLock<Regex> = OnceLock::new();
        PAT.get_or_init(|| {
            Regex::new(r"(?i)([?&](?:access_token|apikey|api_key|token)=)[^&\s]+")
                .expect("url_token_pattern regex is valid")
        })
    }

    let redacted = secret_pattern().replace_all(text, "$1: [REDACTED]");
    let redacted = token_pattern().replace_all(&redacted, "$1: [REDACTED]");
    let redacted = auth_header_pattern().replace_all(&redacted, "$1 [REDACTED]");
    let redacted = cookie_pattern().replace_all(&redacted, "$1 [REDACTED]");
    let redacted = x_api_key_pattern().replace_all(&redacted, "$1 [REDACTED]");
    url_token_pattern()
        .replace_all(&redacted, "${1}[REDACTED]")
        .into_owned()
}

/// A writer wrapper that redacts secrets from every line before passing to the inner writer.
struct RedactingWriter<W: Write> {
    inner: W,
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let redacted = redact_secrets(&text);
        self.inner.write_all(redacted.as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// A `MakeWriter` that wraps another writer with automatic secret redaction.
struct RedactingMakeWriter<W> {
    inner: W,
}

impl<W> RedactingMakeWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<'a, W> tracing_subscriber::fmt::MakeWriter<'a> for RedactingMakeWriter<W>
where
    W: tracing_subscriber::fmt::MakeWriter<'a>,
{
    type Writer = RedactingWriter<W::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.inner.make_writer(),
        }
    }
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
        .filter(|e| e.file_name().to_string_lossy().starts_with("raps.log"))
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

    // ==================== New Redaction Pattern Tests ====================

    #[test]
    fn test_redact_bearer_header() {
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_redact_basic_auth_header() {
        let text = "Authorization: Basic dXNlcjpwYXNzd29yZA==";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("dXNlcjpwYXNzd29yZA=="));
    }

    #[test]
    fn test_redact_cookie_header() {
        let text = "Cookie: session_id=abc123; auth_token=xyz789";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abc123"));
    }

    #[test]
    fn test_redact_set_cookie_header() {
        let text = "Set-Cookie: session=secret_value; Path=/; HttpOnly";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret_value"));
    }

    #[test]
    fn test_redact_x_api_key_header() {
        let text = "X-API-Key: sk-1234567890abcdef";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk-1234567890abcdef"));
    }

    #[test]
    fn test_redact_url_access_token_param() {
        let text = "https://api.example.com/data?access_token=secret123&format=json";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret123"));
        assert!(redacted.contains("format=json"));
    }

    #[test]
    fn test_redact_url_apikey_param() {
        let text = "https://api.example.com/data?apikey=mykey123&limit=10";
        let redacted = redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("mykey123"));
    }

    #[test]
    fn test_redact_non_sensitive_unchanged() {
        let text = "GET /api/v1/projects HTTP/1.1\nHost: example.com\nAccept: application/json";
        let redacted = redact_secrets(text);
        assert_eq!(text, redacted);
    }

    #[test]
    fn test_redact_combined_patterns() {
        let text =
            "Authorization: Bearer eyJtoken123456789012345 Cookie: sess=val X-API-Key: key123";
        let redacted = redact_secrets(text);
        assert!(!redacted.contains("eyJtoken"));
        assert!(!redacted.contains("sess=val"));
        assert!(!redacted.contains("key123"));
    }

    #[test]
    fn test_redacting_writer() {
        let mut buf = Vec::new();
        {
            let mut writer = super::RedactingWriter { inner: &mut buf };
            write!(writer, "Authorization: Bearer secret_token_value_here").unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("secret_token_value_here"));
    }
}
