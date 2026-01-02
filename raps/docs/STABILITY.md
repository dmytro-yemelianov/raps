---
layout: default
title: Stability & Compatibility
---

# Stability & Backward Compatibility

This document outlines RAPS CLI's commitment to stability and backward compatibility starting with version 1.0.0.

## Semantic Versioning

RAPS CLI follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** (1.x.x → 2.x.x): Breaking changes
- **MINOR** (1.0.x → 1.1.x): New features, backward compatible
- **PATCH** (1.0.0 → 1.0.1): Bug fixes, backward compatible

## What We Guarantee (1.0.0+)

### Command Interface Stability

| Aspect | Guarantee |
|--------|-----------|
| Command names | Stable; won't be renamed or removed |
| Required arguments | Stable; won't change position or meaning |
| Flag names | Stable; existing flags won't be removed |
| Exit codes | Stable; documented codes won't change meaning |

### Output Format Stability

**JSON/YAML Output (`--output json`, `--output yaml`)**

- Existing fields will not be removed in minor/patch releases
- New fields may be added in any release
- Field types will not change
- Field order is not guaranteed

**Table Output (`--output table`)**

- Human-readable format; may change in minor releases
- Not intended for parsing; use JSON/YAML for automation

### Configuration Stability

- Environment variable names are stable
- Profile configuration format is stable
- Plugin configuration format is stable

## What May Change

### In Minor Releases

- New commands may be added
- New optional flags may be added
- New output fields may be added
- Default values for optional parameters (documented in changelog)
- Table output formatting

### In Patch Releases

- Bug fixes
- Documentation improvements
- Performance optimizations
- Security patches

## Deprecation Policy

When features need to be removed or changed:

1. **Deprecation Warning**: Feature marked deprecated in a minor release
2. **Warning Period**: At least one minor release cycle with warnings
3. **Removal**: Feature removed in the next major release

### Example Deprecation

```bash
$ raps old-command --deprecated-flag value
WARNING: --deprecated-flag is deprecated and will be removed in v2.0.0.
         Use --new-flag instead.
```

## Breaking Changes

Breaking changes are reserved for major version increments and will be:

1. **Documented**: Listed in CHANGELOG.md under "Breaking Changes"
2. **Communicated**: Announced in release notes
3. **Migration Guide**: Provided when significant changes occur

### Examples of Breaking Changes

- Removing a command or subcommand
- Changing the meaning of an exit code
- Removing a configuration option
- Changing required arguments

### Examples of Non-Breaking Changes

- Adding new commands
- Adding new optional flags
- Adding new fields to JSON output
- Improving error messages

## CI/CD Recommendations

### Pin Major Versions

```yaml
# GitHub Actions example
- name: Install RAPS
  run: cargo install raps-cli@^1.0.0  # Accepts 1.x.x
```

### Use Specific Outputs

```bash
# Robust: Select specific fields
raps bucket list --output json | jq '.[] | .bucketKey'

# Fragile: Assume output structure
raps bucket list --output json | jq '.[0]'
```

### Test Before Upgrading

```yaml
# Test against new versions before production
- name: Test RAPS Upgrade
  run: |
    cargo install raps-cli@latest
    raps --version
    ./test-raps-scripts.sh
```

## Version Support

| Version | Status | Support Until |
|---------|--------|---------------|
| 1.0.x | Current | Active development |
| 0.7.x | Legacy | Security fixes only |
| < 0.7 | EOL | No support |

## Changelog

All changes are documented in [CHANGELOG.md](../CHANGELOG.md) following the [Keep a Changelog](https://keepachangelog.com/) format.

### Changelog Sections

- **Added**: New features
- **Changed**: Changes to existing features
- **Deprecated**: Features marked for removal
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security-related changes

## Reporting Compatibility Issues

If you encounter unexpected breaking changes or compatibility issues:

1. Check [CHANGELOG.md](../CHANGELOG.md) for documented changes
2. Search [GitHub Issues](https://github.com/dmytro-yemelianov/raps/issues)
3. Open a new issue with:
   - Previous working version
   - Current version
   - Command that changed behavior
   - Expected vs actual output

## Questions

For questions about compatibility:

- Open a [GitHub Discussion](https://github.com/dmytro-yemelianov/raps/discussions)
- Check existing issues and documentation
- Review the [ROADMAP](../roadmap/ROADMAP.md) for planned changes

