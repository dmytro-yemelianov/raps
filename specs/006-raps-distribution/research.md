# Research: Multi-Channel RAPS Distribution

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Research Topics

### 1. Install Script Best Practices

**Context**: Need to create robust install scripts for Linux, macOS, and Windows.

**Decision**: Follow patterns from established Rust CLI tools (rustup, starship, deno).

**Rationale**:
- rustup's install script is the gold standard for Rust tooling
- Pattern: detect OS/arch → download binary → extract → install to ~/.local/bin or ~/.cargo/bin → update PATH
- Colorized output with progress indicators improves UX
- POSIX-compatible where possible, bash for advanced features

**Alternatives Considered**:
- Custom installer binary: Rejected - adds complexity, requires building separate tool
- Makefile-based install: Rejected - not user-friendly for one-liner installs
- Platform-specific packages only: Rejected - requires users to have package manager knowledge

**Key Patterns from Research**:

| Tool | Install Location | PATH Modification | Version Selection |
|------|------------------|-------------------|-------------------|
| rustup | ~/.cargo/bin | Modifies shell rc files | RUSTUP_VERSION env var |
| starship | ~/.local/bin | Prints instructions | Via URL parameter |
| deno | ~/.deno/bin | Modifies shell rc files | DENO_VERSION env var |
| raps (proposed) | ~/.raps/bin | Modifies shell rc files | RAPS_VERSION env var |

### 2. Maturin Binary Distribution

**Context**: Need to package Rust CLI binary for PyPI distribution.

**Decision**: Use maturin with `bindings = "bin"` mode for pure binary distribution.

**Rationale**:
- maturin is the standard tool for building Python packages from Rust
- `bindings = "bin"` mode bundles pre-compiled binaries without Python bindings
- Supports all target platforms via cross-compilation
- Integrates with GitHub Actions via maturin-action

**Configuration Pattern**:

```toml
[project]
name = "raps"
requires-python = ">=3.8"

[tool.maturin]
bindings = "bin"
strip = true
```

**Alternatives Considered**:
- PyO3 full bindings: Rejected for Phase 1 - adds complexity, deferred to Phase 2
- setuptools with bundled binary: Rejected - less reliable cross-platform support
- Standalone wheel building: Rejected - maturin handles wheel creation properly

### 3. GitHub Actions Release Workflow

**Context**: Need automated publishing to PyPI on release.

**Decision**: Use maturin-action with trusted publishing (OIDC) for PyPI.

**Rationale**:
- maturin-action handles cross-compilation and wheel building
- Trusted publishing (OIDC) eliminates need for long-lived API tokens
- Matrix builds ensure all platforms are covered
- Can run in parallel with existing release workflow

**Workflow Structure**:

```yaml
jobs:
  build-wheels:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        target: [x86_64, aarch64]
    steps:
      - uses: PyO3/maturin-action@v1
        with:
          target: ${{ matrix.target }}
          manylinux: auto

  publish:
    needs: build-wheels
    environment: pypi
    permissions:
      id-token: write  # OIDC trusted publishing
    steps:
      - uses: pypa/gh-action-pypi-publish@release/v1
```

**Alternatives Considered**:
- Manual PyPI upload: Rejected - error-prone, doesn't scale
- API token authentication: Rejected - less secure than OIDC
- Separate workflow per platform: Rejected - harder to coordinate releases

### 4. Shell Configuration Detection

**Context**: Install script needs to modify PATH in user's shell config.

**Decision**: Detect shell from $SHELL and modify appropriate config file.

**Rationale**:
- Users expect PATH to work immediately after install
- Different shells use different config files
- Should not require user to manually edit configs

**Detection Matrix**:

| Shell | Config File | PATH Addition Method |
|-------|-------------|---------------------|
| bash | ~/.bashrc | export PATH="$HOME/.raps/bin:$PATH" |
| zsh | ~/.zshrc | export PATH="$HOME/.raps/bin:$PATH" |
| fish | ~/.config/fish/config.fish | fish_add_path ~/.raps/bin |

**Alternatives Considered**:
- Only print instructions: Rejected - poor UX, users expect it to "just work"
- Modify /etc/profile: Rejected - requires root, system-wide impact
- Only support bash: Rejected - many users use zsh (macOS default) or fish

### 5. PowerShell PATH Modification

**Context**: Windows install script needs to update User PATH.

**Decision**: Use [Environment]::SetEnvironmentVariable for persistent PATH update.

**Rationale**:
- User PATH is the correct level (not system-wide)
- Changes persist across sessions
- No admin rights required

**Pattern**:

```powershell
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
$newPath = "$env:USERPROFILE\.raps\bin;$currentPath"
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")
```

**Alternatives Considered**:
- Modify registry directly: Rejected - [Environment] API is cleaner
- System PATH: Rejected - requires admin, affects all users
- Temporary PATH only: Rejected - doesn't persist

### 6. Checksum Verification

**Context**: Install scripts should verify downloaded binaries.

**Decision**: Download and verify SHA256 checksums from GitHub releases.

**Rationale**:
- GitHub releases already include checksums
- SHA256 is standard and widely supported
- Prevents tampering and ensures download integrity

**Pattern**:

```bash
# Download checksum file
curl -fsSL "$RELEASE_URL/checksums.txt" -o checksums.txt

# Verify binary
sha256sum -c checksums.txt --ignore-missing
```

**Alternatives Considered**:
- GPG signatures: Rejected - adds complexity, requires key management
- No verification: Rejected - security risk
- MD5: Rejected - cryptographically weak

## Summary of Decisions

| Topic | Decision | Impact |
|-------|----------|--------|
| Install location | ~/.raps/bin | Consistent, user-writeable, no conflicts |
| PATH modification | Auto-detect shell, modify config | Better UX, works immediately |
| Python packaging | maturin bindings="bin" | Simple binary bundling |
| PyPI publishing | OIDC trusted publishing | Secure, no token management |
| Checksum verification | SHA256 from GitHub releases | Secure, standard approach |
| Version selection | RAPS_VERSION env var | Consistent with rustup pattern |

## Open Questions (Resolved)

All research questions resolved. No blockers for Phase 1 design.
