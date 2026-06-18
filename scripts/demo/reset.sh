#!/usr/bin/env bash
# reset.sh — Return to a clean state between demo takes.
#
# Deletes the demo bucket (and its objects) created by run.sh, plus the local
# state file. Safe to run repeatedly; missing resources are ignored.
#
# Usage:
#   scripts/demo/reset.sh [--bucket <name>]
#
# With no --bucket, reads the bucket from scripts/demo/.demo-state (written by
# run.sh). Falls back to the same host-derived name run.sh uses.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Show help before sourcing common.sh (which requires raps on PATH).
for arg in "$@"; do
    if [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
        grep '^#' "${BASH_SOURCE[0]}" | grep -v '^#!' | sed 's/^# \{0,1\}//'
        exit 0
    fi
done

# shellcheck source=/dev/null
source "$SCRIPT_DIR/../use-cases/common.sh"

STATE_FILE="$SCRIPT_DIR/.demo-state"
BUCKET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bucket) BUCKET="$2"; shift 2 ;;
        -h|--help)
            grep '^#' "${BASH_SOURCE[0]}" | grep -v '^#!' | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) error "Unknown option: $1"; exit 2 ;;
    esac
done

# Resolve bucket: --bucket > state file > host-derived default.
if [[ -z "$BUCKET" && -f "$STATE_FILE" ]]; then
    # shellcheck source=/dev/null
    source "$STATE_FILE"
fi
if [[ -z "${BUCKET:-}" ]]; then
    HOST_TAG="$(hostname 2>/dev/null | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9' | cut -c1-12)"
    [[ -z "$HOST_TAG" ]] && HOST_TAG="local"
    BUCKET="raps-demo-${HOST_TAG}"
fi

step "Resetting demo state (bucket: $BUCKET)"

# Deleting the bucket removes its objects too. --yes skips the prompt.
if raps bucket delete "$BUCKET" --yes 2>/dev/null; then
    info "Deleted bucket: $BUCKET"
else
    dim "  (bucket not found or already deleted — nothing to do)"
fi

rm -f "$STATE_FILE"
info "Clean. Ready for the next take."
