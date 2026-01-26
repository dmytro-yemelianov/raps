# Data Model: RCW Migration Automation

**Feature**: 001-rcw-migration
**Date**: 2026-01-23

## Overview

This document defines the data structures for RCW migration. Most entities are transient (API request/response) rather than persisted.

## Entities

### 1. RcwModel

Represents a Revit Cloud Worksharing model eligible for migration.

```rust
/// An RCW model identified in a BIM 360/ACC folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcwModel {
    /// Item ID (e.g., "urn:adsk.wipprod:dm.lineage:xxxxx")
    pub item_id: String,

    /// Display name (e.g., "Building-A.rvt")
    pub name: String,

    /// Project ID containing this item
    pub project_id: String,

    /// Latest version ID
    pub version_id: String,

    /// Storage ID for download (OSS URN)
    pub storage_id: String,

    /// File size in bytes
    pub size: Option<i64>,

    /// Last modified timestamp
    pub last_modified: Option<String>,
}
```

**Validation Rules**:
- `item_id` must start with `urn:adsk.wipprod:dm.lineage:`
- `name` must end with `.rvt` (case-insensitive)
- `storage_id` must be a valid OSS URN

### 2. MigrationParams

Parameters passed to the Revit Design Automation plugin.

```rust
/// Migration target parameters (serialized to params.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MigrationParams {
    /// Target account GUID (hub ID without "b." prefix)
    pub target_account_guid: String,

    /// Target project GUID (project ID without "b." prefix)
    pub target_project_guid: String,

    /// Target folder URN (destination folder)
    pub target_folder_urn: String,

    /// Output model name
    pub target_model_name: String,
}
```

**Validation Rules**:
- GUIDs must not contain `b.` prefix
- `target_folder_urn` must be a valid folder URN
- `target_model_name` must end with `.rvt`

### 3. MigrationJob

Tracks the state of a single migration operation.

```rust
/// A migration job tracking a single RCW migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationJob {
    /// Design Automation workitem ID
    pub workitem_id: String,

    /// Source model information
    pub source: RcwModel,

    /// Target folder URL
    pub destination_folder: String,

    /// Current status
    pub status: MigrationStatus,

    /// Progress percentage (0-100)
    pub progress: Option<u8>,

    /// Error message if failed
    pub error: Option<String>,

    /// DA report URL for debugging
    pub report_url: Option<String>,

    /// Timestamps
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MigrationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}
```

**State Transitions**:
```
Pending → InProgress → Completed
                    → Failed
Pending → Cancelled
InProgress → Cancelled
```

### 4. BatchMigration

Aggregates multiple migration jobs.

```rust
/// A batch of migration jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMigration {
    /// Unique batch ID (UUID)
    pub batch_id: String,

    /// Individual jobs in this batch
    pub jobs: Vec<MigrationJob>,

    /// Overall status summary
    pub summary: BatchSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}
```

### 5. RcwActivity (Configuration)

The Design Automation activity configuration for RCW migration.

```rust
/// RCW migration activity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcwActivityConfig {
    /// Activity ID (e.g., "nickname.RCWMigratorActivity+dev")
    pub activity_id: String,

    /// AppBundle ID
    pub appbundle_id: String,

    /// Revit engine version
    pub engine: RevitEngine,

    /// Alias (e.g., "dev", "prod")
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevitEngine {
    #[serde(rename = "Autodesk.Revit+2025")]
    Revit2025,
    #[serde(rename = "Autodesk.Revit+2026")]
    Revit2026,
}
```

## Extended Existing Types

### VersionWithRelationships (raps-dm)

Extend the existing Version type to include storage relationship.

```rust
/// Version with full relationships (for storage access)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionWithRelationships {
    #[serde(rename = "type")]
    pub version_type: String,
    pub id: String,
    pub attributes: VersionAttributes,
    pub relationships: Option<VersionRelationships>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionRelationships {
    pub storage: Option<StorageRelationship>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageRelationship {
    pub data: StorageData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub id: String,
}
```

### WorkItemArgument Extension (raps-da)

The existing WorkItemArgument needs optional `Headers` field.

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemArgument {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "Headers")]
    pub headers_pascal: Option<HashMap<String, String>>, // For rvtFile input
}
```

## Relationships

```
BatchMigration
    └── MigrationJob (1:N)
           ├── RcwModel (source, 1:1)
           └── MigrationParams (target, 1:1)

RcwActivityConfig
    └── Design Automation Activity (1:1)
           └── AppBundle (1:1)
```

## Serialization Notes

1. **MigrationParams**: Uses PascalCase for Revit plugin compatibility
2. **MigrationStatus**: Uses lowercase for JSON output consistency
3. **RevitEngine**: Uses Autodesk's exact engine ID format
4. **Storage IDs**: Preserve exact format from API (OSS URN)
