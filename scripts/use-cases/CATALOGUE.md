# RAPS Use-Case Scripts Catalogue

> 25 ready-to-use shell scripts for CAD, BIM, PDM, PLM, DevOps, Design Automation, and surveying workflows — organized by role.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Shared Utilities](#shared-utilities)
- [CAD Engineer](#cad-engineer) — Model uploads, translations, derivatives, test file generation
- [BIM Manager](#bim-manager) — Project setup, issues, RFIs, folder access audits
- [PDM Admin](#pdm-admin) — User onboarding/offboarding, access audits, project lifecycle
- [PLM Engineer](#plm-engineer) — Submittals, assets, checklists, portfolio health
- [DevOps](#devops) — CI auth, webhooks, pipelines, multi-profile management
- [DA Engineer](#da-engineer) — Revit/AutoCAD batch processing, workitem monitoring
- [Surveyor](#surveyor) — Photogrammetry pipelines, point cloud ingestion
- [Conventions](#conventions)

---

## Prerequisites

| Dependency | Purpose | Install |
|------------|---------|---------|
| [raps](https://rapscli.xyz) | Autodesk Platform Services CLI | `cargo install raps` |
| [jq](https://jqlang.github.io/jq/) | JSON processing | `apt install jq` / `brew install jq` |
| Valid APS credentials | API access | `raps config set client_id <id>` |

```bash
# Make all scripts executable
chmod +x scripts/use-cases/**/*.sh
```

---

## Shared Utilities

### [`common.sh`](common.sh)

Sourced by every script. Provides:

| Function | Purpose |
|----------|---------|
| `check_auth()` | Verify 2-legged (client credentials) auth, exit with guidance if not |
| `check_3leg()` | Verify 3-legged (user login) auth via `raps auth status` |
| `require_cmd()` | Check that a command (`raps`, `jq`) exists on PATH |
| `info()` / `warn()` / `error()` / `step()` | Colored output helpers |
| `confirm()` | Interactive y/n prompt for destructive operations |
| `extract_json()` | Wrapper around `jq` with null/error handling |
| `check_help()` / `show_usage()` | `--help` flag detection and display |

---

## CAD Engineer

Scripts for model file management: uploading to OSS, translating via Model Derivative, downloading derivatives, and generating synthetic test files.

### [`upload-and-translate.sh`](cad-engineer/upload-and-translate.sh)

Upload a model file to OSS, translate it to SVF2/OBJ/STL, and optionally wait for completion.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<file>` | Model file to upload (required) | — |
| `--format <fmt>` | Output format: `svf2`, `obj`, `stl` | `svf2` |
| `--bucket <name>` | Target OSS bucket | `raps-models-<timestamp>` |
| `--wait` | Wait for translation to complete | off |

```bash
./cad-engineer/upload-and-translate.sh building.rvt --wait
./cad-engineer/upload-and-translate.sh model.ifc --format obj --bucket my-bucket --wait
./cad-engineer/upload-and-translate.sh assembly.stp --format stl
```

---

### [`batch-upload-models.sh`](cad-engineer/batch-upload-models.sh)

Bulk upload all CAD files from a directory to an OSS bucket with parallel uploads and optional translation.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<directory>` | Directory containing model files (required) | — |
| `--bucket <name>` | Target OSS bucket | `raps-batch-<timestamp>` |
| `--parallel <n>` | Concurrent upload count | `4` |
| `--translate` | Translate each uploaded file to SVF2 | off |
| `--extensions <list>` | Comma-separated file extensions to include | `rvt,ifc,dwg,stp,obj,stl,dxf,3dm,fbx` |

```bash
./cad-engineer/batch-upload-models.sh ./models --translate
./cad-engineer/batch-upload-models.sh ./cad-files --bucket project-models --parallel 8
./cad-engineer/batch-upload-models.sh ./exports --extensions rvt,ifc --translate
```

---

### [`download-derivatives.sh`](cad-engineer/download-derivatives.sh)

Download OBJ, STL, STEP, or other derivatives from a translated model, or list what's available.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<urn>` | Base64-encoded model URN (required) | — |
| `--format <fmt>` | Desired format: `obj`, `stl`, `step`, `svf2` | `obj` |
| `--out-dir <dir>` | Output directory | `./exports` |
| `--list` | List available derivatives without downloading | off |

```bash
./cad-engineer/download-derivatives.sh dXJuOmFk... --format obj --out-dir ./models
./cad-engineer/download-derivatives.sh dXJuOmFk... --list
./cad-engineer/download-derivatives.sh dXJuOmFk... --format stl
```

---

### [`generate-test-models.sh`](cad-engineer/generate-test-models.sh)

Generate synthetic engineering files (IFC, OBJ, DXF, STL, STEP) for testing, and optionally upload them.

**Auth:** 2-legged (only if `--upload`)

| Option | Description | Default |
|--------|-------------|---------|
| `--count <n>` | Number of files to generate | `5` |
| `--complexity <level>` | `simple`, `medium`, `complex` | `medium` |
| `--formats <list>` | Comma-separated output formats | `obj,dxf,stl,ifc,step` |
| `--out-dir <dir>` | Output directory | `./test-models` |
| `--upload` | Upload generated files to OSS bucket | off |
| `--bucket <name>` | Bucket for upload | `raps-test-<timestamp>` |

```bash
./cad-engineer/generate-test-models.sh --count 3 --complexity simple
./cad-engineer/generate-test-models.sh --count 10 --upload --bucket my-test-bucket
./cad-engineer/generate-test-models.sh --formats obj,stl --count 2 --out-dir ./samples
```

---

## BIM Manager

Scripts for BIM project management: setting up projects with folder structures, tracking issues from CSV, managing RFIs, and auditing folder permissions.

### [`project-setup.sh`](bim-manager/project-setup.sh)

Create an ACC project, set up a standard folder structure (Plans, Specifications, Submittals, Shop Drawings, RFIs, Photos, Reports), and import users from CSV.

**Auth:** 3-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--name <project>` | Project name (required) | — |
| `--hub <hub-id>` | Hub ID (required) | — |
| `--users-csv <file>` | CSV of users to add (columns: `email,role,company`) | — |
| `--folder-structure <type>` | `standard` or `custom` | `standard` |
| `--dry-run` | Preview actions without executing | off |

```bash
./bim-manager/project-setup.sh --name 'Highway Bridge Phase 2' --hub b.abc123
./bim-manager/project-setup.sh --name 'Office Tower' --hub b.abc123 --users-csv team.csv
./bim-manager/project-setup.sh --name 'Test Project' --hub b.abc123 --dry-run
```

---

### [`issue-tracker.sh`](bim-manager/issue-tracker.sh)

List, create from CSV, and bulk-transition ACC issues.

**Auth:** 3-legged

| Subcommand | Description |
|------------|-------------|
| `create-from-csv <file>` | Create issues from CSV (columns: `title,description,status`) |
| `status-report` | Show issue counts grouped by status and type |
| `close-resolved` | Transition all resolved/answered issues to closed |

| Option | Description |
|--------|-------------|
| `--project <id>` | Project ID (required) |
| `--hub <hub-id>` | Hub ID (optional) |
| `--dry-run` | Preview without executing |

```bash
./bim-manager/issue-tracker.sh create-from-csv issues.csv --project abc123
./bim-manager/issue-tracker.sh status-report --project abc123
./bim-manager/issue-tracker.sh close-resolved --project abc123 --dry-run
```

---

### [`rfi-management.sh`](bim-manager/rfi-management.sh)

Create, bulk-create, report on, and answer RFIs (Requests for Information).

**Auth:** 3-legged

| Subcommand | Description |
|------------|-------------|
| `create` | Create a single RFI interactively |
| `bulk-create <csv>` | Create RFIs from CSV (columns: `title,description,priority`) |
| `overdue-report` | Summary of open RFIs with counts by status |
| `answer <rfi-id> <text>` | Answer a specific RFI |

| Option | Description |
|--------|-------------|
| `--project <id>` | Project ID (required) |

```bash
./bim-manager/rfi-management.sh create --project abc123
./bim-manager/rfi-management.sh bulk-create rfis.csv --project abc123
./bim-manager/rfi-management.sh overdue-report --project abc123
./bim-manager/rfi-management.sh answer RFI-001 'Use 4-inch pipe per spec 22 05 00' --project abc123
```

---

### [`folder-permissions-audit.sh`](bim-manager/folder-permissions-audit.sh)

Audit who has access to what folders in a project. Walks each folder and reports user permissions.

**Auth:** 3-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--project <id>` | Project ID (required) | — |
| `--email <user>` | Filter results to a specific user | all users |
| `--output <file>` | Save full report as JSON | — |

```bash
./bim-manager/folder-permissions-audit.sh --project abc123
./bim-manager/folder-permissions-audit.sh --project abc123 --email jane@company.com
./bim-manager/folder-permissions-audit.sh --project abc123 --output audit-report.json
```

---

## PDM Admin

Scripts for account administration: bulk user management from CSV, access auditing, and project lifecycle operations.

### [`user-onboarding.sh`](pdm-admin/user-onboarding.sh)

Bulk add users from a CSV file, assign roles, and optionally add to a specific project.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--csv <file>` | CSV file (required, columns: `email,role,company`) | — |
| `--account <id>` | Account ID (required) | — |
| `--project <id>` | Add users to a specific project | — |
| `--role <role>` | Default role: `project_admin` or `project_user` | `project_user` |
| `--dry-run` | Preview without making changes | off |

```bash
./pdm-admin/user-onboarding.sh --csv team.csv --account ACC123 --dry-run
./pdm-admin/user-onboarding.sh --csv team.csv --account ACC123 --project PROJ456
./pdm-admin/user-onboarding.sh --csv new-hires.csv --account ACC123 --role project_user
```

---

### [`user-offboarding.sh`](pdm-admin/user-offboarding.sh)

Remove a user from all projects in an account, with an audit trail of what was removed.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--email <user>` | User email to remove (required) | — |
| `--account <id>` | Account ID (required) | — |
| `--dry-run` | Preview what would be removed | off |

```bash
./pdm-admin/user-offboarding.sh --email jane@example.com --account ACC123 --dry-run
./pdm-admin/user-offboarding.sh --email departed@example.com --account ACC123
```

---

### [`weekly-access-audit.sh`](pdm-admin/weekly-access-audit.sh)

Report on all users grouped by role and company. Flags admin accounts with no recent sign-in activity.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--account <id>` | Account ID (required) | — |
| `--output <file>` | Save report to JSON file | — |
| `--warn-days <n>` | Flag admins inactive for N days | `90` |

```bash
./pdm-admin/weekly-access-audit.sh --account ACC123
./pdm-admin/weekly-access-audit.sh --account ACC123 --output audit-2026-02.json
./pdm-admin/weekly-access-audit.sh --account ACC123 --warn-days 60
```

---

### [`project-lifecycle.sh`](pdm-admin/project-lifecycle.sh)

Create, list, archive, and report on projects across an account.

**Auth:** 2-legged

| Subcommand | Description |
|------------|-------------|
| `create` | Create a new project (requires `--name`) |
| `list-active` | List all active projects |
| `archive <id>` | Archive a project (supports `--dry-run`) |
| `status-report` | Summary of all projects grouped by status |

| Option | Description |
|--------|-------------|
| `--account <id>` | Account ID (required) |
| `--name <name>` | Project name (for `create`) |
| `--type <type>` | Project type (for `create`, default: `acc`) |
| `--dry-run` | Preview archive without executing |

```bash
./pdm-admin/project-lifecycle.sh create --account ACC123 --name 'Highway Expansion Phase 3'
./pdm-admin/project-lifecycle.sh list-active --account ACC123
./pdm-admin/project-lifecycle.sh archive PROJ456 --account ACC123 --dry-run
./pdm-admin/project-lifecycle.sh status-report --account ACC123
```

---

## PLM Engineer

Scripts for product lifecycle workflows: submittals, asset inventory, checklists, and cross-project portfolio health.

### [`submittal-workflow.sh`](plm-engineer/submittal-workflow.sh)

Create submittals from CSV, view status summaries, and find overdue submittals.

**Auth:** 3-legged

| Subcommand | Description |
|------------|-------------|
| `create-from-csv <file>` | Create submittals from CSV (columns: `title,description,spec_section`) |
| `status-report` | Show submittal counts grouped by status |
| `overdue` | List submittals past their due date |

| Option | Description |
|--------|-------------|
| `--project <id>` | Project ID (required) |

```bash
./plm-engineer/submittal-workflow.sh create-from-csv submittals.csv --project abc123
./plm-engineer/submittal-workflow.sh status-report --project abc123
./plm-engineer/submittal-workflow.sh overdue --project abc123
```

---

### [`asset-inventory.sh`](plm-engineer/asset-inventory.sh)

List, create, update, and export project assets with barcode tracking.

**Auth:** 3-legged

| Subcommand | Description |
|------------|-------------|
| `list` | List all assets in a project |
| `create <description> <barcode>` | Create a new asset |
| `update <id>` | Update an asset interactively |
| `export` | Export all assets to JSON |

| Option | Description | Default |
|--------|-------------|---------|
| `--project <id>` | Project ID (required) | — |
| `--output <file>` | Output file for export | `assets-export.json` |

```bash
./plm-engineer/asset-inventory.sh list --project abc123
./plm-engineer/asset-inventory.sh create 'HVAC Unit 3rd Floor' 'BC-HVAC-003' --project abc123
./plm-engineer/asset-inventory.sh export --project abc123 --output inventory.json
```

---

### [`checklist-management.sh`](plm-engineer/checklist-management.sh)

Browse checklist templates, create checklists, assign them, and view status summaries.

**Auth:** 3-legged

| Subcommand | Description |
|------------|-------------|
| `templates` | List available checklist templates |
| `create-from-template <tmpl-id>` | Create a checklist from a template |
| `assign <checklist-id> <email>` | Assign a checklist to a user |
| `status` | Show checklist counts grouped by status |

| Option | Description |
|--------|-------------|
| `--project <id>` | Project ID (required) |

```bash
./plm-engineer/checklist-management.sh templates --project abc123
./plm-engineer/checklist-management.sh create-from-template TMPL001 --project abc123
./plm-engineer/checklist-management.sh status --project abc123
```

---

### [`portfolio-health-report.sh`](plm-engineer/portfolio-health-report.sh)

Cross-project dashboard combining issues, RFIs, submittals, checklists, and assets summaries across an entire account.

**Auth:** 3-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--account <id>` | Account ID (required) | — |
| `--since <date>` | Filter by date | 30 days ago |
| `--output <file>` | Save combined report to JSON | — |

```bash
./plm-engineer/portfolio-health-report.sh --account ACC123
./plm-engineer/portfolio-health-report.sh --account ACC123 --since 2026-01-01
./plm-engineer/portfolio-health-report.sh --account ACC123 --output portfolio-report.json
```

---

## DevOps

Scripts for CI/CD integration, webhook management, pipeline execution, and multi-environment profile switching.

### [`ci-auth-check.sh`](devops/ci-auth-check.sh)

Validate 2-legged auth and check token expiry. Returns CI-friendly exit codes for pipeline gates.

**Auth:** 2-legged

| Exit Code | Meaning |
|-----------|---------|
| `0` | Authentication valid, token not expiring soon |
| `1` | Authentication failed |
| `3` | Authentication valid but token expiring soon |

| Option | Description | Default |
|--------|-------------|---------|
| `--warn-seconds <n>` | Warn if token expires within N seconds | `300` |
| `--quiet` | Suppress all output, only return exit code | off |

```bash
./devops/ci-auth-check.sh
./devops/ci-auth-check.sh --warn-seconds 600
./devops/ci-auth-check.sh --quiet && echo 'Auth OK' || echo 'Auth failed'
```

---

### [`webhook-setup.sh`](devops/webhook-setup.sh)

Create webhook subscriptions, test endpoints, list all hooks, browse available events, and clean up inactive subscriptions.

**Auth:** 2-legged

| Subcommand | Description |
|------------|-------------|
| `create <url> <event>` | Create a webhook subscription |
| `test <url>` | Test webhook endpoint connectivity |
| `list` | List all webhook subscriptions |
| `events` | List available webhook events |
| `cleanup` | Delete all inactive/failed webhooks |

| Option | Description |
|--------|-------------|
| `--scope <scope>` | Webhook scope (e.g., `workflow`, `data`, `folder`) |
| `--dry-run` | Preview cleanup without deleting |

```bash
./devops/webhook-setup.sh create https://hooks.example.com/aps dm.version.added
./devops/webhook-setup.sh test https://hooks.example.com/aps
./devops/webhook-setup.sh list
./devops/webhook-setup.sh events
./devops/webhook-setup.sh cleanup --dry-run
```

---

### [`pipeline-runner.sh`](devops/pipeline-runner.sh)

Validate a YAML/JSON pipeline file, preview it with dry-run, and execute it with optional error tolerance.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<pipeline-file>` | Pipeline YAML or JSON file (required) | — |
| `--dry-run` | Validate and preview without executing | off |
| `--continue-on-error` | Continue pipeline even if a step fails | off |

```bash
./devops/pipeline-runner.sh pipeline.yaml --dry-run
./devops/pipeline-runner.sh deploy-pipeline.yaml
./devops/pipeline-runner.sh batch-process.json --continue-on-error
```

---

### [`multi-profile-switch.sh`](devops/multi-profile-switch.sh)

Set up named profiles for different APS environments (dev, staging, prod), switch between them, and batch-test all credentials.

**Auth:** varies

| Subcommand | Description |
|------------|-------------|
| `setup <name> <client-id> <secret>` | Create and configure a new profile |
| `switch <name>` | Switch to a profile and test auth |
| `list` | List all configured profiles |
| `current` | Show the active profile |
| `test-all` | Test authentication on every profile |

```bash
./devops/multi-profile-switch.sh setup staging CLIENT_ID_HERE SECRET_HERE
./devops/multi-profile-switch.sh switch staging
./devops/multi-profile-switch.sh list
./devops/multi-profile-switch.sh test-all
```

---

## DA Engineer

Scripts for Design Automation: batch exporting from Revit, converting AutoCAD drawings, and monitoring workitem progress.

### [`revit-export.sh`](da-engineer/revit-export.sh)

Upload a Revit file, create/reuse a DA activity, run a workitem, wait for completion, and download the result.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<input.rvt>` | Revit file to process (required) | — |
| `--output-format <fmt>` | `pdf`, `dwg`, `ifc` | `pdf` |
| `--activity <name>` | DA activity name | `RevitExport` |
| `--engine <id>` | Revit engine version | `Autodesk.Revit+2025` |
| `--bucket <name>` | OSS bucket for staging | `raps-da-<timestamp>` |
| `--out-dir <dir>` | Output directory | `./da-output` |

```bash
./da-engineer/revit-export.sh building.rvt
./da-engineer/revit-export.sh model.rvt --output-format dwg --out-dir ./exports
./da-engineer/revit-export.sh project.rvt --activity MyCustomActivity
```

---

### [`autocad-conversion.sh`](da-engineer/autocad-conversion.sh)

Upload a DWG file and convert it to PDF or DXF via the AutoCAD Design Automation engine.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<input.dwg>` | DWG file to convert (required) | — |
| `--format <fmt>` | Output format: `pdf`, `dxf` | `pdf` |
| `--activity <name>` | DA activity name | `AutoCADConvert` |
| `--engine <id>` | AutoCAD engine version | `Autodesk.AutoCAD+25` |
| `--bucket <name>` | OSS bucket for staging | `raps-da-<timestamp>` |
| `--out-dir <dir>` | Output directory | `./da-output` |

```bash
./da-engineer/autocad-conversion.sh drawing.dwg
./da-engineer/autocad-conversion.sh floorplan.dwg --format dxf
./da-engineer/autocad-conversion.sh site-plan.dwg --format pdf --out-dir ./pdfs
```

---

### [`monitor-workitems.sh`](da-engineer/monitor-workitems.sh)

List active workitems, wait for a specific workitem to complete, and download results.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `--active` | List only in-progress workitems | all |
| `--wait <id>` | Wait for a specific workitem to complete | — |
| `--download <id>` | Download outputs from a completed workitem | — |
| `--out-dir <dir>` | Output directory for downloads | `./da-output` |
| `--timeout <secs>` | Wait timeout in seconds | `600` |

```bash
./da-engineer/monitor-workitems.sh --active
./da-engineer/monitor-workitems.sh --wait WI-ABC123
./da-engineer/monitor-workitems.sh --download WI-ABC123 --out-dir ./results
```

---

## Surveyor

Scripts for reality capture: end-to-end photogrammetry and point cloud ingestion into ACC.

### [`photogrammetry-pipeline.sh`](surveyor/photogrammetry-pipeline.sh)

End-to-end photogrammetry workflow: create a photoscene, upload photos, process, wait for completion, download the result, and optionally push to ACC.

**Auth:** 2-legged (+ 3-legged if `--upload-to-acc`)

| Option | Description | Default |
|--------|-------------|---------|
| `<photos-dir>` | Directory of photos (required, accepts JPG/PNG/TIF) | — |
| `--scene-type <type>` | `aerial` or `object` | `aerial` |
| `--format <fmt>` | Output format: `rcm`, `obj`, `ortho` | `rcm` |
| `--scene-name <name>` | Custom scene name | `photoscene-<timestamp>` |
| `--upload-to-acc <id>` | Upload result to an ACC project | — |
| `--out-dir <dir>` | Output directory for results | `./reality-output` |

```bash
./surveyor/photogrammetry-pipeline.sh ./site-photos
./surveyor/photogrammetry-pipeline.sh ./drone-images --scene-type aerial --format obj
./surveyor/photogrammetry-pipeline.sh ./photos --upload-to-acc PROJ123 --format rcm
```

---

### [`pointcloud-to-acc.sh`](surveyor/pointcloud-to-acc.sh)

Upload a point cloud file (RCP, E57, LAS) to OSS, then create an ACC item in a specified project folder.

**Auth:** 2-legged

| Option | Description | Default |
|--------|-------------|---------|
| `<pointcloud-file>` | Point cloud file (required) | — |
| `<project-id>` | ACC project ID (required) | — |
| `<folder-id>` | Target folder ID (required) | — |
| `--bucket <name>` | OSS bucket for staging | `raps-pointcloud-<timestamp>` |

```bash
./surveyor/pointcloud-to-acc.sh scan.rcp b.abc123 urn:folder:def456
./surveyor/pointcloud-to-acc.sh site-survey.e57 b.abc123 urn:folder:def456 --bucket my-scans
./surveyor/pointcloud-to-acc.sh terrain.las b.abc123 urn:folder:def456
```

---

## Conventions

All scripts follow these conventions:

| Convention | Detail |
|------------|--------|
| Shebang | `#!/usr/bin/env bash` |
| Strict mode | `set -euo pipefail` |
| Shared helpers | Sources `common.sh` for auth checks, output, prompts |
| Help | Every script responds to `--help` / `-h` |
| Dry run | Destructive scripts support `--dry-run` |
| Exit codes | `0` = success, `1` = error, `2` = usage error |
| Auth check | `raps auth test` or `raps auth status` before API calls |
| No hardcoded IDs | Everything via arguments or environment variables |
| JSON parsing | `--output json` used internally, processed with `jq` |
| Confirmation | Interactive `y/n` prompt before destructive operations |
