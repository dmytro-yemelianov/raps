#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Upload and Translate a Model File

Usage:
  $(basename "$0") <file> [options]

Options:
  --format <fmt>    Output format: svf2 (default), obj, stl
  --bucket <name>   Target bucket (default: raps-models-<timestamp>)
  --wait            Wait for translation to complete
  --help            Show this help

Examples:
  $(basename "$0") building.rvt --wait
  $(basename "$0") model.ifc --format obj --bucket my-bucket --wait
  $(basename "$0") assembly.stp --format stl"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

FILE=""
FORMAT="svf2"
BUCKET=""
WAIT=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format) FORMAT="$2"; shift 2 ;;
        --bucket) BUCKET="$2"; shift 2 ;;
        --wait)   WAIT=true; shift ;;
        -*)       error "Unknown option: $1"; echo; echo "$USAGE"; exit 2 ;;
        *)
            if [[ -z "$FILE" ]]; then
                FILE="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$FILE" ]]; then
    error "Missing required argument: <file>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -f "$FILE" ]]; then
    error "File not found: $FILE"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

# Create bucket if needed
if [[ -z "$BUCKET" ]]; then
    BUCKET="raps-models-$(date +%s)"
    step "Creating bucket: $BUCKET"
    raps bucket create "$BUCKET" --quiet 2>/dev/null || true
else
    step "Using bucket: $BUCKET"
    raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"
fi

# Upload file
step "Uploading: $FILE"
UPLOAD_RESULT=$(raps object upload "$BUCKET" "$FILE" --output json --quiet)
OBJECT_KEY=$(extract_json "$UPLOAD_RESULT" '.objectKey // .objectId // .key')
info "Uploaded: $OBJECT_KEY"

# Build URN from bucket + object key
URN=$(echo -n "urn:adsk.objects:os.object:${BUCKET}/${OBJECT_KEY}" | base64 -w0 | tr '+/' '-_' | tr -d '=')

# Start translation
step "Starting translation (format: $FORMAT)..."
TRANSLATE_ARGS=(raps translate start "$URN" --output-format "$FORMAT" --output json --quiet)
if $WAIT; then
    TRANSLATE_ARGS+=(--wait)
fi
TRANSLATE_RESULT=$("${TRANSLATE_ARGS[@]}")

if $WAIT; then
    info "Translation complete."
    step "Manifest:"
    raps translate manifest "$URN" --output json --quiet | jq .
else
    info "Translation started."
    info "Check status: raps translate status $URN"
fi

echo
info "URN: $URN"
info "Bucket: $BUCKET"
info "Object: $OBJECT_KEY"
