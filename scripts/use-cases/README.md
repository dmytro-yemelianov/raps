# RAPS Use-Case Scripts

Ready-to-use shell scripts for common CAD/PDM/PLM workflows, organized by role.

## Prerequisites

- [raps](https://rapscli.xyz) CLI installed and on PATH
- [jq](https://jqlang.github.io/jq/) for JSON processing
- Valid APS credentials configured (`raps config set client_id <id>`)

## Personas

| Directory | Role | Description |
|-----------|------|-------------|
| `cad-engineer/` | CAD Engineer | Upload models, translate to SVF2/OBJ/STL, batch processing |
| `bim-manager/` | BIM Manager | Project setup, issue tracking, RFI management, access audits |
| `pdm-admin/` | PDM Administrator | User onboarding/offboarding, access audits, project lifecycle |
| `plm-engineer/` | PLM Engineer | Submittals, assets, checklists, portfolio health reports |
| `devops/` | DevOps Engineer | CI auth checks, webhooks, pipelines, multi-profile management |
| `da-engineer/` | Design Automation Engineer | Revit/AutoCAD batch processing, workitem monitoring |
| `surveyor/` | Surveyor | Photogrammetry pipelines, point cloud processing |

## Quick Start

```bash
# Make scripts executable
chmod +x scripts/use-cases/**/*.sh

# Check authentication
./scripts/use-cases/devops/ci-auth-check.sh

# Generate test models
./scripts/use-cases/cad-engineer/generate-test-models.sh --count 3

# Upload and translate a model
./scripts/use-cases/cad-engineer/upload-and-translate.sh model.rvt --wait

# Dry-run user onboarding
./scripts/use-cases/pdm-admin/user-onboarding.sh --csv users.csv --account ACC123 --dry-run
```

## Conventions

- Every script starts with `#!/usr/bin/env bash` and `set -euo pipefail`
- Every script sources `common.sh` for shared helpers
- Every script supports `--help` for usage info
- Destructive scripts support `--dry-run`
- Exit codes: `0` = success, `1` = error, `2` = usage error
- No hardcoded IDs — everything via args or env vars

## Scripts Index

### CAD Engineer
- **upload-and-translate.sh** — Upload a model file, translate to SVF2, and wait for completion
- **batch-upload-models.sh** — Bulk upload a directory of CAD files with optional translation
- **download-derivatives.sh** — Download OBJ/STL/STEP derivatives from translated models
- **generate-test-models.sh** — Generate synthetic IFC/OBJ/DXF files for testing

### BIM Manager
- **project-setup.sh** — Create a project, set up folders, and assign users
- **issue-tracker.sh** — List, create, and transition issues from CSV
- **rfi-management.sh** — Create, list, and update RFIs with priority tracking
- **folder-permissions-audit.sh** — Audit folder access permissions

### PDM Admin
- **user-onboarding.sh** — Bulk add users from CSV with role assignment
- **user-offboarding.sh** — Remove a user from all projects with audit trail
- **weekly-access-audit.sh** — Report on user roles and flag stale admins
- **project-lifecycle.sh** — Create, activate, list, and archive projects

### PLM Engineer
- **submittal-workflow.sh** — Create submittals from CSV and track status
- **asset-inventory.sh** — List, create, update, and export assets
- **checklist-management.sh** — Create checklists from templates and assign them
- **portfolio-health-report.sh** — Cross-project summary of issues, RFIs, and submittals

### DevOps
- **ci-auth-check.sh** — Token validation for CI pipelines with expiry warnings
- **webhook-setup.sh** — Create, test, list, and clean up webhooks
- **pipeline-runner.sh** — Validate and run YAML pipelines
- **multi-profile-switch.sh** — Set up and switch between client profiles

### DA Engineer
- **revit-export.sh** — AppBundle + Activity for Revit batch export
- **autocad-conversion.sh** — DWG to PDF/DXF conversion pipeline
- **monitor-workitems.sh** — Poll DA workitem status and download results

### Surveyor
- **photogrammetry-pipeline.sh** — Create scene, upload photos, process, and download
- **pointcloud-to-acc.sh** — Upload point cloud results to ACC via OSS
