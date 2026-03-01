#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="AutoCAD Conversion — DWG to PDF/DXF conversion pipeline

Upload a DWG file, run a Design Automation workitem with AutoCAD engine.

Usage:
  $(basename "$0") <input.dwg> [options]

Options:
  --format <fmt>    Output format: pdf, dxf (default: pdf)
  --activity <name> DA activity name (default: AutoCADConvert)
  --engine <id>     AutoCAD engine version (default: Autodesk.AutoCAD+25)
  --bucket <name>   OSS bucket for staging (default: raps-da-<timestamp>)
  --out-dir <dir>   Output directory for results (default: ./da-output)
  --help            Show this help

Examples:
  $(basename "$0") drawing.dwg
  $(basename "$0") floorplan.dwg --format dxf
  $(basename "$0") site-plan.dwg --format pdf --out-dir ./pdfs"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

INPUT_FILE=""
FORMAT="pdf"
ACTIVITY="AutoCADConvert"
ENGINE="Autodesk.AutoCAD+25"
BUCKET=""
OUT_DIR="./da-output"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format)   FORMAT="$2"; shift 2 ;;
        --activity) ACTIVITY="$2"; shift 2 ;;
        --engine)   ENGINE="$2"; shift 2 ;;
        --bucket)   BUCKET="$2"; shift 2 ;;
        --out-dir)  OUT_DIR="$2"; shift 2 ;;
        -*)         error "Unknown option: $1"; exit 2 ;;
        *)
            if [[ -z "$INPUT_FILE" ]]; then
                INPUT_FILE="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$INPUT_FILE" ]]; then
    error "Missing required argument: <input.dwg>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -f "$INPUT_FILE" ]]; then
    error "File not found: $INPUT_FILE"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

if [[ -z "$BUCKET" ]]; then
    BUCKET="raps-da-$(date +%s)"
fi

BASENAME=$(basename "$INPUT_FILE")
OUTPUT_KEY="${BASENAME%.*}.${FORMAT}"

# Create bucket
step "Creating staging bucket: $BUCKET"
raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"

# Upload input
step "Uploading: $BASENAME"
raps object upload "$BUCKET" "$INPUT_FILE" --quiet

# Get signed URL
step "Getting signed URL..."
INPUT_URL=$(raps object signed-url "$BUCKET" "$BASENAME" --output json --quiet | jq -r '.signedUrl // .url // empty')

# Check activity
step "Checking DA activity: $ACTIVITY"
ACTIVITIES=$(raps da activities --output json --quiet 2>/dev/null || echo '[]')
HAS_ACTIVITY=$(echo "$ACTIVITIES" | jq --arg name "$ACTIVITY" '[.[] | select(.id // . | contains($name))] | length')

if [[ "$HAS_ACTIVITY" -eq 0 ]]; then
    warn "Activity '$ACTIVITY' not found."
    info "List available activities: raps da activities"
    info "Create one: raps da activity-create --engine $ENGINE"
fi

# Run workitem
step "Creating workitem..."
dim "  Activity: $ACTIVITY"
dim "  Input:    $BASENAME"
dim "  Output:   $OUTPUT_KEY ($FORMAT)"

WORKITEM_RESULT=$(raps da workitem-create \
    --activity "$ACTIVITY" \
    --output json --quiet 2>/dev/null || echo '{}')

WORKITEM_ID=$(echo "$WORKITEM_RESULT" | jq -r '.id // empty')

if [[ -n "$WORKITEM_ID" ]]; then
    info "Workitem created: $WORKITEM_ID"

    step "Waiting for completion..."
    for i in $(seq 1 60); do
        STATUS=$(raps da workitems --output json --quiet 2>/dev/null | jq -r --arg id "$WORKITEM_ID" '.[] | select(.id == $id) | .status // "unknown"')
        case "$STATUS" in
            success|completed)
                info "Conversion complete."
                break
                ;;
            failed|cancelled)
                error "Workitem $STATUS."
                exit 1
                ;;
            *)
                dim "  Status: $STATUS (attempt $i/60)..."
                sleep 10
                ;;
        esac
    done

    # Download
    mkdir -p "$OUT_DIR"
    step "Downloading result..."
    raps object download "$BUCKET" "$OUTPUT_KEY" --output-dir "$OUT_DIR" --quiet 2>/dev/null || warn "Could not download output automatically."
    info "Result: $OUT_DIR/$OUTPUT_KEY"
else
    warn "Workitem creation returned no ID. Check DA configuration."
fi
