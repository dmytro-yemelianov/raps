# GitLab CI Templates Design

**Goal:** Add reusable GitLab CI templates to the `raps-actions` repo, mirroring the 4 GitHub Actions (setup, pipeline, upload, translate).

**Architecture:** GitLab CI `include: remote` templates defining hidden jobs (`.raps-*`) that users `extends:` in their own jobs. Each template installs RAPS via the install script, authenticates via environment variables, and runs RAPS commands. Versioned via the existing `v1` git tag.

**Location:** `gitlab/` directory in `dmytro-yemelianov/raps-actions`

## Templates

### 1. `gitlab/setup.yml`
Hidden job `.raps-setup` with `before_script` that:
- Installs RAPS via install.sh (version controlled by `RAPS_VERSION`)
- Adds `~/.raps/bin` to PATH
- Verifies auth with `raps auth test`

Required CI/CD variables: `APS_CLIENT_ID`, `APS_CLIENT_SECRET`
Optional: `RAPS_VERSION` (default: latest)

### 2. `gitlab/pipeline.yml`
Hidden job `.raps-pipeline` extending `.raps-setup` that:
- Validates pipeline file
- Runs pipeline with optional dry-run and ignore-failure flags

Variables: `RAPS_PIPELINE_FILE` (required), `RAPS_DRY_RUN`, `RAPS_IGNORE_FAILURE`

### 3. `gitlab/upload.yml`
Hidden job `.raps-upload` extending `.raps-setup` that:
- Optionally creates bucket
- Uploads files via glob pattern
- Reports URN and count

Variables: `RAPS_BUCKET` (required), `RAPS_FILES` (required), `RAPS_CREATE_BUCKET`, `RAPS_BUCKET_POLICY`

### 4. `gitlab/translate.yml`
Hidden job `.raps-translate` extending `.raps-setup` that:
- Starts translation
- Polls for completion with timeout
- Optionally downloads derivatives

Variables: `RAPS_URN` (required), `RAPS_WAIT`, `RAPS_TIMEOUT`, `RAPS_DOWNLOAD`, `RAPS_OUTPUT_DIR`

### 5. `gitlab/example.gitlab-ci.yml`
Complete example showing all 4 templates in a real workflow.

## User Experience

```yaml
include:
  - remote: 'https://raw.githubusercontent.com/dmytro-yemelianov/raps-actions/v1/gitlab/setup.yml'

my-job:
  extends: .raps-setup
  script:
    - raps bucket list
```
