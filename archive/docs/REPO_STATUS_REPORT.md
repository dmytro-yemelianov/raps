# RAPS Ecosystem Repository Status Report
**Generated**: 2025-12-30

## Executive Summary

### Critical Issues
- ❌ **v3.2.0 Release**: No artifacts attached (workflow failures)
- ⚠️ **Release Workflow**: Failed 3 times due to GitHub Actions cache issues
- ⚠️ **Package Managers**: `homebrew-tap` and `scoop-bucket` still on v3.1.0

### Status Overview
- ✅ **16 repositories** tracked
- ✅ **Main application** (`raps`) on `main` branch
- ✅ **Service crates** extracted and pushed to GitHub
- ✅ **Website** updated with Community/Pro pages
- ⚠️ **Distribution repos** need version updates

---

## Repository Details

### Core Application

#### `raps` (Primary CLI)
- **Branch**: `main`
- **Latest Commit**: `6c18205` - chore: fix CHANGELOG - remove duplicate release note for 3.2.0
- **Remote**: https://github.com/dmytro-yemelianov/raps.git
- **Releases**: 
  - ✅ v3.2.0 (2025-12-29) - **NO ARTIFACTS** ⚠️
  - ✅ v3.1.0 (2025-12-29)
  - ✅ v3.0.0 (2025-12-27)
- **Open PRs**: 5 (4 dependabot, 1 draft)
- **Workflows**: 6 active
  - ❌ Release workflow failed (3 attempts)
  - ❌ Publish to crates.io failed
  - ✅ CI passing for PRs

**Issues**:
- Release workflow failed due to GitHub Actions cache service outage
- No binaries attached to v3.2.0 release
- Need to retry release workflow

---

### Service Crates (Microkernel Architecture)

#### `raps-kernel`
- **Branch**: `main`
- **Latest Commit**: `2a5c6cb` - Initial commit: RAPS Kernel - Microkernel foundation
- **Remote**: https://github.com/dmytro-yemelianov/raps-kernel.git
- **Status**: ✅ Pushed to GitHub
- **PRs**: 0
- **Releases**: 0

#### `raps-oss`
- **Branch**: `main`
- **Latest Commit**: `e97361d` - Initial commit: RAPS OSS - Object Storage Service
- **Remote**: https://github.com/dmytro-yemelianov/raps-oss.git
- **Status**: ✅ Pushed to GitHub
- **PRs**: 0
- **Releases**: 0

#### `raps-derivative`
- **Branch**: `main`
- **Latest Commit**: `7b67f96` - Initial commit: RAPS Derivative - Model Derivative Service
- **Remote**: https://github.com/dmytro-yemelianov/raps-derivative.git
- **Status**: ✅ Pushed to GitHub
- **PRs**: 0
- **Releases**: 0

#### `raps-dm`
- **Branch**: `main`
- **Latest Commit**: `b045950` - Initial commit: RAPS DM - Data Management Service
- **Remote**: https://github.com/dmytro-yemelianov/raps-dm.git
- **Status**: ✅ Pushed to GitHub
- **PRs**: 0
- **Releases**: 0

#### `raps-community`
- **Branch**: `main`
- **Latest Commit**: `a135396` - Initial commit: RAPS Community - Community tier features
- **Remote**: https://github.com/dmytro-yemelianov/raps-community.git
- **Status**: ✅ Pushed to GitHub
- **PRs**: 0
- **Releases**: 0

---

### Documentation & Website

#### `raps-website`
- **Branch**: `main`
- **Latest Commit**: `40b0b95` - feat: add Community and Pro tier pages
- **Remote**: https://github.com/dmytro-yemelianov/raps-website.git
- **Status**: ✅ Updated with tier pages
- **PRs**: 0
- **Releases**: 0

---

### Distribution Satellites

#### `homebrew-tap`
- **Branch**: `main`
- **Latest Commit**: `0a13e46` - Update to v3.1.0
- **Remote**: https://github.com/dmytro-yemelianov/homebrew-tap.git
- **Status**: ⚠️ **OUTDATED** - Still on v3.1.0, needs update to v3.2.0
- **PRs**: 0
- **Releases**: 0

#### `scoop-bucket`
- **Branch**: `main`
- **Latest Commit**: `eda4e12` - Update to v3.1.0
- **Remote**: https://github.com/dmytro-yemelianov/scoop-bucket.git
- **Status**: ⚠️ **OUTDATED** - Still on v3.1.0, needs update to v3.2.0
- **PRs**: 0
- **Releases**: 0

#### `raps-action`
- **Branch**: `main`
- **Latest Commit**: `0c3b5d0` - feat: complete GitHub Action for marketplace
- **Remote**: https://github.com/dmytro-yemelianov/raps-action.git
- **Status**: ✅ Latest release v1.0.0
- **PRs**: 0
- **Releases**: ✅ v1.0.0 (2025-12-26)

#### `raps-docker`
- **Branch**: `main`
- **Latest Commit**: `c0e4278` - feat: add GHCR support
- **Remote**: https://github.com/dmytro-yemelianov/raps-docker.git
- **Status**: ✅ Updated
- **PRs**: 0
- **Releases**: 0

---

### Ecosystem Extensions

#### `aps-tui`
- **Branch**: `master` (note: different from others)
- **Latest Commit**: `3c90f3d` - feat: add raps microkernel dependencies
- **Remote**: https://github.com/dmytro-yemelianov/aps-tui.git
- **Status**: ✅ Updated with microkernel dependencies
- **PRs**: 0
- **Releases**: 0

#### `aps-wasm-demo`
- **Branch**: `main`
- **Latest Commit**: `f749e1c` - 🌼 Add RAPS ecosystem reference and rapscli.xyz link
- **Remote**: https://github.com/dmytro-yemelianov/aps-wasm-demo.git
- **Status**: ✅ Updated
- **PRs**: 0
- **Releases**: 0

#### `aps-demo-scripts`
- **Branch**: `main`
- **Latest Commit**: `b2f5c7d` - 🌼 Update to RAPS branding and rapscli.xyz
- **Remote**: https://github.com/dmytro-yemelianov/aps-demo-scripts.git
- **Status**: ✅ Updated
- **PRs**: 0
- **Releases**: 0

#### `aps-sdk-openapi`
- **Branch**: `main`
- **Latest Commit**: `56de0d9` - Merge pull request #17 from autodesk-platform-services/development
- **Remote**: https://github.com/autodesk-platform-services/aps-sdk-openapi.git
- **Status**: ✅ Upstream repository (Autodesk)
- **PRs**: N/A
- **Releases**: N/A

#### `raps-smm`
- **Branch**: `main`
- **Latest Commit**: `399a75c` - feat: Introduce MCP Server for AI Integration with Autodesk Platform Services
- **Remote**: https://github.com/dmytro-yemelianov/raps-smm.git
- **Status**: ✅ Updated
- **PRs**: 0
- **Releases**: 0

---

## GitHub Actions Status

### `raps` Repository Workflows

| Workflow | Status | Latest Run | Notes |
|----------|--------|------------|-------|
| **Release** | ❌ Failed | 2025-12-30 07:14 | Failed 3 times - cache service issue |
| **Publish to crates.io** | ❌ Failed | 2025-12-29 23:03 | Depends on Release workflow |
| **CI** | ✅ Passing | 2025-12-29 10:40 | All PR checks passing |
| **Branch Protection Check** | ✅ Active | - | Protection enabled |
| **Deploy Documentation** | ✅ Active | - | Website deployment |
| **Dependabot Updates** | ✅ Active | - | Automated dependency updates |

### Workflow Failures Analysis

**Release Workflow (3 failures)**:
1. **Run #20584602103** (2025-12-29 23:03) - Push trigger - Failed: sccache cache service unavailable
2. **Run #20584883902** (2025-12-30 07:14) - Manual trigger - Failed: Same cache issue
3. **Run #20584605910** (2025-12-29 23:03) - Publish workflow - Failed: Depends on Release

**Root Cause**: GitHub Actions artifact cache service was unavailable during release attempts.

**Solution**: Retry release workflow once cache service is stable.

---

## Pull Requests Summary

### `raps` Repository
- **Total Open PRs**: 5
  - **Dependabot**: 4 (dependency updates)
    - #106: indicatif 0.17.11 → 0.18.3
    - #105: dialoguer 0.11.0 → 0.12.0
    - #103: schemars 0.8.22 → 1.2.0
    - #102: serde_json 1.0.147 → 1.0.148
  - **Draft**: 1
    - #108: Document edition strategy and APS API roadmap (DRAFT)

**Action Required**: Review and merge dependabot PRs.

---

## Releases Summary

### `raps` Repository
- **v3.2.0** (2025-12-29) - ⚠️ **NO ARTIFACTS**
- **v3.1.0** (2025-12-29) - ✅ Complete
- **v3.0.0** (2025-12-27) - ✅ Complete

### Other Repositories
- **`raps-action`**: v1.0.0 (2025-12-26) - ✅ Complete
- **Service crates**: No releases yet (initial commits only)

---

## Action Items

### 🔴 Critical (Immediate)
1. **Fix v3.2.0 Release**
   - Retry release workflow once GitHub Actions cache is stable
   - Verify artifacts are attached (5 binaries + checksums.txt)
   - Update release notes if needed

2. **Update Package Managers**
   - Update `homebrew-tap` to v3.2.0
   - Update `scoop-bucket` to v3.2.0
   - Wait for v3.2.0 artifacts to be available

### 🟡 Important (This Week)
3. **Review Dependabot PRs**
   - Merge compatible dependency updates
   - Test breaking changes (schemars 0.8 → 1.2)

4. **Service Crate Releases**
   - Consider creating initial releases for service crates
   - Tag versions aligned with `raps` v3.2.0

### 🟢 Nice to Have
5. **Documentation**
   - Update installation guides with v3.2.0
   - Document microkernel architecture
   - Add service crate documentation

---

## Repository Health Metrics

| Metric | Value |
|--------|-------|
| **Total Repositories** | 16 |
| **Public Repositories** | 15 |
| **Private Repositories** | 1 (`raps-pro`) |
| **Repositories with Releases** | 2 (`raps`, `raps-action`) |
| **Open Pull Requests** | 5 (all in `raps`) |
| **Failed Workflows** | 3 (all Release-related) |
| **Outdated Distribution** | 2 (`homebrew-tap`, `scoop-bucket`) |

---

## Next Steps

1. **Monitor Release Workflow**: Wait for GitHub Actions cache service to stabilize, then retry
2. **Update Distribution**: Once v3.2.0 artifacts are available, update package managers
3. **Review PRs**: Process dependabot PRs and draft PR #108
4. **Service Crate Releases**: Consider versioning strategy for service crates

---

**Report Generated**: 2025-12-30  
**Last Updated**: 2025-12-30
