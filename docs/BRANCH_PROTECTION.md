# Branch Protection Setup

This repository uses branch protection to ensure code quality and maintain a clean git history. The `main` branch is protected and requires all changes to go through Pull Requests.

## Protection Rules

The `main` branch has the following protection rules enabled:

- ✅ **Require pull request reviews before merging**
  - At least 1 approval required
  - Dismiss stale reviews when new commits are pushed
- ✅ **Require status checks to pass before merging**
  - All CI checks must pass (check, test, fmt, clippy, docs, all-checks-pass)
  - Branches must be up to date before merging
- ✅ **Include administrators**
  - Even repository admins must follow these rules
- ✅ **Do not allow force pushes**
- ✅ **Do not allow branch deletion**

## Setting Up Branch Protection

### Option 1: Using GitHub CLI (Recommended)

1. Install GitHub CLI if you haven't already:
   ```bash
   # Windows (using winget)
   winget install GitHub.cli
   
   # Or download from: https://cli.github.com/
   ```

2. Authenticate with GitHub:
   ```bash
   gh auth login
   ```

3. Run the setup script:
   ```powershell
   cd scripts
   .\setup-branch-protection.ps1
   ```

### Option 2: Manual Setup via GitHub Web UI

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Branches**
3. Under "Branch protection rules", click **Add rule**
4. In "Branch name pattern", enter: `main`
5. Configure the following settings:
   - ✅ **Require a pull request before merging**
     - ✅ Require approvals: `1`
     - ✅ Dismiss stale pull request approvals when new commits are pushed
   - ✅ **Require status checks to pass before merging**
     - ✅ Require branches to be up to date before merging
     - Select the following required checks:
       - `check`
       - `test`
       - `fmt`
       - `clippy`
       - `docs`
       - `all-checks-pass`
   - ✅ **Include administrators**
   - ❌ **Do not allow force pushes**
   - ❌ **Do not allow deletions**

6. Click **Create** to save the rules

### Option 3: Using GitHub API

You can also set up branch protection using the GitHub API directly:

```bash
gh api repos/dmytro-yemelianov/raps/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["check","test","fmt","clippy","docs","all-checks-pass"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
  --field restrictions=null \
  --field required_linear_history=false \
  --field allow_force_pushes=false \
  --field allow_deletions=false
```

## Verifying Protection

After setting up branch protection, verify it's working:

1. Try to push directly to `main`:
   ```bash
   git checkout main
   git commit --allow-empty -m "Test direct push"
   git push origin main
   ```
   
   This should fail with a message about branch protection.

2. Create a test branch and PR:
   ```bash
   git checkout -b test/branch-protection
   git commit --allow-empty -m "Test PR workflow"
   git push origin test/branch-protection
   ```
   
   Then create a PR on GitHub. You should see that:
   - The PR cannot be merged until CI checks pass
   - The PR requires at least one approval
   - You cannot merge without satisfying these requirements

## Workflow for Contributors

1. **Create a feature branch** from `main`
2. **Make your changes** and commit them
3. **Push your branch** to GitHub
4. **Create a Pull Request**
5. **Wait for CI checks to pass**
6. **Get approval** (if required)
7. **Merge the PR** (squash merge recommended)

## Troubleshooting

### "Branch is out of date"

If your PR shows "This branch is out of date", you need to update it:

```bash
git checkout your-branch-name
git fetch origin
git rebase origin/main
# or
git merge origin/main
git push origin your-branch-name
```

### CI Checks Failing

If CI checks are failing:
1. Run checks locally:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-features -- -D warnings
   cargo test --all-features
   ```
2. Fix any issues
3. Commit and push your fixes

### Need to Override Protection (Emergency Only)

If you absolutely need to bypass protection (emergency hotfix), you can temporarily disable it:

1. Go to **Settings** → **Branches**
2. Edit the `main` branch protection rule
3. Temporarily disable protection
4. Make your emergency change
5. Re-enable protection immediately

**⚠️ Warning**: Only do this in true emergencies and re-enable protection immediately after.

