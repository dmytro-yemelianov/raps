---
name: raps-ci-cd-templates
version: "1.0"
description: Use when adding or updating CI/CD integrations in the raps-actions repo — GitHub Actions composite actions, GitLab CI include templates, test workflows, and floating tag management.
---

# RAPS CI/CD Templates

Manage GitHub Actions and GitLab CI templates in the raps-actions repo.

**Repo:** `/root/github/raps/raps-actions`

## Repository Structure

```
raps-actions/
├── setup/action.yml          # GitHub Action: install + auth
├── pipeline/action.yml       # GitHub Action: run pipeline
├── upload/action.yml         # GitHub Action: upload to OSS
├── translate/action.yml      # GitHub Action: translate model
├── gitlab/
│   ├── setup.yml             # GitLab: .raps-setup hidden job
│   ├── pipeline.yml          # GitLab: .raps-pipeline
│   ├── upload.yml            # GitLab: .raps-upload
│   ├── translate.yml         # GitLab: .raps-translate
│   └── example.gitlab-ci.yml # GitLab: complete example
├── .github/workflows/
│   └── test.yml              # Tests setup + pipeline dry-run
└── README.md
```

## GitHub Action Pattern (action.yml)

```yaml
name: "RAPS Action Name"
description: "What it does"
branding:
  icon: "terminal"
  color: "orange"

inputs:
  param:
    description: "Description"
    required: true
    default: "value"

outputs:
  result:
    description: "Description"
    value: ${{ steps.step-id.outputs.result }}

runs:
  using: "composite"
  steps:
    - name: Step name
      shell: bash
      env:
        VAR: ${{ inputs.param }}
      run: |
        # Logic here
        echo "result=value" >> "$GITHUB_OUTPUT"
```

Key: All actions use `shell: bash`. Setup action installs via `install.sh` (not npm).

## GitLab CI Template Pattern

```yaml
.raps-action-name:
  extends: .raps-setup
  script:
    - raps command here
    - |
      if [ "${RAPS_OPTION:-default}" = "value" ]; then
        raps another-command
      fi
```

Key: Hidden jobs (`.raps-*`) with `extends: .raps-setup`. Variables via CI/CD settings, not `with:` inputs.

## Floating v1 Tag

After pushing changes, update the floating tag so users on `@v1` get the latest:

```bash
git tag -f v1
git push origin v1 --force
```

GitHub Actions users reference `@v1`. GitLab users reference the raw URL with `v1` in the path.

## Testing

```bash
# Trigger test workflow manually
gh workflow run test.yml

# Watch results
gh run list --workflow=test.yml --limit 1
gh run view <run-id>
```

Test workflow validates: setup action installs RAPS, pipeline dry-run succeeds.

## Adding a New Action

1. Create `<name>/action.yml` following the composite pattern
2. Create `gitlab/<name>.yml` with matching hidden job
3. Update `gitlab/example.gitlab-ci.yml`
4. Update `README.md` with both GitHub and GitLab docs
5. Update `.github/workflows/test.yml` to cover new action
6. Commit, push, and update v1 tag

## Common Mistakes

- Using `npm install` instead of `install.sh` for setup (npm platform binaries may lag)
- Forgetting `shell: bash` on composite action steps
- Not updating the floating v1 tag after pushing
- Using `jq` in actions (not available by default — use `grep`/`cut` instead)
