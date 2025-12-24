# Script to create GitHub issues from roadmap JSON
# Usage: .\scripts\create-github-issues.ps1

$ErrorActionPreference = "Stop"

# Read the JSON file
$jsonPath = Join-Path $PSScriptRoot "..\roadmap\roadmap-v0.4-v0.6.json"
$issues = Get-Content $jsonPath | ConvertFrom-Json

Write-Host "Found $($issues.Count) issues to create" -ForegroundColor Cyan

# Check repository permissions
Write-Host "`nChecking repository permissions..." -ForegroundColor Yellow
$repoInfo = gh repo view dmytro-yemelianov/raps --json viewerPermission,hasIssuesEnabled | ConvertFrom-Json
$hasWriteAccess = $repoInfo.viewerPermission -in @("WRITE", "ADMIN", "MAINTAIN")

if (-not $hasWriteAccess) {
    Write-Host "  Warning: You have READ-only access. Labels cannot be created." -ForegroundColor Yellow
    Write-Host "  Issues will be created without labels (add them manually via GitHub UI)." -ForegroundColor Yellow
}

# Label mapping (for reference - GitHub labels can't contain colons, so we use dashes)
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

# Try to create labels if we have write access
if ($hasWriteAccess) {
    Write-Host "`nCreating labels..." -ForegroundColor Yellow
    foreach ($originalLabel in $labelMap.Keys) {
        $githubLabel = $labelMap[$originalLabel]
        $labelParts = $originalLabel -split ":"
        $labelName = $labelParts[0]
        $labelValue = $labelParts[1]
        
        # Determine color based on label type
        $color = switch ($labelName) {
            "prio" { switch ($labelValue) { "high" { "d73a4a" }; "med" { "fbca04" }; "low" { "0e8a16" } } }
            "type" { switch ($labelValue) { "feature" { "0e8a16" }; "docs" { "0052cc" }; "chore" { "7057ff" }; "bug" { "d73a4a" } } }
            "epic" { "5319e7" }
            default { "ededed" }
        }
        
        Write-Host "  Creating label: $githubLabel" -ForegroundColor Gray
        $result = & gh api repos/dmytro-yemelianov/raps/labels -X POST -f name="$githubLabel" -f color="$color" 2>&1
        if ($LASTEXITCODE -eq 0 -or $result -match "already exists") {
            Write-Host "    ✓ Created or already exists" -ForegroundColor Green
        } else {
            Write-Host "    ✗ Failed: $result" -ForegroundColor Red
        }
    }
} else {
    Write-Host "`nSkipping label creation (read-only access)" -ForegroundColor Yellow
}

# Create milestones only if we have write access
if ($hasWriteAccess) {
    Write-Host "`nCreating milestones..." -ForegroundColor Yellow
    $milestones = @(
        "v0.4 — CI/CD & Automation Ready",
        "v0.5 — Profiles, Auth, Reliability",
        "v0.6 — Supply-chain, UX polish, Open-source hygiene"
    )

    foreach ($milestone in $milestones) {
        Write-Host "  Creating milestone: $milestone" -ForegroundColor Gray
        $result = gh api repos/dmytro-yemelianov/raps/milestones -X POST -f title="$milestone" -f state=open 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ Created" -ForegroundColor Green
        } else {
            Write-Host "    ✗ Failed: $result" -ForegroundColor Red
        }
    }
} else {
    Write-Host "`nSkipping milestone creation (read-only access)" -ForegroundColor Yellow
    Write-Host "  Milestones will need to be created manually via GitHub UI" -ForegroundColor Yellow
}

# Create issues
Write-Host "`nCreating issues..." -ForegroundColor Yellow
$created = 0
$skipped = 0
$tempDir = Join-Path $env:TEMP "raps-issues-$(Get-Random)"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    foreach ($issue in $issues) {
        $title = $issue.title
        $body = $issue.body
        $labels = $issue.labels
        $milestone = $issue.milestone
        
        Write-Host "  Creating: $title" -ForegroundColor Gray
        
        # Write body to temp file
        $bodyFile = Join-Path $tempDir "body-$(Get-Random).md"
        $body | Out-File -FilePath $bodyFile -Encoding UTF8
        
        # Build gh issue create command arguments
        $args = @(
            "issue", "create",
            "--title", $title,
            "--body-file", $bodyFile
        )
        
        # Add labels only if we have write access and labels exist
        if ($hasWriteAccess -and $labels) {
            # Convert colon-separated labels to GitHub-compatible format
            foreach ($label in $labels) {
                $githubLabel = if ($labelMap.ContainsKey($label)) {
                    $labelMap[$label]
                } else {
                    $label -replace ":", "-"
                }
                # Check if label exists before adding
                $labelExists = gh label list --json name | ConvertFrom-Json | Where-Object { $_.name -eq $githubLabel }
                if ($labelExists) {
                    $args += "--label"
                    $args += $githubLabel
                } else {
                    Write-Host "    Warning: Label '$githubLabel' doesn't exist, skipping" -ForegroundColor Yellow
                }
            }
        } else {
            Write-Host "    Note: Labels will need to be added manually" -ForegroundColor Gray
        }
        
        # Add milestone only if we have write access and it exists
        if ($hasWriteAccess -and $milestone) {
            # Check if milestone exists
            $milestoneExists = gh api repos/dmytro-yemelianov/raps/milestones --jq ".[] | select(.title == `"$milestone`")" 2>&1
            if ($milestoneExists -and $LASTEXITCODE -eq 0) {
                $args += "--milestone"
                $args += $milestone
            } else {
                Write-Host "    Note: Milestone '$milestone' doesn't exist, skipping" -ForegroundColor Gray
            }
        } else {
            if ($milestone) {
                Write-Host "    Note: Milestone '$milestone' will need to be added manually" -ForegroundColor Gray
            }
        }
        
        try {
            $output = & gh $args 2>&1
            if ($LASTEXITCODE -eq 0) {
                $created++
                Write-Host "    ✓ Created" -ForegroundColor Green
            } else {
                Write-Host "    ✗ Failed: $output" -ForegroundColor Red
                $skipped++
            }
        }
        catch {
            Write-Host "    ✗ Exception: $_" -ForegroundColor Red
            $skipped++
        }
        finally {
            # Clean up temp file
            if (Test-Path $bodyFile) {
                Remove-Item $bodyFile -Force
            }
        }
    }
}
finally {
    # Clean up temp directory
    if (Test-Path $tempDir) {
        Remove-Item $tempDir -Recurse -Force
    }
}

Write-Host "`nSummary:" -ForegroundColor Cyan
Write-Host "  Created: $created" -ForegroundColor Green
Write-Host "  Skipped: $skipped" -ForegroundColor Yellow
Write-Host "`nDone!" -ForegroundColor Cyan

