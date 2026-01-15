# Research: Non-interactive Mode

## Decisions

### 1. Fail-Fast Strategy
**Decision**: `prompts::input` and `prompts::select` MUST check `interactive::is_interactive()`. If false, return `ExitCode::InvalidArguments` (or similar error) immediately.
**Rationale**: Prevents hanging or panicking when TTY is missing or `--non-interactive` is set.

### 2. Argument Parsing
**Decision**: Use `clap`'s `Option<T>` for args.
- If `Some(val)`, use it.
- If `None`:
  - call `prompts::input(...)`.
  - `prompts::input` will fail if non-interactive.
**Rationale**: Keeps logic simple and consistent.
