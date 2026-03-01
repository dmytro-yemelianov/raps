#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Point Cloud to ACC — Upload point cloud to ACC via OSS

Upload a point cloud file to OSS, then create an ACC item referencing it.

Usage:
  $(basename "$0") <pointcloud-file> <project-id> <folder-id> [options]

Options:
  --bucket <name>   OSS bucket for staging (default: raps-pointcloud-<timestamp>)
  --help            Show this help

Examples:
  $(basename "$0") scan.rcp b.abc123 urn:folder:def456
  $(basename "$0") site-survey.e57 b.abc123 urn:folder:def456 --bucket my-scans
  $(basename "$0") terrain.las b.abc123 urn:folder:def456"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

PC_FILE=""
PROJECT_ID=""
FOLDER_ID=""
BUCKET=""

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --bucket) BUCKET="$2"; shift 2 ;;
        -*)       error "Unknown option: $1"; exit 2 ;;
        *)        POSITIONAL+=("$1"); shift ;;
    esac
done

PC_FILE="${POSITIONAL[0]:-}"
PROJECT_ID="${POSITIONAL[1]:-}"
FOLDER_ID="${POSITIONAL[2]:-}"

if [[ -z "$PC_FILE" || -z "$PROJECT_ID" || -z "$FOLDER_ID" ]]; then
    error "Missing required arguments: <pointcloud-file> <project-id> <folder-id>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -f "$PC_FILE" ]]; then
    error "File not found: $PC_FILE"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

if [[ -z "$BUCKET" ]]; then
    BUCKET="raps-pointcloud-$(date +%s)"
fi

BASENAME=$(basename "$PC_FILE")
FILE_SIZE=$(stat --printf="%s" "$PC_FILE" 2>/dev/null || stat -f "%z" "$PC_FILE" 2>/dev/null || echo "unknown")

info "File:    $BASENAME ($FILE_SIZE bytes)"
info "Project: $PROJECT_ID"
info "Folder:  $FOLDER_ID"
echo

# Create bucket
step "Creating staging bucket: $BUCKET"
raps bucket create "$BUCKET" --quiet 2>/dev/null || dim "  (bucket already exists)"

# Upload point cloud file
step "Uploading point cloud: $BASENAME"
raps object upload "$BUCKET" "$PC_FILE" --quiet
info "Upload complete."

# Create ACC item from OSS object
step "Creating ACC item from OSS object..."
if raps item create-from-oss --project "$PROJECT_ID" --folder "$FOLDER_ID" --bucket "$BUCKET" --object "$BASENAME" --quiet 2>/dev/null; then
    info "ACC item created successfully."
else
    warn "Automatic item creation failed."
    info "Try manually:"
    info "  raps item create-from-oss --project $PROJECT_ID --folder $FOLDER_ID --bucket $BUCKET --object $BASENAME"
fi

echo
info "Point cloud uploaded and linked to ACC."
info "Bucket: $BUCKET"
info "Object: $BASENAME"
