# SOC 2 Trust Services Criteria — Self-Assessment

**Last Updated:** 2026-03-01
**RAPS Version:** 4.14.1
**Scope:** RAPS CLI tool and supporting libraries
**Note:** This is a self-assessment mapping RAPS controls to TSC principles, not a formal SOC 2 Type II audit.

---

## Summary Matrix

| Category | Full Name | Criteria Assessed | Met | Partial | N/A |
|----------|-----------|-------------------|-----|---------|-----|
| **CC** | Security (Common Criteria) | 13 | 12 | 1 | 0 |
| **A** | Availability | 2 | 2 | 0 | 0 |
| **PI** | Processing Integrity | 3 | 3 | 0 | 0 |
| **C** | Confidentiality | 2 | 2 | 0 | 0 |
| **P** | Privacy | 8 | 8 | 0 | 0 |
| | **Total** | **28** | **27** | **1** | **0** |

**Overall Self-Assessment: 96% Met, 4% Partial**

---

## CC — Security (Common Criteria)

| Criteria ID | Criteria | Status | Control Description | Evidence |
|-------------|----------|--------|---------------------|----------|
| CC1.1 | Integrity and ethical values | Met | RAPS is open source under the Apache 2.0 license. A security policy with responsible disclosure process is published. A Code of Conduct governs community participation. | `LICENSE` (Apache-2.0), `SECURITY.md`, `CODE_OF_CONDUCT.md` |
| CC2.1 | Internal communication of objectives | Met | All changes are documented in the changelog. Release notes accompany every version. GitHub Discussions provide a public channel for design decisions and user communication. | `CHANGELOG.md`, GitHub Releases, GitHub Discussions |
| CC3.1 | Risk assessment processes | Met | A comprehensive ASVS Level 2 audit (CLI-scoped) has been completed across 9 chapters covering authentication, input validation, cryptography, logging, data protection, communications, files/resources, and configuration. Threat modeling informs the security hardening plan. | `docs/security/asvs-l2-compliance-matrix.md` (94% Met), `docs/plans/2026-02-28-security-hardening-plan.md`, `docs/plans/2026-02-28-security-hardening-design.md` |
| CC4.1 | Monitoring activities | Met | Continuous automated monitoring is in place: `cargo-audit` checks for known vulnerabilities in dependencies, Dependabot opens PRs for outdated dependencies, CodeQL performs static analysis for security flaws, Semgrep runs custom and community rules, and OpenSSF Scorecard tracks supply-chain security posture. | `.github/workflows/ci.yml` (cargo-audit, cargo-deny), `.github/workflows/codeql.yml`, `.github/workflows/semgrep.yml`, `.github/workflows/scorecard.yml`, Dependabot configuration |
| CC5.1 | Control activities over security | Met | CI/CD gates enforce quality and security checks before merge. All pull requests must pass clippy linting, the full test suite (488+ tests across 3 OS), security scans (cargo-audit, gitleaks, cargo-deny), and code coverage thresholds. Branch protection rules require these checks. | `.github/workflows/ci.yml`, `docs/BRANCH_PROTECTION.md` |
| CC6.1 | Logical access controls | Met | Credentials are stored in the OS keyring by default (macOS Keychain, Windows DPAPI, Linux Secret Service). Fallback file-based storage uses restrictive permissions (mode `0o600` for token files, mode `0o700` for directories). Authorization is enforced via APS API scopes configured per profile. | `raps-kernel/src/storage.rs` (keyring integration, permission enforcement), `raps-kernel/src/security.rs` (`create_dir_restricted`), `docs/security/asvs-l2-compliance-matrix.md` (V2.5, V2.6, V8.1, V8.2) |
| CC6.2 | Authentication mechanisms | Met | OAuth 2.0 with PKCE (S256 challenge method) is used for all authentication flows, compliant with RFC 7636. The PKCE verifier is generated using cryptographically secure randomness (`rand::thread_rng()` backed by ChaCha12Rng). Token refresh operations use mutex-based coordination to prevent race conditions. CSRF state parameter (UUID v4) is validated on callback. | `raps-kernel/src/auth/device_code.rs` (PKCE implementation), `raps-kernel/src/auth/token_ops.rs` (mutex coordination), `docs/security/asvs-v2-auth-audit.md` |
| CC6.3 | Authorization enforcement | Met | API scopes are configured per profile, ensuring least-privilege access. Users select only the scopes their workflow requires. Profile-level configuration prevents scope creep across different APS applications. | `docs/configuration.md`, per-profile scope configuration |
| CC6.6 | System boundaries and external access | Met | All API communication uses TLS exclusively via `rustls` (memory-safe TLS implementation). Native TLS/OpenSSL is explicitly excluded (`default-features = false`). A domain allowlist restricts outbound connections to known Autodesk API domains with proper subdomain boundary validation. The only HTTP endpoint is the localhost OAuth callback (`127.0.0.1`), which does not traverse the network. No `danger_accept_invalid_certs` or `danger_accept_invalid_hostnames` calls exist in the codebase. | `Cargo.toml` (rustls-tls feature), `raps-kernel/src/http.rs:15-45` (domain allowlist), `raps-kernel/src/http.rs:76-82` (TLS validation), `docs/security/asvs-v6-v9-crypto-comms-audit.md` |
| CC7.1 | Change management | Met | All code changes require pull requests with peer review. Branch protection rules enforce required status checks (CI, tests, security scans). Direct pushes to the main branch are blocked. | `docs/BRANCH_PROTECTION.md`, `.github/workflows/branch-protection-check.yml`, GitHub repository settings |
| CC7.2 | Infrastructure and supply-chain controls | Met | GitHub Actions are pinned to SHA digests to prevent supply-chain attacks via mutable tags. Publishing workflows use OIDC-based authentication (no long-lived secrets). Dependabot monitors dependency freshness. Dependency licensing is validated via `cargo-deny`. | `.github/workflows/ci.yml` (SHA-pinned actions), `.github/workflows/publish.yml` (OIDC publishing), `.github/workflows/release.yml`, `docs/plans/2026-02-28-security-hardening-plan.md` (Task 1-2: SHA pinning) |
| CC8.1 | Incident response | Partial | A security policy documents the vulnerability reporting process (GitHub Security Advisories or email) with a 48-hour initial response SLA and 7-day status update commitment. GitHub Security Advisories are used for coordinated disclosure. However, there is no formally documented incident response runbook beyond the SECURITY.md policy. | `SECURITY.md` (reporting process, response timelines), GitHub Security Advisories tab |
| CC9.1 | Recovery and resilience | Met | RAPS is a local-first CLI tool with no server-side state. All processing is transient. If access is interrupted, token re-authentication restores full functionality. No data migration or state recovery is required. | Local-first architecture, stateless design |

---

## A — Availability

| Criteria ID | Criteria | Status | Control Description | Evidence |
|-------------|----------|--------|---------------------|----------|
| A1.1 | Infrastructure supports availability | Met | RAPS is a CLI tool that runs entirely on the user's local machine. There is no hosted service component, no server-side infrastructure, and no availability SLA to maintain. The tool depends on APS API availability for remote operations, which is governed by Autodesk's own SLAs. | Local-only execution model, no server infrastructure |
| A1.2 | Recovery mechanisms | Met | RAPS has a stateless design with no persistent state to recover. Re-installation is available through 6 independent distribution channels: `cargo install raps` (crates.io), `npm install -g @niceygy/raps` (npm), `pip install raps-cli` (PyPI), `brew install dmytro-yemelianov/tap/raps` (Homebrew), `scoop install raps` (Scoop), and `docker pull` (Docker Hub). Binary integrity is verifiable via SHA256 checksums. | `docs/installation.md`, `docs/cli/checksums.md`, `.github/workflows/publish.yml`, `.github/workflows/release.yml` |

---

## PI — Processing Integrity

| Criteria ID | Criteria | Status | Control Description | Evidence |
|-------------|----------|--------|---------------------|----------|
| PI1.1 | Input validation | Met | ASVS V5 (Input Validation) is 100% Met in the compliance matrix. Controls include: URL validation with SSRF prevention via domain allowlist; path traversal protection using `sanitize_filename` and `validate_path_within`; CSV input validation with email format checking, required field enforcement, and error aggregation; pipeline execution safety with shell metacharacter validation and `shlex` quoting; URL encoding of object keys in API calls; and strict filter expression parsing with known-key-only validation. | `docs/security/asvs-l2-compliance-matrix.md` (V5.1-V5.6, all Met), `docs/security/asvs-v5-v12-input-files-audit.md`, `raps-kernel/src/http.rs:15-45`, `raps-kernel/src/security.rs`, `raps-cli/src/commands/object/download.rs`, `raps-cli/src/commands/admin/csv_ops.rs`, `raps-cli/src/commands/pipeline.rs` |
| PI1.2 | Processing quality assurance | Met | The test suite contains 488+ unit and integration tests. Tests run in a matrix across 3 operating systems (Linux, macOS, Windows). Fuzzing is configured via `cargo-fuzz` with continuous fuzzing in CI. Code coverage is measured with `llvm-cov` and reported to Codecov. The CI pipeline enforces that all tests must pass before merge. | `tests/` directory, `.github/workflows/ci.yml` (nextest runner, matrix strategy), `.github/workflows/fuzz.yml`, `fuzz/Cargo.toml`, `docs/CODE_COVERAGE_PLAN.md` |
| PI1.3 | Error handling and output integrity | Met | RAPS uses structured tracing (`tracing` + `tracing-subscriber`) for all logging. Automatic secret redaction covers 6 pattern categories: client secrets, API keys, access tokens, refresh tokens, bearer tokens, and authorization headers. All error messages pass through `redact_secrets()` before output. Typed exit codes (`ExitCode`) provide machine-parseable error classification. Non-blocking file logging ensures error handling does not impact CLI performance. | `raps-kernel/src/logging.rs:141-221` (redaction patterns), `raps-kernel/src/logging.rs:34-117` (structured logging), `docs/cli/exit-codes.md`, `docs/security/asvs-v7-logging-audit.md` |

---

## C — Confidentiality

| Criteria ID | Criteria | Status | Control Description | Evidence |
|-------------|----------|--------|---------------------|----------|
| C1.1 | Confidential data identification and protection | Met | Credentials (client secrets, access tokens, refresh tokens) are stored in the OS keyring, which provides platform-native encryption (macOS Keychain, Windows DPAPI, Linux Secret Service/libsecret). File-based fallback storage uses mode `0o600` permissions. Log output is automatically redacted via `RedactingMakeWriter`, which intercepts all log layers and applies 6 regex-based pattern categories to prevent credential leakage. Log directories are created with mode `0o700`. | `raps-kernel/src/storage.rs` (keyring storage), `raps-kernel/src/logging.rs` (`RedactingMakeWriter`, `redact_secrets()`), `raps-kernel/src/security.rs` (`create_dir_restricted`), `docs/security/asvs-v7-logging-audit.md` |
| C1.2 | Data disposal and retention controls | Met | Token revocation is supported to invalidate credentials. Log files are automatically rotated daily with a 7-file retention limit and 50MB total size cap, ensuring logs do not accumulate indefinitely. RAPS performs no persistent caching of API responses or user data. All data processing is transient and in-memory. | `raps-kernel/src/logging.rs:80,167-188` (rotation configuration), stateless processing architecture |

---

## P — Privacy

| Criteria ID | Criteria | Status | Control Description | Evidence |
|-------------|----------|--------|---------------------|----------|
| P1.1 | Privacy notice | Met | The project's distribution specification explicitly states a no-telemetry, privacy-first policy. Privacy implications of APS API usage are governed by Autodesk's privacy policy, which is referenced in Autodesk's platform documentation. | `specs/006-raps-distribution/spec.md` (privacy-first policy), Autodesk Platform Services privacy policy |
| P2.1 | Consent mechanisms | Met | Every CLI invocation is an explicit user action. RAPS does not perform any background operations, scheduled tasks, or unsolicited network requests. The user initiates and controls all data processing. | CLI invocation model (explicit user action required) |
| P3.1 | Data collection minimization | Met | RAPS collects no telemetry, no analytics, and no usage statistics. The only data transmitted is what the user explicitly requests via CLI commands. The distribution specification documents this as a design decision: "No telemetry - privacy-first, rely on user-reported issues." | `specs/006-raps-distribution/spec.md` ("No telemetry"), no analytics dependencies in `Cargo.toml` |
| P4.1 | Use, retention, and disposal | Met | All data processing is transient and in-memory. API responses are displayed or written to user-specified output files and then discarded. Local logs auto-rotate with a 7-day, 50MB retention policy. No persistent data store exists beyond the credential keyring and configuration files. | `raps-kernel/src/logging.rs` (rotation policy), stateless architecture |
| P5.1 | User access to data | Met | All RAPS data is stored locally on the user's machine. Configuration files, logs, and credentials are in user-accessible directories (`~/.config/raps/`, `~/.local/share/raps/`). The user has full filesystem-level control over all stored data, including the ability to inspect, modify, or delete any file. | `directories` crate (platform-standard paths), local-only storage model |
| P6.1 | Disclosure to third parties | Met | RAPS does not share user data with any third party. API calls are made directly to Autodesk Platform Services endpoints (governed by Autodesk's privacy policy and the domain allowlist). No intermediary services, proxies, or analytics endpoints receive user data. | `raps-kernel/src/http.rs:15-22` (domain allowlist limited to Autodesk domains), no third-party analytics |
| P7.1 | Data quality and integrity | Met | Data integrity during transmission is ensured by TLS (rustls with certificate validation). Release binary integrity is verifiable via SHA256 checksums published alongside each release. SBOM (Software Bill of Materials) is generated for supply-chain transparency. | `docs/cli/checksums.md`, `docs/sbom.md`, TLS enforcement via rustls |
| P8.1 | Complaints and dispute resolution | Met | Security and privacy concerns can be reported through GitHub Issues for general matters or through the coordinated vulnerability disclosure process (GitHub Security Advisories, email to security@autodesk.com) for security-sensitive matters. The SECURITY.md policy documents response timelines. | `SECURITY.md` (reporting channels, response SLAs), GitHub Issues |

---

## Key Evidence References

| Document | Path | Description |
|----------|------|-------------|
| Security Policy | `SECURITY.md` | Vulnerability reporting, response SLAs, security best practices |
| ASVS L2 Compliance Matrix | `docs/security/asvs-l2-compliance-matrix.md` | 34-criteria audit, 94% Met |
| Auth Audit (V2) | `docs/security/asvs-v2-auth-audit.md` | OAuth2 PKCE, token storage, credential security |
| Input/Files Audit (V5/V12) | `docs/security/asvs-v5-v12-input-files-audit.md` | URL validation, path traversal, CSV, pipeline safety |
| Crypto/Comms Audit (V6/V9) | `docs/security/asvs-v6-v9-crypto-comms-audit.md` | TLS enforcement, domain allowlist, crypto algorithms |
| Logging Audit (V7) | `docs/security/asvs-v7-logging-audit.md` | Structured logging, secret redaction, log rotation |
| Plugin Trust Model | `docs/security/plugin-trust-model.md` | Plugin execution model and trust boundaries |
| Security Hardening Plan | `docs/plans/2026-02-28-security-hardening-plan.md` | ASVS L2, SLSA Build L2, OpenSSF Scorecard targets |
| Security Hardening Design | `docs/plans/2026-02-28-security-hardening-design.md` | Architecture for CI/CD hardening, code audit, scoring |
| Branch Protection | `docs/BRANCH_PROTECTION.md` | Required checks, review policies |
| Exit Codes | `docs/cli/exit-codes.md` | Typed error classification |
| Checksums | `docs/cli/checksums.md` | SHA256 verification for release binaries |
| SBOM | `docs/sbom.md` | Software Bill of Materials |
| License | `LICENSE` | Apache License 2.0 |
| Code of Conduct | `CODE_OF_CONDUCT.md` | Community standards |
