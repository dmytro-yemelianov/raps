# Archived Git Repository Information

This directory contains metadata about separate git repositories that were consolidated into the monorepo.

## Consolidated Repositories

The following repositories were consolidated into the monorepo workspace on 2026-01-01:

- **raps** - Main CLI application
- **raps-kernel** - Microkernel foundation
- **raps-oss** - Object Storage Service
- **raps-derivative** - Model Derivative Service
- **raps-dm** - Data Management Service
- **raps-community** - Community tier features

## Repository Information

Each repository's metadata (remote URL, branch, last commit) is stored in JSON files:
- `raps-info.json`
- `raps-kernel-info.json`
- `raps-oss-info.json`
- `raps-derivative-info.json`
- `raps-dm-info.json`
- `raps-community-info.json`

## Notes

- Git history is preserved on GitHub remotes
- These repositories are now part of the monorepo workspace
- The `.git` directories were removed to consolidate into single repository
- Future changes should be made in the monorepo, not in separate repos

## Consolidation Date

2026-01-01
