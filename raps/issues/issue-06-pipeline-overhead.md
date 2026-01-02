## Summary

The pipeline feature executes each step by spawning a new process of the raps binary. While this provides isolation and simplicity, it incurs overhead for process creation and command parsing on every step.

## Current Behavior

1. Each pipeline step spawns a new `raps` process
2. Steps run sequentially with no parallel execution support
3. For quick steps, launch overhead can become noticeable

## Proposed Solution

1. **In-process step execution** (optional):
   - Add `--in-process` flag to execute steps without spawning new processes
   - Maintain current behavior as default for compatibility

2. **Parallel step execution**:
   - Add support for parallel execution of independent steps
   - Implement step dependency graph
   - Allow `parallel: true` or `depends_on: [step1, step2]` in pipeline definitions

## Expected Benefits

- Reduced overhead for multi-step pipelines
- Faster execution for independent steps
- More powerful automation capabilities

## References

- File: `src/commands/pipeline.rs`
- Related: `docs/limitations.md` explicitly states pipeline steps run sequentially
