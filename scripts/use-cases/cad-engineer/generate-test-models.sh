#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Generate Synthetic Test Models

Generate synthetic engineering files (IFC, OBJ, DXF, STL, STEP) for testing.

Usage:
  $(basename "$0") [options]

Options:
  --count <n>          Number of files to generate (default: 5)
  --complexity <level> Complexity: simple, medium, complex (default: medium)
  --formats <list>     Comma-separated formats (default: obj,dxf,stl,ifc,step)
  --out-dir <dir>      Output directory (default: ./test-models)
  --upload             Upload generated files to OSS bucket
  --bucket <name>      Bucket for upload (default: raps-test-<timestamp>)
  --help               Show this help

Examples:
  $(basename "$0") --count 3 --complexity simple
  $(basename "$0") --count 10 --upload --bucket my-test-bucket
  $(basename "$0") --formats obj,stl --count 2 --out-dir ./samples"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COUNT=5
COMPLEXITY="medium"
FORMATS="obj,dxf,stl,ifc,step"
OUT_DIR="./test-models"
UPLOAD=false
BUCKET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --count)      COUNT="$2"; shift 2 ;;
        --complexity) COMPLEXITY="$2"; shift 2 ;;
        --formats)    FORMATS="$2"; shift 2 ;;
        --out-dir)    OUT_DIR="$2"; shift 2 ;;
        --upload)     UPLOAD=true; shift ;;
        --bucket)     BUCKET="$2"; shift 2 ;;
        -*)           error "Unknown option: $1"; exit 2 ;;
        *)            error "Unexpected argument: $1"; exit 2 ;;
    esac
done

# ── Main flow ───────────────────────────────────────────────────────────────

mkdir -p "$OUT_DIR"

step "Generating $COUNT test files (complexity: $COMPLEXITY, formats: $FORMATS)..."
raps generate files \
    --count "$COUNT" \
    --complexity "$COMPLEXITY" \
    --formats "$FORMATS" \
    --output-dir "$OUT_DIR"

GENERATED=$(find "$OUT_DIR" -type f | wc -l)
info "Generated $GENERATED files in $OUT_DIR"

if $UPLOAD; then
    check_auth

    if [[ -z "$BUCKET" ]]; then
        BUCKET="raps-test-$(date +%s)"
    fi

    step "Creating bucket: $BUCKET"
    raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"

    step "Uploading $GENERATED files..."
    raps object upload-batch "$BUCKET" "$OUT_DIR" --quiet

    info "Uploaded to bucket: $BUCKET"

    # Print URNs
    step "Object URNs:"
    OBJECTS=$(raps object list "$BUCKET" --output json --quiet)
    echo "$OBJECTS" | jq -r '.[] | .objectKey // .key // empty' | while IFS= read -r KEY; do
        URN=$(echo -n "urn:adsk.objects:os.object:${BUCKET}/${KEY}" | base64 -w0 | tr '+/' '-_' | tr -d '=')
        echo "  $KEY → $URN"
    done
fi
