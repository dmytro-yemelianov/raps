# Test Coverage Summary

Generated: 2026-03-05

## Covered commands

| Command | Arg parsing | Operation test | Scenario test | Snapshot |
|---------|-------------|---------------|---------------|----------|
| admin user add-to-all-projects | ✓ | ✓ | ✓ | ✓ |
| admin user add | ✓ | ✓ | — | — |
| admin user remove | ✓ | ✓ | ✓ | ✓ |
| admin project archive | ✓ | ✓ | ✓ | ✓ |
| admin project list | ✓ | — | — | — |
| admin project create | ✓ | — | — | — |
| admin user update | ✓ | — | — | — |
| admin folder rights | ✓ | — | — | — |
| admin operation | ✓ | — | — | — |

## Coverage gaps (known)

- admin project create — no scenario test
- admin user update — no scenario test
- admin folder rights — no scenario test
- rate-limit (429) retry behavior — unit tested in raps-admin, not HTTP-level tested
- invalid role ID (role not found on server) — not tested
- project with suspended status — not tested

## How to run

```bash
# All tests (excluding live API)
cargo test --workspace

# Only scenario/operation tests
cargo test -p raps-cli --test operations
cargo test -p raps-cli --test scenarios

# CLI structure snapshots
cargo test -p raps-cli --test cli_tests

# Update snapshots after intentional changes
cargo insta review
```

## CI requirement

All tests in this list run with no network access. `raps-mock` is the only external dependency and runs in-process.
