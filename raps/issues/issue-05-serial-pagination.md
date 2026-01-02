## Summary

When listing resources, the implementation fetches all pages one by one sequentially. For very large projects or buckets, the user must wait for potentially dozens of sequential requests.

## Current Behavior

Listing OSS objects concatenates results in a loop until no next page remains. This ensures the user gets the full list, but it can be slow and memory-intensive for large datasets. The CLI does not offer options to limit page size or count.

## Proposed Solution

1. **Add pagination controls**:
   - `--limit` flag to cap the number of results
   - `--page-size` flag to control per-request size

2. **Implement streaming output**:
   - Output results as pages arrive rather than accumulating everything
   - Allow early termination with Ctrl+C without losing already-fetched data

3. **Consider parallel page fetching** (where API supports):
   - For APIs that provide total count, fetch pages in parallel

## Expected Benefits

- Faster feedback for large datasets
- Reduced memory usage
- Better user experience for interactive use
- More scalable for automation scenarios

## References

- File: `src/api/oss.rs` - Object listing
- Related: `docs/limitations.md` notes this as a known limitation
