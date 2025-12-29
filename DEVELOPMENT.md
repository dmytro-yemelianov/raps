# Development Guide

This guide covers setting up your development environment for fast, productive Rust development on the RAPS project.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/dmytro-yemelianov/raps.git
cd raps

# Build the project
cargo build

# Run tests
cargo test
```

## Fast Development Setup (Recommended)

For significantly faster build times, install these optional but highly recommended tools:

### Build Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| `cargo check -p raps-kernel` | <5s | Incremental build |
| `cargo check` (full workspace) | <30s | Incremental build |
| sccache hit rate | >80% | After warm cache |

### Windows Setup

```powershell
# 1. Install LLVM for lld-link (fast linker)
winget install LLVM.LLVM

# 2. Install sccache (compilation caching)
cargo install sccache
setx RUSTC_WRAPPER sccache
# Restart your terminal after this

# 3. Install cargo-nextest (parallel test runner)
cargo install cargo-nextest

# 4. Verify setup
sccache --show-stats
cargo check -p raps-kernel  # Should complete in <5s after first build
```

### Linux Setup

```bash
# 1. Install mold linker (fast linker)
sudo apt install mold  # Ubuntu/Debian
# or: brew install mold  # Homebrew

# 2. Install sccache (compilation caching)
cargo install sccache
echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
source ~/.bashrc

# 3. Install cargo-nextest (parallel test runner)
cargo install cargo-nextest

# 4. Verify setup
sccache --show-stats
cargo check -p raps-kernel  # Should complete in <5s after first build
```

### macOS Setup

```bash
# 1. sccache (compilation caching) - linker is default on macOS
cargo install sccache
echo 'export RUSTC_WRAPPER=sccache' >> ~/.zshrc
source ~/.zshrc

# 2. Install cargo-nextest (parallel test runner)
cargo install cargo-nextest

# 3. Optionally install mold for even faster linking
brew install mold

# 4. Verify setup
sccache --show-stats
cargo check -p raps-kernel  # Should complete in <5s after first build
```

## Development Commands

### Fast Iteration (Avoid Linking)

```bash
# Check syntax and types only (fastest)
cargo check

# Check specific crate
cargo check -p raps-kernel

# Check all targets (including tests, benches)
cargo check --all-targets

# Lint with clippy (no linking)
cargo clippy -p raps --no-deps
```

### Running Tests

```bash
# Run all tests with nextest (parallel, better output)
cargo nextest run

# Run tests for a specific crate
cargo nextest run -p raps-kernel

# Run a single test
cargo nextest run test_name

# Traditional cargo test (if nextest not installed)
cargo test
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build with timing report
cargo build --timings
# Opens target/cargo-timings/cargo-timing-*.html

# Build only core tier (minimal)
cargo build --no-default-features --features core

# Build with all features
cargo build --all-features
```

### Auto-Rebuild on Save

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-check on save
cargo watch -x check

# Auto-test on save
cargo watch -x "nextest run"

# Auto-clippy on save
cargo watch -x "clippy -p raps"
```

## Product Tiers

RAPS uses feature flags to enable tiered product builds:

| Tier | Features | Build Command |
|------|----------|---------------|
| **Core** | Auth, OSS, Derivative, DM | `cargo build --no-default-features --features core` |
| **Community** (default) | Core + ACC, DA, Reality, Webhooks, Pipelines, Plugins | `cargo build` |
| **Pro** | Community + Analytics, Audit, Compliance, SSO | `cargo build --features pro` |

Check your version tier:
```bash
raps --version
# Output: raps 3.2.0 Community
```

## Project Structure

```
raps/                    # Main CLI application
├── src/
│   ├── main.rs          # Entry point (tier detection)
│   ├── commands/        # CLI command handlers
│   └── api/             # API adapters (wrapping service crates)

raps-kernel/             # Microkernel (core infrastructure) ~2100 LOC
├── src/
│   ├── auth/            # OAuth2 authentication
│   ├── http/            # HTTP client with retry
│   ├── config/          # Configuration management
│   └── types/           # Domain primitives (URN, ObjectKey, etc.)

raps-oss/                # Object Storage Service
raps-derivative/         # Model Derivative Service  
raps-dm/                 # Data Management Service
raps-community/          # Community tier (ACC, DA, Reality, Webhooks)
raps-pro/                # Pro tier (Analytics, Audit, Compliance, SSO)
```

## Cargo Workspace

This is a Cargo workspace with multiple crates. Key commands:

```bash
# Build entire workspace
cargo build --workspace

# Test entire workspace
cargo nextest run --workspace

# Check specific crate
cargo check -p raps-kernel

# Clean build artifacts
cargo clean
```

## CI/CD

The CI pipeline (`.github/workflows/ci.yml`) uses:

- **sccache**: Compilation caching with GitHub Actions cache backend
- **cargo-nextest**: Parallel test execution
- **mold/lld-link**: Fast linkers for Linux/Windows
- **hyperfine**: Build time benchmarking

Build timing reports are uploaded as artifacts for each build.

## Troubleshooting

### Slow Builds?

1. **Check sccache is active**:
   ```bash
   sccache --show-stats
   # Should show cache hits after first build
   ```

2. **Check linker is configured**:
   ```bash
   # Windows: lld-link should be in PATH
   where lld-link
   
   # Linux: mold should be installed
   which mold
   ```

3. **Run build timing**:
   ```bash
   cargo build --timings
   # Check HTML report for bottlenecks
   ```

4. **Clear and rebuild**:
   ```bash
   cargo clean
   sccache --zero-stats
   cargo build
   ```

### sccache Not Working?

```bash
# Check if RUSTC_WRAPPER is set
echo $RUSTC_WRAPPER  # Should be "sccache"

# Windows: Check environment variable
echo $env:RUSTC_WRAPPER  # PowerShell

# Verify sccache is in PATH
sccache --version
```

### Tests Failing?

```bash
# Run with verbose output
cargo nextest run --no-capture

# Run specific failing test
cargo nextest run test_name -- --nocapture
```

## Performance Tips

1. **Use `cargo check` for iteration** - Don't link until you need to run
2. **Target specific crates** - Use `-p crate_name` to reduce scope
3. **Parallel jobs** - Use `-j N` flag (default is CPU cores)
4. **sccache** - Reduces recompilation of unchanged code
5. **Fast linkers** - lld-link (Windows) and mold (Linux) are 2-5x faster

## Contributing

See [CONTRIBUTING.md](raps/CONTRIBUTING.md) for contribution guidelines.
