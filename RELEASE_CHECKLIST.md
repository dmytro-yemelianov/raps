# Release Checklist for v3.3.0

## ✅ Completed

- [x] Updated CHANGELOG.md with v3.3.0 entry
- [x] Updated distribution channels to version 3.3.0:
  - [x] Homebrew Tap (`homebrew-tap/Formula/raps.rb`)
  - [x] Scoop Bucket (`scoop-bucket/bucket/raps.json`)
  - [x] Docker (`raps-docker/Dockerfile` and `README.md`)
- [x] Verified website references monorepo structure correctly

## ⏳ Requires Manual Action

### Step 1: Merge Dependabot PRs
Merge these PRs via GitHub UI or CLI:
- [ ] PR #106: `chore(deps): bump indicatif from 0.17.11 to 0.18.3`
- [ ] PR #105: `chore(deps): bump dialoguer from 0.11.0 to 0.12.0`
- [ ] PR #103: `chore(deps): bump schemars from 0.8.22 to 1.2.0`
- [ ] PR #102: `chore(deps): bump serde_json from 1.0.147 to 1.0.148`

**Note**: PR #108 is a draft and should be reviewed separately.

### Step 2: Commit and Push Local Changes
```bash
cd "C:\github\Autodesk APS\raps"
git checkout main
git pull origin main
git add raps/CHANGELOG.md
git commit -m "chore: update CHANGELOG for v3.3.0"
git push origin main
```

### Step 3: Push Distribution Channel Updates
```bash
# Homebrew Tap
cd "C:\github\Autodesk APS\homebrew-tap"
git add Formula/raps.rb
git commit -m "chore: update raps to v3.3.0"
git push origin main

# Scoop Bucket
cd "C:\github\Autodesk APS\scoop-bucket"
git add bucket/raps.json
git commit -m "chore: update raps to v3.3.0"
git push origin main

# Docker
cd "C:\github\Autodesk APS\raps-docker"
git add Dockerfile README.md
git commit -m "chore: update raps to v3.3.0"
git push origin main
```

### Step 4: Create GitHub Release
1. Go to: https://github.com/dmytro-yemelianov/raps/releases/new
2. Tag: `v3.3.0`
3. Title: `v3.3.0 - Monorepo Consolidation`
4. Description: Copy from CHANGELOG.md v3.3.0 section
5. Publish release

### Step 5: Update Scoop Hash (After Release)
After CI/CD builds artifacts and publishes release:
1. Download `raps-windows-x64.zip` from release
2. Calculate SHA256 hash: `Get-FileHash raps-windows-x64.zip -Algorithm SHA256`
3. Update `scoop-bucket/bucket/raps.json` with new hash
4. Commit and push

### Step 6: Verify Distribution Channels
- [ ] Homebrew: `brew install dmytro-yemelianov/tap/raps` works
- [ ] Scoop: `scoop install raps` works
- [ ] Docker: `docker pull dmytroyemelianov/raps:3.3.0` works
- [ ] GitHub Action: Test with `raps-action@v1` in workflow

## Release Notes Template

```markdown
# v3.3.0 - Monorepo Consolidation

## What's Changed

### Monorepo Architecture
- Consolidated all workspace crates into single `raps/` repository
- Unified Cargo workspace with shared dependencies
- Removed separate git repositories, unified history

### Distribution Updates
- Updated Homebrew Tap to v3.3.0
- Updated Scoop Bucket to v3.3.0
- Updated Docker image to v3.3.0

### Dependency Updates
- Updated indicatif to 0.18.3
- Updated dialoguer to 0.12.0
- Updated schemars to 1.2.0
- Updated serde_json to 1.0.148

**Full Changelog**: https://github.com/dmytro-yemelianov/raps/compare/v3.2.0...v3.3.0
```

## Notes

- All distribution channels are already updated locally
- CHANGELOG.md has been updated with v3.3.0 entry
- CI/CD will automatically build artifacts when release is created
- Scoop hash needs manual update after artifacts are available
