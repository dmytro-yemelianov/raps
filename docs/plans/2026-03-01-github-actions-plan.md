# GitHub Actions Suite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a `raps-actions` GitHub repository with 4 composite actions: setup, pipeline, upload, and translate.

**Architecture:** Composite actions (YAML-only, no TypeScript). Each action is a directory with `action.yml` that runs shell commands. The `setup` action installs RAPS via npm and configures auth. Other actions depend on setup having run first.

**Tech Stack:** GitHub Actions composite actions, bash scripts, npm (for RAPS installation)

---

### Task 1: Create Repository and Setup Action

**Step 1: Create the raps-actions repository**

Run:
```bash
gh repo create dmytro-yemelianov/raps-actions --public --description "GitHub Actions for RAPS (Rust Autodesk Platform Services CLI)" --clone
```

**Step 2: Create the setup action**

Create file: `setup/action.yml`

```yaml
name: "RAPS Setup"
description: "Install RAPS CLI and configure APS authentication"
branding:
  icon: "terminal"
  color: "orange"

inputs:
  version:
    description: "RAPS version to install (e.g., 4.15.0 or latest)"
    required: false
    default: "latest"
  client-id:
    description: "APS Client ID"
    required: true
  client-secret:
    description: "APS Client Secret"
    required: true

runs:
  using: "composite"
  steps:
    - name: Install RAPS
      shell: bash
      run: |
        if [ "${{ inputs.version }}" = "latest" ]; then
          npm install -g @APS/raps
        else
          npm install -g @APS/raps@${{ inputs.version }}
        fi
        echo "RAPS $(raps --version) installed"

    - name: Configure authentication
      shell: bash
      env:
        APS_CLIENT_ID: ${{ inputs.client-id }}
        APS_CLIENT_SECRET: ${{ inputs.client-secret }}
      run: |
        raps auth test --output json
        echo "APS authentication verified"
```

**Step 3: Create root README.md**

Create file: `README.md`

```markdown
# RAPS GitHub Actions

Official GitHub Actions for [RAPS](https://github.com/dmytro-yemelianov/raps) — the Rust Autodesk Platform Services CLI.

## Actions

| Action | Description |
|--------|-------------|
| [setup](setup/) | Install RAPS and configure APS authentication |
| [upload](upload/) | Upload files to Autodesk OSS |
| [translate](translate/) | Translate models via Model Derivative API |
| [pipeline](pipeline/) | Run a RAPS pipeline file |

## Quick Start

See each action's README for usage details.
```

**Step 4: Commit and push**

```bash
git add -A
git commit -m "feat: add setup action for RAPS CLI installation and auth"
git push -u origin main
```

---

### Task 2: Pipeline Action

**Step 1: Create the pipeline action**

Create file: `pipeline/action.yml`

```yaml
name: "RAPS Pipeline"
description: "Run a RAPS pipeline file"
branding:
  icon: "play"
  color: "orange"

inputs:
  file:
    description: "Path to pipeline YAML/JSON file"
    required: true
  variables:
    description: "Pipeline variables (key=value, one per line)"
    required: false
    default: ""
  dry-run:
    description: "Preview pipeline without executing"
    required: false
    default: "false"
  ignore-failure:
    description: "Continue on step failures"
    required: false
    default: "false"

runs:
  using: "composite"
  steps:
    - name: Validate pipeline
      shell: bash
      run: raps pipeline validate "${{ inputs.file }}"

    - name: Run pipeline
      shell: bash
      run: |
        ARGS="pipeline run ${{ inputs.file }}"
        if [ "${{ inputs.dry-run }}" = "true" ]; then
          ARGS="$ARGS --dry-run"
        fi
        if [ "${{ inputs.ignore-failure }}" = "true" ]; then
          ARGS="$ARGS --ignore-failure"
        fi

        # Export variables as env vars for RAPS
        if [ -n "${{ inputs.variables }}" ]; then
          while IFS='=' read -r key value; do
            [ -n "$key" ] && export "RAPS_VAR_${key}=${value}"
          done <<< "${{ inputs.variables }}"
        fi

        raps $ARGS
```

**Step 2: Commit**

```bash
git add pipeline/
git commit -m "feat: add pipeline action for running RAPS pipeline files"
```

---

### Task 3: Upload Action

**Step 1: Create the upload action**

Create file: `upload/action.yml`

```yaml
name: "RAPS Upload"
description: "Upload files to Autodesk OSS buckets"
branding:
  icon: "upload-cloud"
  color: "orange"

inputs:
  bucket:
    description: "OSS bucket key"
    required: true
  files:
    description: "File path or glob pattern to upload"
    required: true
  create-bucket:
    description: "Create bucket if it doesn't exist"
    required: false
    default: "false"
  bucket-policy:
    description: "Bucket retention policy (transient, temporary, persistent)"
    required: false
    default: "transient"

outputs:
  urn:
    description: "URN of the last uploaded object"
    value: ${{ steps.upload.outputs.urn }}
  object-count:
    description: "Number of objects uploaded"
    value: ${{ steps.upload.outputs.count }}

runs:
  using: "composite"
  steps:
    - name: Create bucket if needed
      if: inputs.create-bucket == 'true'
      shell: bash
      run: |
        raps bucket info "${{ inputs.bucket }}" 2>/dev/null || \
        raps bucket create --key "${{ inputs.bucket }}" --policy "${{ inputs.bucket-policy }}"

    - name: Upload files
      id: upload
      shell: bash
      run: |
        COUNT=0
        LAST_URN=""
        for file in ${{ inputs.files }}; do
          if [ -f "$file" ]; then
            echo "Uploading: $file"
            OUTPUT=$(raps object upload "${{ inputs.bucket }}" "$file" --output json 2>/dev/null)
            LAST_URN=$(echo "$OUTPUT" | grep -o '"urn":"[^"]*"' | head -1 | cut -d'"' -f4 || true)
            COUNT=$((COUNT + 1))
          fi
        done

        echo "urn=${LAST_URN}" >> "$GITHUB_OUTPUT"
        echo "count=${COUNT}" >> "$GITHUB_OUTPUT"
        echo "Uploaded ${COUNT} file(s)"
```

**Step 2: Commit**

```bash
git add upload/
git commit -m "feat: add upload action for OSS file uploads"
```

---

### Task 4: Translate Action

**Step 1: Create the translate action**

Create file: `translate/action.yml`

```yaml
name: "RAPS Translate"
description: "Translate models via Autodesk Model Derivative API"
branding:
  icon: "refresh-cw"
  color: "orange"

inputs:
  urn:
    description: "URN of the object to translate"
    required: true
  wait:
    description: "Wait for translation to complete"
    required: false
    default: "true"
  timeout:
    description: "Maximum wait time (e.g., 60m)"
    required: false
    default: "60m"
  download:
    description: "Download derivatives after translation"
    required: false
    default: "false"
  output-dir:
    description: "Directory to download derivatives to"
    required: false
    default: "./derivatives"

outputs:
  status:
    description: "Translation status (success, failed, timeout)"
    value: ${{ steps.translate.outputs.status }}

runs:
  using: "composite"
  steps:
    - name: Start translation
      shell: bash
      run: raps translate start "${{ inputs.urn }}"

    - name: Wait for translation
      id: translate
      if: inputs.wait == 'true'
      shell: bash
      run: |
        DEADLINE=$((SECONDS + $(echo "${{ inputs.timeout }}" | sed 's/m/*60/;s/h/*3600/;s/s//' | bc)))
        STATUS="inprogress"
        while [ "$STATUS" = "inprogress" ] && [ $SECONDS -lt $DEADLINE ]; do
          sleep 15
          OUTPUT=$(raps translate status "${{ inputs.urn }}" --output json 2>/dev/null || true)
          STATUS=$(echo "$OUTPUT" | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "inprogress")
          echo "Translation status: $STATUS"
        done

        if [ "$STATUS" = "success" ]; then
          echo "status=success" >> "$GITHUB_OUTPUT"
        elif [ $SECONDS -ge $DEADLINE ]; then
          echo "status=timeout" >> "$GITHUB_OUTPUT"
          echo "::warning::Translation timed out after ${{ inputs.timeout }}"
        else
          echo "status=failed" >> "$GITHUB_OUTPUT"
          echo "::error::Translation failed with status: $STATUS"
          exit 1
        fi

    - name: Download derivatives
      if: inputs.download == 'true' && steps.translate.outputs.status == 'success'
      shell: bash
      run: |
        mkdir -p "${{ inputs.output-dir }}"
        raps translate download "${{ inputs.urn }}" --output "${{ inputs.output-dir }}"
        echo "Derivatives downloaded to ${{ inputs.output-dir }}"
```

**Step 2: Commit**

```bash
git add translate/
git commit -m "feat: add translate action for Model Derivative API"
```

---

### Task 5: Test Workflow

**Step 1: Create a test workflow in the raps-actions repo**

Create file: `.github/workflows/test.yml`

```yaml
name: Test Actions

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test-setup:
    name: Test Setup Action
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: ./setup
        with:
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}

      - name: Verify RAPS is installed
        run: |
          raps --version
          raps auth status --output json

  test-pipeline-dry-run:
    name: Test Pipeline Dry Run
    runs-on: ubuntu-latest
    needs: test-setup
    steps:
      - uses: actions/checkout@v4

      - uses: ./setup
        with:
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}

      - name: Create test pipeline
        run: raps pipeline sample --out-file test-pipeline.yaml

      - uses: ./pipeline
        with:
          file: test-pipeline.yaml
          dry-run: "true"
```

**Step 2: Commit and push**

```bash
git add .github/
git commit -m "ci: add test workflow for actions"
git push origin main
```

---

### Task 6: Tag v1 Release

**Step 1: Create initial release**

```bash
git tag -a v1.0.0 -m "Initial release: setup, pipeline, upload, translate actions"
git push origin v1.0.0
```

**Step 2: Create floating major version tag**

```bash
git tag -f v1 v1.0.0
git push -f origin v1
```

This allows users to reference `@v1` and get the latest v1.x.x.

**Step 3: Create GitHub release**

```bash
gh release create v1.0.0 --title "v1.0.0 — Initial Release" --notes "## RAPS GitHub Actions

Initial release with 4 composite actions:

- **setup** — Install RAPS CLI and configure APS authentication
- **upload** — Upload files to Autodesk OSS buckets
- **translate** — Translate models via Model Derivative API
- **pipeline** — Run RAPS pipeline files

### Usage

\`\`\`yaml
- uses: dmytro-yemelianov/raps-actions/setup@v1
  with:
    client-id: \${{ secrets.APS_CLIENT_ID }}
    client-secret: \${{ secrets.APS_CLIENT_SECRET }}
\`\`\`

See [README](https://github.com/dmytro-yemelianov/raps-actions) for full documentation."
```
