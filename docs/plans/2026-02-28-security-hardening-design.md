# Security Hardening & Housekeeping Design

**Date:** 2026-02-28
**Version:** RAPS 4.14.0
**Targets:** OWASP ASVS L2 (CLI-scoped), SLSA Build L2, OpenSSF Scorecard, Continuous Security Scanning

## Goals

Comprehensive security posture improvement for RAPS across three parallel tracks:

1. **CI/CD Hardening** — SLSA L2 provenance, pinned deps, Semgrep, fuzzing, automated SBOM
2. **Code Audit** — ASVS L2 verification scoped to CLI-relevant chapters
3. **Scoring & Measurement** — OpenSSF Scorecard baseline, Best Practices badge, compliance matrix

## Current State

### Strengths

| Area | Tool/Config |
|------|-------------|
| Dependency audit | `cargo-audit` via `rustsec/audit-check@v2` in CI |
| License/advisory policy | `cargo-deny` with `deny.toml` |
| Secret scanning | Gitleaks in CI |
| SAST (basic) | CodeQL for Rust (weekly + PR) |
| License compliance | FOSSA in CI |
| Code coverage | llvm-cov + Codecov |
| Code quality | clippy -D warnings, rustfmt |
| Branch protection | all-checks-pass gate, CODEOWNERS |
| TLS | rustls (no openssl) |
| Token storage | OS keyring via `keyring` crate |
| SQL safety | Parameterized queries (rusqlite) in mock server |
| Secret redaction | Logging layer redacts tokens/secrets |

### Gaps

| Gap | Impact | Effort |
|-----|--------|--------|
| GitHub Actions not pinned to SHA | Supply chain risk (tag force-push) | Low |
| No SLSA provenance attestation | Build integrity unverifiable | Medium |
| No Semgrep (custom SAST rules) | Missing Rust-specific pattern detection | Medium |
| No fuzzing in CI | Missing input robustness testing | Medium |
| SECURITY.md says "0.2.x" supported | Misleading (current is 4.14.0) | Trivial |
| ~527 `unwrap()` calls (some in prod code) | Potential panics in production paths | Medium |
| Plugin system has no sandboxing/signing | Arbitrary command execution | High (design) |
| No signed releases | Release integrity unverifiable | Medium |
| No OpenSSF Scorecard tracking | No measurable security posture | Low |
| CodeQL not using `security-extended` queries | Missing deeper static analysis | Trivial |
| SBOM not automated in CI | No automatic SBOM with releases | Medium |
| `deny.toml` ignoring 3 advisories | Technical debt | Low |
| No `cargo-vet` for supply chain review | Dependency trust unverified | Medium |

---

## Track 1: CI/CD Hardening (SLSA L2 + OpenSSF)

### 1.1 Pin GitHub Actions to SHA

Pin every action reference in all 6 workflow files to full commit SHA:

```yaml
# Before:
- uses: actions/checkout@v4
# After:
- uses: actions/checkout@b4ffde65f46336ab88eb53be808477a3936bae11 # v4.1.7
```

Files: `ci.yml`, `release.yml`, `publish.yml`, `test-install.yml`, `codeql.yml`, `branch-protection-check.yml`, `docs.yml`

### 1.2 SLSA Build L2 Provenance

Add `actions/attest-build-provenance` to the release workflow for all platform artifacts:

```yaml
- uses: actions/attest-build-provenance@v2
  with:
    subject-path: 'dist/*'
```

Consumers verify with: `gh attestation verify <artifact> --repo dmytro-yemelianov/raps`

### 1.3 Semgrep SAST

Add Semgrep CI job with:
- Community rulesets: `p/rust`, `p/security-audit`
- Custom rules for raps:
  - `unwrap()` in non-test code
  - `unsafe` blocks outside tests
  - Hardcoded URLs/IPs
  - Unvalidated input to `Command::new()`
  - Unvalidated file paths in download operations

Run on PRs only (not push to main).

### 1.4 Cargo Fuzz Infrastructure

Set up `cargo-fuzz` with initial targets:
- OAuth token response JSON parsing
- Configuration file TOML parsing
- Pipeline YAML/JSON parsing
- URL validation (`http.rs`)
- CSV input parsing (admin module)

Nightly CI job: 5 minutes per target, bounded execution.

### 1.5 Automated SBOM in CI

Generate CycloneDX SBOM during release workflow, attach as release artifact:

```yaml
- name: Generate SBOM
  run: cargo cyclonedx --format json --output-file raps-sbom-${{ github.ref_name }}.json
- name: Upload SBOM
  uses: actions/upload-artifact@v4
  with:
    name: sbom
    path: raps-sbom-*.json
```

### 1.6 Housekeeping

- Update `SECURITY.md` supported versions: `0.2.x` → `4.x`
- Upgrade CodeQL to use `security-extended` query suite
- Review 3 ignored advisories in `deny.toml` (RUSTSEC-2024-0388, RUSTSEC-2024-0384, RUSTSEC-2025-0134) — resolve or document timeline
- Add `cargo-vet` for first-party supply chain auditing
- Audit workflow token permissions (principle of least privilege)

---

## Track 2: Code Audit (ASVS L2, CLI-Scoped)

### Relevant ASVS Chapters

| Chapter | Relevance | Key Focus |
|---------|-----------|-----------|
| V1 Architecture | High | Trust boundaries, threat model |
| V2 Authentication | High | OAuth flows, token lifecycle, credentials |
| V3 Session Mgmt | Low (CLI) | Token refresh/expiry only |
| V5 Validation | High | CLI input, URLs, file paths |
| V6 Cryptography | Medium | PKCE SHA-256, TLS, plugin signing |
| V7 Error/Logging | High | Secret redaction, error info leakage |
| V8 Data Protection | Medium | Token storage, config permissions |
| V9 Communications | High | TLS enforcement, domain allowlisting |
| V10 Malicious Code | Medium | Dependency integrity, plugin trust |
| V12 Files/Resources | Medium | Upload/download, path traversal |
| V14 Configuration | High | Secrets handling, build security |

### Audit Items

**V2 - Authentication:**
- Verify PKCE code_verifier uses cryptographically random source
- Verify token refresh Mutex prevents race conditions
- Verify no tokens appear in logs (test redaction completeness)
- Verify file-based token storage uses mode 600
- Verify 3-legged OAuth callback validates `state` parameter

**V5 - Input Validation:**
- Audit clap argument handling for injection vectors
- Verify URL validation rejects non-HTTPS in production
- Audit file path arguments for path traversal (`../`)
- Audit pipeline YAML/JSON for arbitrary code execution
- Audit CSV parsing for formula injection

**V6 - Cryptography:**
- Verify SHA-256 PKCE implementation correctness
- Verify TLS 1.2+ enforcement via rustls
- Document crypto inventory

**V7 - Error Handling & Logging:**
- Audit `unwrap()` in non-test production code — replace with Result propagation
- Verify no stack traces leak in release builds
- Verify secret redaction covers: client_secret, access_token, refresh_token, api_key
- Verify log files exclude credentials

**V8 - Data Protection:**
- Verify keyring fallback file has restrictive permissions
- Verify `.env` loading doesn't expose secrets in process listing
- Verify config export excludes secrets

**V10 - Plugin System:**
- Document plugin trust model
- Implement plugin signature verification (ed25519-dalek available)
- Add user confirmation before first plugin execution
- Audit `std::process::Command` for argument injection

**V12 - Files & Resources:**
- Audit multipart upload for file descriptor leaks
- Verify download path sanitization (no writes outside target)
- Verify temp file cleanup on error paths

**V14 - Configuration:**
- Verify no secrets in build artifacts
- Audit CI secret scoping
- Verify `.env.example` contains no real values

---

## Track 3: Scoring & Continuous Measurement

### 3.1 OpenSSF Scorecard

Run scorecard to establish baseline. Expected initial gaps:

| Check | Expected | Action |
|-------|----------|--------|
| Fuzzing | Fail → Pass | Track 1.4 |
| Pinned-Dependencies | Fail → Pass | Track 1.1 |
| Signed-Releases | Fail → Pass | Add sigstore signing |
| SAST | Partial → Pass | Track 1.3 |
| CII-Best-Practices | Missing → Pass | Apply for badge |
| Token-Permissions | Partial → Pass | Track 1.6 |

Add `ossf/scorecard-action` for weekly monitoring:

```yaml
- uses: ossf/scorecard-action@v2
  with:
    results_file: results.sarif
    publish_results: true
```

### 3.2 OpenSSF Best Practices Badge

Apply at bestpractices.dev. Most criteria already met. Remaining:
- Formal threat model document
- Reproducible build documentation
- Dynamic analysis evidence (fuzzing)

### 3.3 ASVS L2 Compliance Matrix

Create `docs/security/asvs-l2-matrix.md` mapping each requirement to:
- Status: Met / Partial / N/A / Gap
- Evidence: file path, test name, or CI job
- Notes and remediation plan

Living document updated with each audit cycle.

### 3.4 Ongoing Monitoring

| Cadence | Activity |
|---------|----------|
| Every PR | cargo-audit, cargo-deny, Semgrep, Gitleaks, CodeQL |
| Weekly | OpenSSF Scorecard, Dependabot security alerts |
| Nightly | Fuzz target runs (5 min/target) |
| Quarterly | ASVS matrix review, deny.toml advisory review |
| Per release | SBOM generation, SLSA provenance attestation |

---

## Implementation Priority

| Priority | Item | Track | Effort |
|----------|------|-------|--------|
| P0 | Pin actions to SHA | T1 | Low |
| P0 | Update SECURITY.md versions | T1 | Trivial |
| P0 | CodeQL security-extended queries | T1 | Trivial |
| P1 | SLSA provenance attestation | T1 | Medium |
| P1 | Semgrep CI integration | T1 | Medium |
| P1 | Audit unwrap() in prod code | T2 | Medium |
| P1 | OpenSSF Scorecard baseline | T3 | Low |
| P2 | Cargo fuzz infrastructure | T1 | Medium |
| P2 | Automated SBOM in releases | T1 | Medium |
| P2 | ASVS auth/input validation audit | T2 | Medium |
| P2 | Plugin signature verification | T2 | High |
| P3 | cargo-vet integration | T1 | Medium |
| P3 | OpenSSF Best Practices badge | T3 | Medium |
| P3 | ASVS compliance matrix | T3 | Medium |
| P3 | Threat model document | T3 | Medium |

---

## Non-Goals

- Web application security (RAPS is a CLI tool)
- ASVS chapters V3 (sessions), V4 (access control), V11 (business logic) — not applicable to CLI
- SLSA Build L3 (hermetic builds) — significant investment, deferred
- Runtime sandboxing for plugins — deferred to future design
- Penetration testing — out of scope for this plan (separate engagement)

## Success Criteria

- OpenSSF Scorecard: 7+/10 overall
- All GitHub Actions pinned to SHA
- SLSA L2 provenance on every release
- Zero `unwrap()` in production code paths (test code exempt)
- Semgrep + cargo-fuzz running in CI
- ASVS L2 compliance matrix with >80% "Met" for relevant chapters
- SBOM attached to every release
- SECURITY.md accurate and current
