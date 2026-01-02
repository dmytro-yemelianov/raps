## Problem
Users often need to inspect contents of large package-like artifacts (zip/bundles) but are forced to download the entire file.

## Goal
Enable fast inspection using HTTP Range requests wherever supported.

## Proposal
- Implement `Range` support for signed URLs and object URLs where possible.
- Add commands (naming bikeshedding):
  - `raps object inspect-zip <bucket/key> --list`
  - `raps translate inspect <urn> --list-files`
- Technique:
  - Fetch end-of-file to read zip central directory.
  - Fetch only required ranges for small internal files.

## Acceptance criteria
- Can list zip contents with <1% of bytes downloaded for typical archives.
- Graceful fallback when server does not support ranges.
- Clear messaging when inspect is not possible.

## Out of scope
- Full random-access extraction for all archive formats.
