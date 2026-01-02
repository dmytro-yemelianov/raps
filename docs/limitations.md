---
layout: default
title: Known Limitations
---

# Known Limitations

This page documents current limitations and unsupported features in RAPS CLI. We aim for transparency about what's available and what's planned for future releases.

## ACC (Autodesk Construction Cloud) Modules

All ACC modules are fully supported with CRUD operations as of v1.0.0.

### Issues
**Status:** ✅ Fully Supported

Full CRUD operations including comments, attachments, and state transitions.

### RFIs (Requests for Information)
**Status:** ✅ Fully Supported

- `raps rfi list` - List RFIs in a project
- `raps rfi get` - Get RFI details
- `raps rfi create` - Create new RFI
- `raps rfi update` - Update RFI status and details

### Assets
**Status:** ✅ Fully Supported

- `raps acc asset list` - List assets in a project
- `raps acc asset get` - Get asset details
- `raps acc asset create` - Create new asset
- `raps acc asset update` - Update asset

### Submittals
**Status:** ✅ Fully Supported

- `raps acc submittal list` - List submittals in a project
- `raps acc submittal get` - Get submittal details
- `raps acc submittal create` - Create new submittal
- `raps acc submittal update` - Update submittal

### Checklists
**Status:** ✅ Fully Supported

- `raps acc checklist list` - List checklists in a project
- `raps acc checklist get` - Get checklist details
- `raps acc checklist create` - Create new checklist
- `raps acc checklist update` - Update checklist
- `raps acc checklist templates` - List checklist templates

## Pipeline Execution

### Sequential Execution Only

Pipeline steps are executed sequentially. Parallel step execution is not currently supported.

```yaml
# This pipeline runs steps one after another
steps:
  - name: upload
    command: object upload mybucket file1.dwg
  - name: translate
    command: translate start ${upload.urn} --format svf2
```

**Workaround:** For parallel operations, use the `--batch` and `--parallel` flags on individual commands, or run multiple RAPS instances.

### No Looping Constructs

Pipelines do not support loop constructs (for/while). Each step must be explicitly defined.

**Workaround:** Use shell scripts or external automation tools for complex iteration patterns.

## Pagination

### Default Page Sizes

RAPS uses APS API default pagination settings. When listing resources:

- Most APIs return 20-100 items per page by default
- RAPS automatically handles pagination for list commands
- No custom page-size control is currently exposed

### Large Result Sets

For projects with many items (hundreds or thousands), list commands may take longer as they fetch all pages. Consider:

- Using filters where available (e.g., `--status`, `--assignee`)
- Using `--output json` for programmatic processing
- Implementing your own pagination with direct API calls if needed

## Plugin System

### Status: ✅ Fully Supported
...
See [Plugin Documentation](plugins.md) for details.

## Test Data Generation

### Status: ✅ Fully Supported

- `raps generate files` - Generate synthetic engineering files for testing.

Supported formats: OBJ, DXF, STL, IFC, JSON, XYZ.

## Output Format Stability

### JSON/YAML Schema

While RAPS provides consistent JSON and YAML output:

- Output schemas are not formally versioned
- Minor fields may be added in patch releases
- Breaking schema changes will only occur in major versions

For maximum stability in CI/CD pipelines, we recommend:

- Using `jq` or similar tools with specific field selectors
- Testing against new RAPS versions before production deployment
- Following the [changelog](../CHANGELOG.md) for output changes

## Performance Considerations

### Large File Uploads

For files larger than 5MB:

- Multipart chunked upload is automatically used
- Upload can be resumed with `--resume` if interrupted
- Progress is shown in interactive mode

### Concurrent Operations

Default concurrency limits:

- Batch operations: 5 concurrent requests (configurable with `--concurrency`)
- Rate limiting: Automatic retry with exponential backoff for 429 responses

See [Configuration](configuration.md) for tuning these values.

## Platform-Specific Notes

### Windows

- Shell completions work best in PowerShell 7+
- Some terminal emulators may not render colors correctly; use `--no-color` if needed

### macOS/Linux

- OS keychain integration available via `RAPS_USE_KEYCHAIN=true`
- Shell completions available for bash, zsh, fish, and elvish

## Reporting Issues

If you encounter limitations not documented here, or have feature requests:

1. Check [GitHub Issues](https://github.com/dmytro-yemelianov/raps/issues) for existing reports
2. Open a new issue with:
   - RAPS version (`raps --version`)
   - Operating system
   - Expected vs actual behavior
   - Minimal reproduction steps

## Roadmap

Planned improvements for future releases:

- [ ] Parallel pipeline step execution
- [ ] Pipeline loop constructs
- [ ] Custom pagination controls
- [ ] Formal JSON schema definitions for outputs
- [ ] Delete operations for ACC modules

See [ROADMAP.md](../roadmap/ROADMAP.md) for the full development roadmap.

