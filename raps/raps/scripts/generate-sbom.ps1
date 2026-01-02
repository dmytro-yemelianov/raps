# Script to generate SBOM (Software Bill of Materials) for RAPS CLI
# Uses cargo-cyclonedx or cargo-audit (if available)
# Usage: .\scripts\generate-sbom.ps1 [format]
# Format: cyclonedx (default) or spdx

param(
    [Parameter(Mandatory=$false)]
    [string]$Format = "cyclonedx"
)

$ErrorActionPreference = "Stop"

$OutputDir = "sbom"
$OutputFile = Join-Path $OutputDir "raps-sbom.json"

Write-Host "Generating SBOM in $Format format..." -ForegroundColor Cyan

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Check if cargo-cyclonedx is installed
$cyclonedx = Get-Command cargo-cyclonedx -ErrorAction SilentlyContinue
$audit = Get-Command cargo-audit -ErrorAction SilentlyContinue

if ($cyclonedx) {
    Write-Host "Using cargo-cyclonedx..." -ForegroundColor Green
    cargo cyclonedx --format json --output $OutputFile
    Write-Host "SBOM generated: $OutputFile" -ForegroundColor Green
} elseif ($audit) {
    Write-Host "Using cargo-audit (fallback)..." -ForegroundColor Yellow
    cargo audit --json | Out-File -FilePath $OutputFile -Encoding UTF8
    Write-Host "Audit report generated: $OutputFile" -ForegroundColor Yellow
    Write-Host "Note: For full SBOM, install cargo-cyclonedx: cargo install cargo-cyclonedx" -ForegroundColor Yellow
} else {
    Write-Error "No SBOM tool found. Install one of:"
    Write-Host "  cargo install cargo-cyclonedx  # Recommended for CycloneDX" -ForegroundColor Yellow
    Write-Host "  cargo install cargo-audit     # For security audit" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "SBOM generation complete!" -ForegroundColor Green
Write-Host "File: $OutputFile"

