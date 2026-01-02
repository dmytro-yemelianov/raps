# Publishing Guide

This document describes the process for publishing releases of APS CLI to GitHub and crates.io.

## Prerequisites

1. **GitHub Secrets** (configured in repository settings):
   - `CARGO_REGISTRY_TOKEN`: Token for publishing to crates.io
     - Get from: https://crates.io/settings/tokens
     - Required scopes: `publish-new-crate` or `api`

2. **Repository Permissions**:
   - Write access to `contents` (for creating releases)
   - Write access to `actions` (for running workflows)

## Release Process

### Option 1: Tag-Based Release (Recommended)

1. **Update version in `Cargo.toml`**:
   ```toml
   version = "0.3.0"
   ```

2. **Commit and push**:
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 0.3.0"
   git push origin main
   ```

3. **Create and push a tag**:
   ```bash
   git tag v0.3.0
   git push origin v0.3.0
   ```

4. **GitHub Actions will automatically**:
   - Build binaries for all platforms (Windows x64, macOS Intel/ARM64, Linux x64/ARM64)
   - Create a GitHub Release with all artifacts
   - Generate checksums for verification
   - Publish to crates.io (if `CARGO_REGISTRY_TOKEN` is configured)

### Option 2: Manual Workflow Dispatch

1. Go to **Actions** → **Release** → **Run workflow**
2. Enter the version tag (e.g., `v0.3.0`)
3. Click **Run workflow**
4. The workflow will build and create a release

## Release Artifacts

Each release includes:

- **Windows x64**: `raps-windows-x64.zip`
- **macOS Intel**: `raps-macos-x64.tar.gz`
- **macOS Apple Silicon**: `raps-macos-arm64.tar.gz`
- **Linux x64**: `raps-linux-x64.tar.gz`
- **Linux ARM64**: `raps-linux-arm64.tar.gz`
- **Checksums**: `checksums.txt` (SHA256)

## Publishing to crates.io

Publishing to crates.io happens **automatically** when a GitHub Release is published. The `publish.yml` workflow is triggered by the `release` event and uses **Trusted Publishing** (OIDC) - no API tokens needed!

### Automatic Publishing Flow

1. **Create and push a version tag** (see Option 1 above)
2. **GitHub Actions builds binaries** and creates a GitHub Release
3. **Publishing the GitHub Release triggers** the `publish.yml` workflow
4. **The workflow automatically publishes** to crates.io using Trusted Publishing (OIDC)

### Prerequisites for crates.io Publishing (Trusted Publishing)

**No API tokens needed!** This uses crates.io's Trusted Publishing feature with OIDC.

1. **Publish the first version manually** (one-time setup):
   ```bash
   cargo login
   # Enter your crates.io token when prompted
   cargo publish
   ```
   This establishes the crate on crates.io.

2. **Set up Trusted Publishing**:
   - Go to your crate page on crates.io: https://crates.io/crates/raps
   - Click the **"Settings"** tab
   - Scroll to **"Trusted Publishers"** section
   - Click **"Add Trusted Publisher"**
   - Select **"GitHub"**
   - Enter your repository: `dmytro-yemelianov/raps` (or your org/repo)
   - Click **"Add Publisher"**

3. **That's it!** The workflow will automatically authenticate using OIDC for all future releases.

### Alternative: Token-Based Publishing (Legacy)

If you prefer the old token-based method:

1. **Get a crates.io API token**:
   - Go to https://crates.io/settings/tokens
   - Click "New Token"
   - Name it (e.g., "GitHub Actions")
   - Select scope: `publish-new-crate` or `api`
   - Copy the token (starts with `cargo_`)

2. **Add token to GitHub Secrets**:
   - Go to your repository → **Settings** → **Secrets and variables** → **Actions**
   - Click **New repository secret**
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: Paste your crates.io token
   - Click **Add secret**

3. **Update `publish.yml`** to use the token instead of Trusted Publishing.

### Manual Publishing (if needed)

If you need to publish manually (e.g., for testing or if automatic publishing fails):

```bash
# 1. Login to crates.io (first time only)
cargo login
# Enter your token when prompted

# 2. Verify the package
cargo package --allow-dirty

# 3. Check what will be published
cargo publish --dry-run

# 4. Publish
cargo publish
```

Or using an environment variable:

```bash
# Set token
export CARGO_REGISTRY_TOKEN="your_token_here"

# Verify
cargo package --allow-dirty

# Publish
cargo publish --token $CARGO_REGISTRY_TOKEN
```

**Note**: Make sure the version in `Cargo.toml` hasn't been published before. crates.io doesn't allow republishing the same version.

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR** (1.0.0): Breaking changes
- **MINOR** (0.2.0): New features, backward compatible
- **PATCH** (0.2.1): Bug fixes, backward compatible

Pre-release versions (e.g., `v0.3.0-beta.1`) are automatically marked as pre-releases.

## Troubleshooting

### Release fails to create
- Check that the tag doesn't already exist
- Verify GitHub Actions permissions
- Check workflow logs for errors

### crates.io publish fails
- **Trusted Publishing issues** (if using OIDC):
  - Verify Trusted Publisher is configured on crates.io (Settings → Trusted Publishers)
  - Check that repository name matches exactly (owner/repo)
  - Ensure workflow has `id-token: write` permission
  - Verify the first version was published manually
- **Token issues** (if using legacy token method):
  - Verify `CARGO_REGISTRY_TOKEN` is set in GitHub Secrets
  - Check token hasn't expired (regenerate if needed)
  - Ensure token has `publish-new-crate` or `api` scope
- **Version already exists**: Check https://crates.io/crates/raps/versions
- **Metadata issues**:
  - Ensure `Cargo.toml` has all required fields (name, version, description, license, authors)
  - Check that `repository`, `homepage`, `documentation` URLs are valid
  - Verify `readme` file exists and is included
- **Package validation**:
  - Run `cargo package --allow-dirty` locally to check for errors
  - Check for excluded files that shouldn't be excluded
  - Verify no large files (>10MB) are included

### Build fails for specific platform
- Check the build matrix in `.github/workflows/release.yml`
- Verify cross-compilation tools are installed (for Linux ARM64)
- Check Rust toolchain compatibility

## Post-Release Checklist

- [ ] Verify all artifacts are present in the GitHub Release
- [ ] Test installation from crates.io: `cargo install raps`
- [ ] Test downloading and running binaries from GitHub Releases
- [ ] Update any external documentation referencing the version
- [ ] Announce the release (if applicable)

