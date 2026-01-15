# Research: Standardize Exit Codes

**Feature**: Standardize Exit Codes

## Decisions

### 1. Exit Code Mapping
**Decision**:
- `0`: Success
- `2`: Usage error (clap default) - invalid args
- `3`: Auth error (401, 403, missing token)
- `4`: Not Found (404)
- `5`: Remote Error (5xx, Timeout, Connection Refused)
- `6`: Internal/Generic Error (everything else)

**Rationale**:
- `1` is often used as a catch-all by shells, so avoiding it for specific errors helps clarity, or we can use it for generic errors. But `6` is explicit "Internal Error".
- `clap` uses `2` by default for usage errors. We should align with that.
- `sysexits.h` suggests `EX_USAGE` (64), `EX_NOPERM` (77), etc., but standard small integers are more common in modern CLIs (like `curl`, `aws`).

### 2. Implementation Strategy
**Decision**: Implement `ExitCode` enum in `raps-kernel::error`.
**Rationale**:
- Allows sharing the definition.
- `ExitCode::from_error(e: &anyhow::Error) -> ExitCode` will inspect the error chain.
- We need to downcast errors to find `reqwest::Error` status codes or custom `AuthError` types.

### 3. Error Types
**Decision**: We rely on checking `reqwest::StatusCode` for 404/401/403/5xx.
**Rationale**: Most errors come from HTTP.
