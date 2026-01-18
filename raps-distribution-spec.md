# RAPS Distribution Implementation Specification

## Overview

This document specifies the implementation of multiple distribution channels for RAPS, a Rust CLI tool for Autodesk Platform Services. The target LLM should implement each section independently, following the constraints and requirements outlined.

---

## Project Context

| Property | Value |
|----------|-------|
| **Repository** | `https://github.com/dmytro-yemelianov/raps/` |
| **Language** | Rust |
| **Current Distribution** | GitHub Releases, `cargo install` |
| **Target Platforms** | Windows (x64, arm64), macOS (x64, arm64), Linux (x64, arm64) |

---

## 1. Python Wheels (CLI Distribution)

### Objective

Package the RAPS binary for distribution via PyPI so users can install with `pip install raps` and run `raps` commands directly.

### Approach

Use **maturin** for building platform-specific wheels that bundle the pre-compiled Rust binary.

### Requirements

- Create a minimal Python package structure that wraps the binary
- Entry point should invoke the bundled binary, passing through all CLI arguments
- Support all target platforms with separate wheel builds
- Package metadata should include: description, author, license, repository URL, keywords (autodesk, aps, forge, cad, bim)
- Version should sync with Cargo.toml version

### File Structure to Create

```
python/
├── pyproject.toml
├── src/
│   └── raps/
│       ├── __init__.py
│       └── __main__.py
└── README.md (symlink or copy from root)
```

### Build Configuration

- Use maturin with `bindings = "bin"` mode
- Configure for all target triples
- Set Python version compatibility: py3, 3.8+
- Classifier tags: Development Status, Environment, Intended Audience, License, Operating System, Programming Language

### GitHub Actions Integration

Create workflow that:

- Triggers on release publish
- Builds wheels for all platforms using maturin-action
- Uploads to PyPI using trusted publishing (OIDC)
- Tests installation on each platform after upload

### User Experience

```bash
pip install raps
raps --version
raps bucket list
```

---

## 2. npm Distribution

### Objective

Package RAPS for distribution via npm so users can install with `npm install -g raps` or run with `npx raps`.

### Approach

Use the **platform-specific optional dependencies** pattern (like esbuild, turbo, swc).

### Requirements

- Main package should be minimal, only containing logic to select and execute the correct platform binary
- Platform packages should be scoped: `@raps/cli-{platform}-{arch}`
- Support: win32-x64, win32-arm64, darwin-x64, darwin-arm64, linux-x64, linux-arm64
- Main package detects platform at install time and pulls correct binary
- Fallback error message if platform unsupported

### Package Structure

```
npm/
├── package.json              # Main @raps/cli package
├── src/
│   └── index.js              # Platform detection and binary execution
├── scripts/
│   └── postinstall.js        # Verify binary availability
└── platforms/
    ├── win32-x64/
    │   └── package.json      # @raps/cli-win32-x64
    ├── darwin-arm64/
    │   └── package.json      # @raps/cli-darwin-arm64
    └── ... (other platforms)
```

### Main Package Requirements

- Use `optionalDependencies` for platform packages
- Binary should be exposed via `bin` field in package.json
- Include engines field specifying Node.js version compatibility
- postinstall script should verify binary works

### GitHub Actions Integration

Create workflow that:

- Triggers on release publish
- Downloads release binaries for all platforms
- Packages each platform binary into its npm package
- Publishes all packages to npm in correct order (platform packages first, then main)
- Uses npm automation token stored as secret

### User Experience

```bash
npm install -g @raps/cli
raps --version

# Or without install
npx @raps/cli bucket list
```

---

## 3. Python Bindings (PyO3)

### Objective

Create native Python bindings using PyO3, exposing RAPS functionality as a Python library for programmatic use in scripts, notebooks, and automation.

### Approach

Use **PyO3** with **maturin** to create a native Python extension module.

### Requirements

- Expose core RAPS functionality as Python classes and functions
- Async operations should be exposed with both sync and async interfaces
- Error handling should convert Rust errors to appropriate Python exceptions
- Type hints should be provided via stub files (.pyi)
- Documentation strings for all public API

### API Design

#### Module Structure

```
Module: raps

Classes:
├── Client          # Main entry point, handles authentication
├── Bucket          # Represents an OSS bucket
├── Object          # Represents an object in a bucket
├── TranslationJob  # Represents a Model Derivative job
├── Hub             # Represents a Data Management hub
└── Project         # Represents a project in a hub

Exceptions:
├── RapsError           # Base exception
├── AuthenticationError # Auth failures
├── NotFoundError       # Resource not found
├── RateLimitError      # API rate limit exceeded
└── ValidationError     # Invalid parameters
```

#### Client API Surface

**Authentication:**

| Method | Description |
|--------|-------------|
| `Client(client_id, client_secret)` | 2-legged auth |
| `Client.from_env()` | Load credentials from environment |
| `Client.login()` | Interactive 3-legged OAuth |

**OSS Operations:**

| Method | Return Type |
|--------|-------------|
| `client.buckets.list(region?, limit?)` | `List[Bucket]` |
| `client.buckets.create(key, policy?, region?)` | `Bucket` |
| `client.buckets.get(key)` | `Bucket` |
| `client.buckets.delete(key)` | `None` |
| `bucket.objects.list(limit?)` | `List[Object]` |
| `bucket.objects.upload(path, object_key?)` | `Object` |
| `bucket.objects.download(object_key, path)` | `Path` |
| `bucket.objects.delete(object_key)` | `None` |
| `object.signed_url(minutes?)` | `str` |
| `object.urn` | `str` (base64 encoded) |

**Model Derivative:**

| Method/Property | Return Type |
|-----------------|-------------|
| `client.translate(urn, format?, options?)` | `TranslationJob` |
| `job.status` | `str` |
| `job.progress` | `int` |
| `job.wait(timeout?, poll_interval?)` | `TranslationJob` |
| `job.manifest` | `dict` |

**Data Management (3-legged):**

| Method | Return Type |
|--------|-------------|
| `client.hubs.list()` | `List[Hub]` |
| `hub.projects.list()` | `List[Project]` |

### File Structure

```
python-bindings/
├── pyproject.toml
├── Cargo.toml
├── src/
│   └── lib.rs              # PyO3 module definition
├── raps/
│   ├── __init__.py         # Re-exports
│   └── __init__.pyi        # Type stubs
└── tests/
    ├── test_auth.py
    ├── test_buckets.py
    └── test_translate.py
```

### Implementation Guidelines

- Reuse existing RAPS Rust library code, don't duplicate logic
- Use `pyo3-asyncio` for async support if needed
- Implement `__repr__` and `__str__` for all classes
- Support context managers where appropriate (e.g., Client)
- Use Python naming conventions (snake_case) not Rust conventions

### GitHub Actions Integration

Create workflow that:

- Builds wheels for all platforms using maturin-action
- Runs pytest test suite
- Publishes to PyPI as `raps` package (same as CLI, but with library extras)
- Generates and publishes documentation

### User Experience

```python
from raps import Client

client = Client.from_env()

# List buckets
for bucket in client.buckets.list():
    print(f"{bucket.key}: {bucket.policy}")

# Upload and translate
bucket = client.buckets.get("my-bucket")
obj = bucket.objects.upload("model.rvt")
job = client.translate(obj.urn, format="svf2")
job.wait()
print(f"Translation complete: {job.status}")
```

---

## 4. Install Scripts

### Objective

Create one-liner install scripts for quick installation without package managers.

---

### 4a. Bash Script (Linux/macOS)

#### Requirements

- Single file, POSIX-compatible where possible, bash for advanced features
- Detect OS (linux, darwin) and architecture (x64, arm64)
- Download correct binary from GitHub releases
- Install to `~/.raps/bin` by default, configurable via environment variable
- Add to PATH by modifying appropriate shell config (.bashrc, .zshrc, .config/fish/config.fish)
- Verify installation by running `raps --version`
- Support specific version install via environment variable
- Colorized output for better UX
- Graceful error handling with helpful messages
- Uninstall option via flag

#### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RAPS_VERSION` | Specific version to install | latest |
| `RAPS_INSTALL_DIR` | Custom install directory | `~/.raps/bin` |
| `RAPS_NO_MODIFY_PATH` | Skip PATH modification if set | unset |

#### Output Requirements

- ASCII art banner
- Progress indicators for download/extract/install steps
- Success message with next steps
- Instructions for PATH if manual setup needed

#### URL

```
https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh
```

#### Invocation

```bash
curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash
```

---

### 4b. PowerShell Script (Windows)

#### Requirements

- PowerShell 5.1+ compatible (default on Windows 10/11)
- Detect architecture (x64, arm64)
- Download correct binary from GitHub releases
- Install to `~\.raps\bin` by default, configurable via parameter
- Add to User PATH environment variable
- Verify installation
- Support specific version via parameter
- Colorized output
- Proper error handling with try/catch
- Support `-NoPathUpdate` switch
- Uninstall option via `-Uninstall` switch

#### Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `-Version` | Specific version to install | latest |
| `-InstallDir` | Custom install directory | `~\.raps\bin` |
| `-NoPathUpdate` | Skip PATH modification | false |
| `-Uninstall` | Remove RAPS installation | false |

#### URL

```
https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1
```

#### Invocation

```powershell
irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex
```

---

### 4c. Uninstall Scripts

Create separate uninstall scripts for both platforms:

| Script | Behavior |
|--------|----------|
| `uninstall.sh` | Remove installation directory, notify about manual PATH cleanup |
| `uninstall.ps1` | Remove installation directory and PATH entry |

---

### Testing Requirements

GitHub Actions workflow should:

- Test install script on ubuntu-latest, macos-latest, windows-latest
- Test with default options
- Test with custom install directory
- Test version pinning
- Test uninstall
- Verify binary works after install

---

## 5. GitHub Actions Workflows

### Release Workflow

Create a unified release workflow that triggers on version tag push and:

1. Builds Rust binaries for all platforms
2. Creates GitHub release with binaries and checksums
3. Builds and publishes Python wheels (CLI)
4. Builds and publishes Python bindings
5. Builds and publishes npm packages
6. Tests install scripts on all platforms

### Workflow Structure

```
.github/workflows/
├── release.yml           # Main release workflow
├── test-install.yml      # Test install scripts (called by release)
└── ci.yml                # Regular CI (existing)
```

### Secrets Required

| Secret | Purpose |
|--------|---------|
| `PYPI_API_TOKEN` | PyPI publishing (or use OIDC trusted publishing) |
| `NPM_TOKEN` | npm publishing |
| `CARGO_REGISTRY_TOKEN` | crates.io publishing (if applicable) |

---

## 6. Documentation Updates

### README.md Installation Section

Update README.md with installation section covering all methods:

- Quick install (scripts)
- Package managers (pip, npm, cargo, brew)
- Manual download
- Verification steps
- Uninstallation

### Dedicated Installation Guide

Create `docs/installation.md` with detailed instructions for each method including troubleshooting.

---

## Implementation Order

| Priority | Component | Complexity | Value |
|----------|-----------|------------|-------|
| 1 | Install scripts | Low | Immediate |
| 2 | Python wheels (CLI) | Low-Medium | High |
| 3 | npm distribution | Medium | High |
| 4 | Python bindings | High | Very High (automation users) |

---

## Constraints

- All implementations must work without modifying core RAPS Rust code
- Version numbers must stay in sync across all distribution channels
- All packages must include LICENSE file
- All packages must link to GitHub repository
- No external services beyond GitHub, PyPI, npm

---

## Appendix: Platform Matrix

### Target Triples

| Platform | Architecture | Rust Target |
|----------|--------------|-------------|
| Windows | x64 | `x86_64-pc-windows-msvc` |
| Windows | arm64 | `aarch64-pc-windows-msvc` |
| macOS | x64 | `x86_64-apple-darwin` |
| macOS | arm64 | `aarch64-apple-darwin` |
| Linux | x64 | `x86_64-unknown-linux-gnu` |
| Linux | arm64 | `aarch64-unknown-linux-gnu` |

### Package Naming Convention

| Channel | Package Name |
|---------|--------------|
| PyPI (CLI) | `raps` |
| PyPI (bindings) | `raps` (with extras) |
| npm (main) | `@raps/cli` |
| npm (platform) | `@raps/cli-{os}-{arch}` |
| Cargo | `raps` |
