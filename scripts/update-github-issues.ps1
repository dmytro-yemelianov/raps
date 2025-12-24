# Script to update existing GitHub issues with labels and milestones
# Usage: .\scripts\update-github-issues.ps1

$ErrorActionPreference = "Stop"

# Read the JSON file
$jsonPath = Join-Path $PSScriptRoot "..\roadmap\roadmap-v0.4-v0.6.json"
$issues = Get-Content $jsonPath | ConvertFrom-Json

Write-Host "Found $($issues.Count) issues to update" -ForegroundColor Cyan

# Label mapping
$labelMap = @{
    "prio:high" = "prio-high"
    "prio:med" = "prio-med"
    "prio:low" = "prio-low"
    "type:feature" = "type-feature"
    "type:docs" = "type-docs"
    "type:chore" = "type-chore"
    "type:bug" = "type-bug"
    "epic:ci" = "epic-ci"
    "epic:auth" = "epic-auth"
    "epic:profiles" = "epic-profiles"
    "epic:reliability" = "epic-reliability"
    "epic:release" = "epic-release"
}

# Get all existing issues
Write-Host "`nFetching existing issues..." -ForegroundColor Yellow
$existingIssues = gh issue list --limit 100 --json number,title | ConvertFrom-Json

# Create title to issue number mapping
$titleToNumber = @{}
foreach ($issue in $existingIssues) {
    $titleToNumber[$issue.title] = $issue.number
}

$updated = 0
$skipped = 0

Write-Host "`nUpdating issues..." -ForegroundColor Yellow

foreach ($issue in $issues) {
    $title = $issue.title
    $labels = $issue.labels
    $milestone = $issue.milestone
    
    # Find issue number by title
    if (-not $titleToNumber.ContainsKey($title)) {
        Write-Host "  Skipping: $title (not found)" -ForegroundColor Yellow
        $skipped++
        continue
    }
    
    $issueNumber = $titleToNumber[$title]
    Write-Host "  Updating issue #$issueNumber : $title" -ForegroundColor Gray
    
    # Update labels
    if ($labels) {
        $githubLabels = @()
        foreach ($label in $labels) {
            $githubLabel = if ($labelMap.ContainsKey($label)) {
                $labelMap[$label]
            } else {
                $label -replace ":", "-"
            }
            $githubLabels += $githubLabel
        }
        
        if ($githubLabels.Count -gt 0) {
            $labelsArg = $githubLabels -join ","
            $result = gh issue edit $issueNumber --add-label $labelsArg 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Host "    ✓ Added labels: $labelsArg" -ForegroundColor Green
            } else {
                Write-Host "    ✗ Failed to add labels: $result" -ForegroundColor Red
            }
        }
    }
    
    # Update milestone
    if ($milestone) {
        $result = gh issue edit $issueNumber --milestone $milestone 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ Added milestone: $milestone" -ForegroundColor Green
        } else {
            Write-Host "    ✗ Failed to add milestone: $result" -ForegroundColor Red
        }
    }
    
    $updated++
}

Write-Host "`nSummary:" -ForegroundColor Cyan
Write-Host "  Updated: $updated" -ForegroundColor Green
Write-Host "  Skipped: $skipped" -ForegroundColor Yellow
Write-Host "`nDone!" -ForegroundColor Cyan

