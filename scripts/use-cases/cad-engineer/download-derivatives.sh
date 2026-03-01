#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Download Derivatives from Translated Models

Download OBJ, STL, STEP, or other derivatives from a translated model.

Usage:
  $(basename "$0") <urn> [options]

Options:
  --format <fmt>   Desired format: obj, stl, step, svf2 (default: obj)
  --out-dir <dir>  Output directory (default: ./exports)
  --list           List available derivatives without downloading
  --help           Show this help

Examples:
  $(basename "$0") dXJuOmFk... --format obj --out-dir ./models
  $(basename "$0") dXJuOmFk... --list
  $(basename "$0") dXJuOmFk... --format stl"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

URN=""
FORMAT="obj"
OUT_DIR="./exports"
LIST_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format)  FORMAT="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --list)    LIST_ONLY=true; shift ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)
            if [[ -z "$URN" ]]; then
                URN="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$URN" ]]; then
    error "Missing required argument: <urn>"
    echo
    echo "$USAGE"
    exit 2
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

# Check manifest exists
step "Checking translation manifest..."
MANIFEST=$(raps translate manifest "$URN" --output json --quiet 2>/dev/null) || {
    error "No manifest found for URN. Has translation completed?"
    error "Check status: raps translate status $URN"
    exit 1
}

STATUS=$(echo "$MANIFEST" | jq -r '.status // "unknown"')
if [[ "$STATUS" != "success" && "$STATUS" != "complete" ]]; then
    warn "Translation status: $STATUS (may not have derivatives yet)"
fi

# List derivatives
step "Available derivatives:"
raps translate derivatives "$URN" --output json --quiet | jq -r '.[] | "  \(.outputType // .type) — \(.name // .urn // "unnamed")"'

if $LIST_ONLY; then
    exit 0
fi

# Download
mkdir -p "$OUT_DIR"
step "Downloading $FORMAT derivatives to $OUT_DIR..."
raps translate download "$URN" --output-dir "$OUT_DIR" --quiet || {
    error "Download failed. Check that derivatives exist for format: $FORMAT"
    exit 1
}

info "Download complete: $OUT_DIR"
