# Research: RCW Migration Automation

**Feature**: 001-rcw-migration
**Date**: 2026-01-23

## Research Tasks Completed

### 1. RCW Migration API Workflow

**Source**: [aps-revit-rcw-migrate-automation](https://github.com/autodesk-platform-services/aps-revit-rcw-migrate-automation)

**Decision**: Use Design Automation with custom AppBundle/Activity pattern

**Rationale**:
- Autodesk's reference implementation demonstrates a proven workflow
- Downloads RCW as detached model, then uses `SaveAsCloudModel()` to republish to ACC
- Works without desktop Revit installation

**Alternatives Considered**:
- Direct API approach: Not possible - Revit model processing requires DA
- Desktop Revit automation: Requires local installation, not scriptable

### 2. AppBundle and Activity Configuration

**Decision**: Create pre-built AppBundle with RCW migration plugin

**Key Parameters**:
```
Activity Parameters:
- rvtFile: verb=get, description="Input Revit model file (downloaded from URL)"
- inputParams: verb=get, localName="params.json", description="Migration target info"
- adsk3LeggedToken: Special parameter for 3-legged token passthrough

Command Line:
- $(engine.path)\\revitcoreconsole.exe /i "$(args[rvtFile].path)" /al "$(appbundles[RCWMigratorAppBundle].path)"
```

**Engine Support**: Autodesk.Revit+2025, Autodesk.Revit+2026

**Rationale**: Following the official sample ensures compatibility with Autodesk's cloud infrastructure

### 3. Authentication Requirements

**Decision**: Dual authentication - 2-legged for DA, 3-legged for DM

**Workflow**:
1. Use 2-legged token (client credentials) for Design Automation API calls
2. Use 3-legged token (user auth) for Data Management API and pass to workitem
3. The Revit plugin uses the 3-legged token to call `SaveAsCloudModel()`

**Rationale**:
- DA API requires 2-legged auth (code:all scope)
- DM API and cloud publishing require user context (3-legged)

### 4. Input JSON Format for Migration

**Decision**: Use JSON params file with target location info

**Format**:
```json
{
  "TargetAccountGuid": "account-guid-without-b-prefix",
  "TargetProjectGuid": "project-guid-without-b-prefix",
  "TargetFolderUrn": "urn:adsk.wipprod:fs.folder:co.xxxxx",
  "TargetModelName": "filename.rvt"
}
```

**Rationale**: Matches the expected format of the RCW Migrator plugin

### 5. Identifying RCW Models

**Decision**: Filter by extension type `items:autodesk.bim360:C4RModel` or `versions:autodesk.bim360:C4RModel`

**Implementation**:
- List folder contents via DM API
- Check `attributes.extension.type` for C4RModel pattern
- Standard Revit files have type `items:autodesk.bim360:File`

**Rationale**: C4RModel is the official extension type for Cloud Worksharing models

### 6. Version Relationships for Storage Access

**Decision**: Extend `raps-dm` Version struct to include relationships

**Required Fields**:
```rust
pub struct VersionRelationships {
    pub storage: Option<StorageData>,
}

pub struct StorageData {
    pub data: StorageRef,
}

pub struct StorageRef {
    pub id: String,  // OSS storage URN
}
```

**Rationale**: Storage ID is required to construct the download URL for DA input

### 7. Polling Strategy for Job Status

**Decision**: Poll every 5 seconds with exponential backoff on errors

**Workflow**:
1. Create workitem, get workitem ID
2. Poll `GET /workitems/{id}` every 5 seconds
3. Check status: pending → inprogress → success/failed
4. On network errors, backoff: 5s → 10s → 20s → fail

**Rationale**: 5 seconds is standard for DA workitems per APS documentation

### 8. Batch Migration Approach

**Decision**: Sequential submission with parallel status tracking

**Workflow**:
1. Enumerate all RCW models in source folder
2. Submit migration workitems sequentially (to avoid rate limits)
3. Track all workitem IDs
4. Poll status for all in parallel
5. Report aggregate progress and final summary

**Constraint**: Maximum 50 files per batch (configurable)

**Rationale**: Sequential submission prevents API rate limiting; parallel tracking provides responsive UX

### 9. Error Handling Strategy

**Decision**: Provide actionable error messages with remediation steps

**Error Categories**:
| Error | Message | Remediation |
|-------|---------|-------------|
| 401 Unauthorized | Token expired or invalid | Re-authenticate with `raps auth login` |
| 403 Forbidden | No access to project/folder | Verify permissions in BIM 360/ACC Admin |
| 404 Not Found | Item/folder doesn't exist | Check the item/folder ID |
| Invalid C4RModel | Not a cloud worksharing model | Only RCW models can be migrated |
| DA Timeout | Processing exceeded time limit | Try smaller/simpler model |

**Rationale**: Users need clear guidance to resolve issues independently

### 10. CLI Command Structure

**Decision**: Add `rcw` subcommand group under `da`

**Commands**:
```
raps da rcw configure --engine 2025|2026
raps da rcw list <folder-url> [--recursive]
raps da rcw migrate <source-item-url> <dest-folder-url> [--engine 2025|2026]
raps da rcw batch <source-folder-url> <dest-folder-url> [--limit N]
raps da rcw status <workitem-id> [--wait]
raps da rcw cancel <workitem-id>
```

**Rationale**: Groups RCW-specific commands while maintaining consistency with existing DA commands

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| raps-kernel | workspace | Auth, config, HTTP client |
| raps-da | workspace | Design Automation client |
| raps-dm | workspace | Data Management client |
| clap | 4.5 | CLI parsing |
| tokio | 1.49 | Async runtime |
| serde_json | 1.0 | JSON serialization |

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| AppBundle not available | High - feature broken | Document setup in quickstart; provide error message pointing to configure |
| Token expiry mid-batch | Medium - partial completion | Refresh token before each workitem submission |
| Large models timeout | Medium - job failure | Document size limits; suggest splitting linked models |
| API rate limiting | Low - slow batches | Sequential submission with delays |
