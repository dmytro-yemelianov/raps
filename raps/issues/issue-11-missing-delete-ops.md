## Summary

Several modules are missing delete operations, creating asymmetric CRUD functionality. Users cannot fully manage resources through the CLI alone.

## Current State

The following resources lack delete operations:
- Issues - no `raps issue delete`
- Issue comments/attachments - partial support
- Some ACC modules may have incomplete CRUD

## Proposed Solution

Audit all resource types and implement missing delete operations:

1. **Issues module**:
   - Add `raps issue delete` command
   - Add `raps issue comment delete` (if missing)
   - Add `raps issue attachment delete`

2. **Maintain consistency**:
   - Follow existing patterns for confirmation prompts
   - Support `--force` flag to skip confirmation
   - Support batch delete operations

## Expected Benefits

- Complete CRUD operations for all resources
- Self-sufficient CLI for resource management
- Better automation support

## References

- File: `src/commands/issues.rs`
- Related: ACC modules full CRUD (added in 1.0.0)
