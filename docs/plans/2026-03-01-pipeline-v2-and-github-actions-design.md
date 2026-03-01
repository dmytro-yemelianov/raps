# Pipeline v2 & GitHub Actions Design

**Date:** 2026-03-01
**Status:** Approved
**Target Version:** 4.16.0+

## Context

RAPS is a comprehensive APS CLI used by developers, automation engineers, and AEC professionals. The current pipeline system (v1) supports sequential steps with basic variable substitution and continue-on-error. CI/CD users have no official GitHub Actions integration.

These two features form the foundation for future capabilities: RAPS daemon, config-as-code, webhook-driven pipelines, and Azure DevOps tasks.

No backward compatibility constraints — there are no existing pipeline users to break.

## Decision

- **Pipeline v2:** Incremental enhancement of the YAML format. Add error handling/retries, conditionals, parallel steps, and loops. Keep the format simple and familiar to non-DevOps users (architects, engineers).
- **GitHub Actions:** Composite actions in a `raps-actions` repository. Lightweight wrappers that install RAPS and run commands directly. No TypeScript build step.

## Pipeline v2

### Error Handling & Retries

```yaml
name: "Model Processing"

defaults:
  retry:
    max_attempts: 3
    backoff: exponential  # or fixed
    delay: 5s
  timeout: 5m

steps:
  - name: Upload model
    command: "object upload ${bucket} ${model}"
    retry:
      max_attempts: 5
      delay: 10s
      on: [network_error, timeout]
    timeout: 30m

  - name: Start translation
    command: "translate start urn:${bucket}/${model}"
    on_failure:
      - name: "Cleanup on failure"
        command: "object delete ${bucket} ${model}"

  - name: Wait for translation
    command: "translate status urn:${bucket}/${model} --wait"
    timeout: 60m
    retry:
      max_attempts: 2
      delay: 30s
```

- `retry` — per-step or in `defaults`. Fields: `max_attempts`, `backoff` (exponential/fixed), `delay`, optional `on` filter.
- `timeout` — per-step or default. Kills step after duration.
- `on_failure` — cleanup steps that run when a step fails.

### Conditionals & Step Outputs

```yaml
steps:
  - name: Check bucket exists
    id: check_bucket
    command: "bucket info ${bucket}"
    ignore_failure: true

  - name: Create bucket
    command: "bucket create --key ${bucket} --policy transient"
    if: "${{ steps.check_bucket.exit_code != 0 }}"

  - name: Upload model
    id: upload
    command: "object upload ${bucket} ${model}"

  - name: Start translation
    command: "translate start ${urn}"
    unless: "${{ steps.check_status.exit_code == 0 }}"
```

- `id` — names a step for referencing later.
- `if` / `unless` — expression evaluated against step context.
- `${{ steps.<id>.exit_code }}` — reference previous step results.
- Expressions support `==`, `!=`, `&&`, `||`, `!` operators.

### Parallel Steps & Loops

```yaml
steps:
  - name: Upload all models
    parallel:
      - command: "object upload ${bucket} building-a.rvt"
        id: upload_a
      - command: "object upload ${bucket} building-b.rvt"
        id: upload_b
      - command: "object upload ${bucket} site.dwg"
        id: upload_c
    max_concurrency: 3

  - name: Translate each model
    for_each:
      var: model
      in: ["building-a.rvt", "building-b.rvt", "site.dwg"]
    steps:
      - name: "Start translation"
        command: "translate start urn:${bucket}/${model}"
      - name: "Wait for completion"
        command: "translate status urn:${bucket}/${model} --wait"
        timeout: 60m

  - name: Download all results
    for_each:
      var: model
      in: ["building-a.rvt", "building-b.rvt", "site.dwg"]
      parallel: true
      max_concurrency: 5
    command: "translate download urn:${bucket}/${model} --output ./output/${model}"
```

- `parallel` — list of steps that run concurrently, with `max_concurrency`.
- `for_each` — iterate over a list with `var` binding. Supports nested `steps` or single `command`.
- `for_each.parallel: true` — run iterations concurrently.

### Breaking Changes from v1

- Drop `version` field — single format.
- Rename `continue_on_error` to `ignore_failure`.
- Replace `condition` field with `if`/`unless`.
- Pipeline file extension: `.raps.yaml`.

## GitHub Actions

A `dmytro-yemelianov/raps-actions` repository with 4 composite actions.

### `setup` — Install & Authenticate

```yaml
- uses: dmytro-yemelianov/raps-actions/setup@v1
  with:
    version: "latest"
    client-id: ${{ secrets.APS_CLIENT_ID }}
    client-secret: ${{ secrets.APS_CLIENT_SECRET }}
```

Internally: installs via npm, sets env vars, runs `raps auth test`, caches install.

### `upload` — Upload Files to OSS

```yaml
- uses: dmytro-yemelianov/raps-actions/upload@v1
  with:
    bucket: "my-bucket"
    files: "models/*.rvt"
    create-bucket: true
    bucket-policy: "transient"
```

### `translate` — Translate + Wait + Download

```yaml
- uses: dmytro-yemelianov/raps-actions/translate@v1
  with:
    urn: ${{ steps.upload.outputs.urn }}
    wait: true
    timeout: "60m"
    download: true
    output-dir: "./output"
```

### `pipeline` — Run Pipeline File

```yaml
- uses: dmytro-yemelianov/raps-actions/pipeline@v1
  with:
    file: ".raps/pipeline.yaml"
    variables: |
      bucket=ci-models-${{ github.run_id }}
      model=building.rvt
```

### Example: Full CI Workflow

```yaml
name: Model Processing CI
on:
  push:
    paths: ["models/**"]

jobs:
  process:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dmytro-yemelianov/raps-actions/setup@v1
        with:
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}

      - uses: dmytro-yemelianov/raps-actions/upload@v1
        id: upload
        with:
          bucket: "ci-models-${{ github.run_id }}"
          files: "models/*.rvt"
          create-bucket: true

      - uses: dmytro-yemelianov/raps-actions/translate@v1
        with:
          urn: ${{ steps.upload.outputs.urn }}
          wait: true
          download: true
          output-dir: "./derivatives"

      - uses: actions/upload-artifact@v4
        with:
          name: derivatives
          path: ./derivatives/
```

## Scope

### In Scope

**Pipeline v2:**

| Feature | Priority |
|---------|----------|
| retry with max_attempts, backoff, delay | P0 |
| timeout per-step and defaults | P0 |
| on_failure cleanup steps | P0 |
| id and step output references | P1 |
| if / unless conditionals | P1 |
| parallel step groups | P1 |
| for_each with var binding | P2 |
| for_each.parallel | P2 |
| defaults block | P2 |

**GitHub Actions:**

| Action | Priority |
|--------|----------|
| setup — install + auth | P0 |
| pipeline — run pipeline file | P1 |
| upload — upload with glob | P2 |
| translate — translate + wait + download | P2 |

### Out of Scope

- Webhook-triggered pipelines (future: daemon)
- Step output capture beyond exit codes
- Remote pipeline execution
- Pipeline composition (sub-pipelines)
- Approval gates
- Azure DevOps tasks (future release)
- Marketplace publishing
- 3-legged auth in CI
- OIDC token federation

## Testing

- **Pipeline v2:** Unit tests for expression parser, conditionals, retry logic. Integration tests with `raps-mock` for parallel/for_each.
- **GitHub Actions:** Test workflows in `raps-actions` repo using real GitHub Actions runs against a sandbox APS app.
