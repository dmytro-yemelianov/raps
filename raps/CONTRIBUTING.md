# Contributing to APS CLI

Thank you for your interest in contributing to APS CLI! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

## Branch Protection Policy

**Important**: The `main` branch is protected. All changes must be made through feature branches and Pull Requests (PRs). Direct pushes to `main` are not allowed.

### Workflow Requirements

- ✅ All changes must go through a Pull Request
- ✅ All CI checks must pass before a PR can be merged
- ✅ At least one approval may be required (depending on repository settings)
- ✅ PRs must be up to date with the main branch before merging

## Getting Started

1. **Fork the repository** on GitHub (if you don't have write access)
2. **Clone the repository** locally:
   ```bash
   git clone https://github.com/dmytro-yemelianov/raps.git
   cd raps
   ```
3. **Add the upstream remote** (if you forked):
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

**⚠️ Never commit directly to `main` branch. Always create a feature branch.**

1. **Ensure you're on the main branch and up to date**:
   ```bash
   git checkout main
   git pull origin main
   ```

2. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   # or
   git checkout -b docs/update-readme
   ```

   Branch naming conventions:
   - `feature/` - New features
   - `fix/` - Bug fixes
   - `docs/` - Documentation updates
   - `refactor/` - Code refactoring
   - `test/` - Test additions/updates
   - `chore/` - Maintenance tasks

3. **Make your changes** following the coding standards:
   - Use meaningful variable and function names
   - Add comments for complex logic
   - Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
   - Keep functions focused and small

4. **Write or update tests** as needed

5. **Run quality checks locally** before committing:
   ```bash
   # Format code
   cargo fmt
   
   # Check formatting
   cargo fmt --all -- --check
   
   # Run clippy
   cargo clippy --all-features -- -D warnings
   
   # Run tests
   cargo test --all-features
   ```

6. **Commit your changes**:
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

7. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

8. **Create a Pull Request** on GitHub:
   - Go to the repository on GitHub
   - Click "New Pull Request"
   - Select your branch
   - Fill out the PR template
   - Wait for CI checks to pass
   - Request review if needed

## Pull Request Guidelines

- **Keep PRs focused**: One feature or fix per PR
- **Write clear descriptions**: Explain what changes you made and why
- **Reference issues**: Link to related issues using `Fixes #123` or `Closes #123`
- **Update documentation**: If you add features, update the README.md
- **Add tests**: Include tests for new functionality
- **Ensure CI passes**: All GitHub Actions checks must pass before merging
- **Keep PRs small**: Smaller PRs are easier to review and merge faster
- **Update your branch**: Rebase or merge main into your branch if it's behind
- **Respond to feedback**: Address review comments promptly

### PR Checklist

Before requesting review, ensure:
- [ ] Code follows the project's style guidelines
- [ ] All tests pass locally
- [ ] Documentation is updated if needed
- [ ] Commit messages are clear and descriptive
- [ ] Branch is up to date with main

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

