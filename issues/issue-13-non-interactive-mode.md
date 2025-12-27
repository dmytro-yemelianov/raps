## Summary

The CLI uses interactive prompts when inputs are not provided, but these cannot work in non-interactive environments. While a `--non-interactive` flag exists, there may be inconsistent checking throughout the codebase.

## Current Behavior

- The `--non-interactive` global flag is intended to make the CLI fail instead of hang on prompts
- The flag is honored in `interactive::init`
- However, if any prompt was missed (e.g., a confirm that doesn't check the flag), it could cause an automation script to stall

## Proposed Solution

1. **Audit all interactive prompts**:
   - Find all uses of `dialoguer` crate
   - Ensure each checks the non-interactive mode flag
   - Add integration tests for non-interactive mode

2. **Centralize prompt handling**:
   - Create wrapper functions that automatically check the flag
   - Return clear errors instead of hanging

3. **Document required parameters**:
   - Clearly document which parameters are required in non-interactive mode
   - Provide helpful error messages listing missing parameters

## Expected Benefits

- Reliable behavior in CI/CD environments
- No unexpected hangs in automation
- Clear feedback when parameters are missing

## References

- File: `src/main.rs` - `--non-interactive` flag
- Related: Interactive module and `dialoguer` usage
