# ASVS V2 Audit: Authentication

**Audit Date:** 2026-02-28
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Scope

Audit of the RAPS authentication subsystem covering:
- PKCE implementation (`raps-kernel/src/auth/device_code.rs`)
- 3-legged OAuth authorization code flow (`raps-kernel/src/auth/three_leg.rs`)
- 2-legged OAuth client credentials flow (`raps-kernel/src/auth/two_leg.rs`)
- Token lifecycle management (`raps-kernel/src/auth/token_ops.rs`)
- Token storage backends (`raps-kernel/src/storage.rs`)
- Auth client structure (`raps-kernel/src/auth/mod.rs`, `raps-kernel/src/auth/types.rs`)

## Findings

### V2.1.1 - PKCE Code Verifier Uses Cryptographically Secure Random Source

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/device_code.rs:25-33`
- **Details:** The `generate_code_verifier()` function uses `rand::thread_rng()` which delegates to the OS CSPRNG (`getrandom` on Linux/macOS, `BCryptGenRandom` on Windows). The verifier is 128 characters drawn from the RFC 7636 section 4.1 unreserved character set (`[A-Za-z0-9\-._~]`). The implementation is well within the RFC-mandated 43-128 character range. Unit tests verify length, charset, and uniqueness across invocations.

### V2.1.2 - PKCE S256 Challenge Derivation

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/device_code.rs:37-39`
- **Details:** The code challenge is derived using SHA-256 (`sha2::Sha256`) and Base64-URL-encoded without padding (`URL_SAFE_NO_PAD`), matching RFC 7636 section 4.2 exactly. A known test vector from RFC 7636 Appendix B is verified at line 253-258.

### V2.1.3 - State Parameter Validates CSRF on Callback (Device/PKCE Flow)

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/device_code.rs:57` (generation), `raps-kernel/src/auth/device_code.rs:117-123` (validation)
- **Details:** A UUID v4 state parameter is generated per login attempt. When the user pastes back a full callback URL, the state parameter is extracted and compared against the original. On mismatch, the flow is aborted with "State mismatch -- possible CSRF attack." If the user pastes a bare authorization code (no URL), the state check is necessarily skipped since there is no callback URL to parse.

### V2.1.4 - State Parameter Validates CSRF on Callback (Browser Flow)

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/three_leg.rs:80` (generation), `raps-kernel/src/auth/three_leg.rs:196-203` (validation)
- **Details:** The browser-based 3-legged OAuth flow generates a UUID v4 state and includes it in the authorization URL. When the local HTTP server receives the callback, the returned state is validated against the original. On mismatch, a 400 response is returned to the browser and the flow is aborted with "State mismatch - possible CSRF attack". This is robust because the state never leaves the process memory.

### V2.1.5 - Token Refresh Uses Proper Concurrency Control

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/mod.rs:34` (Mutex declaration), `raps-kernel/src/auth/three_leg.rs:20-58` (refresh coordination)
- **Details:** The 3-legged token cache is protected by `tokio::sync::Mutex`, and a `refreshing` boolean flag prevents concurrent refresh requests. When one task is refreshing, other callers yield (sleep 100ms + loop). On success or failure, the `refreshing` flag is always reset. The 2-legged token cache uses `tokio::sync::RwLock` with multiple-reader/single-writer semantics. Note: There is a theoretical TOCTOU window between checking `cache.refreshing` and setting it, but since both operations occur within the same `lock().await` scope, this is safe under Tokio's single-lock guarantee.

### V2.1.6 - Token Expiry Check Includes Safety Margin

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/types.rs:56-58`
- **Details:** The `CachedToken::is_valid()` method for 2-legged tokens checks `expires_at > Instant::now() + Duration::from_secs(60)`, providing a 60-second safety margin to avoid race conditions with near-expired tokens.

### V2.1.7 - File-Based Token Storage Sets Restrictive Permissions

- **Status:** Met
- **Evidence:** `raps-kernel/src/storage.rs:158-163`
- **Details:** On Unix systems, after writing the token file, permissions are set to `0o600` (read/write for owner only) using `std::os::unix::fs::PermissionsExt`. The directory creation (`create_dir_all`) does not explicitly set restrictive permissions on the parent directory, relying on the user's umask.

### V2.1.8 - Default Token Storage Uses OS Keychain

- **Status:** Met
- **Evidence:** `raps-kernel/src/storage.rs:23-51`
- **Details:** The `StorageBackend::from_env()` defaults to `StorageBackend::Keychain`, which uses the `keyring` crate (Windows Credential Manager, macOS Keychain, Linux Secret Service). File-based storage is only enabled when explicitly configured via profile (`use_keychain: false`) or environment variable (`RAPS_USE_FILE_STORAGE=true`). When file storage is active, a warning is logged. The keychain backend gracefully falls back to file storage when the keyring is unavailable (e.g., headless CI/CD servers).

### V2.1.9 - Credentials Sent via HTTP Basic Auth

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/device_code.rs:185`, `raps-kernel/src/auth/three_leg.rs:261`, `raps-kernel/src/auth/two_leg.rs:58`
- **Details:** All token exchange and refresh requests send `client_id` and `client_secret` via HTTP Basic Authentication (`.basic_auth()`), not as form body parameters. This is the recommended OAuth 2.0 approach per RFC 6749 section 2.3.1.

### V2.1.10 - No Tokens Appear in Logs

- **Status:** Partial
- **Evidence:** `raps-kernel/src/logging.rs:141-163` (redaction), `raps-kernel/src/auth/three_leg.rs:165` (debug log)
- **Details:** The `redact_secrets()` function redacts patterns matching `client_secret`, `secret_key`, `api_key`, `token`, `access_token`, `refresh_token`, and `bearer` with values >= 20 characters. However, this redaction function must be explicitly called by the caller -- it is not automatically applied to all tracing output. The tracing framework logs HTTP URLs and status codes at debug level (e.g., `raps-kernel/src/http.rs:148-152`), which could include tokens in URL query parameters if they were ever placed there (they are not in the current code, but this is not structurally prevented). Auth code exchange error messages at `raps-kernel/src/auth/device_code.rs:195` include the full error text from the server, which could potentially contain token fragments.

### V2.1.11 - Credential Validation Before Use

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/device_code.rs:50`, `raps-kernel/src/auth/three_leg.rs:78`, `raps-kernel/src/auth/two_leg.rs:42`
- **Details:** All auth flows call `self.config.require_credentials()` at the start, which validates that `client_id` and `client_secret` are non-empty before attempting any network requests.

### V2.1.12 - Login-with-Token Validates Token

- **Status:** Met
- **Evidence:** `raps-kernel/src/auth/token_ops.rs:58-59`
- **Details:** The `login_with_token()` method (for CI/CD scenarios) validates the provided access token by calling the `/userinfo` endpoint before storing it. Invalid tokens are rejected with an error.

## Summary

| Requirement | Status | Evidence |
|---|---|---|
| PKCE code_verifier uses CSPRNG | Met | `device_code.rs:25-33` |
| PKCE S256 challenge derivation | Met | `device_code.rs:37-39` |
| State parameter CSRF (device flow) | Met | `device_code.rs:57,117-123` |
| State parameter CSRF (browser flow) | Met | `three_leg.rs:80,196-203` |
| Token refresh concurrency control | Met | `three_leg.rs:20-58`, `mod.rs:34` |
| Token expiry safety margin | Met | `types.rs:56-58` |
| File storage restrictive permissions | Met | `storage.rs:158-163` |
| Default to OS keychain | Met | `storage.rs:23-51` |
| Credentials via HTTP Basic Auth | Met | `device_code.rs:185`, `three_leg.rs:261`, `two_leg.rs:58` |
| No tokens in logs | Partial | `logging.rs:141-163` |
| Credential validation before use | Met | `device_code.rs:50`, `three_leg.rs:78`, `two_leg.rs:42` |
| Login-with-token validates token | Met | `token_ops.rs:58-59` |

## Recommendations

1. **Automatic log redaction layer:** Consider implementing a custom `tracing` layer that automatically applies `redact_secrets()` to all log events before they reach any subscriber (file or stderr). This would provide defense-in-depth against accidental token leakage in log output, removing reliance on individual call sites to invoke the redaction function.

2. **Parent directory permissions:** When creating the token storage directory (`create_dir_all`), explicitly set directory permissions to `0o700` on Unix systems rather than relying on the user's umask.

3. **Bare authorization code bypass:** In the PKCE device flow, when a user pastes a bare authorization code (not a URL), the CSRF state check is skipped. Consider documenting this tradeoff or requiring the full URL to maintain CSRF protection.

4. **Error message sanitization:** Error messages from token exchange failures (`device_code.rs:195`, `three_leg.rs:271`) include raw server response text. Consider redacting these before display to prevent accidental exposure of sensitive data in error responses.
