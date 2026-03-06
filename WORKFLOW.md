---
# Symphony-style workflow policy for guided test coverage improvement.
# An agent session picks the next target from the priority list, writes tests,
# verifies coverage improved, and commits. One target per session.

coverage_threshold: 40        # minimum line % to consider a file "covered"
success_exit_states:
  - covered                   # file reached threshold
  - skipped                   # file needs live API, clearly documented why

priority:
  # Smallest 0%-covered command files first — all testable with raps-mock.
  # Each entry: source file → existing help-test file (for pattern reference).
  - target: commands/admin/folder.rs         # 110 lines — admin folder rights
    ref:    tests/admin_commands.rs
  - target: commands/config/config_ops.rs    # 148 lines — config get/set ops
    ref:    tests/config_commands.rs
  - target: commands/da/appbundles.rs        # 179 lines — DA appbundle CRUD
    ref:    tests/da_commands.rs
  - target: commands/job.rs                  # 194 lines — job status/list/cancel
    ref:    tests/job_commands.rs
  - target: commands/config/context.rs       # 252 lines — context env var ops
    ref:    tests/config_commands.rs
  - target: commands/da/activities.rs        # 261 lines — DA activity CRUD
    ref:    tests/da_commands.rs
  - target: commands/da/workitems.rs         # 264 lines — DA work items
    ref:    tests/da_commands.rs
  - target: commands/object/copy.rs          # 305 lines — object copy
    ref:    tests/object_commands.rs
  - target: commands/admin/operations.rs     # 451 lines — admin bulk operations
    ref:    tests/admin_commands.rs
  - target: commands/rfi/crud.rs             # 634 lines — RFI create/update/get
    ref:    tests/rfi_commands.rs

  # Deferred — require complex setup:
  # commands/reality.rs        needs live photogrammetry API
  # commands/object/upload.rs  needs raps-mock OSS upload route
  # commands/demo.rs           demo-only, no production value to test
  # commands/acc/*             need live ACC project
  # mcp/tools_*.rs             MCP layer, separate effort
---

# RAPS Test Coverage Workflow

You are a test engineer improving coverage for the RAPS CLI codebase at
`/root/github/raps/raps`. Your job is to pick the **first uncovered target**
from the priority list above, write tests that bring its line coverage above
`{coverage_threshold}%`, verify with `cargo llvm-cov`, and commit.

## Step 0 — Pick your target

Run coverage to find the first priority file still below threshold:

```bash
cargo llvm-cov --package raps-cli --summary-only 2>/dev/null \
  | grep -E "commands/admin/folder|commands/config/config_ops|commands/da/appbundles|commands/job|commands/config/context|commands/da/activities|commands/da/workitems|commands/object/copy|commands/admin/operations|commands/rfi/crud"
```

Pick the **first file in priority order** that is below {coverage_threshold}%.
That is your target for this session. Work on exactly one target.

## Step 1 — Understand the target

1. Read the target source file fully.
2. Read its reference help-test file (see `ref:` in WORKFLOW.md).
3. Read a scenario test for pattern:
   `raps-cli/tests/scenarios/hub_scenarios.rs`
4. Check what raps-mock serves for the relevant API:
   ```bash
   grep -r "<keyword>" /root/github/raps/raps/raps-mock/src/ --include="*.rs" | grep -v target
   ```
5. Run the command's help to understand flags:
   ```bash
   cargo run -p raps-cli -- <command> --help 2>&1
   ```

## Step 2 — Write tests (TDD)

Decide which test tier fits:

| Situation | Tier | Location |
|---|---|---|
| Pure helper functions in the source file | **unit** | inline `#[cfg(test)]` in the source file |
| Mock server can handle the API calls | **scenario** | `tests/scenarios/<cmd>_scenarios.rs` |
| Command only does arg parsing + routing | **smoke** | `tests/<cmd>_commands.rs` (already exists — add cases) |

### Unit tests (inline)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fn_name_describes_behavior() { ... }
}
```

### Scenario tests (mock server)
```rust
// tests/scenarios/<cmd>_scenarios.rs
use crate::test_utils::start_cli_test;
use predicates::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_<cmd>_<subcommand>_<behavior>() {
    let (_server, mut cmd) = start_cli_test().await;
    // 3-legged commands need: cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");
    cmd.args(["<cmd>", "<subcommand>", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("expected_field"));
}
```

Register in `tests/scenarios/mod.rs`:
```rust
pub mod <cmd>_scenarios;
```

### Rules
- No `#![allow(deprecated)]` needed in scenario files (they call `start_cli_test`, not `cargo_bin` directly)
- Add `#![allow(deprecated)]` to any new `tests/*.rs` binary test file
- Use `predicate::ne(101)` (no `_i32` suffix) for no-panic checks
- Use single `contains("stable text")` predicates — no `or()` fallbacks
- Do NOT change implementation code — only add tests

## Step 3 — Measure

```bash
cargo llvm-cov --package raps-cli --summary-only 2>/dev/null \
  | grep "<target_file_basename>"
```

If line coverage is below {coverage_threshold}%:
- Read what lines are NOT covered (use `--html` if needed)
- Add tests that exercise those code paths
- Repeat until threshold is reached

If the file genuinely cannot reach {coverage_threshold}% without live credentials:
- Comment each untestable function with `// requires live <API_NAME> credentials`
- Document the gap in `docs/TEST_COVERAGE_MATRIX.md` under the file's entry
- Exit with state `skipped`

## Step 4 — Commit

```bash
git add <changed files>
git commit -m "test(<cmd>): add coverage for <target_file> — <N>% line coverage"
```

Commit message must include the final coverage percentage.

## Step 5 — Report

Output exactly:

```
TARGET:   commands/<path>/<file>.rs
BEFORE:   <N>% line coverage
AFTER:    <N>% line coverage
STATUS:   covered | skipped
TESTS:    <list of test function names added>
COMMIT:   <hash>
REASON:   (if skipped: why it cannot reach threshold without live credentials)
```
