## Summary

There is built-in support for HTTP retry logic (in `http::execute_with_retry`) intended to handle rate limits and transient server errors. However, this is applied inconsistently - some operations use it while others fail immediately on errors.

## Current Behavior

- OSS bucket creation uses the retry mechanism
- Listing objects or downloading files use direct `request` calls with no retry loop
- Users experience inconsistent behavior: some operations are resilient to flakes while others aren't

## Proposed Solution

1. **Apply consistent retry strategy**:
   - Use `execute_with_retry` for all idempotent GET requests
   - Apply retry logic to all operations that can safely be retried

2. **Add user controls**:
   - Global `--retry` flag to enable/disable retries
   - `--max-retries` flag to control retry count
   - Environment variable for default retry behavior

3. **Document retry behavior**:
   - Clearly document which operations support retries
   - Explain the backoff strategy in user documentation

## Expected Benefits

- Consistent resilience across all operations
- Better experience in environments with intermittent connectivity
- More reliable automation/CI/CD usage

## References

- File: `src/http.rs` - `execute_with_retry` implementation
- File: `src/api/oss.rs` - Mixed usage of retry logic
