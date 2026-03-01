#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Revit Export — Batch export Revit files via Design Automation

Upload a Revit file, create/use a DA activity, run a workitem, and download the result.

Usage:
  $(basename "$0") <input.rvt> [options]

Options:
  --output-format <fmt>  Output format: pdf, dwg, ifc (default: pdf)
  --activity <name>      DA activity name (default: RevitExport)
  --engine <id>          Revit engine version (default: Autodesk.Revit+2025)
  --bucket <name>        OSS bucket for staging (default: raps-da-<timestamp>)
  --out-dir <dir>        Output directory for results (default: ./da-output)
  --help                 Show this help

Examples:
  $(basename "$0") building.rvt
  $(basename "$0") model.rvt --output-format dwg --out-dir ./exports
  $(basename "$0") project.rvt --activity MyCustomActivity"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

INPUT_FILE=""
OUTPUT_FORMAT="pdf"
ACTIVITY="RevitExport"
ENGINE="Autodesk.Revit+2025"
BUCKET=""
OUT_DIR="./da-output"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-format) OUTPUT_FORMAT="$2"; shift 2 ;;
        --activity)      ACTIVITY="$2"; shift 2 ;;
        --engine)        ENGINE="$2"; shift 2 ;;
        --bucket)        BUCKET="$2"; shift 2 ;;
        --out-dir)       OUT_DIR="$2"; shift 2 ;;
        -*)              error "Unknown option: $1"; exit 2 ;;
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
    error "Missing required argument: <input.rvt>"
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

# Create bucket
step "Creating staging bucket: $BUCKET"
raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"

# Upload input file
BASENAME=$(basename "$INPUT_FILE")
step "Uploading: $BASENAME"
raps object upload "$BUCKET" "$INPUT_FILE" --quiet

# Get signed URL for input
step "Getting signed URL for input..."
INPUT_URL=$(raps object signed-url "$BUCKET" "$BASENAME" --output json --quiet | jq -r '.signedUrl // .url // empty')
if [[ -z "$INPUT_URL" ]]; then
    error "Failed to get signed URL for input file."
    exit 1
fi

# Check if activity exists
step "Checking DA activity: $ACTIVITY"
ACTIVITIES=$(raps da activities --output json --quiet 2>/dev/null || echo '[]')
HAS_ACTIVITY=$(echo "$ACTIVITIES" | jq --arg name "$ACTIVITY" '[.[] | select(.id // . | contains($name))] | length')

if [[ "$HAS_ACTIVITY" -eq 0 ]]; then
    warn "Activity '$ACTIVITY' not found. You may need to create it first."
    info "List available activities: raps da activities"
    info "Create one: raps da activity-create --engine $ENGINE"
fi

# Create output signed URL
OUTPUT_KEY="output.${OUTPUT_FORMAT}"
OUTPUT_URL=$(raps object signed-url "$BUCKET" "$OUTPUT_KEY" --access write --output json --quiet 2>/dev/null | jq -r '.signedUrl // .url // empty' || echo "")

# Run workitem
step "Creating workitem..."
dim "  Activity: $ACTIVITY"
dim "  Input:    $BASENAME"
dim "  Output:   $OUTPUT_KEY"

WORKITEM_RESULT=$(raps da workitem-create \
    --activity "$ACTIVITY" \
    --output json --quiet 2>/dev/null || echo '{}')

WORKITEM_ID=$(echo "$WORKITEM_RESULT" | jq -r '.id // empty')

if [[ -n "$WORKITEM_ID" ]]; then
    info "Workitem created: $WORKITEM_ID"

    step "Waiting for workitem to complete..."
    # Poll status
    for i in $(seq 1 60); do
        STATUS=$(raps da workitems --output json --quiet 2>/dev/null | jq -r --arg id "$WORKITEM_ID" '.[] | select(.id == $id) | .status // "unknown"')
        case "$STATUS" in
            success|completed)
                info "Workitem completed successfully."
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

    # Download result
    mkdir -p "$OUT_DIR"
    step "Downloading result..."
    raps object download "$BUCKET" "$OUTPUT_KEY" --output-dir "$OUT_DIR" --quiet 2>/dev/null || warn "Could not download output automatically."

    info "Result saved to: $OUT_DIR"
else
    warn "Workitem creation returned no ID. Check DA configuration."
    info "List engines: raps da engines"
    info "List activities: raps da activities"
fi
