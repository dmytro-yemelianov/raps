# RAPS CLI Roadmap (v0.4 → v0.6)

This directory contains the roadmap for RAPS CLI development from version 0.4 through 0.6.

## Structure

- **`.github/ISSUES/`** - Individual issue markdown files ready for GitHub
- **`roadmap-v0.4-v0.6.json`** - JSON export for programmatic import into GitHub Issues

## Milestones Overview

### v0.4 — CI/CD & Automation Ready

**Goal:** Every command can run in CI without prompts, colors, or unstable output.

**Key Features:**
- Global output format selector (`--output json|yaml|table`)
- Standardized exit codes
- Logging flags (`--no-color`, `--quiet`, `--verbose`, `--debug`)
- Non-interactive mode (`--yes`, `--non-interactive`)

**Epic:** Non-interactive + CI-friendly behavior

### v0.5 — Profiles, Auth, Reliability

**Goal:** Easy switching between environments and robust operation under adverse conditions.

**Key Features:**
- Profile management (`raps config profile`)
- Headless authentication (device-code flow, token-based)
- Retry/backoff strategy for API errors
- Configurable timeouts and concurrency limits
- Proxy support documentation

**Epics:**
- Profiles (contexts) & secrets handling
- Headless-friendly authentication
- Reliability: retries, backoff, timeouts, rate limits

### v0.6 — Supply-chain, UX polish, Open-source hygiene

**Goal:** Make binaries trustworthy for enterprise adoption and improve contributor experience.

**Key Features:**
- SHA256 checksums for releases
- Optional SBOM + build provenance
- CHANGELOG.md
- Issue/PR templates
- Repository cleanup

**Epics:**
- Release integrity & provenance
- Repo quality & contributor workflow

## Label System

The roadmap uses the following label structure:

### Priority
- `prio:high` - Critical for milestone completion
- `prio:med` - Important but can be deferred
- `prio:low` - Nice to have

### Type
- `type:feature` - New functionality
- `type:docs` - Documentation
- `type:chore` - Maintenance tasks
- `type:bug` - Bug fixes

### Epic
- `epic:ci` - CI/CD & Automation
- `epic:auth` - Authentication improvements
- `epic:profiles` - Profile management
- `epic:reliability` - Reliability & robustness
- `epic:release` - Release process

## Importing Issues

### Using GitHub CLI

```bash
# Install GitHub CLI if not already installed
# https://cli.github.com/

# Authenticate
gh auth login

# Import issues from JSON
gh issue create --title "$(jq -r '.[0].title' roadmap-v0.4-v0.6.json)" \
  --body "$(jq -r '.[0].body' roadmap-v0.4-v0.6.json)" \
  --label "$(jq -r '.[0].labels | join(",")' roadmap-v0.4-v0.6.json)"
```

### Using GitHub API

See the [GitHub Issues API documentation](https://docs.github.com/en/rest/issues/issues#create-an-issue) for programmatic creation.

### Manual Import

Each issue is available as a markdown file in `.github/ISSUES/` with the format:
- `v0.4-001-global-output-format.md`
- `v0.4-002-standardize-exit-codes.md`
- etc.

Copy the content and create issues manually in GitHub.

## Current Status

- **Current Version:** v0.3.0
- **Next Milestone:** v0.4 — CI/CD & Automation Ready
- **Total Issues:** 17
  - v0.4: 5 issues (1 partially implemented, 1 new)
  - v0.5: 8 issues (1 new)
  - v0.6: 4 issues

## Review Status

✅ **Reviewed:** All issues have been reviewed against the current codebase implementation. See `REVIEW_SUMMARY.md` for detailed findings.

## Notes

- All issues include acceptance criteria for clear completion definitions
- Issues are prioritized within each milestone
- Some features (like YAML output) may be deferred to later versions if implementation complexity is high
- Security considerations are noted where relevant (e.g., secret redaction in debug logs)

