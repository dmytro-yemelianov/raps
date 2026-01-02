# Release Process

This guide explains how to create a new release of RAPS.

## Prerequisites

- ✅ All changes merged to `main` branch
- ✅ CI checks passing
- ✅ Version number decided (following [Semantic Versioning](https://semver.org/))

## Release Steps

### Step 1: Create Release Branch

```bash
# Make sure you're on main and up to date
git checkout main
git pull origin main

# Create a release branch
git checkout -b release/v0.2.0
```

### Step 2: Update Version

Update the version in `Cargo.toml`:

```toml
version = "0.2.0"  # Change to your new version
```

### Step 3: Update CHANGELOG (if you have one)

Document what changed in this release. If you don't have a CHANGELOG, consider creating one.

### Step 4: Commit and Push

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.2.0"
git push origin release/v0.2.0
```

### Step 5: Create Pull Request

1. Go to GitHub and create a PR from `release/v0.2.0` to `main`
2. Title: `Release v0.2.0`
3. Wait for CI to pass
4. Merge the PR

### Step 6: Create Git Tag

After the PR is merged:

```bash
# Pull the merged changes
git checkout main
git pull origin main

# Create and push the tag
git tag v0.2.0
git push origin v0.2.0
```

### Step 7: GitHub Actions Will Automatically

- ✅ Build binaries for all platforms:
  - Windows x64 (`raps-windows-x64.zip`)
  - macOS Intel (`raps-macos-x64.tar.gz`)
  - macOS Apple Silicon (`raps-macos-arm64.tar.gz`)
  - Linux x64 (`raps-linux-x64.tar.gz`)
  - Linux ARM64 (`raps-linux-arm64.tar.gz`)
- ✅ Generate SHA256 checksums
- ✅ Create GitHub Release with all artifacts
- ✅ Generate release notes automatically

### Alternative: Manual Workflow Dispatch

If you prefer to trigger the release manually:

1. Go to **Actions** → **Release** workflow
2. Click **Run workflow**
3. Enter version tag (e.g., `v0.2.0`)
4. Click **Run workflow**

The workflow will build and create the release.

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking changes
- **MINOR** (0.2.0): New features, backward compatible
- **PATCH** (0.2.1): Bug fixes, backward compatible

Pre-release versions (e.g., `v0.3.0-beta.1`) are automatically marked as pre-releases.

## Verifying the Release

After the release is created:

1. ✅ Check GitHub Releases page: https://github.com/dmytro-yemelianov/raps/releases
2. ✅ Verify all platform binaries are present
3. ✅ Test downloading and running a binary
4. ✅ Verify checksums match

## Publishing to crates.io

If you want to publish to crates.io:

1. Set up `CARGO_REGISTRY_TOKEN` secret in GitHub repository settings
2. Get token from: https://crates.io/settings/tokens
3. The `publish.yml` workflow will automatically publish when a release is created

## Quick Reference

```bash
# Full release workflow
git checkout main
git pull origin main
git checkout -b release/v0.2.0

# Edit Cargo.toml version
# ... make changes ...

git add Cargo.toml
git commit -m "chore: bump version to 0.2.0"
git push origin release/v0.2.0

# Create PR, merge it, then:
git checkout main
git pull origin main
git tag v0.2.0
git push origin v0.2.0
```

