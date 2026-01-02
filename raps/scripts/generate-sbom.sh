#!/bin/bash
# Script to generate SBOM (Software Bill of Materials) for RAPS CLI
# Uses cargo-audit or cargo-cyclonedx (if available)
# Usage: ./scripts/generate-sbom.sh [format]
# Format: cyclonedx (default) or spdx

set -e

FORMAT="${1:-cyclonedx}"
OUTPUT_DIR="sbom"
OUTPUT_FILE="${OUTPUT_DIR}/raps-sbom.json"

echo "Generating SBOM in ${FORMAT} format..."

# Create output directory
mkdir -p "${OUTPUT_DIR}"

# Check if cargo-cyclonedx is installed
if command -v cargo-cyclonedx &> /dev/null; then
    echo "Using cargo-cyclonedx..."
    cargo cyclonedx --format json --output "${OUTPUT_FILE}"
    echo "SBOM generated: ${OUTPUT_FILE}"
elif command -v cargo-audit &> /dev/null; then
    echo "Using cargo-audit (fallback)..."
    cargo audit --json > "${OUTPUT_FILE}" || true
    echo "Audit report generated: ${OUTPUT_FILE}"
    echo "Note: For full SBOM, install cargo-cyclonedx: cargo install cargo-cyclonedx"
else
    echo "Error: No SBOM tool found. Install one of:"
    echo "  cargo install cargo-cyclonedx  # Recommended for CycloneDX"
    echo "  cargo install cargo-audit     # For security audit"
    exit 1
fi

echo ""
echo "SBOM generation complete!"
echo "File: ${OUTPUT_FILE}"

