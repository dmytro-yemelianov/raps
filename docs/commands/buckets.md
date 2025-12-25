---
layout: default
title: Bucket Commands
---

# Bucket Commands

Manage Object Storage Service (OSS) buckets for storing files.

## Commands

### `raps bucket create`

Create a new OSS bucket interactively.

**Usage:**
```bash
raps bucket create [--key KEY] [--policy POLICY] [--region REGION]
```

**Options:**
- `--key, -k`: Bucket key (optional, will prompt if not provided)
- `--policy, -p`: Retention policy: `transient`, `temporary`, or `persistent`
- `--region, -r`: Region: `US` or `EMEA`

**Example:**
```bash
$ raps bucket create
Note: Bucket keys must be globally unique across all APS applications.
Suggestion: Use a prefix like 'aps-1234567890-yourname'
Enter bucket key: aps-1234567890-mybucket
Select region:
  > US
    EMEA
Select retention policy:
  > transient (deleted after 24 hours)
    temporary (deleted after 30 days)
    persistent (kept until deleted)

✓ Bucket created successfully!
  Key: aps-1234567890-mybucket
  Policy: transient
  Owner: abc123xyz
```

**Non-interactive Example:**
```bash
$ raps bucket create --key my-bucket --policy persistent --region US
✓ Bucket created successfully!
  Key: my-bucket
  Policy: persistent
  Owner: abc123xyz
```

**Retention Policies:**
- `transient` - Deleted automatically after 24 hours
- `temporary` - Deleted automatically after 30 days
- `persistent` - Kept until manually deleted

**Regions:**
- `US` - United States
- `EMEA` - Europe, Middle East, and Africa

**Bucket Key Rules:**
- Must be globally unique across all APS applications
- 3-128 characters
- Lowercase letters, numbers, hyphens, underscores, and dots only

**Requirements:**
- 2-legged OAuth authentication (`raps auth test`)

### `raps bucket list`

List all buckets from all regions.

**Usage:**
```bash
raps bucket list
```

**Example:**
```bash
$ raps bucket list
Fetching buckets from all regions...

Buckets:
──────────────────────────────────────────────────────────────────────────────────────────
Bucket Key                              Policy        Region   Created
──────────────────────────────────────────────────────────────────────────────────────────
aps-1234567890-mybucket                 transient     US       2 hours ago
my-persistent-bucket                     persistent    EMEA     5 days ago
test-bucket                              temporary     US       1 day ago
──────────────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication

### `raps bucket info`

Show detailed information about a bucket.

**Usage:**
```bash
raps bucket info <bucket-key>
```

**Example:**
```bash
$ raps bucket info aps-1234567890-mybucket
Fetching bucket details...

Bucket Details
────────────────────────────────────────────────────────────
  Key: aps-1234567890-mybucket
  Owner: abc123xyz
  Policy: transient
  Created: 2 hours ago

  Permissions:
    • def456uvw: full
    • ghi789rst: read
────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication

### `raps bucket delete`

Delete a bucket.

**Usage:**
```bash
raps bucket delete [bucket-key] [--yes]
```

**Options:**
- `bucket-key`: Bucket key to delete (optional, will prompt if not provided)
- `--yes, -y`: Skip confirmation prompt

**Example:**
```bash
$ raps bucket delete aps-1234567890-mybucket
Are you sure you want to delete bucket 'aps-1234567890-mybucket'? [y/N]: y
Deleting bucket...
✓ Bucket 'aps-1234567890-mybucket' deleted successfully!
```

**Non-interactive Example:**
```bash
$ raps bucket delete aps-1234567890-mybucket --yes
Deleting bucket...
✓ Bucket 'aps-1234567890-mybucket' deleted successfully!
```

**Note:** Buckets must be empty before deletion. Delete all objects first using `raps object delete`.

**Requirements:**
- 2-legged OAuth authentication

## Common Workflows

### Create a Bucket for Model Translation

```bash
# Create a persistent bucket in US region
raps bucket create --key my-models-bucket --policy persistent --region US

# Upload a model
raps object upload my-models-bucket model.dwg

# Translate the model
raps translate start <urn> --format svf2
```

### Clean Up Temporary Buckets

```bash
# List all buckets
raps bucket list

# Delete transient/temporary buckets
raps bucket delete old-transient-bucket --yes
```

## Best Practices

1. **Use descriptive bucket keys** with a prefix (e.g., `aps-1234567890-project-name`)
2. **Choose the right retention policy**:
   - Use `transient` for temporary test files
   - Use `temporary` for short-term storage
   - Use `persistent` for production data
3. **Select the appropriate region** based on your location and data residency requirements
4. **Clean up unused buckets** to avoid unnecessary storage costs

## Related Commands

- [Objects](commands/objects) - Manage objects in buckets
- [Translation](commands/translation) - Translate files stored in buckets
- [Authentication](commands/auth) - Set up authentication

