## Summary

The CLI supports resumable uploads, but its implementation writes the upload state to disk on every chunk completion. For very large files (hundreds of chunks), this results in hundreds of file write operations that can significantly slow down the upload process due to I/O overhead.

## Current Behavior

In the multipart upload loop, after each part is uploaded, the code calls `state.save()` which serializes the state to a JSON file via `std::fs::write`. These synchronous writes occur for every single chunk.

## Proposed Solution

Optimize state persistence to reduce I/O overhead:
- Batch state updates (e.g., save every N chunks instead of every chunk)
- Use periodic flushes for safety (time-based rather than chunk-based)
- Mark state as dirty and write once at the end for normal completions
- Keep immediate saves only for error recovery scenarios

## Expected Benefits

- Reduced I/O overhead during uploads
- Faster overall upload times for large files
- Lower disk wear on SSDs

## References

- File: `src/api/oss.rs` - `upload_multipart` function
- Related: Resumable upload feature (`--resume` flag)
