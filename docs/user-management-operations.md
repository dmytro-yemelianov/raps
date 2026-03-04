# User & Project Management Operations — RAPS CLI Guide

This document covers eight common administrative operations for Autodesk Construction Cloud (ACC) and BIM 360, their feasibility with the RAPS CLI, and step-by-step reproduction instructions where applicable.

---

## Prerequisites (all operations)

**Authentication:** Most admin operations require 3-legged OAuth (logged in as an Account Admin).

```bash
raps auth login          # opens browser for 3-legged OAuth
raps auth status         # confirm you are logged in as Account Admin
```

**Account ID:** Required for all admin commands. Retrieve it via RAPS after logging in:

```bash
raps hub list
```

This lists all accessible hubs. The hub ID (e.g. `b.abc123...`) is your Account ID — use it directly:

```bash
export APS_ACCOUNT_ID=<hub-id-from-raps-hub-list>
```

Alternatively, find it in the Autodesk Construction Cloud web UI under **Account Settings > Account Info**.

Alternatively, pass it directly with `--account <id>` on any admin command.

---

## 1. Mass Add New User to Projects

**Status: ✅ Fully Supported**

Adds a single user to multiple ACC/BIM360 projects in one command, with filtering, dry-run, and parallel execution.

### Setup

- Account Admin role
- 3-legged OAuth login
- Target user's email address
- Desired role name (e.g. "Project Admin", "Document Manager")

### Steps

**1. Preview which projects the user will be added to (dry run):**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin" \
  --dry-run
```

This shows a table of matched projects without making any changes.

**2. Add to all active projects:**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin"
```

**3. Add to a subset of projects by name pattern:**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Document Manager" \
  --filter "status:active,name:*Hospital*"
```

**4. Add to specific projects via ID list (create a file `project-ids.txt`, one ID per line):**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin" \
  --project-ids project-ids.txt
```

**5. Speed up with parallel execution (up to 50 concurrent):**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin" \
  --concurrency 20
```

**6. Export results to CSV:**

```bash
raps admin user add john.smith@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin" \
  --output csv > add-results.csv
```

### Output

The command reports: Total projects matched, Added, Skipped (already a member), Failed.

---

## 2. Change User Name / Email Address While Retaining Permissions

**Status: ⚠️ Partially Supported**

**Important limitation:** The Autodesk Platform Services API does not support changing a user's email address. Email is the user's identity in the platform — it cannot be changed via any API, including RAPS.

**What RAPS can do:**
- Update the user's **company assignment** across projects
- Update the user's **role** across projects
- All existing project memberships and permissions are preserved automatically

### When does this actually help?

If a consultant's firm merges and their email stays the same but their company changes, you can update their company affiliation:

```bash
raps admin user update john.smith@oldcompany.com \
  --account $APS_ACCOUNT_ID \
  --company "New Company Name"
```

To also update their role across projects:

```bash
raps admin user update john.smith@oldcompany.com \
  --account $APS_ACCOUNT_ID \
  --role "Document Manager" \
  --filter "status:active"
```

### What to do when email actually changes

When a consultant changes their email (new domain), there is no automated way to transfer permissions via the API. The recommended workflow:

1. **Export the old user's project access to CSV** (see Operation 5 below)
2. **Add the new user** to all the same projects (Operation 1 above)
3. **Remove the old user** from projects:

```bash
raps admin user remove old.email@company.com \
  --account $APS_ACCOUNT_ID \
  --dry-run          # preview first
raps admin user remove old.email@company.com \
  --account $APS_ACCOUNT_ID
```

Folder-level permissions must be re-applied manually (see Operation 3).

---

## 3. Copy / Paste User Project Access / Folder Permissions to New Employee

**Status: ❌ Not Directly Supported — Manual Workaround Available**

There is no "clone permissions from User A to User B" command. However, you can approximate this with a multi-step workflow.

### Workaround: Mirror project access

**Step 1: Identify the source user's current projects and role**

```bash
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --search source.user@company.com \
  --output csv > source-user-projects.csv
```

**Step 2: Add the new user to the same projects**

If the source user is in all active projects:

```bash
raps admin user add new.user@company.com \
  --account $APS_ACCOUNT_ID \
  --role "Document Manager" \
  --filter "status:active"
```

**Step 3: Set folder-level permissions (if required)**

Folder permissions must be set per folder type. The default folder is `project-files`; pass `--folder` for others:

```bash
# Preview first
raps admin folder rights new.user@company.com \
  --account $APS_ACCOUNT_ID \
  --level view-download-upload \
  --folder project-files \
  --filter "status:active" \
  --dry-run

# Apply
raps admin folder rights new.user@company.com \
  --account $APS_ACCOUNT_ID \
  --level view-download-upload \
  --folder project-files \
  --filter "status:active"
```

**Available permission levels** (pass to `--level`):

| Level | Description |
|-------|-------------|
| `view-only` | Read-only access |
| `view-download` | View and download |
| `upload-only` | Upload only |
| `view-download-upload` | View, download, upload |
| `view-download-upload-edit` | Full edit access |
| `folder-control` | Admin-level folder control |

### Cost estimate for native copy-permissions feature

Building an automated permission-mirror command in RAPS would require:
- Querying the source user's folder permissions per project (APS API: `GET /construction/admin/v1/projects/{projectId}/users/{userId}/access-levels`)
- Writing a mapping layer and applying those permissions to the target user

**Estimated development effort: 2–3 days.** This is feasible and could be added as `raps admin user clone-permissions <source-email> <target-email>`.

---

## 4. Copy / Paste Existing Project with User Permissions / Folder Permissions

**Status: ⚠️ Partially Supported**

RAPS supports creating projects from templates (which is how ACC implements project cloning). Full user/permission roster copy depends on what the ACC template captures, which is set when the template was created.

### Create project from a template

**Step 1: List available templates**

```bash
raps template list --account $APS_ACCOUNT_ID
```

**Step 2: Get template details**

```bash
raps template info --account $APS_ACCOUNT_ID --template-id <TEMPLATE_ID>
```

**Step 3: Create a project from the template**

```bash
raps project create \
  --account $APS_ACCOUNT_ID \
  --name "New Hospital Wing C" \
  --template <TEMPLATE_ID>
```

### Limitation

The ACC template system determines which elements are copied (folder structure, companies, locations). **User rosters and folder-level permissions are not reliably included** in ACC project templates — this is an ACC platform limitation, not a RAPS limitation.

### Recommendation

Use ACC's native "Create Project from Template" workflow in the web UI for the most control over what gets cloned. RAPS can then be used post-creation to bulk-add users (Operation 1) and set folder permissions (Operation 3).

### Cost estimate for full project-copy command

A full `raps admin project copy <source-id> <new-name>` command that copies user roster + folder permissions would require:
- Fetching all members of the source project
- Fetching all folder permission assignments per member
- Creating the new project
- Re-applying all memberships and permissions

**Estimated development effort: 4–5 days.** Feasible.

---

## 5. Extract User Projects / Folder Permissions to CSV

**Status: ✅ Fully Supported**

### Export all users in a specific project

```bash
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --project <PROJECT_ID> \
  --output csv > project-users.csv
```

### Export all users across all projects (account-wide)

```bash
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --output csv > all-account-users.csv
```

### Filter by status before exporting

```bash
# Active users only
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --status active \
  --output csv > active-users.csv

# Users not yet accepted invite
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --status not_invited \
  --output csv > pending-users.csv
```

### Filter by role before exporting

```bash
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --role "Project Admin" \
  --output csv > project-admins.csv
```

**CSV fields included:** id, email, name, role, company, status, project

> Note: Folder-level permission data is not directly exportable to CSV via RAPS. The `folder rights` command is a write operation (it applies permissions). Folder assignments must be reviewed in the ACC web UI.

---

## 6. Extract Current User / Company List to CSV

**Status: ✅ Fully Supported**

### Export all account users to CSV

```bash
raps admin user list \
  --account $APS_ACCOUNT_ID \
  --output csv > account-users.csv
```

### Export all companies to CSV

```bash
raps admin company-list \
  --account $APS_ACCOUNT_ID \
  --output csv > companies.csv
```

**Company CSV fields:** id, name, trade, city, country, member_count

### Export project list to CSV

```bash
raps admin project list \
  --account $APS_ACCOUNT_ID \
  --output csv > projects.csv
```

### Combined: full account roster snapshot

```bash
# Users
raps admin user list --account $APS_ACCOUNT_ID --output csv > snapshot-users.csv

# Companies
raps admin company-list --account $APS_ACCOUNT_ID --output csv > snapshot-companies.csv

# Projects
raps admin project list --account $APS_ACCOUNT_ID --output csv > snapshot-projects.csv
```

---

## 7. Permanently Delete Projects from BIM360 / ACC

**Status: ❌ Not Supported — Platform Limitation**

RAPS cannot permanently delete ACC/BIM360 projects. This is not a RAPS limitation — **Autodesk's API does not expose a delete endpoint for projects.** This applies to all tools, not just RAPS.

The closest available operations are:

### Archive a project (hides it from active lists)

```bash
raps admin project archive \
  --account $APS_ACCOUNT_ID \
  --project <PROJECT_ID>
```

Archived projects are hidden from normal project lists but remain in the system for audit and compliance purposes.

### Suspend a project

```bash
raps admin project update \
  --account $APS_ACCOUNT_ID \
  --project <PROJECT_ID> \
  --status suspended
```

### Filter out archived projects in listings

```bash
raps admin project list \
  --account $APS_ACCOUNT_ID \
  --status active    # only shows non-archived projects
```

### For actual deletion

Contact Autodesk Support directly. Permanent project deletion in ACC is only possible through Autodesk's backend and requires a support request.

---

## 8. Archive Entire Project to Zip for Backup (Local / OneDrive / Vault)

**Status: ❌ Not Supported**

RAPS does not have a bulk project export or archive-to-zip feature. This capability does not exist in the current Autodesk Platform Services API in a way that would allow full project archival (documents, models, metadata, issues, RFIs) in one operation.

### What RAPS can export today

**Metadata exports (CSV/JSON):**

```bash
# Issues summary across projects
raps report issues-summary --account $APS_ACCOUNT_ID --output csv > issues.csv

# RFI summary across projects
raps report rfi-summary --account $APS_ACCOUNT_ID --output csv > rfis.csv

# Submittals summary
raps report submittals-summary --account $APS_ACCOUNT_ID --output csv > submittals.csv

# User roster
raps admin user list --account $APS_ACCOUNT_ID --output csv > users.csv
```

**Individual file downloads (not bulk):**

```bash
# Download a specific object from OSS
raps object download <bucket-key> <object-key> --output ./local-file.rvt

# Download a translated derivative
raps translate download <model-urn> --format obj --output ./model.obj
```

### Recommended alternatives for full project backup

1. **ACC Data Connector** (Autodesk's native tool) — designed for bulk data export from ACC to BI tools or local storage. Supports scheduled exports.
2. **ACC/BIM360 Export via web UI** — for smaller projects, documents can be exported folder by folder.
3. **Autodesk Vault** (for Inventor/AutoCAD workflows) — handles PDM-style archival natively.

### Cost estimate for bulk export feature in RAPS

Building `raps admin project export <project-id> --output ./backup.zip` would require:
- Recursively listing all folders and items via Data Management API
- Downloading each file version via signed OSS URLs
- Collecting issues, RFIs, submittals, assets via ACC APIs
- Packaging into a structured zip

**Estimated development effort: 10–15 days.** Complex but feasible. The main constraint is API rate limits and large file sizes making this a long-running operation requiring checkpoint/resume support (which RAPS already has infrastructure for).

---

## Summary

| Operation | Status | Command |
|-----------|--------|---------|
| 1. Mass add user to projects | ✅ Supported | `raps admin user add <email> --role <role>` |
| 2. Change user name/email retaining permissions | ⚠️ Partial | Email change not possible via API; company/role update supported |
| 3. Copy permissions from user to user | ❌ Manual workaround | Add new user + apply folder rights manually; ~2-3 days to build natively |
| 4. Copy project with permissions | ⚠️ Partial | `raps project create --template <id>`; full copy ~4-5 days to build |
| 5. Extract user permissions to CSV | ✅ Supported | `raps admin user list --output csv` |
| 6. Extract user/company list to CSV | ✅ Supported | `raps admin user list --output csv` + `raps admin company-list --output csv` |
| 7. Permanently delete projects | ❌ Not possible | Platform limitation — Autodesk API has no delete endpoint |
| 8. Archive project to zip for backup | ❌ Not supported | ~10-15 days to build; use ACC Data Connector in the meantime |
