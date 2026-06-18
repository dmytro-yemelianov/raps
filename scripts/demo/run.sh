#!/usr/bin/env bash
# run.sh — Executive demo: the "wow beat" for RAPS.
#
# Shows that 6 manual Autodesk APS API steps (OAuth, base64 URN encoding,
# chunked upload, polling, retries) collapse into a handful of clean `raps`
# commands ending in a live Autodesk Viewer URL.
#
#   1. raps auth test            (2-legged OAuth — no manual token juggling)
#   2. raps bucket create        (idempotent)
#   3. raps object upload        (auto-chunking + base64 URN, printed for you)
#   4. raps translate start --watch   (live progress, ends with a Viewer URL)
#
# Usage:
#   scripts/demo/run.sh [--file <path>] [--keep]
#
# Options:
#   --file <path>   Model to upload (default: scripts/demo/sample-cube.obj)
#   --keep          Skip teardown reminder note (bucket/object are left in place)
#
# Requirements: raps on PATH, APS_CLIENT_ID + APS_CLIENT_SECRET set (or a
# configured profile). Run scripts/demo/reset.sh between takes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Show help before sourcing common.sh (which requires raps on PATH).
for arg in "$@"; do
    if [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
        grep '^#' "${BASH_SOURCE[0]}" | grep -v '^#!' | sed 's/^# \{0,1\}//'
        exit 0
    fi
done

# Reuse the shared use-case helpers (info/step/error/check_auth, require raps+jq).
# shellcheck source=/dev/null
source "$SCRIPT_DIR/../use-cases/common.sh"

# ── Args ─────────────────────────────────────────────────────────────────────

FILE="$SCRIPT_DIR/sample-cube.obj"
KEEP=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --file) FILE="$2"; shift 2 ;;
        --keep) KEEP=true; shift ;;
        -h|--help)
            grep '^#' "${BASH_SOURCE[0]}" | grep -v '^#!' | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) error "Unknown option: $1"; exit 2 ;;
    esac
done

if [[ ! -f "$FILE" ]]; then
    error "Model file not found: $FILE"
    exit 1
fi

# Stable bucket name per machine so reset.sh can find it. Lowercase + dash only
# (OSS bucket key rules). Falls back to a fixed name if hostname is unusual.
HOST_TAG="$(hostname 2>/dev/null | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9' | cut -c1-12)"
[[ -z "$HOST_TAG" ]] && HOST_TAG="local"
BUCKET="raps-demo-${HOST_TAG}"
OBJECT_KEY="$(basename "$FILE")"
STATE_FILE="$SCRIPT_DIR/.demo-state"

echo
step "RAPS demo — 4 commands, one Viewer URL"
dim  "Bucket: $BUCKET   Model: $OBJECT_KEY"
echo

# ── 1. Auth (2-legged) ───────────────────────────────────────────────────────
step "[1/4] raps auth test"
check_auth

# ── 2. Bucket (idempotent) ───────────────────────────────────────────────────
step "[2/4] raps bucket create --key $BUCKET"
raps bucket create --key "$BUCKET" 2>/dev/null \
    || dim "  (bucket already exists — continuing)"

# ── 3. Upload (auto-chunking + base64 URN) ───────────────────────────────────
step "[3/4] raps object upload $BUCKET $FILE"
raps object upload "$BUCKET" "$FILE"

# Compute the same base64url URN raps prints, so we can drive translate + reset.
URN="$(printf 'urn:adsk.objects:os.object:%s/%s' "$BUCKET" "$OBJECT_KEY" \
        | base64 -w0 2>/dev/null | tr '+/' '-_' | tr -d '=')"

# Persist state for reset.sh
printf 'BUCKET=%s\nOBJECT_KEY=%s\nURN=%s\n' "$BUCKET" "$OBJECT_KEY" "$URN" > "$STATE_FILE"

# ── 4. Translate with live progress → Viewer URL ─────────────────────────────
step "[4/4] raps translate start <urn> --format svf2 --watch"
raps translate start "$URN" --format svf2 --watch

echo
info "Done. The Viewer URL above opens the translated model in the Autodesk Viewer."
if ! $KEEP; then
    dim "Reset before the next take:  scripts/demo/reset.sh"
fi
