#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Batch Upload Model Files

Upload all CAD files from a directory to an OSS bucket, optionally translating each.

Usage:
  $(basename "$0") <directory> [options]

Options:
  --bucket <name>     Target bucket (default: raps-batch-<timestamp>)
  --parallel <n>      Concurrent uploads (default: 4)
  --translate         Translate each uploaded file to SVF2
  --extensions <list> Comma-separated extensions to include (default: rvt,ifc,dwg,stp,obj,stl,dxf,3dm,fbx)
  --help              Show this help

Examples:
  $(basename "$0") ./models --translate
  $(basename "$0") ./cad-files --bucket project-models --parallel 8
  $(basename "$0") ./exports --extensions rvt,ifc --translate"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

DIR=""
BUCKET=""
PARALLEL=4
TRANSLATE=false
EXTENSIONS="rvt,ifc,dwg,stp,obj,stl,dxf,3dm,fbx"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bucket)     BUCKET="$2"; shift 2 ;;
        --parallel)   PARALLEL="$2"; shift 2 ;;
        --translate)  TRANSLATE=true; shift ;;
        --extensions) EXTENSIONS="$2"; shift 2 ;;
        -*)           error "Unknown option: $1"; exit 2 ;;
        *)
            if [[ -z "$DIR" ]]; then
                DIR="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$DIR" ]]; then
    error "Missing required argument: <directory>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -d "$DIR" ]]; then
    error "Directory not found: $DIR"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

# Build find pattern from extensions
IFS=',' read -ra EXTS <<< "$EXTENSIONS"
FIND_ARGS=()
for i in "${!EXTS[@]}"; do
    if [[ $i -gt 0 ]]; then FIND_ARGS+=("-o"); fi
    FIND_ARGS+=("-iname" "*.${EXTS[$i]}")
done

FILE_COUNT=$(find "$DIR" -type f \( "${FIND_ARGS[@]}" \) | wc -l)
if [[ "$FILE_COUNT" -eq 0 ]]; then
    warn "No matching files found in $DIR (extensions: $EXTENSIONS)"
    exit 0
fi
info "Found $FILE_COUNT files to upload."

# Create bucket
if [[ -z "$BUCKET" ]]; then
    BUCKET="raps-batch-$(date +%s)"
fi
step "Creating bucket: $BUCKET"
raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"

# Batch upload
step "Uploading $FILE_COUNT files (concurrency: $PARALLEL)..."
raps object upload-batch "$BUCKET" "$DIR" --concurrency "$PARALLEL" --output json --quiet

info "Upload complete."

# Translate if requested
if $TRANSLATE; then
    step "Starting translations..."
    OBJECTS=$(raps object list "$BUCKET" --output json --quiet)
    OBJECT_KEYS=$(echo "$OBJECTS" | jq -r '.[].objectKey // .[].key // empty')

    TRANSLATED=0
    while IFS= read -r KEY; do
        [[ -z "$KEY" ]] && continue
        URN=$(echo -n "urn:adsk.objects:os.object:${BUCKET}/${KEY}" | base64 -w0 | tr '+/' '-_' | tr -d '=')
        dim "  Translating: $KEY"
        raps translate start "$URN" --output-format svf2 --quiet 2>/dev/null || warn "Failed to start translation for $KEY"
        TRANSLATED=$((TRANSLATED + 1))
    done <<< "$OBJECT_KEYS"

    info "Started $TRANSLATED translations."
    info "Check status: raps translate status <urn>"
fi

echo
info "Bucket: $BUCKET"
info "Files uploaded: $FILE_COUNT"
