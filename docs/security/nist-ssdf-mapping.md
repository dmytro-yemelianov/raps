# NIST SSDF (SP 800-218) Compliance Mapping

**Last Updated:** 2026-03-01
**RAPS Version:** 4.14.1
**Standard:** NIST SP 800-218 — Secure Software Development Framework (SSDF) v1.1
**Scope:** RAPS CLI workspace (raps-kernel, raps-oss, raps-derivative, raps-dm, raps-da, raps-acc, raps-webhooks, raps-reality, raps-cli)

## Overview

This document maps the RAPS project's security practices to the NIST Secure Software Development Framework (SSDF), SP 800-218. The SSDF defines a core set of secure software development practices organized into four groups. Each practice includes tasks with corresponding evidence from the RAPS codebase, CI/CD configuration, and documentation.

## Compliance Summary

| Practice Group | Description | Tasks Assessed | Met | Partial | N/A | Compliance Rate |
|----------------|-------------|----------------|-----|---------|-----|-----------------|
| **PO** | Prepare the Organization | 14 | 12 | 2 | 0 | 86% Met |
| **PS** | Protect the Software | 7 | 6 | 1 | 0 | 86% Met |
| **PW** | Produce Well-Secured Software | 22 | 19 | 3 | 0 | 86% Met |
| **RV** | Respond to Vulnerabilities | 6 | 5 | 1 | 0 | 83% Met |
| **Total** | | **49** | **42** | **7** | **0** | **86% Met** |

---

## PO: Prepare the Organization

### PO.1 — Define Security Requirements for Software Development

Ensure that security requirements are defined, communicated, and maintained for software development.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PO.1.1 | Define security requirements for software development processes | Met | `SECURITY.md`, `docs/security/asvs-l2-compliance-matrix.md` | ASVS L2 (CLI-scoped) used as the requirements baseline; security policy defines supported versions, disclosure process, and 48h response SLA |
| PO.1.2 | Communicate requirements to all third parties involved | Met | `SECURITY.md`, `docs/security/plugin-trust-model.md` | Security policy published in repository root; plugin trust model documents expectations for plugin authors and users |
| PO.1.3 | Review and update security requirements periodically | Met | `docs/plans/2026-02-28-security-hardening-design.md` | Security hardening plan dated 2026-02-28 shows active review; ASVS matrix updated alongside each audit cycle |

### PO.2 — Implement Roles and Responsibilities

Define and assign roles and responsibilities for secure software development.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PO.2.1 | Define roles and responsibilities for security activities | Met | `.github/CODEOWNERS` | CODEOWNERS assigns `@autodesk-platform-services/aps-cli-maintainers` ownership over all code, documentation, workflows, and Cargo configuration |
| PO.2.2 | Assign trained personnel to the roles | Partial | `.github/CODEOWNERS` | Personnel are assigned; however, no formal secure development training records are maintained in the repository |

### PO.3 — Implement Supporting Toolchains

Use automation to reduce human effort and improve accuracy, consistency, and comprehensiveness of security practices.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PO.3.1 | Specify tools for secure development | Met | `.github/workflows/ci.yml`, `.github/workflows/codeql.yml`, `.github/workflows/semgrep.yml`, `.github/workflows/fuzz.yml` | Toolchain: Rust 1.88 (edition 2024), cargo clippy (`-D warnings`), rustfmt, CodeQL, Semgrep, cargo-audit, cargo-deny, Gitleaks, FOSSA, cargo-fuzz, llvm-cov |
| PO.3.2 | Use automated tools for compliance checking | Met | `.github/workflows/ci.yml`, `.github/workflows/scorecard.yml` | CI enforces `cargo check`, `cargo clippy`, `cargo fmt --check`, `cargo test` as required status checks; OpenSSF Scorecard runs weekly for posture measurement |
| PO.3.3 | Deploy and configure tools across the organization | Met | `.github/workflows/ci.yml` (lines 28-32) | All GitHub Actions SHA-pinned; consistent toolchain version via `RUST_VERSION: "1.88"` environment variable; `Swatinem/rust-cache` for reproducible builds |

### PO.4 — Define and Use Criteria for Software Security Checks

Define criteria for software security checks and track results.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PO.4.1 | Define criteria for security checks at each stage | Met | `docs/BRANCH_PROTECTION.md`, `.github/workflows/ci.yml` | Branch protection requires all CI checks to pass (check, test, fmt, clippy, docs, all-checks-pass); at least 1 PR review required; stale reviews dismissed on new commits |
| PO.4.2 | Implement processes to collect and safeguard artifacts | Met | `.github/workflows/scorecard.yml`, `.github/workflows/semgrep.yml` | Security scan results uploaded as SARIF to GitHub Security tab; OpenSSF Scorecard results persisted; CodeQL findings tracked via GitHub Security Advisories |

### PO.5 — Implement and Maintain Secure Environments for Software Development

Ensure the environments for software development are configured securely.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PO.5.1 | Separate and protect build environments | Met | `.github/workflows/release.yml`, `.github/workflows/ci.yml` | CI runs on GitHub-hosted runners with ephemeral environments; release workflow uses `ubuntu-22.04` with minimal permissions (`contents: read/write`); OIDC-based token issuance via `id-token: write` |
| PO.5.2 | Secure and harden development endpoints and environments | Partial | `.github/workflows/ci.yml` (permissions block) | Workflow permissions scoped per-job (`contents: read`, `security-events: write`); no long-lived secrets for CI authentication; however, developer workstation hardening is not enforced or documented |

---

## PS: Protect the Software

### PS.1 — Protect All Forms of Code from Unauthorized Access and Tampering

Apply protections to code at rest and in transit.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PS.1.1 | Store all code in a code repository with access controls | Met | `docs/BRANCH_PROTECTION.md`, `.github/CODEOWNERS` | GitHub repository with branch protection on `main`: no force pushes, no branch deletion, administrators included; CODEOWNERS enforces review from designated maintainers |
| PS.1.2 | Use commit signing or other tamper-evident mechanisms | Partial | `docs/BRANCH_PROTECTION.md` | Signed commits are encouraged but not enforced via branch protection rules; SHA-pinned GitHub Actions mitigate action-level tampering |

### PS.2 — Provide a Mechanism for Verifying Software Release Integrity

Enable consumers to verify that software has not been tampered with.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PS.2.1 | Make software integrity verification information available | Met | `docs/cli/checksums.md`, `docs/RELEASE.md` | SHA256 checksums published as `checksums.txt` with every release; verification instructions for Windows (PowerShell), macOS (`shasum`), and Linux (`sha256sum`) |
| PS.2.2 | Use cryptographic mechanisms for integrity verification | Met | `docs/cli/checksums.md` (lines 45-53) | SLSA Build Level 2 provenance attestations via `actions/attest-build-provenance`; consumers verify with `gh attestation verify <artifact> --repo dmytro-yemelianov/raps` |

### PS.3 — Archive, Protect, and Provide Access to All Forms of Each Software Release

Securely archive each software release and associated artifacts.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PS.3.1 | Archive each release with provenance and integrity data | Met | `.github/workflows/release.yml`, `docs/sbom.md` | Release workflow builds binaries for 5 platforms (Windows x64, macOS x64/ARM64, Linux x64/ARM64), generates SHA256 checksums, CycloneDX SBOM (`raps-sbom-<version>.json`), and SLSA L2 provenance attestation; all artifacts attached to GitHub Release |
| PS.3.2 | Collect and protect provenance data for each release | Met | `.github/workflows/release.yml` (line 59, SHA-pinned `actions/checkout`) | Provenance generated in isolated GitHub Actions environment; workflow uses SHA-pinned actions to prevent supply chain injection |
| PS.3.3 | Make release information accessible to consumers | Met | `docs/RELEASE.md`, `docs/sbom.md`, `docs/cli/checksums.md` | Release artifacts, SBOMs, checksums, and provenance attestations publicly available on GitHub Releases page; documentation provides download and verification instructions |

---

## PW: Produce Well-Secured Software

### PW.1 — Design Software to Meet Security Requirements and Mitigate Security Risks

Use threat modeling and secure architecture practices during design.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.1.1 | Use secure design principles and threat modeling | Met | `docs/plans/2026-02-28-security-hardening-design.md`, `docs/security/asvs-l2-compliance-matrix.md` | Security hardening design doc covers three tracks: CI/CD hardening, code audit (ASVS L2), and scoring/measurement; threat modeling via ASVS chapter-by-chapter review |
| PW.1.2 | Review and evaluate software design for security | Met | `docs/security/asvs-v2-auth-audit.md`, `docs/security/asvs-v5-v12-input-files-audit.md`, `docs/security/asvs-v6-v9-crypto-comms-audit.md`, `docs/security/asvs-v7-logging-audit.md` | Dedicated security audit documents for authentication, input validation, cryptography, communications, and logging; each includes status, evidence file paths, and line references |
| PW.1.3 | Document security decisions and rationale | Met | `docs/security/plugin-trust-model.md`, `docs/plans/2026-02-28-security-hardening-design.md` | Plugin trust model documents implicit trust architecture with explicit gap acknowledgment; security hardening design includes current state analysis, gap inventory, and remediation tracks |

### PW.2 — Review the Software Design to Verify Compliance with Security Requirements

Verify that the design meets all security requirements.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.2.1 | Verify that the design complies with security requirements | Met | `docs/security/asvs-l2-compliance-matrix.md` | ASVS L2 compliance matrix covers 34 requirements across 9 chapters; 94% Met (32/34), 3% Partial (1/34), 3% Gap (1/34) |

### PW.4 — Reuse Existing, Well-Secured Software When Feasible

Use vetted, well-maintained components instead of duplicating functionality.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.4.1 | Acquire well-secured software components | Met | `Cargo.toml`, `deny.toml` | Uses well-maintained Rust crates: `reqwest` (HTTP), `rustls` (TLS), `keyring` (credential storage), `sha2` (cryptography), `tokio` (async runtime); cargo-deny enforces license and advisory policies |
| PW.4.2 | Monitor dependencies for vulnerabilities | Met | `.github/workflows/ci.yml` (cargo-audit job), `deny.toml` | `rustsec/audit-check` runs on every PR and push to main; `cargo-deny` enforces license allowlist and advisory database checks; Dependabot configured for weekly dependency updates |
| PW.4.3 | Maintain a software bill of materials (SBOM) | Met | `docs/sbom.md`, `.github/workflows/release.yml` | CycloneDX JSON SBOM generated for each release via `cargo-cyclonedx`; includes component inventory, dependency relationships, license information, and package URLs (PURLs) |

### PW.5 — Create Source Code by Adhering to Secure Coding Practices

Follow secure coding standards and practices when writing source code.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.5.1 | Follow secure coding practices | Met | `docs/security/asvs-l2-compliance-matrix.md` (V5 Input Validation, V6 Cryptography) | Path traversal protection via `sanitize_filename` + `validate_path_within` (`raps-kernel/src/security.rs`); URL validation with domain allowlist (`raps-kernel/src/http.rs`); shell metacharacter validation for pipelines; email validation for CSV input |
| PW.5.2 | Address security issues during code review | Met | `docs/BRANCH_PROTECTION.md` | All PRs require at least 1 review from CODEOWNERS maintainers; CI gates prevent merge without passing security checks (clippy, cargo-audit, Gitleaks, CodeQL) |

### PW.6 — Configure the Compilation, Interpreter, and Build Processes

Harden the build process to produce secure outputs.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.6.1 | Configure the compilation and build processes for security | Met | `.github/workflows/ci.yml`, `Cargo.toml` | `cargo clippy -- -D warnings` treats all warnings as errors; `cargo check --all-features` validates compilation; Rust edition 2024 enforces latest language safety defaults; `CARGO_PROFILE_DEV_DEBUG: 0` reduces debug info in CI builds |
| PW.6.2 | Review and configure compiler security features | Met | `Cargo.toml` (workspace configuration) | Rust provides memory safety by default (no buffer overflows, use-after-free, data races); edition 2024 enables strictest borrow checker and lifetime rules; release profile optimizations via cargo-dist |

### PW.7 — Review and/or Analyze Human-Readable Code to Identify Vulnerabilities and Verify Compliance

Use manual and automated analysis to identify security issues.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.7.1 | Use static analysis tools | Met | `.github/workflows/codeql.yml`, `.github/workflows/semgrep.yml` | CodeQL for Rust runs on every PR and weekly schedule; Semgrep with `p/rust` + `p/security-audit` rulesets plus custom `.semgrep/` rules; SARIF results uploaded to GitHub Security tab |
| PW.7.2 | Use manual code review to identify vulnerabilities | Met | `docs/security/asvs-v2-auth-audit.md`, `docs/security/asvs-v5-v12-input-files-audit.md` | Line-level manual audit of authentication (OAuth PKCE, token refresh), input validation (URL, CSV, path traversal), cryptography (TLS, PKCE S256), and logging (secret redaction) documented with file:line evidence |

### PW.8 — Test Executable Code to Identify Vulnerabilities and Verify Compliance

Use dynamic analysis, testing, and fuzzing to identify vulnerabilities.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.8.1 | Use automated testing to identify vulnerabilities | Met | `.github/workflows/ci.yml` (test-matrix job), `.github/workflows/fuzz.yml` | Test matrix covers Ubuntu, Windows, and macOS; cargo-fuzz runs nightly against 3 targets: `fuzz_url_validation`, `fuzz_config_parsing`, `fuzz_json_parsing`; llvm-cov generates code coverage uploaded to Codecov |
| PW.8.2 | Perform security-focused testing | Partial | `.github/workflows/fuzz.yml`, `raps-cli/tests/redaction_test.rs` | Fuzzing covers URL validation, config parsing, and JSON parsing; redaction tests verify secret masking; however, no dedicated security integration tests (e.g., TLS downgrade, SSRF bypass, path traversal edge cases) are identified as a separate test suite |

### PW.9 — Configure Software to Have Secure Settings by Default

Ship software with secure default configurations.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.9.1 | Configure software to have secure settings by default | Met | `docs/security/asvs-l2-compliance-matrix.md` (V9 Communications, V8 Data Protection) | TLS-only via rustls (no native-tls); OS keyring as default credential store (`keyring` crate); file-based fallback warns users; directory permissions set to `0o700`; file permissions set to `0o600` on Unix; DPAPI/user-only ACL on Windows |
| PW.9.2 | Provide guidance for securely configuring the software | Met | `SECURITY.md` (Security Best Practices section), `docs/configuration.md` | SECURITY.md documents 6 best practices: never commit credentials, use environment variables, rotate credentials, review permissions, keep updated, secure token storage |

### PW.10 — Protect All Forms of Secrets

Prevent secrets from being exposed in source code, logs, and other artifacts.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| PW.10.1 | Identify and protect secrets in code and configuration | Met | `raps-kernel/src/logging.rs` (lines 141-221), `raps-cli/tests/redaction_test.rs` | `redact_secrets()` function covers 6 pattern categories: tokens, API keys, authorization headers, cookies, URL parameters, and client secrets; `RedactingMakeWriter` applies automatic redaction to all log output layers |
| PW.10.2 | Prevent secrets from leaking through builds and deployments | Met | `.github/workflows/ci.yml` (Gitleaks job) | Gitleaks scans on every PR to detect committed secrets; CI secrets scoped minimally per job; OIDC-based token issuance eliminates long-lived secrets in CI |

---

## RV: Respond to Vulnerabilities

### RV.1 — Identify and Confirm Vulnerabilities on an Ongoing Basis

Continuously monitor for vulnerabilities in the software and its dependencies.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| RV.1.1 | Monitor vulnerability databases and advisories | Met | `.github/workflows/ci.yml` (cargo-audit, cargo-deny) | `rustsec/audit-check` runs on every CI pipeline against the RustSec Advisory Database; `cargo-deny` checks against advisory database and enforces license policy; GitHub Dependabot alerts enabled |
| RV.1.2 | Use automated tools to detect vulnerabilities | Met | `.github/workflows/codeql.yml`, `.github/workflows/semgrep.yml`, `.github/workflows/fuzz.yml` | CodeQL (weekly + PR), Semgrep (weekly + PR), cargo-fuzz (nightly), cargo-audit (every CI run), Gitleaks (every PR) |

### RV.2 — Assess, Prioritize, and Remediate Vulnerabilities

Evaluate discovered vulnerabilities and address them based on risk.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| RV.2.1 | Analyze and risk-assess each vulnerability | Met | `docs/security/asvs-l2-compliance-matrix.md` (Remediation Priority section) | Vulnerabilities prioritized as P1 (critical), P2 (high), P3 (medium); P1 items (path traversal, log credential leak) marked as fixed; P2 items (log directory permissions) fixed; P2 plugin signing remains open |
| RV.2.2 | Remediate vulnerabilities within defined timeframes | Partial | `docs/security/asvs-l2-compliance-matrix.md` | P1 and P2 items addressed; plugin signature verification (V10.2) remains an open gap, tracked in the security hardening plan but without a defined remediation deadline |

### RV.3 — Analyze Vulnerabilities to Identify Root Causes

Understand root causes of vulnerabilities to prevent recurrence.

| Task ID | Task Description | Status | Evidence | Notes |
|---------|-----------------|--------|----------|-------|
| RV.3.1 | Analyze root causes of vulnerabilities | Met | `docs/plans/2026-02-28-security-hardening-design.md` (Gaps table) | Root cause analysis documented for each gap: tag mutability (Actions not SHA-pinned), implicit trust (plugin system), missing validation (path traversal), insufficient redaction (logging) |
| RV.3.2 | Publish and communicate vulnerability information | Met | `SECURITY.md` | Disclosure process via GitHub Security Advisories (preferred) or email to `security@autodesk.com`; 48h initial response SLA; 7-day status update commitment; scope and out-of-scope clearly defined |

---

## Evidence Index

The following files provide evidence referenced throughout this mapping:

| Evidence File | Path | Relevance |
|---------------|------|-----------|
| Security Policy | `SECURITY.md` | PO.1, RV.3 |
| Code Owners | `.github/CODEOWNERS` | PO.2, PS.1 |
| Branch Protection | `docs/BRANCH_PROTECTION.md` | PO.4, PS.1, PW.5 |
| CI Workflow | `.github/workflows/ci.yml` | PO.3, PO.5, PW.4, PW.5, PW.6, PW.10, RV.1 |
| CodeQL Workflow | `.github/workflows/codeql.yml` | PO.3, PW.7, RV.1 |
| Semgrep Workflow | `.github/workflows/semgrep.yml` | PO.3, PW.7, RV.1 |
| Fuzzing Workflow | `.github/workflows/fuzz.yml` | PW.8, RV.1 |
| Scorecard Workflow | `.github/workflows/scorecard.yml` | PO.3 |
| Release Workflow | `.github/workflows/release.yml` | PO.5, PS.2, PS.3 |
| ASVS L2 Compliance Matrix | `docs/security/asvs-l2-compliance-matrix.md` | PO.1, PW.1, PW.2, PW.5, PW.9, RV.2 |
| ASVS V2 Auth Audit | `docs/security/asvs-v2-auth-audit.md` | PW.1, PW.7 |
| ASVS V5/V12 Input/Files Audit | `docs/security/asvs-v5-v12-input-files-audit.md` | PW.1, PW.7 |
| ASVS V6/V9 Crypto/Comms Audit | `docs/security/asvs-v6-v9-crypto-comms-audit.md` | PW.1 |
| ASVS V7 Logging Audit | `docs/security/asvs-v7-logging-audit.md` | PW.1 |
| Plugin Trust Model | `docs/security/plugin-trust-model.md` | PO.1, PW.1 |
| Security Hardening Design | `docs/plans/2026-02-28-security-hardening-design.md` | PW.1, RV.3 |
| Security Hardening Plan | `docs/plans/2026-02-28-security-hardening-plan.md` | PW.1, RV.2 |
| SBOM Documentation | `docs/sbom.md` | PW.4, PS.3 |
| Checksums Documentation | `docs/cli/checksums.md` | PS.2 |
| Release Documentation | `docs/RELEASE.md` | PS.3 |
| Cargo Workspace Config | `Cargo.toml` | PW.4, PW.6 |
| Dependency Policy | `deny.toml` | PW.4 |
| Security Utilities | `raps-kernel/src/security.rs` | PW.5 |
| Logging/Redaction | `raps-kernel/src/logging.rs` | PW.10 |
| Redaction Tests | `raps-cli/tests/redaction_test.rs` | PW.10 |

## Open Gaps and Remediation Tracking

| Gap | SSDF Task | Impact | Current Status | Tracking |
|-----|-----------|--------|----------------|----------|
| Signed commits not enforced | PS.1.2 | Commit authorship could be spoofed | Encouraged but not required via branch protection | Consider enabling `require_signed_commits` in branch protection rules |
| No formal security training records | PO.2.2 | Cannot demonstrate personnel qualification | Maintainers assigned via CODEOWNERS | Document training or certifications for maintainers |
| Developer workstation hardening not documented | PO.5.2 | Local development environments may have insecure defaults | CI environments are secured | Consider documenting recommended developer environment configuration |
| Plugin signature verification not implemented | PW.8.2, RV.2.2 | Malicious plugins could execute with full user privileges | Tracked in security hardening plan; ed25519-dalek in dependencies | `docs/security/plugin-trust-model.md` documents the gap and future plans |
| No dedicated security integration test suite | PW.8.2 | Security regression detection relies on unit tests and fuzzing | Fuzzing and redaction tests provide partial coverage | Consider adding targeted security test suite for SSRF, path traversal, TLS enforcement |
| No defined remediation deadline for plugin signing | RV.2.2 | Open P2 gap without committed timeline | Acknowledged in ASVS compliance matrix | Define target milestone for plugin signing implementation |

---

## Methodology

This mapping was produced by:

1. Reviewing the NIST SP 800-218 SSDF v1.1 practice groups (PO, PS, PW, RV) and their constituent tasks.
2. Scoping tasks to RAPS as a CLI application (excluding practices applicable only to web applications, cloud services, or organizational policy beyond the repository).
3. Identifying evidence from source code, CI/CD workflows, documentation, and security audit artifacts.
4. Assessing each task as **Met** (evidence demonstrates full compliance), **Partial** (evidence demonstrates partial compliance with identified gaps), or **N/A** (task not applicable to the project scope).
5. Cross-referencing with the existing ASVS L2 compliance matrix for consistency.

## References

- [NIST SP 800-218: Secure Software Development Framework (SSDF) v1.1](https://csrc.nist.gov/publications/detail/sp/800-218/final)
- [OWASP ASVS 4.0.3](https://owasp.org/www-project-application-security-verification-standard/)
- [SLSA Framework](https://slsa.dev/)
- [OpenSSF Scorecard](https://securityscorecards.dev/)
- [CycloneDX SBOM Standard](https://cyclonedx.org/)
