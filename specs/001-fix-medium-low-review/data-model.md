# Data Model: Fix MEDIUM and LOW Severity Review Findings

**Date**: 2026-02-24
**Feature**: 001-fix-medium-low-review

## New Types

### Model Derivative Metadata Types (raps-derivative)

**ModelViews** — Response from GET /metadata
- guid: String (model GUID)
- name: String (model name)
- role: String (e.g., "3d", "2d")
- mime_type: Option<String>
- has_thumbnail: bool
- progress: Option<String>
- children: Option<Vec<ModelView>>

**ModelView** — Single view/viewable within ModelViews
- guid: String
- name: String
- role: String
- mime_type: Option<String>

**ObjectTree** — Response from GET /metadata/{guid}
- object_id: i64 (node ID)
- name: String
- objects: Option<Vec<ObjectTree>> (recursive children)

**PropertiesResult** — Response from GET /metadata/{guid}/properties
- object_id: i64
- name: String
- external_id: Option<String>
- properties: HashMap<String, HashMap<String, serde_json::Value>> (category → property → value)

**PropertyQuery** — Request body for POST /metadata/{guid}/properties:query
- query: PropertyQueryFilter
- fields: Option<Vec<String>>
- pagination: Option<PropertyPagination>

**PropertyQueryFilter**
- filter: Vec<String> (object IDs or external IDs to filter by)

**PropertyPagination**
- offset: usize
- limit: usize

### OSS Batch Types (raps-oss)

**BatchResult<T>** — Generic batch operation result
- total: usize
- succeeded: usize
- failed: usize
- results: Vec<BatchItemResult<T>>

**BatchItemResult<T>**
- key: String (object key)
- result: Result<T, String> (success value or error message)

### No New Types for Other Changes

- Parallel user imports: Reuses existing `ImportUsersResult`
- DA upload: Uses existing `UploadParameters` and `AppBundleDetails`
- Polling timeouts: No new types needed
- Webhook validation: Uses existing `WEBHOOK_EVENTS` constant
- Filesize helpers: Methods on existing `PhotosceneResult` / `UploadedFile`
- BIM360 folder: Logic change only, no new types

## Relationships

```
DerivativeClient
  └── get_metadata(urn) → ModelViews
  └── get_object_tree(urn, guid) → ObjectTree
  └── get_properties(urn, guid) → PropertiesResult
  └── query_properties(urn, guid, query) → PropertiesResult

OssClient
  └── copy_object(src_bucket, object_key, dest_bucket, dest_key) → ObjectDetails
  └── batch_copy_objects(src_bucket, dest_bucket, keys) → BatchResult<ObjectDetails>
  └── batch_rename_object(bucket, renames) → BatchResult<ObjectDetails>

DesignAutomationClient
  └── upload_appbundle(upload_params, file_path) → ()

ProjectUsersClient
  └── import_users(project_id, users) → ImportUsersResult  [now concurrent]
```

## Validation Rules

- URN must be base64-encoded (existing validation in DerivativeClient)
- Model GUID must be non-empty string
- Object keys must not contain path traversal characters
- Batch operations limited to 1000 items per call (safety cap)
- Concurrency semaphore capped at 10 for imports and batch ops
- App bundle file must exist and be readable before upload attempt
