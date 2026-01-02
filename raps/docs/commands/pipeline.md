---
layout: default
title: Pipeline Commands
---

# Pipeline Commands

Execute batch operations and automated pipelines using YAML or JSON configuration files.

## Commands

### `raps pipeline run`

Execute a pipeline from a YAML or JSON file.

**Usage:**
```bash
raps pipeline run <file> [--dry-run] [--continue-on-error] [--var KEY=VALUE]
```

**Arguments:**
- `file`: Path to pipeline file (YAML or JSON)

**Options:**
- `--dry-run, -d`: Show what would be executed without running
- `--continue-on-error, -c`: Continue even if a step fails
- `--var, -v`: Override or add pipeline variables (can be used multiple times)

**Example:**
```bash
$ raps pipeline run upload-workflow.yaml
Running pipeline: Upload Workflow
────────────────────────────────────────────────────────────
Step 1/3: Create bucket
  → Running: bucket create ${BUCKET}
  ✓ Completed (1.2s)

Step 2/3: Upload model
  → Running: object upload ${BUCKET} ${FILE}
  ✓ Completed (5.3s)

Step 3/3: Start translation
  → Running: translate start ${URN} --format svf2
  ✓ Completed (0.8s)

────────────────────────────────────────────────────────────
Pipeline completed: 3/3 steps successful
Total time: 7.3s
```

**Dry Run:**
```bash
$ raps pipeline run workflow.yaml --dry-run
Dry run mode - no commands will be executed

Pipeline: Upload Workflow
Variables:
  BUCKET: my-bucket
  FILE: ./model.dwg

Steps to execute:
  1. Create bucket
     Command: bucket create my-bucket

  2. Upload model
     Command: object upload my-bucket ./model.dwg

  3. Start translation
     Command: translate start <urn> --format svf2
```

**With Variable Override:**
```bash
$ raps pipeline run workflow.yaml --var BUCKET=prod-bucket --var FILE=./new-model.dwg
```

**Requirements:**
- Valid YAML or JSON pipeline file
- Appropriate authentication for commands

### `raps pipeline validate`

Validate a pipeline file without executing it.

**Usage:**
```bash
raps pipeline validate <file>
```

**Example:**
```bash
$ raps pipeline validate workflow.yaml
✓ Pipeline 'Upload Workflow' is valid
  Steps: 3
  Variables: BUCKET, FILE
```

**Validation Errors:**
```bash
$ raps pipeline validate invalid.yaml
✗ Pipeline validation failed:
  - Step 2: Missing 'command' field
  - Variable 'BUCKET' is used but not defined
```

### `raps pipeline sample`

Generate a sample pipeline file.

**Usage:**
```bash
raps pipeline sample [--output FILE] [--format yaml|json]
```

**Options:**
- `--output, -o`: Output file path (default: stdout)
- `--format, -f`: Output format (yaml or json, default: yaml)

**Example:**
```bash
$ raps pipeline sample --output my-pipeline.yaml
✓ Sample pipeline written to my-pipeline.yaml
```

## Pipeline File Format

### YAML Format

```yaml
name: Upload and Translate Workflow
description: Upload a model and start translation

# Variables can be used in commands with ${VAR_NAME}
variables:
  BUCKET: my-bucket
  PROJECT_ID: "12345"

# Steps are executed in order
steps:
  - name: List buckets
    command: bucket list
    
  - name: Create bucket
    command: bucket create ${BUCKET}
    continue_on_error: true  # Don't fail if bucket exists
    
  - name: Upload model
    command: object upload ${BUCKET} model.dwg
    
  - name: Start translation
    command: translate start ${URN} --format svf2
    condition: ${TRANSLATE_ENABLED}  # Only run if condition is truthy
```

### JSON Format

```json
{
  "name": "Upload and Translate Workflow",
  "description": "Upload a model and start translation",
  "variables": {
    "BUCKET": "my-bucket",
    "PROJECT_ID": "12345"
  },
  "steps": [
    {
      "name": "List buckets",
      "command": "bucket list"
    },
    {
      "name": "Create bucket",
      "command": "bucket create ${BUCKET}",
      "continue_on_error": true
    },
    {
      "name": "Upload model",
      "command": "object upload ${BUCKET} model.dwg"
    }
  ]
}
```

## Step Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | Human-readable step name |
| `command` | string | Yes | RAPS command to execute (without `raps` prefix) |
| `continue_on_error` | boolean | No | Continue if step fails (default: false) |
| `condition` | string | No | Only execute if condition evaluates to truthy |

## Variable Substitution

Variables are substituted in commands using `${VAR_NAME}` syntax:

```yaml
variables:
  BUCKET: my-bucket
  FILE: model.dwg

steps:
  - name: Upload
    command: object upload ${BUCKET} ${FILE}
    # Becomes: object upload my-bucket model.dwg
```

**Override variables at runtime:**
```bash
raps pipeline run workflow.yaml --var BUCKET=other-bucket
```

## Conditional Execution

Steps can be conditionally executed based on variable values:

```yaml
variables:
  ENABLE_TRANSLATION: "true"

steps:
  - name: Translate
    command: translate start ${URN}
    condition: ${ENABLE_TRANSLATION}  # Only runs if truthy
```

**Truthy values:** `true`, `1`, `yes`, any non-empty string
**Falsy values:** `false`, `0`, empty string, undefined

## Error Handling

### Continue on Error

Use `continue_on_error` to allow the pipeline to continue if a step fails:

```yaml
steps:
  - name: Create bucket (may already exist)
    command: bucket create ${BUCKET}
    continue_on_error: true
    
  - name: Upload file (runs even if bucket creation failed)
    command: object upload ${BUCKET} file.dwg
```

### Pipeline Exit Codes

| Exit Code | Description |
|-----------|-------------|
| 0 | All steps completed successfully |
| 1 | One or more steps failed |
| 2 | Pipeline validation failed |
| 3 | Pipeline file not found |

## Common Workflows

### Upload and Translate

```yaml
name: Upload and Translate
variables:
  BUCKET: my-bucket
  FILE: model.dwg

steps:
  - name: Ensure bucket exists
    command: bucket create ${BUCKET}
    continue_on_error: true
    
  - name: Upload model
    command: object upload ${BUCKET} ${FILE}
    
  - name: Start translation
    command: translate start auto --format svf2 --wait
```

### Batch File Upload

```yaml
name: Batch Upload
variables:
  BUCKET: uploads-bucket

steps:
  - name: Upload drawings
    command: object upload ${BUCKET} *.dwg --batch --parallel 5
    
  - name: Upload models
    command: object upload ${BUCKET} *.rvt --batch --parallel 3
```

### CI/CD Integration

```yaml
name: CI Pipeline
description: Automated build pipeline

steps:
  - name: Authenticate
    command: auth test
    
  - name: Upload artifacts
    command: object upload ci-artifacts build/output.zip
    
  - name: Trigger translation
    command: translate start auto --format svf2
    
  - name: Wait for completion
    command: translate status auto --wait
```

## Best Practices

1. **Use meaningful step names** for clear output
2. **Use variables** for reusable values
3. **Use `continue_on_error`** for idempotent operations (like bucket creation)
4. **Validate pipelines** before running in production
5. **Use `--dry-run`** to preview execution
6. **Use conditions** for optional steps

## Related Commands

- [Authentication](auth.md) - Set up credentials
- [Objects](objects.md) - File operations
- [Translation](translation.md) - Model translation
- [Buckets](buckets.md) - Bucket management

