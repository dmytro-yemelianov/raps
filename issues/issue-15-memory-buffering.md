## Summary

For small files (≤5MB), `upload_single_part` reads the entire file into memory before uploading. While this is usually fine, it could cause issues with many concurrent uploads or if the threshold is increased.

## Current Behavior

- Files ≤5MB are read entirely into memory using `std::fs::read`
- A 5MB file is buffered completely before upload begins
- Multiple concurrent uploads could spike memory usage

## Proposed Solution

1. **Implement streaming for all uploads**:
   - Use async file reader to stream body to request
   - Avoid loading entire file into memory
   - Use `tokio::fs::File` with `Body::wrap_stream`

2. **Add memory-efficient options**:
   - Consider `--low-memory` flag for constrained environments
   - Make the threshold configurable

## Expected Benefits

- Lower memory footprint
- Better support for concurrent operations
- More predictable memory usage

## Trade-offs

- Slightly more complex implementation
- Possible minor performance impact for small files

## References

- File: `src/api/oss.rs` - `upload_single_part` function
- Related: Multipart upload threshold (5MB)
