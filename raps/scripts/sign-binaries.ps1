<# 
.SYNOPSIS
    Sign RAPS CLI binaries for Windows, macOS, and Linux

.DESCRIPTION
    This script handles code signing for RAPS CLI binaries:
    - Windows: Authenticode signing with certificate
    - macOS: Apple Developer ID signing and notarization
    - Linux: GPG signing

.PARAMETER Platform
    Target platform: windows, macos, linux, or all

.PARAMETER BinaryPath
    Path to the binary to sign

.PARAMETER Certificate
    Path to signing certificate (Windows) or certificate name (macOS)

.PARAMETER KeyPath
    Path to GPG key (Linux)

.EXAMPLE
    .\sign-binaries.ps1 -Platform windows -BinaryPath .\target\release\raps.exe

.NOTES
    Requires appropriate signing tools to be installed:
    - Windows: signtool.exe (Windows SDK)
    - macOS: codesign, notarytool (Xcode)
    - Linux: gpg
#>

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("windows", "macos", "linux", "all")]
    [string]$Platform,

    [Parameter(Mandatory=$true)]
    [string]$BinaryPath,

    [string]$Certificate = "",
    [string]$KeyPath = "",
    [string]$TimestampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"

function Sign-Windows {
    param([string]$Binary, [string]$Cert, [string]$Timestamp)
    
    Write-Host "Signing Windows binary: $Binary" -ForegroundColor Cyan
    
    if (-not $Cert) {
        Write-Error "Certificate path required for Windows signing"
        exit 1
    }

    # Find signtool
    $signtool = Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits" -Recurse -Filter "signtool.exe" | 
                Select-Object -First 1 -ExpandProperty FullName

    if (-not $signtool) {
        Write-Error "signtool.exe not found. Install Windows SDK."
        exit 1
    }

    # Sign the binary
    $signArgs = @(
        "sign",
        "/f", $Cert,
        "/fd", "SHA256",
        "/tr", $Timestamp,
        "/td", "SHA256",
        "/v",
        $Binary
    )

    Write-Host "Running: $signtool $($signArgs -join ' ')"
    & $signtool @signArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Signing failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    # Verify signature
    Write-Host "Verifying signature..." -ForegroundColor Cyan
    & $signtool verify /pa /v $Binary

    Write-Host "Windows binary signed successfully!" -ForegroundColor Green
}

function Sign-MacOS {
    param([string]$Binary, [string]$Identity)
    
    Write-Host "Signing macOS binary: $Binary" -ForegroundColor Cyan
    
    if (-not $Identity) {
        Write-Error "Developer ID identity required for macOS signing"
        exit 1
    }

    # Sign with codesign
    $signArgs = @(
        "--sign", $Identity,
        "--timestamp",
        "--options", "runtime",
        $Binary
    )

    Write-Host "Running: codesign $($signArgs -join ' ')"
    codesign @signArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Signing failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    # Verify signature
    Write-Host "Verifying signature..." -ForegroundColor Cyan
    codesign --verify --verbose=2 $Binary

    Write-Host "macOS binary signed successfully!" -ForegroundColor Green
    Write-Host "Note: Notarization may be required for distribution" -ForegroundColor Yellow
}

function Sign-Linux {
    param([string]$Binary, [string]$Key)
    
    Write-Host "Creating GPG signature for Linux binary: $Binary" -ForegroundColor Cyan
    
    $sigFile = "$Binary.sig"
    
    $gpgArgs = @(
        "--armor",
        "--detach-sign",
        "--output", $sigFile
    )

    if ($Key) {
        $gpgArgs += @("--local-user", $Key)
    }

    $gpgArgs += $Binary

    Write-Host "Running: gpg $($gpgArgs -join ' ')"
    gpg @gpgArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Signing failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    # Verify signature
    Write-Host "Verifying signature..." -ForegroundColor Cyan
    gpg --verify $sigFile $Binary

    Write-Host "Linux binary signed successfully!" -ForegroundColor Green
    Write-Host "Signature file: $sigFile"
}

# Main execution
Write-Host "RAPS CLI Binary Signing" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan

if (-not (Test-Path $BinaryPath)) {
    Write-Error "Binary not found: $BinaryPath"
    exit 1
}

switch ($Platform) {
    "windows" {
        Sign-Windows -Binary $BinaryPath -Cert $Certificate -Timestamp $TimestampServer
    }
    "macos" {
        Sign-MacOS -Binary $BinaryPath -Identity $Certificate
    }
    "linux" {
        Sign-Linux -Binary $BinaryPath -Key $KeyPath
    }
    "all" {
        Write-Host "Signing all platforms not supported in single run. Run separately for each platform."
    }
}

Write-Host "`nSigning complete!" -ForegroundColor Green

