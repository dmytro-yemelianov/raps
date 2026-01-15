# Research: Global Logging Flags

## Decisions

### 1. Secret Redaction Strategy
**Decision**: Apply regex-based redaction to ALL verbose/debug output before printing.
**Rationale**:
- Simple and effective for known patterns (client_secret, bearer token).
- Centralized in `logging::log_verbose` and `logging::log_debug`.

### 2. Quiet Mode
**Decision**: `--quiet` disables all `info` logs (e.g. "Creating bucket...").
**Rationale**:
- Users parsing JSON output don't want "Fetching..." messages on stdout (or even stderr if they want clean logs).
- We already use `output_format.supports_colors()` as a proxy for this in some commands, but `--quiet` should be explicit.
- `logging::quiet()` check should be used.
