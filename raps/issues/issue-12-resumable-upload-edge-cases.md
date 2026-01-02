## Summary

The resumable upload feature has some edge cases that aren't fully handled, potentially causing issues in failure scenarios.

## Current Behavior

1. **File modification check**: The code checks `can_resume()` by comparing file size and mod time, which is good.

2. **Unhandled scenarios**:
   - If an upload fails mid-chunk, the code saves state and exits. The user must manually rerun with `--resume`.
   - If the state file gets corrupted, there's no recovery mechanism.
   - If the upload session at the server expires, the logic doesn't cover renewing the upload URLs.
   - No command exists to list or manage ongoing multipart uploads.

## Proposed Solution

1. **Add upload management commands**:
   - `raps upload list` - List pending/incomplete uploads
   - `raps upload abort` - Abort a resumable upload and clean up
   - `raps upload cleanup` - Remove stale state files

2. **Improve resilience**:
   - Detect expired upload sessions and auto-renew URLs
   - Add state file validation with recovery options
   - Implement auto-resume on transient failures

3. **Better error messages**:
   - Clear guidance when resume fails
   - Suggest cleanup when state is corrupted

## Expected Benefits

- More robust resumable upload experience
- Better handling of edge cases
- Self-service for users when things go wrong

## References

- File: `src/api/oss.rs` - Upload state management
- Code comments mention "maybe safer to fetch fresh URLs" as a TODO
