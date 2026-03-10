---
name: cutting-raps-release
version: "1.0"
description: Use when bumping the RAPS version for a new release — updates Cargo.toml workspace version, 9 internal crate dependencies, 6 npm package.json files, creates git tag, and triggers the release workflow.
---

# Cutting a RAPS Release

Bump version across all files, tag, and trigger the release workflow.

**Repo:** `/root/github/raps/raps`

## Files to Update (16 version locations)

### Cargo.toml (workspace root)

| Location | Pattern |
|----------|---------|
| Line ~18: `[workspace.package]` | `version = "X.Y.Z"` |
| Lines ~29-37: `[workspace.dependencies]` | 9 crates with `version = "X.Y.Z"` |

Internal crates: raps-kernel, raps-oss, raps-derivative, raps-dm, raps-da, raps-acc, raps-webhooks, raps-reality, raps-admin.

### npm packages (6 files)

| File | Field |
|------|-------|
| `npm/package.json` | `"version"` + 5 `optionalDependencies` values |
| `npm/platforms/win32-x64/package.json` | `"version"` |
| `npm/platforms/darwin-x64/package.json` | `"version"` |
| `npm/platforms/darwin-arm64/package.json` | `"version"` |
| `npm/platforms/linux-x64/package.json` | `"version"` |
| `npm/platforms/linux-arm64/package.json` | `"version"` |

### NOT updated (separate version)

- `python-bindings/Cargo.toml` — independent version (currently 4.3.0)
- `python-bindings/pyproject.toml` — same

## Procedure

```bash
OLD="4.16.0"
NEW="4.17.0"

# 1. Workspace version + internal deps (Cargo.toml)
sed -i "s/version = \"${OLD}\"/version = \"${NEW}\"/g" Cargo.toml

# 2. npm main package version
sed -i "s/\"version\": \"${OLD}\"/\"version\": \"${NEW}\"/" npm/package.json

# 3. npm optionalDependencies (5 entries in npm/package.json)
sed -i "s/\"${OLD}\"/\"${NEW}\"/g" npm/package.json

# 4. Platform packages
for p in win32-x64 darwin-x64 darwin-arm64 linux-x64 linux-arm64; do
  sed -i "s/\"version\": \"${OLD}\"/\"version\": \"${NEW}\"/" npm/platforms/${p}/package.json
done

# 5. Verify
grep -r "$NEW" Cargo.toml npm/package.json npm/platforms/*/package.json

# 6. Commit
git add Cargo.toml npm/
git commit -m "chore: bump version to ${NEW}"

# 7. Tag and push (triggers release workflow)
git tag "v${NEW}"
git push origin main --tags
```

## Release Workflow Stages

After tag push, `.github/workflows/release.yml` runs:

1. **plan** — cargo-dist determines build matrix
2. **build** — platform binaries (5 targets)
3. **host** — GitHub Release with artifacts
4. **sbom** — supply chain transparency
5. **provenance** — SLSA attestation
6. **test-install-scripts** — verify install.sh on multiple OS
7. **publish-npm** — extracts binaries, publishes 6 npm packages
8. **publish-pypi** — publishes Python wheel

## Common Mistakes

- Forgetting optionalDependencies in `npm/package.json` (5 version strings, not just the top-level version)
- Using a sed pattern that replaces ALL dependency names with one platform name (the bug that broke v4.15.0)
- Bumping python-bindings when not intended (separate release cycle)
- Pushing tag before committing version bump
