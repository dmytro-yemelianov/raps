# Code Review Issues

This folder contains issue templates created from the **Comprehensive Code Review of the RAPS Repository** (December 2025).

## Summary

15 GitHub issues were created based on the code review findings:

### Performance Issues (7 issues)

| Issue | Title | Priority |
|-------|-------|----------|
| [#70](https://github.com/dmytro-yemelianov/raps/issues/70) | Implement parallel multipart upload for large files | High |
| [#71](https://github.com/dmytro-yemelianov/raps/issues/71) | Reduce disk writes during resumable uploads | Medium |
| [#72](https://github.com/dmytro-yemelianov/raps/issues/72) | Implement buffer reuse for chunk uploads | Medium |
| [#73](https://github.com/dmytro-yemelianov/raps/issues/73) | Fix blocking calls in async contexts | Medium |
| [#74](https://github.com/dmytro-yemelianov/raps/issues/74) | Improve pagination with streaming and limits | Medium |
| [#75](https://github.com/dmytro-yemelianov/raps/issues/75) | Reduce pipeline step overhead and add parallel execution | Medium |
| [#84](https://github.com/dmytro-yemelianov/raps/issues/84) | Implement streaming for small file uploads | Low |

### Architecture Issues (4 issues)

| Issue | Title | Priority |
|-------|-------|----------|
| [#76](https://github.com/dmytro-yemelianov/raps/issues/76) | Use config-based URLs consistently across all API clients | High |
| [#77](https://github.com/dmytro-yemelianov/raps/issues/77) | Apply retry logic consistently across all API operations | High |
| [#78](https://github.com/dmytro-yemelianov/raps/issues/78) | Unify and apply concurrency flag consistently | Medium |
| [#79](https://github.com/dmytro-yemelianov/raps/issues/79) | Formalize output schemas for CLI output | Low |

### Feature Issues (4 issues)

| Issue | Title | Priority |
|-------|-------|----------|
| [#80](https://github.com/dmytro-yemelianov/raps/issues/80) | Add missing delete operations for Issues and related resources | Medium |
| [#81](https://github.com/dmytro-yemelianov/raps/issues/81) | Handle resumable upload edge cases and add management commands | Medium |
| [#82](https://github.com/dmytro-yemelianov/raps/issues/82) | Audit and ensure non-interactive mode works everywhere | High |
| [#83](https://github.com/dmytro-yemelianov/raps/issues/83) | Improve plugin and alias system clarity and usability | Low |

## Labels Created

- `performance` - Performance improvements
- `architecture` - Architectural improvements
- `code-review` - Issues from code review

## Source

Issues derived from: `Comprehensive Code Review of the __RAPS__ Repository.pdf`

## File References

Each issue file contains:
- Summary of the problem
- Current behavior
- Proposed solution
- Expected benefits
- Code references

## Created

December 27, 2025
