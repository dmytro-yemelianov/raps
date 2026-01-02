## Summary

The Config struct is designed to hold base URLs for all APS endpoints, but not all modules use it consistently. Some clients hard-code the Autodesk production URL in format strings, which could cause issues if users want to point the CLI at a different APS environment.

## Current Behavior

- The Issues client correctly uses `self.config.issues_url()` to build its endpoints
- The ACC extended clients (Assets, Submittals, Checklists) and the RFI client hard-code the Autodesk production URL in format strings
- The presence of `config` in client structs (often marked with `#[allow(dead_code)]`) suggests an oversight where the config isn't actually used

## Proposed Solution

1. Ensure all API clients derive base URLs from the Config struct
2. Remove `#[allow(dead_code)]` attributes and actually use the config
3. Add configuration options for all APS endpoints
4. Consider adding environment variable overrides for advanced use cases

## Expected Benefits

- Consistent behavior across all modules
- Easier testing with mock endpoints
- Support for future APS regions or environments
- Reduced maintenance burden

## References

- File: `src/api/issues.rs` - Correct implementation
- File: `src/api/acc.rs` - Hard-coded URLs
- File: `src/api/rfi.rs` - Hard-coded URLs (has `#[allow(dead_code)]` on config)
- File: `src/config.rs` - Config struct with URL methods
