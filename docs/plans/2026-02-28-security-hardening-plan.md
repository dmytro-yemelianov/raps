# Security Hardening & Housekeeping Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Achieve comprehensive security posture for RAPS — OWASP ASVS L2 (CLI-scoped), SLSA Build L2 provenance, OpenSSF Scorecard 7+/10, and continuous security scanning in CI.

**Architecture:** Three parallel tracks: (T1) CI/CD hardening — pin actions, add SLSA provenance, Semgrep, fuzzing, SBOM automation; (T2) Code audit — fix production `unwrap()` calls, verify auth/crypto/input validation against ASVS L2; (T3) Scoring — establish OpenSSF Scorecard baseline, create ASVS compliance matrix, add ongoing monitoring.

**Tech Stack:** GitHub Actions, cargo-fuzz, Semgrep, cargo-cyclonedx, ossf/scorecard-action, actions/attest-build-provenance, cargo-vet

**Design doc:** `docs/plans/2026-02-28-security-hardening-design.md`

---

## Track 1: CI/CD Hardening

### Task 1: Pin all GitHub Actions to SHA in ci.yml

**Files:**
- Modify: `raps/.github/workflows/ci.yml`

**Step 1: Replace all action tag references with SHA-pinned versions**

Replace the `uses:` lines in `ci.yml` with SHA-pinned equivalents. Use the following mapping:

```yaml
# ci.yml pinning map:
actions/checkout@v4                   → actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
dtolnay/rust-toolchain@master         → dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e18c1a97b10f1a2 # master
Swatinem/rust-cache@v2                → Swatinem/rust-cache@9d47f7b6d94010288df3ec1b7fe334a217471554 # v2
taiki-e/install-action@nextest        → taiki-e/install-action@735e5933943122c5ac182670a935f174702599b2 # v2
taiki-e/install-action@cargo-llvm-cov → taiki-e/install-action@735e5933943122c5ac182670a935f174702599b2 # v2
codecov/codecov-action@v4             → codecov/codecov-action@18283e04ce6e62d37312384ff67231eb8fd56d24 # v5
fossas/fossa-action@v1                → fossas/fossa-action@c414b9ad82eaad041e47a7cf62a4f02411f427a0 # v1
rustsec/audit-check@v2.0.0           → rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998 # v2.0.0
EmbarkStudios/cargo-deny-action@v2   → EmbarkStudios/cargo-deny-action@3fd3802e88374d3fe9159b834c7714ec57d6c979 # v2
gitleaks/gitleaks-action@v2           → gitleaks/gitleaks-action@ff98106e4c7b2bc287b24eaf42907196329070c7 # v2
crate-ci/typos@master                → crate-ci/typos@631208b7aac2daa8b707f55e7331f9112b0e062d # v1.44.0
```

> **Note:** Look up current SHAs at implementation time. Tags are mutable — verify each SHA matches the expected tag by checking the GitHub release page.

**Step 2: Verify CI passes**

Run: Push to a feature branch and verify all CI jobs still pass.

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "security: pin all GitHub Actions to SHA in ci.yml

Prevents supply-chain attacks via mutable tag references.
Part of security hardening track T1."
```

---

### Task 2: Pin all GitHub Actions to SHA in remaining workflow files

**Files:**
- Modify: `raps/.github/workflows/release.yml`
- Modify: `raps/.github/workflows/publish.yml`
- Modify: `raps/.github/workflows/test-install.yml`
- Modify: `raps/.github/workflows/codeql.yml`
- Modify: `raps/.github/workflows/docs.yml`

**Step 1: Pin actions in release.yml**

Additional actions to pin (beyond those in ci.yml):
```yaml
actions/upload-artifact@v4         → actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
actions/download-artifact@v4       → actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4
PyO3/maturin-action@v1            → PyO3/maturin-action@<lookup SHA> # v1
pypa/gh-action-pypi-publish@release/v1 → pypa/gh-action-pypi-publish@<lookup SHA> # release/v1
actions/setup-python@v5            → actions/setup-python@<lookup SHA> # v5
peter-evans/repository-dispatch@v4 → peter-evans/repository-dispatch@<lookup SHA> # v4
actions/setup-node@v4              → actions/setup-node@<lookup SHA> # v4
```

**Step 2: Pin actions in publish.yml**

```yaml
dtolnay/rust-toolchain@stable      → dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e18c1a97b10f1a2 # stable
```

**Step 3: Pin actions in codeql.yml**

```yaml
github/codeql-action/init@v3      → github/codeql-action/init@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
github/codeql-action/autobuild@v3 → github/codeql-action/autobuild@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
github/codeql-action/analyze@v3   → github/codeql-action/analyze@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
```

**Step 4: Pin actions in docs.yml**

```yaml
actions/setup-python@v6            → actions/setup-python@<lookup SHA> # v6
actions/upload-pages-artifact@v4   → actions/upload-pages-artifact@<lookup SHA> # v4
actions/deploy-pages@v4            → actions/deploy-pages@<lookup SHA> # v4
```

**Step 5: Pin actions in test-install.yml**

```yaml
actions/setup-python@v5            → actions/setup-python@<lookup SHA> # v5
```

> **Note:** Look up all SHAs at implementation time. For each action, go to the GitHub repo releases page, find the tag, and get the commit SHA.

**Step 6: Commit**

```bash
git add .github/workflows/release.yml .github/workflows/publish.yml .github/workflows/codeql.yml .github/workflows/docs.yml .github/workflows/test-install.yml
git commit -m "security: pin all GitHub Actions to SHA across all workflows

Covers release.yml, publish.yml, codeql.yml, docs.yml, test-install.yml.
Part of security hardening track T1."
```

---

### Task 3: Upgrade CodeQL to security-extended queries

**Files:**
- Modify: `raps/.github/workflows/codeql.yml:39-44`

**Step 1: Enable security-extended query suite**

In `codeql.yml`, change the `Initialize CodeQL` step:

```yaml
    - name: Initialize CodeQL
      uses: github/codeql-action/init@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
      with:
        languages: ${{ matrix.language }}
        queries: security-extended,security-and-quality
```

Uncomment and set the `queries:` line that's currently commented out.

**Step 2: Commit**

```bash
git add .github/workflows/codeql.yml
git commit -m "security: enable CodeQL security-extended and security-and-quality queries

Deeper static analysis beyond default query suite.
Part of security hardening track T1."
```

---

### Task 4: Update SECURITY.md supported versions

**Files:**
- Modify: `raps/SECURITY.md:7-9`

**Step 1: Update version table**

Replace:
```markdown
| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2.0 | :x:                |
```

With:
```markdown
| Version | Supported          |
| ------- | ------------------ |
| 4.x     | :white_check_mark: |
| 3.x     | :x: (upgrade to 4.x) |
| < 3.0   | :x:                |
```

**Step 2: Commit**

```bash
git add SECURITY.md
git commit -m "docs: update SECURITY.md supported versions to reflect 4.x

Previous version table referenced 0.2.x; current release is 4.14.0.
Part of security hardening track T1."
```

---

### Task 5: Audit and tighten workflow token permissions

**Files:**
- Modify: `raps/.github/workflows/ci.yml:1-7`
- Modify: `raps/.github/workflows/codeql.yml`
- Modify: `raps/.github/workflows/release.yml`

**Step 1: Review ci.yml permissions**

Current ci.yml has:
```yaml
permissions:
  contents: read
  checks: write
  pull-requests: write
  security-events: write
```

Verify each permission is needed:
- `contents: read` — required for checkout
- `checks: write` — needed for status checks
- `pull-requests: write` — needed for Codecov PR comments
- `security-events: write` — needed if uploading SARIF (currently not used in ci.yml)

If `security-events: write` is not needed, remove it. If SARIF upload is added later (Semgrep), keep it.

**Step 2: Add top-level permissions to workflows that don't have them**

`test-install.yml` and `branch-protection-check.yml` have no top-level permissions block. Add:
```yaml
permissions:
  contents: read
```

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/test-install.yml .github/workflows/branch-protection-check.yml
git commit -m "security: tighten workflow token permissions to least privilege

Add explicit permissions blocks to all workflows.
Part of security hardening track T1."
```

---

### Task 6: Add Semgrep SAST scanning to CI

**Files:**
- Create: `raps/.github/workflows/semgrep.yml`
- Create: `raps/.semgrep/` directory with custom rules

**Step 1: Create Semgrep workflow**

Create `.github/workflows/semgrep.yml`:

```yaml
name: Semgrep SAST

permissions:
  contents: read
  security-events: write

on:
  pull_request:
    branches: [main, master]
  # Weekly scheduled scan
  schedule:
    - cron: '0 3 * * 1'

jobs:
  semgrep:
    name: semgrep
    runs-on: ubuntu-latest
    container:
      image: semgrep/semgrep
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4

      - name: Run Semgrep
        run: semgrep scan --config p/rust --config p/security-audit --config .semgrep/ --sarif --output semgrep-results.sarif .
        env:
          SEMGREP_RULES: p/rust p/security-audit

      - name: Upload SARIF
        if: always()
        uses: github/codeql-action/upload-sarif@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
        with:
          sarif_file: semgrep-results.sarif
```

**Step 2: Create custom Semgrep rules**

Create `.semgrep/raps-rules.yml`:

```yaml
rules:
  - id: unwrap-in-production-code
    patterns:
      - pattern: $X.unwrap()
      - pattern-not-inside: |
          #[cfg(test)]
          mod $MOD { ... }
      - pattern-not-inside: |
          #[test]
          fn $FN() { ... }
    message: "Avoid unwrap() in production code. Use expect(), unwrap_or(), or propagate with ?."
    languages: [rust]
    severity: WARNING
    metadata:
      category: correctness
      cwe: ["CWE-248: Uncaught Exception"]

  - id: command-injection-risk
    pattern: std::process::Command::new($INPUT)
    message: "Verify $INPUT is validated before use in Command::new() to prevent command injection."
    languages: [rust]
    severity: WARNING
    metadata:
      category: security
      cwe: ["CWE-78: OS Command Injection"]

  - id: hardcoded-url
    pattern-regex: '"https?://[^"]*\.(autodesk|forge)\.[^"]*"'
    message: "Hardcoded URL detected. Consider using configuration for API base URLs."
    languages: [rust]
    severity: INFO
    metadata:
      category: maintainability
```

**Step 3: Test locally (optional)**

Run: `docker run --rm -v $(pwd):/src semgrep/semgrep scan --config .semgrep/ /src`

**Step 4: Commit**

```bash
git add .github/workflows/semgrep.yml .semgrep/
git commit -m "security: add Semgrep SAST scanning with custom Rust rules

Runs on PRs and weekly. Includes community p/rust + p/security-audit
rulesets plus custom rules for unwrap-in-prod, command injection, and
hardcoded URLs. Results uploaded as SARIF to GitHub Security tab.
Part of security hardening track T1."
```

---

### Task 7: Add cargo-fuzz infrastructure with initial targets

**Files:**
- Create: `raps/fuzz/Cargo.toml`
- Create: `raps/fuzz/fuzz_targets/fuzz_url_validation.rs`
- Create: `raps/fuzz/fuzz_targets/fuzz_config_parsing.rs`
- Create: `raps/.github/workflows/fuzz.yml`

**Step 1: Initialize cargo-fuzz**

Run from the `raps/` directory:
```bash
cargo install cargo-fuzz
cargo fuzz init
```

**Step 2: Create URL validation fuzz target**

The HTTP URL validation in `raps-kernel/src/http.rs` is a high-value target. Create `fuzz/fuzz_targets/fuzz_url_validation.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the URL parsing path that validates allowed domains
        let _ = url::Url::parse(s);
    }
});
```

**Step 3: Create config parsing fuzz target**

Create `fuzz/fuzz_targets/fuzz_config_parsing.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz TOML config parsing
        let _ = toml::from_str::<toml::Value>(s);
    }
});
```

**Step 4: Update fuzz/Cargo.toml**

Ensure it has the right dependencies:
```toml
[package]
name = "raps-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
url = "2"
toml = "0.8"

[[bin]]
name = "fuzz_url_validation"
path = "fuzz_targets/fuzz_url_validation.rs"
test = false
doc = false

[[bin]]
name = "fuzz_config_parsing"
path = "fuzz_targets/fuzz_config_parsing.rs"
test = false
doc = false
```

**Step 5: Create nightly fuzz CI workflow**

Create `.github/workflows/fuzz.yml`:

```yaml
name: Fuzzing

permissions:
  contents: read

on:
  schedule:
    # Run nightly at 3 AM UTC
    - cron: '0 3 * * *'
  workflow_dispatch:

jobs:
  fuzz:
    name: cargo-fuzz
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target: [fuzz_url_validation, fuzz_config_parsing]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - name: Run fuzzer for 5 minutes
        run: cargo fuzz run ${{ matrix.target }} -- -max_total_time=300
      - name: Upload crash artifacts
        if: failure()
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: fuzz-crashes-${{ matrix.target }}
          path: fuzz/artifacts/
```

**Step 6: Commit**

```bash
git add fuzz/ .github/workflows/fuzz.yml
git commit -m "security: add cargo-fuzz infrastructure with URL and config parsing targets

Nightly CI runs each target for 5 minutes. Crash artifacts uploaded on failure.
Part of security hardening track T1."
```

---

### Task 8: Automate SBOM generation in release workflow

**Files:**
- Modify: `raps/.github/workflows/release.yml`

**Step 1: Add SBOM generation job**

Add a new job to `release.yml` after the build jobs:

```yaml
  sbom:
    name: Generate SBOM
    runs-on: ubuntu-latest
    needs: [plan]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
      - uses: dtolnay/rust-toolchain@b3b07ba8b418998c39fb20f53e18c1a97b10f1a2 # master
        with:
          toolchain: stable
      - name: Install cargo-cyclonedx
        run: cargo install cargo-cyclonedx
      - name: Generate CycloneDX SBOM
        run: cargo cyclonedx --format json --output-file raps-sbom-${{ github.ref_name }}.json
      - name: Upload SBOM artifact
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
        with:
          name: sbom
          path: raps-sbom-*.json
```

Ensure the SBOM artifact is included in the release assets (add to the release publish step).

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "security: automate CycloneDX SBOM generation in release workflow

SBOM attached as release artifact for supply chain transparency.
Part of security hardening track T1."
```

---

### Task 9: Add SLSA Build L2 provenance attestation

**Files:**
- Modify: `raps/.github/workflows/release.yml`

**Step 1: Add provenance attestation**

Add to the release workflow after artifacts are built, using GitHub's built-in attestation:

```yaml
  provenance:
    name: Generate SLSA Provenance
    runs-on: ubuntu-latest
    needs: [upload-local-artifacts]
    permissions:
      id-token: write
      contents: read
      attestations: write
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4
        with:
          path: dist/
      - name: Generate provenance attestation
        uses: actions/attest-build-provenance@e8998f949152b193b063cb0ec769d69d929409be # v2
        with:
          subject-path: 'dist/**/*'
```

> **Note:** This requires the repository to have GitHub Actions attestation enabled (Settings → Code security → Artifact attestations).

**Step 2: Document verification**

Add to `docs/cli/checksums.md`:
```markdown
## Verifying Build Provenance

RAPS releases include SLSA Build Level 2 provenance attestations. Verify with:

\`\`\`bash
gh attestation verify <path-to-binary> --repo dmytro-yemelianov/raps
\`\`\`
```

**Step 3: Commit**

```bash
git add .github/workflows/release.yml docs/cli/checksums.md
git commit -m "security: add SLSA Build L2 provenance attestation to releases

Uses actions/attest-build-provenance for signed provenance.
Consumers can verify with: gh attestation verify <binary> --repo dmytro-yemelianov/raps
Part of security hardening track T1."
```

---

### Task 10: Review and address deny.toml advisory ignores

**Files:**
- Modify: `raps/deny.toml`
- Read: Check upstream status of ignored advisories

**Step 1: Check each advisory**

For each ignored advisory, check if the upstream issue is resolved:

1. **RUSTSEC-2024-0388** (`derivative` — unmaintained, used by zbus): Check if zbus has migrated away from derivative.
2. **RUSTSEC-2024-0384** (`instant` — unmaintained, used by fastrand): Check if fastrand has removed instant dependency.
3. **RUSTSEC-2025-0134** (`rustls-pemfile` — unmaintained, used by reqwest): Check if reqwest has updated.

**Step 2: For each resolved advisory, remove the ignore entry**

If the upstream dependency has been updated:
```toml
[advisories]
version = 2
# Remove resolved entries, keep only those still outstanding
ignore = [
    # Only keep advisories that are still unresolvable
]
```

**Step 3: For remaining ignores, add expiry dates and tracking comments**

```toml
[advisories]
version = 2
ignore = [
    { id = "RUSTSEC-2024-0388", reason = "derivative unmaintained, transitive via zbus. Tracking: https://github.com/zbus-rs/zbus/issues/XXX" },
]
```

**Step 4: Commit**

```bash
git add deny.toml
git commit -m "security: review and update deny.toml advisory ignores

Resolve or document remaining advisory ignores with tracking links.
Part of security hardening track T1."
```

---

## Track 2: Code Audit (ASVS L2)

### Task 11: Fix production unwrap() calls — MCP server client factories

**Files:**
- Modify: `raps/raps-cli/src/mcp/server.rs:94-158`

**Step 1: Replace unwrap() with expect() in double-checked locking pattern**

The MCP server has 4 client factory methods that use `guard.as_ref().unwrap().clone()` after setting the value. While logically safe (the None case is handled just above), replace with `expect()` for clarity:

```rust
// In get_auth_client(), get_oss_client(), get_derivative_client(), get_dm_client():
// Replace:
guard.as_ref().unwrap().clone()
// With:
guard.as_ref().expect("client was just initialized above").clone()
```

This is a minimal change — the code is logically correct, but `expect()` with a message is preferred per Rust conventions.

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass (no behavioral change).

**Step 3: Commit**

```bash
git add raps-cli/src/mcp/server.rs
git commit -m "fix: replace unwrap() with expect() in MCP server client factories

Adds descriptive panic messages for the double-checked locking pattern.
Part of ASVS L2 code audit (V7 - error handling)."
```

---

### Task 12: Fix production unwrap() calls — semaphore acquisitions

**Files:**
- Modify: `raps/raps-oss/src/batch.rs:81,143`
- Modify: `raps/raps-acc/src/users.rs:383`
- Modify: `raps/raps-admin/src/bulk/executor.rs:226`
- Modify: `raps/raps-cli/src/mcp/tools_oss.rs:358`
- Modify: `raps/raps-cli/src/commands/demo.rs:816`

**Step 1: Replace semaphore unwrap() with expect()**

Tokio semaphore `acquire()` only fails if the semaphore is closed, which doesn't happen in normal operation. Replace with `expect()`:

```rust
// Replace:
let _permit = sem.acquire().await.unwrap();
// With:
let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");
```

Apply to all 6 locations listed above.

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add raps-oss/src/batch.rs raps-acc/src/users.rs raps-admin/src/bulk/executor.rs raps-cli/src/mcp/tools_oss.rs raps-cli/src/commands/demo.rs
git commit -m "fix: replace semaphore unwrap() with expect() in production code

Semaphore acquire only fails if closed; expect() provides clear panic context.
Part of ASVS L2 code audit (V7 - error handling)."
```

---

### Task 13: Fix production unwrap() calls — file operations and JSON

**Files:**
- Modify: `raps/raps-cli/src/commands/demo.rs:226,364,584,756,806,1012`
- Modify: `raps/raps-derivative/src/download.rs:56`
- Modify: `raps/raps-cli/src/commands/dashboard/mod.rs:619`

**Step 1: Fix file_name().unwrap() in demo.rs**

Replace all `path.file_name().unwrap()` with safe alternatives:

```rust
// Replace:
let name = path.file_name().unwrap();
// With:
let name = path.file_name().unwrap_or_default();
// Or for cases where we need a string:
let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
```

**Step 2: Fix as_array_mut().unwrap() in demo.rs:584**

```rust
// Replace:
value.as_array_mut().unwrap()
// With:
value.as_array_mut().expect("expected JSON array in demo data")
```

This is demo code so `expect()` is acceptable — it's not user-facing production path.

**Step 3: Fix ProgressStyle template unwrap() in download.rs:56**

```rust
// Replace:
ProgressStyle::default_bar().template("...").unwrap()
// With:
ProgressStyle::default_bar().template("...").expect("hardcoded progress template is valid")
```

The template string is hardcoded and known-valid, so `expect()` with a clear message is appropriate.

**Step 4: Fix dashboard stack unwrap() in dashboard/mod.rs:619**

```rust
// Replace:
let view = stack.last().unwrap().clone();
// With:
let view = stack.last().expect("navigation stack is never empty").clone();
```

**Step 5: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add raps-cli/src/commands/demo.rs raps-derivative/src/download.rs raps-cli/src/commands/dashboard/mod.rs
git commit -m "fix: replace remaining production unwrap() calls with safe alternatives

- demo.rs: file_name().unwrap_or_default() for path operations
- download.rs: expect() for hardcoded template
- dashboard: expect() for non-empty stack invariant
Part of ASVS L2 code audit (V7 - error handling)."
```

---

### Task 14: Verify and document auth security (ASVS V2)

**Files:**
- Read: `raps/raps-kernel/src/auth/device_code.rs`
- Read: `raps/raps-kernel/src/auth/three_leg.rs`
- Read: `raps/raps-kernel/src/auth/two_leg.rs`
- Read: `raps/raps-kernel/src/auth/token_ops.rs`
- Read: `raps/raps-kernel/src/storage.rs`
- Create: `raps/docs/security/asvs-v2-auth-audit.md`

**Step 1: Verify PKCE implementation**

Check `device_code.rs`:
- `generate_code_verifier()` uses `rand::thread_rng()` — this is `ThreadRng` which is cryptographically secure (wraps `ChaCha12Rng` seeded from OS entropy). **PASS**
- Verifier is 128 chars from RFC 7636 §4.1 charset. **PASS**
- `derive_code_challenge()` uses `sha2::Sha256` with URL-safe base64 no-pad encoding. **PASS**
- RFC 7636 Appendix B test vector is tested. **PASS**

**Step 2: Verify state/CSRF validation**

Check `device_code.rs:117-123`:
- State parameter is UUID v4 (random). **PASS**
- State is validated on callback: mismatch → bail. **PASS**

**Step 3: Verify token storage security**

Check `storage.rs`:
- Default: OS keyring (platform-specific secure storage). **PASS**
- Fallback: file-based storage — verify file permissions are set to 600.
- Check: Does the file fallback warn users? (Per existing docs, yes.)

**Step 4: Verify token refresh race conditions**

Check `token_ops.rs`:
- Token refresh uses Mutex-based coordination. **VERIFY** — read the actual implementation.

**Step 5: Document findings**

Create `docs/security/asvs-v2-auth-audit.md` with findings, status for each check, and any remediation items.

**Step 6: Commit**

```bash
git add docs/security/
git commit -m "docs: add ASVS V2 authentication audit results

Documents PKCE, CSRF state, token storage, and refresh race condition verification.
Part of ASVS L2 code audit (V2 - authentication)."
```

---

### Task 15: Verify and document crypto and communications (ASVS V6/V9)

**Files:**
- Read: `raps/raps-kernel/src/http.rs` (TLS config, allowed domains)
- Create: `raps/docs/security/asvs-v6-v9-crypto-comms-audit.md`

**Step 1: Verify TLS configuration**

Check `raps-kernel/src/http.rs`:
- reqwest with `rustls-tls` feature — uses rustls which defaults to TLS 1.2+. **VERIFY**
- No option to disable TLS verification. **VERIFY**
- Domain allowlisting prevents credential leakage. **VERIFY**

**Step 2: Document crypto inventory**

| Algorithm | Purpose | Location | Library |
|-----------|---------|----------|---------|
| SHA-256 | PKCE S256 code challenge | `auth/device_code.rs` | `sha2` 0.10 |
| AES (via rustls) | TLS transport | `http.rs` | `rustls` (via reqwest) |
| Ed25519 | Plugin signatures (future) | dependency only | `ed25519-dalek` 2.1 |

**Step 3: Commit**

```bash
git add docs/security/
git commit -m "docs: add ASVS V6/V9 crypto and communications audit

Documents TLS config, crypto inventory, and domain allowlisting.
Part of ASVS L2 code audit (V6/V9 - cryptography/communications)."
```

---

### Task 16: Audit input validation and file operations (ASVS V5/V12)

**Files:**
- Read: `raps/raps-cli/src/commands/object/` (upload/download)
- Read: `raps/raps-cli/src/commands/pipeline.rs`
- Read: `raps/raps-admin/src/` (CSV parsing)
- Read: `raps/raps-kernel/src/http.rs` (URL validation)
- Create: `raps/docs/security/asvs-v5-v12-input-files-audit.md`

**Step 1: Audit file path handling**

Check download commands for path traversal:
- Do download operations sanitize output paths?
- Can a malicious API response cause writing outside the target directory?
- Check for `../` in filenames from API responses.

**Step 2: Audit URL validation**

Check `http.rs`:
- Is the allowed domains list enforced on all outbound requests?
- Can SSRF be triggered via user-provided URLs (e.g., signed URL generation)?

**Step 3: Audit CSV parsing**

Check admin module CSV import:
- Is CSV input validated before processing?
- Are email addresses validated?
- Is there protection against formula injection (cells starting with `=`, `+`, `-`, `@`)?

**Step 4: Audit pipeline YAML/JSON parsing**

Check `pipeline.rs`:
- Does pipeline execution allow arbitrary command execution?
- Is the pipeline schema validated before execution?

**Step 5: Document findings and create remediation items**

**Step 6: Commit**

```bash
git add docs/security/
git commit -m "docs: add ASVS V5/V12 input validation and file operations audit

Covers path traversal, URL validation, CSV injection, pipeline safety.
Part of ASVS L2 code audit (V5/V12 - validation/files)."
```

---

### Task 17: Audit logging secret redaction completeness (ASVS V7)

**Files:**
- Read: `raps/raps-kernel/src/logging.rs`
- Read: `raps/raps-kernel/src/http.rs` (request/response tracing)

**Step 1: Identify all secret patterns that should be redacted**

Check logging.rs for the redaction filter. Verify it covers:
- `client_secret`
- `access_token`
- `refresh_token`
- `Authorization` header values
- `api_key`
- Any `Bearer` token values in URLs or bodies

**Step 2: Test redaction by grep**

Search the codebase for any logging of potentially sensitive data:
```
grep -rn "tracing::" raps-kernel/src/ raps-cli/src/ | grep -i "secret\|token\|password\|key\|auth"
```

**Step 3: Document findings**

**Step 4: Commit**

```bash
git add docs/security/
git commit -m "docs: add ASVS V7 logging and secret redaction audit

Verifies completeness of secret redaction in structured logging.
Part of ASVS L2 code audit (V7 - error handling/logging)."
```

---

### Task 18: Document plugin system trust model (ASVS V10)

**Files:**
- Read: `raps/raps-cli/src/plugins.rs`
- Create: `raps/docs/security/plugin-trust-model.md`

**Step 1: Audit plugin discovery and execution**

Read `plugins.rs` and document:
- How plugins are discovered (PATH-based `raps-<name>` executables)
- How plugin commands are executed (`std::process::Command`)
- What arguments are passed (check for injection)
- Whether plugins have access to credentials
- Whether there's any sandboxing

**Step 2: Document the trust model**

Create `docs/security/plugin-trust-model.md`:
- Current state: Plugins execute with full user permissions
- Risk: Malicious plugins can access credentials, files, network
- Mitigations in place: None (intentional extension mechanism)
- Future: Signature verification with ed25519-dalek (dependency exists)
- Recommendations: Users should only install trusted plugins

**Step 3: Commit**

```bash
git add docs/security/
git commit -m "docs: document plugin system trust model and security considerations

Covers discovery, execution, credential access, and future signing plans.
Part of ASVS L2 code audit (V10 - malicious code)."
```

---

## Track 3: Scoring & Measurement

### Task 19: Add OpenSSF Scorecard GitHub Action

**Files:**
- Create: `raps/.github/workflows/scorecard.yml`

**Step 1: Create scorecard workflow**

Create `.github/workflows/scorecard.yml`:

```yaml
name: OpenSSF Scorecard

permissions:
  contents: read
  security-events: write
  id-token: write

on:
  # Weekly scan
  schedule:
    - cron: '30 2 * * 1'
  # On pushes to main for baseline
  push:
    branches: [main, master]

jobs:
  analysis:
    name: Scorecard analysis
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
        with:
          persist-credentials: false

      - name: Run Scorecard
        uses: ossf/scorecard-action@62b2cac7ed8198b15735ed49ab1e5cf35480ba46 # v2.4.0
        with:
          results_file: results.sarif
          results_format: sarif
          publish_results: true

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@45580472a5bb82c4681c4ac726cfdb60060c2ee1 # v3
        with:
          sarif_file: results.sarif
```

**Step 2: Commit**

```bash
git add .github/workflows/scorecard.yml
git commit -m "security: add OpenSSF Scorecard weekly analysis

Publishes results to OpenSSF API and GitHub Security tab via SARIF.
Part of security hardening track T3."
```

---

### Task 20: Create ASVS L2 compliance matrix

**Files:**
- Create: `raps/docs/security/asvs-l2-compliance-matrix.md`

**Step 1: Create the matrix document**

Compile findings from Tasks 14-18 into a structured compliance matrix:

```markdown
# ASVS L2 Compliance Matrix (CLI-Scoped)

**Last Updated:** YYYY-MM-DD
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Summary

| Chapter | Total | Met | Partial | N/A | Gap |
|---------|-------|-----|---------|-----|-----|
| V2 Authentication | X | X | X | X | X |
| V5 Validation | X | X | X | X | X |
...

## V2 - Authentication

| Req ID | Requirement | Status | Evidence | Notes |
|--------|-------------|--------|----------|-------|
| 2.1.1 | Verify user set passwords are at least 12 characters | N/A | CLI uses OAuth tokens, not passwords | |
| 2.7.1 | Verify OAuth implementation uses PKCE | Met | `auth/device_code.rs:24-40` + tests | RFC 7636 compliant |
...
```

Fill in all relevant requirements from the ASVS 4.0.3 specification, marking each as Met/Partial/N/A/Gap with evidence.

**Step 2: Commit**

```bash
git add docs/security/
git commit -m "docs: create ASVS L2 compliance matrix for RAPS

Living document mapping ASVS 4.0.3 requirements to RAPS evidence.
Part of security hardening track T3."
```

---

### Task 21: Run initial OpenSSF Scorecard and document baseline

**Files:**
- Create: `raps/docs/security/openssf-scorecard-baseline.md`

**Step 1: Run scorecard locally**

```bash
# Install scorecard
go install github.com/ossf/scorecard/v5/cmd/scorecard@latest

# Run against the repo
scorecard --repo=github.com/dmytro-yemelianov/raps --format=json > scorecard-baseline.json
```

Or if the repo is private, use the GitHub Action result from Task 19.

**Step 2: Document baseline scores**

Create `docs/security/openssf-scorecard-baseline.md`:

```markdown
# OpenSSF Scorecard Baseline

**Date:** YYYY-MM-DD
**Overall Score:** X/10

| Check | Score | Notes |
|-------|-------|-------|
| Binary-Artifacts | X/10 | ... |
| Branch-Protection | X/10 | ... |
...
```

**Step 3: Identify improvement targets**

For each check scoring below 8, document what's needed to improve.

**Step 4: Commit**

```bash
git add docs/security/
git commit -m "docs: document OpenSSF Scorecard baseline with improvement targets

Establishes measurable security posture baseline.
Part of security hardening track T3."
```

---

### Task 22: Apply for OpenSSF Best Practices badge

**Step 1: Go to https://www.bestpractices.dev/ and start the application**

Answer the questionnaire using information from the codebase:
- Basics: Apache-2.0, active development, CONTRIBUTING.md, etc.
- Change control: Git, semantic versioning, CHANGELOG.md
- Reporting: SECURITY.md with vulnerability reporting process
- Quality: Test suite, CI, multi-platform testing
- Security: Secure development, dependency scanning, SAST
- Analysis: Static analysis (clippy, CodeQL, Semgrep), dynamic (fuzzing)

**Step 2: Add badge to README.md**

Once approved, add the badge to the top of README.md:

```markdown
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/XXXXX/badge)](https://www.bestpractices.dev/projects/XXXXX)
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add OpenSSF Best Practices badge to README

Part of security hardening track T3."
```

---

## Final Verification

### Task 23: Run full CI and verify all checks pass

**Step 1: Run local verification**

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --workspace
cargo doc --no-deps --all-features
```

**Step 2: Push and verify CI**

Push the branch and verify all CI jobs pass, including new ones:
- Semgrep SAST
- OpenSSF Scorecard
- Existing: check, test-matrix, fmt, clippy, docs, license-scan, audit, deny, secrets, typos

**Step 3: Create summary PR**

Create a PR with all changes, referencing the design doc and listing all improvements.

---

## Execution Order

Tasks can be parallelized across tracks:

```
Track 1 (CI/CD):     T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9 → T10
Track 2 (Code Audit): T11 → T12 → T13 → T14 → T15 → T16 → T17 → T18
Track 3 (Scoring):    T19 → T20 → T21 → T22

Final:                T23 (depends on all tracks)
```

Within each track, tasks are sequential. Across tracks, they are independent and can run in parallel.

**Recommended execution:** Use subagent-driven development with one agent per track, plus a final verification pass.
