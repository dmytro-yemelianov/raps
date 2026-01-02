## Summary

There are places where potentially blocking operations occur on the async runtime thread, which violates async best practices and could become bottlenecks as the codebase grows.

## Current Behavior

1. **OAuth 3-legged login**: The tiny HTTP server uses `server.recv()` in a loop to wait for the browser callback - this is a blocking call from the `tiny_http` library that blocks the Tokio thread.

2. **Interactive prompts**: Uses blocking calls from `dialoguer` (e.g., `Select::interact()` for bucket selection) within async functions.

## Proposed Solution

Offload blocking operations to separate threads using `tokio::task::spawn_blocking`:

```rust
// Instead of blocking the async runtime:
let result = tokio::task::spawn_blocking(|| {
    Select::new().items(&options).interact()
}).await??;
```

## Expected Benefits

- Proper async runtime hygiene
- Better support for concurrent tasks
- More robust behavior as codebase grows
- Preparation for potential future scenarios with concurrent operations

## References

- File: `src/api/auth.rs` - OAuth callback server
- File: `src/commands/object.rs` - Interactive prompts
- Related: `dialoguer` and `tiny_http` crate usage
