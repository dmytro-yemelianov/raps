# Setup Branch Protection for main branch
# This script configures branch protection rules using GitHub CLI
# 
# Prerequisites:
# - GitHub CLI (gh) must be installed: https://cli.github.com/
# - You must be authenticated: gh auth login
# - You must have admin access to the repository

param(
    [string]$Repository = "dmytro-yemelianov/raps",
    [string]$Branch = "main"
)

Write-Host "Setting up branch protection for branch: $Branch" -ForegroundColor Cyan
Write-Host "Repository: $Repository" -ForegroundColor Cyan
Write-Host ""

# Check if gh CLI is installed
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Host "Error: GitHub CLI (gh) is not installed." -ForegroundColor Red
    Write-Host "Install it from: https://cli.github.com/" -ForegroundColor Yellow
    exit 1
}

# Check if authenticated
$authStatus = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: Not authenticated with GitHub CLI." -ForegroundColor Red
    Write-Host "Run: gh auth login" -ForegroundColor Yellow
    exit 1
}

Write-Host "Configuring branch protection rules..." -ForegroundColor Green

# Set branch protection rules
# This requires the following checks to pass:
# - check
# - test
# - fmt
# - clippy
# - docs
# - all-checks-pass

$requiredChecks = @(
    "check",
    "test",
    "fmt",
    "clippy",
    "docs",
    "all-checks-pass"
)

$checksString = $requiredChecks -join ","

# Configure branch protection
gh api repos/$Repository/branches/$Branch/protection `
    --method PUT `
    --field required_status_checks='{"strict":true,"contexts":["' + $checksString + '"]}' `
    --field enforce_admins=true `
    --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true,"require_code_owner_reviews":false}' `
    --field restrictions=null `
    --field required_linear_history=false `
    --field allow_force_pushes=false `
    --field allow_deletions=false

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✓ Branch protection configured successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Protection rules:" -ForegroundColor Cyan
    Write-Host "  - Require pull request reviews before merging" -ForegroundColor White
    Write-Host "  - Require status checks to pass before merging" -ForegroundColor White
    Write-Host "  - Require branches to be up to date before merging" -ForegroundColor White
    Write-Host "  - Include administrators" -ForegroundColor White
    Write-Host "  - Do not allow force pushes" -ForegroundColor White
    Write-Host "  - Do not allow branch deletion" -ForegroundColor White
    Write-Host ""
    Write-Host "Required status checks:" -ForegroundColor Cyan
    foreach ($check in $requiredChecks) {
        Write-Host "  - $check" -ForegroundColor White
    }
} else {
    Write-Host ""
    Write-Host "Error: Failed to configure branch protection." -ForegroundColor Red
    Write-Host "Make sure you have admin access to the repository." -ForegroundColor Yellow
    exit 1
}

