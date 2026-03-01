#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Folder Permissions Audit

Audit who has access to what folders in a project.

Usage:
  $(basename "$0") --project <id> [options]

Options:
  --project <id>   Project ID (required)
  --email <user>   Filter results to a specific user
  --output <file>  Save report to file (JSON)
  --help           Show this help

Examples:
  $(basename "$0") --project abc123
  $(basename "$0") --project abc123 --email jane@company.com
  $(basename "$0") --project abc123 --output audit-report.json"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

PROJECT_ID=""
FILTER_EMAIL=""
OUTPUT_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --project) PROJECT_ID="$2"; shift 2 ;;
        --email)   FILTER_EMAIL="$2"; shift 2 ;;
        --output)  OUTPUT_FILE="$2"; shift 2 ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$PROJECT_ID" ]]; then
    error "Missing required option: --project"
    echo
    echo "$USAGE"
    exit 2
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_3leg

step "Listing folders for project: $PROJECT_ID"
FOLDERS=$(raps folder list --project "$PROJECT_ID" --output json --quiet)

FOLDER_COUNT=$(echo "$FOLDERS" | jq 'length')
info "Found $FOLDER_COUNT folders."

REPORT="[]"

step "Checking permissions on each folder..."
echo "$FOLDERS" | jq -r '.[] | "\(.id)\t\(.name // .displayName // "unnamed")"' | while IFS=$'\t' read -r FOLDER_ID FOLDER_NAME; do
    [[ -z "$FOLDER_ID" ]] && continue
    dim "  Checking: $FOLDER_NAME ($FOLDER_ID)"

    RIGHTS=$(raps folder rights --project "$PROJECT_ID" --folder "$FOLDER_ID" --output json --quiet 2>/dev/null || echo "[]")

    if [[ -n "$FILTER_EMAIL" ]]; then
        FILTERED=$(echo "$RIGHTS" | jq --arg email "$FILTER_EMAIL" '[.[] | select(.email == $email or .autodeskId == $email)]')
        if [[ $(echo "$FILTERED" | jq 'length') -gt 0 ]]; then
            echo "  $FOLDER_NAME:"
            echo "$FILTERED" | jq -r '.[] | "    \(.email // .autodeskId // "unknown") — \(.role // .permission // "unknown")"'
        fi
    else
        USER_COUNT=$(echo "$RIGHTS" | jq 'length')
        echo "  $FOLDER_NAME: $USER_COUNT users"
        echo "$RIGHTS" | jq -r '.[] | "    \(.email // .autodeskId // "unknown") — \(.role // .permission // "unknown")"'
    fi
done

if [[ -n "$OUTPUT_FILE" ]]; then
    step "Saving full report to $OUTPUT_FILE..."
    # Re-collect for JSON output
    FULL_REPORT="[]"
    echo "$FOLDERS" | jq -r '.[] | .id' | while IFS= read -r FID; do
        [[ -z "$FID" ]] && continue
        FNAME=$(echo "$FOLDERS" | jq -r --arg id "$FID" '.[] | select(.id == $id) | .name // .displayName // "unnamed"')
        RIGHTS=$(raps folder rights --project "$PROJECT_ID" --folder "$FID" --output json --quiet 2>/dev/null || echo "[]")
        jq -n --arg folder "$FNAME" --arg id "$FID" --argjson rights "$RIGHTS" \
            '{folder: $folder, id: $id, rights: $rights}'
    done | jq -s '.' > "$OUTPUT_FILE"
    info "Report saved: $OUTPUT_FILE"
fi

echo
info "Audit complete."
