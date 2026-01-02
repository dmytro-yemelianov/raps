#!/bin/bash
# Script to generate SHA256 checksums for release artifacts
# Usage: ./scripts/generate-checksums.sh <artifact-directory>

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <artifact-directory>"
    exit 1
fi

ARTIFACT_DIR="$1"
CHECKSUM_FILE="${ARTIFACT_DIR}/checksums.txt"

if [ ! -d "$ARTIFACT_DIR" ]; then
    echo "Error: Directory does not exist: $ARTIFACT_DIR"
    exit 1
fi

echo "Generating SHA256 checksums for artifacts in: $ARTIFACT_DIR"

# Generate checksums for all files except checksums.txt
cd "$ARTIFACT_DIR"
find . -type f ! -name "checksums.txt" -exec sha256sum {} \; > "$CHECKSUM_FILE"

echo ""
echo "Checksums written to: $CHECKSUM_FILE"
echo ""
echo "Checksums:"
cat "$CHECKSUM_FILE"

echo ""
echo "Done!"

