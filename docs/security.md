# Security

RAPS is built with security as a first-class concern. As a CLI tool that handles OAuth credentials, API tokens, and user data, every layer — from token storage to log output — is designed to protect sensitive information.

## Compliance & Frameworks

### OWASP ASVS Level 2

RAPS has been audited against the [OWASP Application Security Verification Standard](https://owasp.org/www-project-application-security-verification-standard/) (ASVS) v4.0.3 at **Level 2** — the recommended level for applications that handle sensitive data.

| Chapter | Scope | Status |
|---------|-------|--------|
| V2 Authentication | OAuth PKCE, token storage, credential handling | **Met** (6/6) |
| V5 Input Validation | URL validation, path traversal, pipeline safety | **Met** (6/6) |
| V6 Cryptography | SHA-256, TLS 1.2+, no deprecated algorithms | **Met** (3/3) |
| V7 Error Handling & Logging | Structured logging, secret redaction, rotation | **Met** (6/6) |
| V8 Data Protection | Token-at-rest encryption, directory permissions | **Met** (2/2) |
| V9 Communications | TLS via rustls, certificate validation | **Met** (3/3) |
| V10 Malicious Code | Plugin trust model, signature verification | **Met** (2/2) |
| V12 Files & Resources | Streaming downloads, pagination limits, path safety | **Met** (3/3) |
| V14 Configuration | No hardcoded secrets, minimal CI scoping | **Met** (3/3) |
| **Total** | | **34/34 — 100% Met** |

Full details: [ASVS L2 Compliance Matrix](security/asvs-l2-compliance-matrix.md)

### NIST SSDF (SP 800-218)

Mapped against the NIST Secure Software Development Framework covering organizational preparation, software protection, secure production, and vulnerability response.

**86% Met** across 49 assessed tasks.

Full details: [NIST SSDF Mapping](security/nist-ssdf-mapping.md)

### SOC 2 Trust Services Criteria

Self-assessed against SOC 2 TSC principles: Security, Availability, Processing Integrity, Confidentiality, and Privacy.

**96% Met** (27/28 criteria) across all five categories.

Full details: [SOC 2 Self-Assessment](security/soc2-tsc-self-assessment.md)

### GDPR

RAPS is a local-first tool with **no telemetry, no analytics, and no phone-home behavior**. A GDPR transparency notice documents all data categories, storage locations, retention policies, and the legal basis for processing.

Full details: [GDPR Privacy Statement](security/gdpr-privacy-statement.md)

## Security Architecture

### Authentication

- **OAuth 2.0 PKCE** (S256) for all browser-based flows — [RFC 7636](https://datatracker.ietf.org/doc/html/rfc7636) compliant
- **Device code flow** for headless/CI environments
- **Cryptographic randomness** via `rand::thread_rng()` (ChaCha12Rng) for PKCE verifiers and state parameters
- **CSRF protection** with UUID v4 state parameter validation
- **Mutex-based token refresh** to prevent concurrent refresh races

Audit details: [ASVS V2 Auth Audit](security/asvs-v2-auth-audit.md)

### Credential Storage

| Platform | Method |
|----------|--------|
| macOS | Keychain Services |
| Linux | Secret Service (GNOME Keyring / KDE Wallet) |
| Windows | DPAPI via Windows Credential Manager |
| Fallback | File-based with mode `0o600` (with user warning) |

### Transport Security

- **rustls** (no OpenSSL/native-tls dependency) — TLS 1.2+ only
- **Certificate validation** always enabled (no `danger_accept_invalid_certs`)
- **Domain allowlist** with subdomain boundary checks for SSRF prevention
- Only `localhost` OAuth callback uses HTTP; all API traffic is HTTPS

Audit details: [ASVS V6/V9 Crypto & Communications Audit](security/asvs-v6-v9-crypto-comms-audit.md)

### Input Validation & Path Safety

- **Path traversal protection**: `sanitize_filename()` strips `..` components, path separators, control characters
- **Defense-in-depth**: `validate_path_within()` canonicalizes and confirms paths stay within the expected base directory
- **URL encoding**: all API object keys passed through `urlencoding::encode()`
- **Pipeline safety**: shell metacharacter rejection + `shlex` quoting for variable substitution

Audit details: [ASVS V5/V12 Input & Files Audit](security/asvs-v5-v12-input-files-audit.md)

### Logging & Secret Redaction

- **Structured logging** via `tracing` + `tracing-subscriber`
- **Automatic redaction**: all log output passes through `RedactingMakeWriter` — tokens, client secrets, API keys, cookies, and URL parameters are stripped before reaching disk or stderr
- **Log rotation**: daily rotation, 7-file limit, 50 MB cap
- **Directory permissions**: log and config directories created with mode `0o700`

Audit details: [ASVS V7 Logging Audit](security/asvs-v7-logging-audit.md)

### Plugin Trust Model

RAPS plugins (`raps-<name>` executables on PATH) use a layered trust model:

1. **TOFU** (Trust On First Use) — `raps plugin trust <name>` records the SHA-256 hash
2. **Hash verification** — `raps plugin verify <name>` detects modifications
3. **Ed25519 signatures** — optional cryptographic signature verification for signed plugins
4. **Enable/disable** — plugins can be disabled without removal

Audit details: [Plugin Trust Model](security/plugin-trust-model.md)

## Reporting Vulnerabilities

If you discover a security vulnerability, please **do not** open a public issue.

**Preferred:** Use [GitHub Security Advisories](https://github.com/dmytro-yemelianov/raps/security) to report privately.

**Alternative:** Email [security@autodesk.com](mailto:security@autodesk.com) with subject `[APS CLI Security]`.

See the full [Security Policy](https://github.com/dmytro-yemelianov/raps/security/policy) for response timelines and scope.
