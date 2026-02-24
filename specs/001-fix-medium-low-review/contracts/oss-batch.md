# Contract: OSS Batch Operations

## Single Copy (base method)

**CLI**: `raps oss copy <BUCKET> <OBJECT_KEY> <DEST_BUCKET> [--dest-key <KEY>]`
**Client**: `client.copy_object(src_bucket: &str, object_key: &str, dest_bucket: &str, dest_key: Option<&str>) -> Result<ObjectDetails>`
**APS endpoint**: `PUT {base}/oss/v2/buckets/{destBucketKey}/objects/{destObjectKey}`
**Auth**: 2-legged, scope `data:write`
**Headers**: `x-ads-copy-from: {sourceBucketKey}/objects/{sourceObjectKey}`

**Error cases**:
- 404: Source object not found
- 403: No write permission to destination bucket
- 409: Destination object already exists (use `x-ads-copy-if-not-exist` header)

---

## Batch Copy

**CLI**: `raps oss batch-copy <SRC_BUCKET> <DEST_BUCKET> [--prefix <PREFIX>] [--keys <KEY1,KEY2,...>]`
**Client**: `client.batch_copy_objects(src_bucket: &str, dest_bucket: &str, object_keys: &[String]) -> Result<BatchResult<ObjectDetails>>`

**Behavior**:
1. List objects in source bucket (optionally filtered by prefix)
2. Copy each to destination bucket with same key, concurrency=10
3. Collect per-object success/failure
4. Report summary

**Error cases**:
- Partial failure: Some copies succeed, some fail — report all results
- Empty source: No objects match — report 0 copied

---

## Batch Rename

**CLI**: `raps oss batch-rename <BUCKET> --from <PATTERN> --to <REPLACEMENT>`
**Client**: `client.batch_rename_object(bucket: &str, renames: &[(String, String)]) -> Result<BatchResult<ObjectDetails>>`

**Behavior**:
1. For each (old_key, new_key) pair:
   a. Copy object from old_key to new_key (same bucket)
   b. Delete old_key on copy success
2. Concurrency=10 via semaphore
3. If copy succeeds but delete fails, report as partial success

**Error cases**:
- Old key not found: Skip with error in results
- New key already exists: Overwrite (APS default behavior)
