# Distribution Channels Update

**Date**: 2026-01-16
**Workspace Version**: 4.0.0

## Updated Distribution Channels

### ✅ Homebrew Tap (`homebrew-tap`)

- **Version**: Updated from `3.11.0` → `4.0.0`
- **File**: `homebrew-tap/Formula/raps.rb`
- **Status**: ⏳ Pending (Wait for artifacts)
- **Note**: SHA256 hashes will be updated automatically when v4.0.0 release is published

### ✅ Scoop Bucket (`scoop-bucket`)

- **Version**: Updated from `3.11.0` → `4.0.0`
- **File**: `scoop-bucket/bucket/raps.json`
- **Status**: ⏳ Pending (Wait for artifacts)
- **Note**: Hash needs manual update after v4.0.0 release artifacts are available

### ✅ Docker Image (`raps-docker`)

- **Version**: Updated from `3.11.0` → `4.0.0`
- **File**: `raps-docker/Dockerfile`
- **Status**: ⏳ Pending
- **README**: Updated version references and build examples

### ✅ GitHub Action (`raps-action`)

- **Version**: Uses latest API (no hardcoded version)
- **File**: `raps-action/action.yml`
- **Status**: ✅ No update needed (uses dynamic version detection)

## Website Status

### ✅ RAPS Website (`raps-website`)

- **Repository Reference**: Correctly points to `github.com/dmytro-yemelianov/raps`
- **Architecture Docs**: Already document microkernel structure
- **Monorepo References**: No separate repo references found
- **Status**: ✅ No updates needed

## Next Steps

1. **Release v4.0.0** with artifacts:
   - `raps-macos-x64.tar.gz`
   - `raps-macos-arm64.tar.gz`
   - `raps-linux-x64.tar.gz`
   - `raps-linux-arm64.tar.gz`
   - `raps-windows-x64.zip`

2. **Update SHA256 hashes**:
   - Homebrew: Hashes will be auto-updated by formula
   - Scoop: Update hash in `scoop-bucket/bucket/raps.json` after release

3. **Build and push Docker images**:
   ```bash
   docker buildx build --platform linux/amd64,linux/arm64 \
     -t dmytroyemelianov/raps:4.0.0 \
     --build-arg VERSION=4.0.0 \
     --push .
   ```

4. **Verify distribution channels**:
   - Test Homebrew installation: `brew install dmytro-yemelianov/tap/raps`
   - Test Scoop installation: `scoop install raps`
   - Test Docker image: `docker pull dmytroyemelianov/raps:4.0.0`
   - Test GitHub Action: Use `raps-action@v1` in workflow

## Notes

- All distribution channels now reference version `4.0.0`
- Website correctly references monorepo structure
- Major version bump for Account Admin Bulk Management Tool feature
- GitHub Action uses dynamic version detection (no update needed)
