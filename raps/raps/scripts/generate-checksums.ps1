# Script to generate SHA256 checksums for release artifacts
# Usage: .\scripts\generate-checksums.ps1 <artifact-directory>

param(
    [Parameter(Mandatory=$true)]
    [string]$ArtifactDirectory
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ArtifactDirectory)) {
    Write-Error "Directory does not exist: $ArtifactDirectory"
    exit 1
}

$checksumFile = Join-Path $ArtifactDirectory "checksums.txt"

Write-Host "Generating SHA256 checksums for artifacts in: $ArtifactDirectory" -ForegroundColor Cyan

# Get all files in the directory (excluding checksums.txt itself)
$files = Get-ChildItem -Path $ArtifactDirectory -File | Where-Object { $_.Name -ne "checksums.txt" }

if ($files.Count -eq 0) {
    Write-Warning "No files found in $ArtifactDirectory"
    exit 0
}

# Generate checksums
$checksums = @()
foreach ($file in $files) {
    Write-Host "  Processing: $($file.Name)" -ForegroundColor Gray
    $hash = Get-FileHash -Path $file.FullName -Algorithm SHA256
    $checksums += "$($hash.Hash)  $($file.Name)"
}

# Write checksums file
$checksums | Out-File -FilePath $checksumFile -Encoding UTF8

Write-Host "`nChecksums written to: $checksumFile" -ForegroundColor Green
Write-Host "`nChecksums:" -ForegroundColor Cyan
$checksums | ForEach-Object { Write-Host $_ }

Write-Host "`nDone!" -ForegroundColor Green

