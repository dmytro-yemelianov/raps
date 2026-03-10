---
name: verifying-raps-release
version: "1.0"
description: Use after a RAPS release workflow completes to verify all distribution channels — GitHub release assets, npm packages, PyPI, install script, and binary functionality.
---

# Verifying a RAPS Release

Post-release verification checklist across all distribution channels.

**Repo:** `/root/github/raps/raps`

## Verification Steps

### 1. GitHub Release

```bash
VERSION="4.16.0"

# Check release exists and has all 5 platform binaries
gh release view "v${VERSION}" --json assets --jq '.assets[].name'
```

Expected assets (5 binaries + installers):
- `raps-x86_64-pc-windows-msvc.zip`
- `raps-x86_64-apple-darwin.tar.xz`
- `raps-aarch64-apple-darwin.tar.xz`
- `raps-x86_64-unknown-linux-gnu.tar.xz`
- `raps-aarch64-unknown-linux-gnu.tar.xz`

### 2. npm Packages

```bash
# Main package — verify version and all 5 optionalDependencies
npm view @dmytro-yemelianov/raps-cli@${VERSION} version optionalDependencies

# Each platform package exists
for pkg in win32-x64 darwin-x64 darwin-arm64 linux-x64 linux-arm64; do
  npm view "@dmytro-yemelianov/raps-cli-${pkg}@${VERSION}" version
done
```

Critical check: `optionalDependencies` must list ALL 5 platforms, not just one.

### 3. Install Script

```bash
# Test install.sh (uses GitHub release)
curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | RAPS_VERSION="${VERSION}" bash
~/.raps/bin/raps --version
```

### 4. npm Install

```bash
# Test global npm install (in a clean env or with npx)
npm install -g @dmytro-yemelianov/raps-cli@${VERSION}
raps --version
```

### 5. pip Install

```bash
pip install raps-aps==${VERSION}
raps --version
```

### 6. Functional Smoke Test

```bash
raps --version               # Version string matches
raps auth test --output json # Auth works (requires configured credentials)
raps bucket list             # API call succeeds
```

## Known Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| npm install fails with "not found" | Platform package missing | Re-publish platform package |
| npm install succeeds but `raps` not found | Only 1 of 5 optionalDeps published | Check release.yml sed commands |
| install.sh downloads wrong version | Tag not pushed or release not created | Check `gh release list` |
| PyPI package missing | publish-pypi job failed | Re-run workflow or manual `maturin publish` |
