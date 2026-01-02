# Roadmap: RAPS CLI (v0.4 → v0.6)

This document provides a comprehensive overview of the roadmap for RAPS CLI development from version 0.4 through 0.6.

## Milestone v0.4 — CI/CD & Automation Ready

### EPIC: Non-interactive + CI-friendly behavior

**Goal:** Every command can run in CI without prompts, colors, or unstable output.

#### Issue: Add global `--output` with `json|yaml|table`

**Labels:** `epic:ci`, `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.4-001-global-output-format.md`

**Description:** Introduce a global output selector for consistent machine-readable output.

**Acceptance Criteria:**

* `raps ... --output json` prints valid JSON to stdout
* `--output table` matches current human-readable behavior
* `--output yaml` supported (optional if too heavy, can be v0.5)
* Documented in README + each relevant command page
* Tests cover JSON schema stability for at least 5 representative commands

#### Issue: Standardize exit codes across commands

**Labels:** `epic:ci`, `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.4-002-standardize-exit-codes.md`

**Description:** Ensure deterministic exit codes for scripting.

**Acceptance Criteria:**

* `0` success
* `2` invalid arguments / validation failure
* `3` auth failure
* `4` not found
* `5` remote/API error
* `6` internal error
* Documented in `docs/cli/exit-codes.md`

#### Issue: Add global `--no-color`, `--quiet`, `--verbose`, `--debug`

**Labels:** `epic:ci`, `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.4-003-global-logging-flags.md`

**Description:** Make logs predictable in CI and allow proper troubleshooting.

**Acceptance Criteria:**

* `--no-color` disables ANSI everywhere
* `--quiet` prints only the result payload (especially useful with JSON)
* `--verbose` shows request summaries
* `--debug` includes full trace (but redacts secrets)
* Works consistently across subcommands

#### Issue: Add `--yes` / `--non-interactive` and remove mandatory prompts

**Labels:** `epic:ci`, `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.4-004-non-interactive-mode.md`

**Description:** Commands like create/delete must be fully parameterizable.

**Acceptance Criteria:**

* Any prompt can be bypassed via flags
* In `--non-interactive`, missing required info results in clear error + exit code 2
* `--yes` auto-confirms destructive actions

---

## Milestone v0.5 — Profiles, Auth, Reliability

### EPIC: Profiles (contexts) & secrets handling

**Goal:** Easy switching between environments and multiple APS apps.

#### Issue: Introduce `raps config profile` (create/list/use/delete)

**Labels:** `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.5-001-profile-management.md`

**Description:** Add profile management (`dev`, `prod`, `clientA`, etc.).

**Acceptance Criteria:**

* `raps config profile create <name>`
* `raps config profile use <name>`
* `raps config get/set` are profile-aware
* `.env` remains supported, but profile config takes priority when active
* Docs include a "CI example" and "local dev example"

#### Issue: Config precedence spec (env vs config vs flags)

**Labels:** `type:docs`, `prio:high`  
**File:** `.github/ISSUES/v0.5-002-config-precedence-spec.md`

**Description:** Document deterministic precedence rules.

**Acceptance Criteria:**

* Single doc page: flags > env vars > active profile > default profile
* Includes examples for CI/CD and local shell usage

#### Issue: Optional OS keychain integration (credential storage)

**Labels:** `type:feature`, `prio:med`  
**File:** `.github/ISSUES/v0.5-003-os-keychain-integration.md`

**Description:** Allow storing tokens in OS credential manager (where feasible).

**Acceptance Criteria:**

* Feature flag or config toggle (keep default simple)
* Falls back to existing storage if unsupported
* Secrets never printed in debug logs

### EPIC: Headless-friendly authentication

**Goal:** Work on servers/SSH/CI without browser login.

#### Issue: Add device-code flow or "paste token" auth mode

**Labels:** `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.5-004-device-code-auth.md`

**Description:** Support a browserless login UX.

**Acceptance Criteria:**

* `raps auth login --device` (or equivalent) works without launching a browser
* `raps auth login --token <...>` supported for CI scenarios (document security caveats)
* `raps auth status` shows active profile + token expiry (redacted)

### EPIC: Reliability: retries, backoff, timeouts, rate limits

**Goal:** Make it robust under APS throttling and unstable networks.

#### Issue: Implement retry/backoff strategy for 429/5xx

**Labels:** `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.5-005-retry-backoff-strategy.md`

**Description:** Add standardized retry policy with jitter.

**Acceptance Criteria:**

* Default retry for 429, 500–599
* Configurable max retries + max wait
* Clear logging in verbose/debug
* Unit tests for retry logic

#### Issue: Add configurable request timeouts + concurrency limits

**Labels:** `type:feature`, `prio:med`  
**File:** `.github/ISSUES/v0.5-006-timeouts-concurrency-limits.md`

**Description:** Avoid hanging jobs and control parallelism in bulk operations.

**Acceptance Criteria:**

* `--timeout <sec>` or config
* `--concurrency <n>` for bulk commands
* Safe defaults documented

#### Issue: Proxy support documentation (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`)

**Labels:** `type:docs`, `prio:med`  
**File:** `.github/ISSUES/v0.5-007-proxy-support-docs.md`

**Description:** Make corporate network usage straightforward.

**Acceptance Criteria:**

* One doc page with examples
* Troubleshooting section for TLS interception, cert issues (as guidance)

---

## Milestone v0.6 — Supply-chain, UX polish, Open-source hygiene

### EPIC: Release integrity & provenance

**Goal:** Make binaries trustworthy for enterprise adoption.

#### Issue: Publish SHA256 checksums for release artifacts

**Labels:** `type:feature`, `prio:high`  
**File:** `.github/ISSUES/v0.6-001-sha256-checksums.md`

**Acceptance Criteria:**

* Each GitHub release includes `checksums.txt`
* Docs show how to verify checksums on Windows/macOS/Linux

#### Issue: (Optional) SBOM + build provenance

**Labels:** `type:feature`, `prio:med`  
**File:** `.github/ISSUES/v0.6-002-sbom-build-provenance.md`

**Acceptance Criteria:**

* Generate SBOM (CycloneDX or SPDX)
* Attach to releases or publish in artifacts
* Document consumption

### EPIC: Repo quality & contributor workflow

**Goal:** Reduce friction, keep repo clean.

#### Issue: Add `CHANGELOG.md` with Keep a Changelog format

**Labels:** `type:docs`, `prio:med`  
**File:** `.github/ISSUES/v0.6-003-changelog.md`

**Acceptance Criteria:**

* Changelog entries for v0.4+ changes
* Linked to GitHub releases

#### Issue: Add Issue/PR templates + CODE_OF_CONDUCT

**Labels:** `type:chore`, `prio:low`  
**File:** `.github/ISSUES/v0.6-004-issue-pr-templates.md`

**Acceptance Criteria:**

* Bug report template includes `raps --version`, OS, repro steps
* Feature request template includes expected output format + CI requirements

#### Issue: Remove accidental artifacts from repo + extend `.gitignore`

**Labels:** `type:chore`, `prio:med`  
**File:** `.github/ISSUES/v0.6-005-cleanup-repo-artifacts.md`

**Acceptance Criteria:**

* Dev logs/build artifacts are not tracked
* `.gitignore` updated
* CI remains green

---

## Suggested label set (create once)

* `prio:high`, `prio:med`, `prio:low`
* `type:feature`, `type:docs`, `type:chore`, `type:bug`
* `epic:ci`, `epic:auth`, `epic:profiles`, `epic:reliability`, `epic:release`

---

## Summary

- **Total Issues:** 15
  - **v0.4:** 4 issues (all high priority)
  - **v0.5:** 7 issues (4 high, 3 medium priority)
  - **v0.6:** 4 issues (1 high, 2 medium, 1 low priority)

- **Epics:** 5
  - Non-interactive + CI-friendly behavior (v0.4)
  - Profiles (contexts) & secrets handling (v0.5)
  - Headless-friendly authentication (v0.5)
  - Reliability: retries, backoff, timeouts, rate limits (v0.5)
  - Release integrity & provenance (v0.6)
  - Repo quality & contributor workflow (v0.6)

## Import Instructions

See `roadmap/README.md` for detailed instructions on importing these issues into GitHub.

