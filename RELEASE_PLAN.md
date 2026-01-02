# Release Plan for v3.3.0

## Current Status

- **Workspace Version**: 3.3.0
- **Latest Release**: v3.2.0
- **Current Branch**: `001-gitlab-orchestration` (local)
- **Main Branch**: `main` (GitHub)

## Open Pull Requests

1. **PR #108** - Draft: Document edition strategy (skip for now)
2. **PR #106** - Dependabot: indicatif 0.17.11 → 0.18.3 (merge)
3. **PR #105** - Dependabot: dialoguer 0.11.0 → 0.12.0 (merge)
4. **PR #103** - Dependabot: schemars 0.8.22 → 1.2.0 (merge)
5. **PR #102** - Dependabot: serde_json 1.0.147 → 1.0.148 (merge)

## Distribution Channels Updated

- ✅ Homebrew Tap: 3.1.0 → 3.3.0
- ✅ Scoop Bucket: 3.1.0 → 3.3.0 (hash needs update after release)
- ✅ Docker: 2.0.0 → 3.3.0
- ✅ GitHub Action: Uses latest API (no update needed)

## Release Steps

### Step 1: Merge Dependabot PRs
- Merge PR #106 (indicatif)
- Merge PR #105 (dialoguer)
- Merge PR #103 (schemars)
- Merge PR #102 (serde_json)

### Step 2: Update CHANGELOG.md
Add v3.3.0 entry with:
- Monorepo consolidation
- Distribution channel updates
- Dependency updates

### Step 3: Create Release Branch
- Create `release/v3.3.0` branch from `main`
- Commit any pending changes
- Push to GitHub

### Step 4: Create GitHub Release
- Tag: `v3.3.0`
- Title: `v3.3.0 - Monorepo Consolidation`
- Description: Include changelog entry

### Step 5: Push Distribution Updates
- Push homebrew-tap changes
- Push scoop-bucket changes
- Push raps-docker changes

### Step 6: Build and Publish
- CI/CD will build artifacts automatically
- Update Scoop hash after artifacts are available

## Notes

- PR #108 is a draft and should be reviewed separately
- Distribution channels are already updated locally
- Release will trigger CI/CD to build artifacts
