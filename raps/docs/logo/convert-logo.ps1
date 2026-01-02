# PowerShell script to convert SVG logo to various formats
# Requires: Inkscape, ImageMagick, or Python with cairosvg

param(
    [string]$InputFile = "raps-logo.svg",
    [string]$OutputDir = "output",
    [int[]]$Sizes = @(256, 512, 1024, 2048)
)

$ErrorActionPreference = "Stop"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
    Write-Host "Created output directory: $OutputDir" -ForegroundColor Green
}

$inputPath = Join-Path $PSScriptRoot $InputFile
if (-not (Test-Path $inputPath)) {
    Write-Error "Input file not found: $inputPath"
    exit 1
}

# Check for available conversion tools
$hasInkscape = $false
$hasImageMagick = $false
$hasPython = $false

# Check for Inkscape
try {
    $inkscapeVersion = & inkscape --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        $hasInkscape = $true
        Write-Host "Found Inkscape: $inkscapeVersion" -ForegroundColor Green
    }
} catch {
    # Inkscape not found
}

# Check for ImageMagick
try {
    $magickVersion = & magick -version 2>&1 | Select-Object -First 1
    if ($LASTEXITCODE -eq 0) {
        $hasImageMagick = $true
        Write-Host "Found ImageMagick" -ForegroundColor Green
    }
} catch {
    # ImageMagick not found
}

# Check for Python
try {
    $pythonVersion = & python --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        $hasPython = $true
        Write-Host "Found Python: $pythonVersion" -ForegroundColor Green
    }
} catch {
    try {
        $pythonVersion = & python3 --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            $hasPython = $true
            Write-Host "Found Python3: $pythonVersion" -ForegroundColor Green
        }
    } catch {
        # Python not found
    }
}

if (-not ($hasInkscape -or $hasImageMagick -or $hasPython)) {
    Write-Error "No conversion tool found. Please install one of:"
    Write-Error "  - Inkscape: https://inkscape.org/release/"
    Write-Error "  - ImageMagick: https://imagemagick.org/script/download.php"
    Write-Error "  - Python with cairosvg: pip install cairosvg"
    exit 1
}

# Function to convert using Inkscape
function Convert-WithInkscape {
    param($InputFile, $OutputFile, $Size)
    $width = $Size
    $height = $Size  # Square aspect ratio
    
    # Calculate DPI for high quality (96 DPI base, scale up for larger sizes)
    # For high-res images, use higher DPI to ensure sharp rendering
    $dpi = if ($Size -ge 1024) { 300 } elseif ($Size -ge 512) { 200 } else { 150 }
    
    & inkscape --export-type=png --export-filename=$OutputFile --export-width=$width --export-height=$height --export-dpi=$dpi $InputFile
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  Created: $OutputFile ($width x $height @ $dpi DPI)" -ForegroundColor Cyan
        return $true
    }
    return $false
}

# Function to convert using ImageMagick
function Convert-WithImageMagick {
    param($InputFile, $OutputFile, $Size)
    $width = $Size
    $height = $Size  # Square aspect ratio
    
    # For high-res images, render at higher density first, then resize for better quality
    # Use Lanczos filter for best quality when resizing
    $density = if ($Size -ge 1024) { 300 } elseif ($Size -ge 512) { 200 } else { 150 }
    
    # Use 'magick' directly (IMv7) - input file first, then operations, then output
    # Use argument array for proper argument passing
    $magickArgs = @(
        $InputFile,
        "-density", $density,
        "-background", "none",
        "-filter", "Lanczos",
        "-resize", "${width}x${height}",
        "-quality", "100",
        $OutputFile
    )
    
    & magick $magickArgs
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  Created: $OutputFile ($width x $height @ $density DPI)" -ForegroundColor Cyan
        return $true
    }
    return $false
}

# Function to convert using Python cairosvg
function Convert-WithPython {
    param($InputFile, $OutputFile, $Size)
    $width = $Size
    $height = $Size  # Square aspect ratio
    
    # Check if cairosvg is installed
    $pythonCmd = if (Get-Command python -ErrorAction SilentlyContinue) { "python" } else { "python3" }
    $checkCairo = & $pythonCmd -c "import cairosvg" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "cairosvg not installed. Installing..."
        & $pythonCmd -m pip install cairosvg --quiet
    }
    
    # Calculate DPI for high quality (higher DPI for larger sizes)
    $dpi = if ($Size -ge 1024) { 300 } elseif ($Size -ge 512) { 200 } else { 150 }
    
    # Convert Windows paths to forward slashes to avoid escape sequence issues
    # Python handles forward slashes correctly on Windows
    $inputPath = $InputFile -replace '\\', '/'
    $outputPath = $OutputFile -replace '\\', '/'
    
    # Use raw strings (r'...') to prevent any remaining escape sequence issues
    $script = @"
import cairosvg
import os
input_path = r'$inputPath'
output_path = r'$outputPath'
cairosvg.svg2png(url=input_path, write_to=output_path, output_width=$width, output_height=$height, dpi=$dpi)
"@
    
    $script | & $pythonCmd
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  Created: $OutputFile ($width x $height @ $dpi DPI)" -ForegroundColor Cyan
        return $true
    }
    return $false
}

Write-Host "`nConverting logo to PNG formats..." -ForegroundColor Yellow

# Convert to PNG at different sizes
foreach ($size in $Sizes) {
    $outputFile = Join-Path $OutputDir "raps-logo-${size}.png"
    
    $success = $false
    if ($hasInkscape) {
        $success = Convert-WithInkscape -InputFile $inputPath -OutputFile $outputFile -Size $size
    } elseif ($hasImageMagick) {
        $success = Convert-WithImageMagick -InputFile $inputPath -OutputFile $outputFile -Size $size
    } elseif ($hasPython) {
        $success = Convert-WithPython -InputFile $inputPath -OutputFile $outputFile -Size $size
    }
    
    if (-not $success) {
        Write-Warning "Failed to create $outputFile"
    }
}

# Also create a high-resolution version for general use
$outputFile = Join-Path $OutputDir "raps-logo.png"
if ($hasInkscape) {
    Convert-WithInkscape -InputFile $inputPath -OutputFile $outputFile -Size 1024
} elseif ($hasImageMagick) {
    Convert-WithImageMagick -InputFile $inputPath -OutputFile $outputFile -Size 1024
} elseif ($hasPython) {
    Convert-WithPython -InputFile $inputPath -OutputFile $outputFile -Size 1024
}

Write-Host "`nConversion complete! Output files are in: $OutputDir" -ForegroundColor Green

