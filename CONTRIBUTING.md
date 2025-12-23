# Contributing to APS CLI

Thank you for your interest in contributing to APS CLI! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/dmytro-yemelianov/raps.git
   cd raps
   ```
3. **Add the upstream remote** (if contributing back):
   ```bash
   git remote add upstream https://github.com/dmytro-yemelianov/raps.git
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later ([rustup.rs](https://rustup.rs/))
- APS account with application credentials from [APS Developer Portal](https://aps.autodesk.com/myapps)

### Building

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run with all features
cargo test --all-features
```

### Code Quality

Before submitting a PR, ensure your code passes:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --all-features -- -D warnings

# Build documentation
cargo doc --no-deps --all-features
```

## Making Changes

1. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

2. **Make your changes** following the coding standards:
   - Use meaningful variable and function names
   - Add comments for complex logic
   - Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
   - Keep functions focused and small

3. **Write or update tests** as needed

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "Description of your changes"
   ```
   
   Use clear, descriptive commit messages. Follow the format:
   - `feat: Add new feature`
   - `fix: Fix bug description`
   - `docs: Update documentation`
   - `refactor: Refactor code`
   - `test: Add tests`
   - `chore: Maintenance tasks`

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request** on GitHub

## Pull Request Guidelines

- **Keep PRs focused**: One feature or fix per PR
- **Write clear descriptions**: Explain what changes you made and why
- **Reference issues**: Link to related issues using `Fixes #123` or `Closes #123`
- **Update documentation**: If you add features, update the README.md
- **Add tests**: Include tests for new functionality
- **Ensure CI passes**: All GitHub Actions checks must pass

## Testing

- Write unit tests for new functions and modules
- Test error cases as well as success cases
- Run the full test suite before submitting:
  ```bash
  cargo test --all-features
  ```

## API Changes

If you're adding new API endpoints or modifying existing ones:

1. Verify against the [APS OpenAPI Specifications](https://github.com/autodesk-platform-services/aps-sdk-openapi)
2. Update the API coverage section in README.md
3. Add examples to the README.md usage section

## Documentation

- Update README.md for user-facing changes
- Add code comments for complex logic
- Update command help text if adding/modifying commands
- Keep API documentation in sync with code

## Release Process

Releases are managed by maintainers. To trigger a release:

1. Update version in `Cargo.toml`
2. Create a git tag: `git tag v0.x.x`
3. Push the tag: `git push origin v0.x.x`
4. GitHub Actions will automatically build and publish releases

## Questions?

- Open an issue for questions or discussions
- Check existing issues and PRs before creating new ones
- Be patient and respectful in all interactions

Thank you for contributing! 🎉

