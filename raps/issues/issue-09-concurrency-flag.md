## Summary

There is a global `--concurrency` flag (defaulting to 5) in the CLI options, but it's not uniformly applied across all multi-operation commands. This inconsistent usage means opportunities for concurrency aren't fully exploited in some areas.

## Current Behavior

- The demo command uses the concurrency flag to limit concurrent tasks
- UploadBatch has its own `--parallel` option for file uploads
- Pipeline command ignores the global flag (runs steps sequentially)
- Other bulk operations may not respect the flag

## Proposed Solution

1. **Audit all bulk operations** and apply concurrency consistently
2. **Unify concurrency flags**:
   - Use the global `--concurrency` flag everywhere
   - Deprecate command-specific flags like `--parallel`
3. **Document concurrency behavior** for each command
4. **Consider future pipeline parallelization** with concurrency controls

## Expected Benefits

- Consistent user experience across commands
- Full utilization of available concurrency
- Clearer mental model for users

## References

- File: `src/main.rs` - Global concurrency flag definition
- File: `src/commands/object.rs` - UploadBatch with separate `--parallel` option
- Related: Pipeline sequential execution limitation
