# ASVS L2 Compliance Matrix (CLI-Scoped)

**Last Updated:** 2026-02-28
**RAPS Version:** 4.14.1
**ASVS Version:** 4.0.3

## Summary

| Chapter | Total Checked | Met | Partial | N/A | Gap |
|---------|---------------|-----|---------|-----|-----|
| V2 Authentication | 6 | 6 | 0 | 0 | 0 |
| V5 Validation | 6 | 6 | 0 | 0 | 0 |
| V6 Cryptography | 3 | 3 | 0 | 0 | 0 |
| V7 Error/Logging | 6 | 6 | 0 | 0 | 0 |
| V8 Data Protection | 2 | 2 | 0 | 0 | 0 |
| V9 Communications | 3 | 3 | 0 | 0 | 0 |
| V10 Malicious Code | 2 | 0 | 1 | 0 | 1 |
| V12 Files/Resources | 3 | 3 | 0 | 0 | 0 |
| V14 Configuration | 3 | 3 | 0 | 0 | 0 |
| **Total** | **34** | **32** | **1** | **0** | **1** |

**Compliance Rate: 94% Met, 3% Partial, 3% Gap**

## V2 — Authentication

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V2.1 | OAuth uses PKCE (S256) | Met | `auth/device_code.rs:37-39` | RFC 7636 compliant, test vector verified |
| V2.2 | PKCE verifier is cryptographically random | Met | `auth/device_code.rs:25-27` | Uses `rand::thread_rng()` (ChaCha12Rng) |
| V2.3 | CSRF state parameter validated on callback | Met | `auth/device_code.rs:118-123` | UUID v4 state, mismatch = bail |
| V2.4 | Token refresh uses concurrency control | Met | `auth/token_ops.rs` | Mutex-based coordination |
| V2.5 | Credentials stored securely | Met | `storage.rs` | OS keyring default, file fallback warns |
| V2.6 | File token storage has restrictive permissions | Met | `storage.rs:158-163` | Unix: mode 0o600 set explicitly; Windows: user-only ACL via DPAPI |

See: `docs/security/asvs-v2-auth-audit.md`

## V5 — Input Validation

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V5.1 | URL validation / SSRF prevention | Met | `http.rs:15-45` | Domain allowlist with subdomain boundary checks |
| V5.2 | URL encoding in API calls | Met | `objects.rs:331,366,412` | `urlencoding::encode()` for object keys |
| V5.3 | CSV input validated | Met | `csv_ops.rs:72-116` | Email, required fields, error aggregation |
| V5.4 | Pipeline execution safety | Met | `pipeline.rs:179-182,265` | Shell metachar validation + shlex quoting |
| V5.5 | Download path traversal protection | Met | `security.rs`, `download.rs:66` | sanitize_filename + validate_path_within |
| V5.6 | Filter expression parsing safety | Met | `filter.rs:77-163` | Strict key-value parsing, known keys only |

See: `docs/security/asvs-v5-v12-input-files-audit.md`

## V6 — Cryptography

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V6.1 | Strong crypto algorithms only | Met | Crypto inventory | SHA-256, TLS 1.2+, no deprecated algos |
| V6.2 | PKCE S256 implementation correct | Met | `device_code.rs:37-39` | sha2 crate, RFC test vector passes |
| V6.3 | No hardcoded crypto keys | Met | Codebase search | No hardcoded keys found |

See: `docs/security/asvs-v6-v9-crypto-comms-audit.md`

## V7 — Error Handling & Logging

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V7.1 | Structured logging framework | Met | `logging.rs:34-117` | tracing + tracing-subscriber |
| V7.2 | Secret redaction patterns | Met | `logging.rs:141-221` | Covers tokens, keys, auth headers, cookies, URL params |
| V7.3 | Automatic redaction in log output | Met | `logging.rs:RedactingMakeWriter` | All log layers use RedactingMakeWriter |
| V7.4 | Log rotation and retention | Met | `logging.rs:80,167-188` | Daily rotation, 7 file limit, 50MB cap |
| V7.5 | Error info leakage in release builds | Met | `auth/*.rs` | All bail!() error text passed through redact_secrets() |
| V7.6 | Non-blocking file logging | Met | `logging.rs:81-84` | tracing_appender::non_blocking |

See: `docs/security/asvs-v7-logging-audit.md`

## V8 — Data Protection

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V8.1 | Token-at-rest encryption | Met | `storage.rs` | OS keyring (DPAPI/Keychain/SecretService) |
| V8.2 | Log directory permissions | Met | `security.rs:create_dir_restricted` | Mode 0o700 on Unix for log and config dirs |

## V9 — Communications

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V9.1 | TLS via rustls (no native-tls) | Met | `Cargo.toml:49` | `default-features = false, rustls-tls` |
| V9.2 | TLS certificate validation enabled | Met | `http.rs:76-82` | No danger_accept_invalid_certs |
| V9.3 | No plaintext HTTP for APIs | Met | `http.rs:15-22` | Only localhost OAuth callback is HTTP |

See: `docs/security/asvs-v6-v9-crypto-comms-audit.md`

## V10 — Malicious Code / Plugin System

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V10.1 | Plugin trust model documented | Partial | `plugins.rs` | Plugins run with full user permissions, no docs |
| V10.2 | Plugin signature verification | Gap | — | ed25519-dalek in deps but not integrated |

See: `docs/security/plugin-trust-model.md`

## V12 — Files & Resources

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V12.1 | Streaming downloads (no memory exhaustion) | Met | `objects.rs:174-184` | bytes_stream() with chunked writes |
| V12.2 | Pagination safety | Met | `objects.rs:242-243` | MAX_PAGES = 100 hard limit |
| V12.3 | Path traversal in downloads | Met | `security.rs`, `download.rs:66` | sanitize_filename strips traversal + validate_path_within |

## V14 — Configuration

| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V14.1 | No secrets in build artifacts | Met | Codebase search | No hardcoded secrets in source |
| V14.2 | .env.example contains no real values | Met | `.env.example` | Placeholder values only |
| V14.3 | CI secrets scoped minimally | Met | Workflow analysis | Secrets only in jobs that need them |

## Remediation Priority

| Priority | Gap | Impact | Status |
|----------|-----|--------|--------|
| P1 | Download path traversal (V5.5/V12.3) | Arbitrary file write | **Fixed** — `security.rs` |
| P1 | Automatic log redaction (V7.3) | Credential leak in logs | **Fixed** — `RedactingMakeWriter` |
| P2 | Log directory permissions (V8.2) | Logs readable by other users | **Fixed** — `create_dir_restricted` |
| P2 | Plugin signature verification (V10.2) | Malicious plugin execution | Open |
| P3 | Pipeline variable injection (V5.4) | Argument injection via variables | **Fixed** — metachar validation + shlex |
