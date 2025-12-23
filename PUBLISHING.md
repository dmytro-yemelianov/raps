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

Publishing to crates.io happens automatically when a release is published (via the `publish.yml` workflow).

### Manual Publishing (if needed)

```bash
# Verify the package
cargo package --allow-dirty

# Publish (requires CARGO_REGISTRY_TOKEN)
cargo publish --token $CARGO_REGISTRY_TOKEN
```

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
- Verify `CARGO_REGISTRY_TOKEN` is set correctly
- Check that the version hasn't been published before
- Ensure `Cargo.toml` metadata is correct

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

