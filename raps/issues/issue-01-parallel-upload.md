## Summary

Large file uploads are handled sequentially, uploading one chunk at a time. This means multipart uploads (for files >5MB) do not utilize parallelism, potentially underutilizing network bandwidth and CPU on multi-core systems.

## Current Behavior

In `upload_multipart` (src/api/oss.rs), each part is processed with a `for` loop and an individual HTTP PUT request. The code uploads chunks one at a time in sequence.

## Proposed Solution

Implement parallel chunk uploads since S3 APIs support out-of-order part uploads:
- Use `tokio::spawn` or `futures::stream::FuturesUnordered` to upload multiple chunks concurrently
- Respect the global `--concurrency` flag to limit parallel uploads
- Implement proper error handling for partial failures

## Expected Benefits

- Significantly faster uploads for large files
- Better utilization of available network bandwidth
- Improved performance on multi-core systems

## References

- File: `src/api/oss.rs`
- Related: `--concurrency` CLI flag (currently underutilized)
