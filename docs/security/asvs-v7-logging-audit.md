# ASVS V7 Audit: Logging & Error Handling

**Audit Date:** 2026-02-28
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Scope

Audit of the RAPS logging and error handling subsystem covering:
- Logging setup and configuration (`raps-kernel/src/logging.rs`)
- Secret redaction patterns and coverage
- Error information leakage in release builds
- Log file management and retention

## Findings

### V7.1.1 - Logging Framework and Configuration

- **Status:** Met
- **Evidence:** `raps-kernel/src/logging.rs:34-117`
- **Details:** RAPS uses the `tracing` / `tracing-subscriber` framework with two output targets:
  - **Console (stderr):** Controlled by `--quiet`, `--verbose`, `--debug` flags. Default level is `warn`.
  - **File log:** Written to `~/.local/share/raps/logs/raps.log.YYYY-MM-DD` using `tracing_appender::rolling::daily`. Default level is `raps=debug,info`.

  The logging levels are configurable via:
  - CLI flags (`--quiet` = error, `--verbose` = info, `--debug` = debug)
  - Environment variables (`RAPS_LOG` or `RUST_LOG` for console, `RAPS_FILE_LOG` for file)
  - JSON format option (`RAPS_FILE_FORMAT=json`)

  Environment variables take precedence over CLI flags, allowing operators to override log levels without modifying the command line.

### V7.1.2 - Secret Redaction Patterns

- **Status:** Partial
- **Evidence:** `raps-kernel/src/logging.rs:141-163`
- **Details:** The `redact_secrets()` function uses two regex patterns:

  **Pattern 1 - Secrets:** `(?i)(client[_-]?secret|secret[_-]?key|api[_-]?key)\s*[:=]\s*[^\s]+`
  Matches:
  - `client_secret`, `clientsecret`, `client-secret`
  - `secret_key`, `secretkey`, `secret-key`
  - `api_key`, `apikey`, `api-key`

  **Pattern 2 - Tokens:** `(?i)(token|access[_-]?token|refresh[_-]?token|bearer)\s*"?\s*[:=]\s*"?\s*([A-Za-z0-9_\-\.]{20,})`
  Matches:
  - `token`, `access_token`, `refresh_token`, `bearer`
  - Only redacts values >= 20 characters (to avoid false positives on short strings)
  - Handles both quoted and unquoted values

  **Patterns NOT covered:**
  - `Authorization: Bearer <token>` HTTP header format (the pattern expects `=` or `:` after the keyword, not the space-separated header format commonly seen in HTTP debugging)
  - `password` or `passwd` fields
  - `code_verifier` (PKCE verifier sent during token exchange)
  - Base64-encoded credentials in `Authorization: Basic <base64>` headers
  - Tokens embedded in URLs as query parameters (e.g., `?access_token=...`)

### V7.1.3 - Redaction Is Not Automatic

- **Status:** Gap
- **Evidence:** `raps-kernel/src/logging.rs:141` (function definition)
- **Details:** The `redact_secrets()` function is available as a utility but is NOT automatically applied to all log output. It must be explicitly called at each call site. A search of the codebase for `redact_secrets` shows limited usage. The `tracing` framework does not have a redaction layer installed, meaning any `tracing::debug!`, `tracing::info!`, or `tracing::warn!` call could potentially emit sensitive data to both console and file logs.

  The `--debug` flag enables `tracing` at the debug level, which includes HTTP request/response details. While the current code avoids logging tokens directly, there is no structural guarantee that future code changes will maintain this discipline.

### V7.1.4 - Log File Retention and Rotation

- **Status:** Met
- **Evidence:** `raps-kernel/src/logging.rs:80,167-188`
- **Details:** Log rotation is implemented with:
  - **Daily rotation:** `tracing_appender::rolling::daily` creates a new log file each day (line 80)
  - **File count limit:** `cleanup_old_logs()` keeps at most 7 log files (line 78)
  - **Size limit:** Total log storage is capped at 50 MB (`MAX_LOG_BYTES` at line 167)
  - **Automatic cleanup:** Old files are removed by modification date (most recent first), with files beyond the count or size limit deleted

  This prevents unbounded disk usage from log accumulation.

### V7.1.5 - Non-Blocking File Logging

- **Status:** Met
- **Evidence:** `raps-kernel/src/logging.rs:81-84`
- **Details:** File logging uses `tracing_appender::non_blocking()` to avoid blocking the main execution thread on I/O. The `WorkerGuard` is stored in a global `Mutex` and can be explicitly flushed via the `flush()` function (lines 28-32).

### V7.2.1 - Error Information Leakage in Release Builds

- **Status:** Partial
- **Evidence:** `raps-kernel/src/auth/device_code.rs:195`, `raps-kernel/src/auth/three_leg.rs:271`, `raps-kernel/src/auth/two_leg.rs:68-72`
- **Details:** Error messages from API failures include raw server response text:
  ```rust
  anyhow::bail!("Token exchange failed ({status}): {error_text}");
  anyhow::bail!("Authentication failed with status {}: {}", status, error_text);
  ```
  In production, server error responses could contain internal details such as:
  - Stack traces from the OAuth server
  - Internal server hostnames or IP addresses
  - Request correlation IDs that could aid attackers

  The `anyhow` error chain propagates these messages to the user in all build profiles. There is no differentiation between debug and release builds for error verbosity.

  **Mitigating factors:** The Autodesk API servers generally return sanitized error messages, and the error text is shown only to the local CLI user, not to remote parties.

### V7.2.2 - Debug Mode Exposes Additional Information

- **Status:** Met (by design)
- **Evidence:** `raps-kernel/src/logging.rs:51-52`, `raps-kernel/src/http.rs:147-152`
- **Details:** When `--debug` is enabled:
  - HTTP response status codes and URLs are logged at debug level
  - Module targets are included in log output (`with_target(debug)` at line 65)
  - Full tracing spans are emitted

  This is appropriate since debug mode is explicitly opt-in and the documentation (`--debug: Include full trace (redacts secrets)`) sets expectations. However, the claim "redacts secrets" in the flag description is aspirational -- secrets are only redacted when `redact_secrets()` is explicitly called.

### V7.3.1 - Log Directory Permissions

- **Status:** Gap
- **Evidence:** `raps-kernel/src/logging.rs:77`
- **Details:** The log directory is created with `std::fs::create_dir_all(&log_dir)` without explicitly setting permissions. On Unix systems, the directory permissions depend on the user's umask. Log files may contain sensitive information (API URLs, error messages), so the log directory should be restricted to the owner.

## Summary

| Requirement | Status | Evidence |
|---|---|---|
| Logging framework and configuration | Met | `logging.rs:34-117` |
| Secret redaction patterns | Partial | `logging.rs:141-163` |
| Automatic redaction in log output | Gap | `logging.rs:141` |
| Log file retention and rotation | Met | `logging.rs:80,167-188` |
| Non-blocking file logging | Met | `logging.rs:81-84` |
| Error information leakage | Partial | `device_code.rs:195`, `three_leg.rs:271`, `two_leg.rs:68-72` |
| Debug mode documentation | Met | `logging.rs:51-52` |
| Log directory permissions | Gap | `logging.rs:77` |

## Redaction Coverage Matrix

| Secret Type | Pattern Covered | Notes |
|---|---|---|
| `client_secret: <value>` | Yes | Case-insensitive, supports `_` and `-` |
| `api_key: <value>` | Yes | Case-insensitive |
| `secret_key: <value>` | Yes | Case-insensitive |
| `access_token: <value>` | Yes | Value must be >= 20 chars |
| `refresh_token: <value>` | Yes | Value must be >= 20 chars |
| `bearer: <value>` | Yes | Value must be >= 20 chars |
| `Authorization: Bearer <token>` | No | HTTP header format not matched |
| `Authorization: Basic <base64>` | No | Basic auth header not covered |
| `password: <value>` | No | Not in pattern list |
| `code_verifier: <value>` | No | PKCE verifier not covered |
| Token in URL query parameter | No | URL-embedded tokens not covered |

## Recommendations

1. **Implement a tracing redaction layer (High Priority):** Create a custom `tracing_subscriber::Layer` that intercepts all log events and applies `redact_secrets()` (or an enhanced version) before forwarding to the file and console subscribers. This provides defense-in-depth and eliminates reliance on individual call sites.

2. **Expand redaction patterns:** Add coverage for:
   - `Authorization: Bearer <token>` (HTTP header format)
   - `Authorization: Basic <base64>` (Basic auth headers)
   - `password` / `passwd` fields
   - `code_verifier` values
   - Tokens embedded in URLs (`?access_token=...` or `?token=...`)

3. **Set log directory permissions:** After `create_dir_all`, set directory permissions to `0o700` on Unix:
   ```rust
   #[cfg(unix)]
   {
       use std::os::unix::fs::PermissionsExt;
       let perms = std::fs::Permissions::from_mode(0o700);
       std::fs::set_permissions(&log_dir, perms).ok();
   }
   ```

4. **Sanitize error messages in production:** Consider truncating or redacting server error response bodies before including them in error messages. At minimum, apply `redact_secrets()` to error text from API responses.

5. **Correct the `--debug` flag description:** Update the help text from "redacts secrets" to something like "enables verbose output (sensitive data may be present)" to accurately reflect current behavior, or implement the automatic redaction layer to make the description accurate.
