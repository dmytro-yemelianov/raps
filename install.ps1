<#
.SYNOPSIS
    RAPS Install Script for Windows
.DESCRIPTION
    Downloads and installs the RAPS CLI for Windows.
    https://rapscli.xyz
.PARAMETER Version
    Specific version to install (default: latest)
.PARAMETER InstallDir
    Installation directory (default: ~/.raps/bin)
.PARAMETER NoPathUpdate
    Skip PATH environment variable modification
.PARAMETER Uninstall
    Remove RAPS installation
.PARAMETER Help
    Show help message
.EXAMPLE
    irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex
.EXAMPLE
    .\install.ps1 -Version "3.10.0"
.EXAMPLE
    .\install.ps1 -InstallDir "C:\Tools\raps"
.EXAMPLE
    .\install.ps1 -Uninstall
#>

[CmdletBinding()]
param(
    [string]$Version = $env:RAPS_VERSION,
    [string]$InstallDir = $env:RAPS_INSTALL_DIR,
    [switch]$NoPathUpdate,
    [switch]$Uninstall,
    [switch]$Help
)

# Strict mode for better error handling
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# GitHub repository
$Repo = "dmytro-yemelianov/raps"
$GitHubApi = "https://api.github.com/repos/$Repo"
$GitHubReleases = "https://github.com/$Repo/releases"

# Default install directory
$DefaultInstallDir = Join-Path $env:USERPROFILE ".raps\bin"
if (-not $InstallDir) {
    $InstallDir = $DefaultInstallDir
}

# Default version
if (-not $Version) {
    $Version = "latest"
}

function Write-Banner {
    $Cyan = "`e[36m"
    $Reset = "`e[0m"

    Write-Host "${Cyan}"
    Write-Host "     ____  ___    ____  _____"
    Write-Host "    / __ \/ _ |  / __ \/ ___/"
    Write-Host "   / /_/ / __ | / /_/ (__  ) "
    Write-Host "  / _, _/ /_/ |/ ____/____/  "
    Write-Host " /_/ |_/_/ |_/_/             "
    Write-Host "${Reset}"
}

function Write-Info {
    param([string]$Message)
    Write-Host "`e[34m→`e[0m $Message"
}

function Write-Success {
    param([string]$Message)
    Write-Host "`e[32m✓`e[0m $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Host "`e[33m!`e[0m $Message"
}

function Write-Error {
    param([string]$Message)
    Write-Host "`e[31m✗`e[0m $Message" -ForegroundColor Red
}

function Show-Help {
    Write-Host "RAPS Install Script for Windows"
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex"
    Write-Host "  .\install.ps1 [OPTIONS]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Version <string>    Specific version to install (default: latest)"
    Write-Host "  -InstallDir <path>   Installation directory (default: ~/.raps/bin)"
    Write-Host "  -NoPathUpdate        Skip PATH modification"
    Write-Host "  -Uninstall           Remove RAPS installation"
    Write-Host "  -Help                Show this help message"
    Write-Host ""
    Write-Host "Environment Variables:"
    Write-Host "  RAPS_VERSION         Specific version to install"
    Write-Host "  RAPS_INSTALL_DIR     Installation directory"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  # Install latest version"
    Write-Host "  irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex"
    Write-Host ""
    Write-Host "  # Install specific version"
    Write-Host "  `$env:RAPS_VERSION = '3.10.0'; irm ... | iex"
    Write-Host ""
    Write-Host "  # Install with parameters"
    Write-Host "  .\install.ps1 -Version '3.10.0' -InstallDir 'C:\Tools\raps'"
    Write-Host ""
    Write-Host "For more information, visit: https://rapscli.xyz"
}

function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    switch ($arch) {
        "AMD64" { return "x64" }
        "ARM64" { return "arm64" }
        default {
            Write-Error "Unsupported architecture: $arch"
            Write-Error "Supported: AMD64, ARM64"
            exit 1
        }
    }
}

function Get-LatestVersion {
    $apiUrl = "$GitHubApi/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $apiUrl -UseBasicParsing
        return $response.tag_name -replace '^v', ''
    }
    catch {
        Write-Error "Failed to fetch latest version from GitHub"
        Write-Error $_.Exception.Message
        exit 1
    }
}

function Test-Checksum {
    param(
        [string]$FilePath,
        [string]$ExpectedChecksum
    )

    $actualChecksum = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLower()
    $expectedLower = $ExpectedChecksum.ToLower()

    if ($actualChecksum -eq $expectedLower) {
        return $true
    }
    else {
        Write-Error "Checksum verification failed"
        Write-Error "Expected: $expectedLower"
        Write-Error "Actual:   $actualChecksum"
        return $false
    }
}

function Update-UserPath {
    param([string]$Directory)

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    # Check if already in PATH
    if ($currentPath -split ';' | Where-Object { $_ -eq $Directory }) {
        Write-Success "PATH already contains $Directory"
        return
    }

    # Add to PATH
    $newPath = "$Directory;$currentPath"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Success "Added $Directory to User PATH"
}

function Remove-FromUserPath {
    param([string]$Directory)

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathParts = $currentPath -split ';' | Where-Object { $_ -ne $Directory -and $_ -ne "" }
    $newPath = $pathParts -join ';'

    if ($newPath -ne $currentPath) {
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Success "Removed $Directory from User PATH"
    }
}

function Invoke-Uninstall {
    Write-Banner
    Write-Info "Uninstalling RAPS..."

    $binaryPath = Join-Path $InstallDir "raps.exe"

    if (Test-Path $binaryPath) {
        Remove-Item -Path $binaryPath -Force
        Write-Success "Removed binary from $InstallDir"
    }
    else {
        Write-Warning "Binary not found at $binaryPath"
    }

    # Remove from PATH
    Remove-FromUserPath $InstallDir

    # Remove empty directories
    if (Test-Path $InstallDir) {
        $items = Get-ChildItem -Path $InstallDir -Force
        if ($items.Count -eq 0) {
            Remove-Item -Path $InstallDir -Force
            Write-Success "Removed empty directory $InstallDir"
        }
    }

    $rapsDir = Join-Path $env:USERPROFILE ".raps"
    if (Test-Path $rapsDir) {
        $items = Get-ChildItem -Path $rapsDir -Force
        if ($items.Count -eq 0) {
            Remove-Item -Path $rapsDir -Force
            Write-Success "Removed empty directory $rapsDir"
        }
    }

    Write-Host ""
    Write-Success "RAPS has been uninstalled."
}

function Invoke-Install {
    Write-Banner

    # Check PowerShell version
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Write-Error "PowerShell 5.1 or later required. Current: $($PSVersionTable.PSVersion)"
        exit 1
    }

    # Detect architecture
    $arch = Get-Architecture

    # Get version
    if ($Version -eq "latest") {
        Write-Info "Fetching latest version..."
        $Version = Get-LatestVersion
    }

    Write-Host "Installing RAPS `e[1mv$Version`e[0m for `e[1mwindows-$arch`e[0m..."
    Write-Host ""

    # Construct download URLs
    $archiveName = "raps-windows-$arch.zip"
    $downloadUrl = "$GitHubReleases/download/v$Version/$archiveName"
    $checksumsUrl = "$GitHubReleases/download/v$Version/raps-$Version-checksums.txt"

    # Create temp directory
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) "raps-install-$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        # Download binary archive
        Write-Info "Downloading $archiveName..."
        $archivePath = Join-Path $tempDir $archiveName
        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
            Write-Success "Downloaded"
        }
        catch {
            Write-Error "Failed to download RAPS. Check your internet connection."
            Write-Error "URL: $downloadUrl"
            Write-Error $_.Exception.Message
            exit 1
        }

        # Download and verify checksum
        Write-Info "Verifying checksum..."
        $checksumsPath = Join-Path $tempDir "checksums.txt"
        try {
            Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath -UseBasicParsing
            $checksums = Get-Content $checksumsPath
            $checksumLine = $checksums | Where-Object { $_ -match $archiveName }
            if ($checksumLine) {
                $expectedChecksum = ($checksumLine -split '\s+')[0]
                if (-not (Test-Checksum -FilePath $archivePath -ExpectedChecksum $expectedChecksum)) {
                    Write-Error "Checksum verification failed. The download may be corrupted."
                    exit 1
                }
                Write-Success "Checksum verified"
            }
            else {
                Write-Warning "Checksum not found in checksums.txt, skipping verification"
            }
        }
        catch {
            Write-Warning "Could not download checksums file, skipping verification"
        }

        # Create install directory
        Write-Info "Installing to $InstallDir..."
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        # Extract archive
        $extractPath = Join-Path $tempDir "extracted"
        Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force

        # Find and move binary
        $binaryPath = Get-ChildItem -Path $extractPath -Recurse -Filter "raps.exe" | Select-Object -First 1
        if (-not $binaryPath) {
            Write-Error "Could not find raps.exe in archive"
            exit 1
        }

        $destPath = Join-Path $InstallDir "raps.exe"
        Copy-Item -Path $binaryPath.FullName -Destination $destPath -Force
        Write-Success "Installed"

        # Update PATH
        if (-not $NoPathUpdate) {
            Write-Info "Updating User PATH..."
            Update-UserPath $InstallDir
        }
        else {
            Write-Warning "Skipping PATH modification (-NoPathUpdate specified)"
        }

        # Verify installation
        Write-Host ""
        Write-Info "Verifying installation..."
        $env:Path = "$InstallDir;$env:Path"
        try {
            $installedVersion = & $destPath --version 2>&1
            Write-Success "raps $installedVersion installed successfully!"
        }
        catch {
            Write-Error "Installation verification failed"
            Write-Error "Binary may be corrupted or incompatible with your system"
            exit 1
        }

        # Print success message
        Write-Host ""
        Write-Host "`e[32m`e[1mInstallation complete!`e[0m"
        Write-Host ""
        Write-Host "To get started, run:"
        Write-Host "  `e[36mraps --help`e[0m"
        Write-Host ""
        Write-Host "Note: You may need to restart your terminal for PATH changes to take effect."
    }
    finally {
        # Cleanup temp directory
        if (Test-Path $tempDir) {
            Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Main entry point
if ($Help) {
    Show-Help
    exit 0
}

if ($Uninstall) {
    Invoke-Uninstall
    exit 0
}

Invoke-Install
