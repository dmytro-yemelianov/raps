## Summary

In the multipart upload implementation, the code allocates a fresh buffer `Vec<u8>` for each chunk, then frees it after the chunk is sent. This repeated allocation can be costly, especially if there are thousands of chunks.

## Current Behavior

For every chunk uploaded, a new buffer is allocated and then deallocated. This puts pressure on the memory allocator and could lead to memory fragmentation.

## Proposed Solution

Implement buffer pooling to reuse allocations:
- Maintain a single buffer (or small pool of buffers) for all chunks
- Recycle buffers to avoid repeated alloc/free cycles
- Consider using a pre-allocated buffer sized to the chunk size

## Expected Benefits

- Reduced allocation overhead
- Lower memory fragmentation
- Better cache locality
- Improved performance for large file uploads

## References

- File: `src/api/oss.rs`
- Related: Multipart upload implementation
