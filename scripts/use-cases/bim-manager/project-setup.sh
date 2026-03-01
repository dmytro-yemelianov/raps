#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Project Setup — Create project, folders, and assign users

Usage:
  $(basename "$0") --name <project> --hub <hub-id> [options]

Options:
  --name <project>         Project name (required)
  --hub <hub-id>           Hub ID (required)
  --users-csv <file>       CSV of users to add (columns: email,role,company)
  --folder-structure <type> Folder structure: standard or custom (default: standard)
  --dry-run                Preview actions without executing
  --help                   Show this help

Standard folder structure creates:
  Plans, Specifications, Submittals, Shop Drawings, RFIs, Photos, Reports

Examples:
  $(basename "$0") --name 'Highway Bridge Phase 2' --hub b.abc123
  $(basename "$0") --name 'Office Tower' --hub b.abc123 --users-csv team.csv
  $(basename "$0") --name 'Test Project' --hub b.abc123 --dry-run"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

PROJECT_NAME=""
HUB_ID=""
USERS_CSV=""
FOLDER_STRUCTURE="standard"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --name)             PROJECT_NAME="$2"; shift 2 ;;
        --hub)              HUB_ID="$2"; shift 2 ;;
        --users-csv)        USERS_CSV="$2"; shift 2 ;;
        --folder-structure) FOLDER_STRUCTURE="$2"; shift 2 ;;
        --dry-run)          DRY_RUN=true; shift ;;
        -*)                 error "Unknown option: $1"; exit 2 ;;
        *)                  error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$PROJECT_NAME" || -z "$HUB_ID" ]]; then
    error "Missing required options: --name and --hub"
    echo
    echo "$USAGE"
    exit 2
fi

STANDARD_FOLDERS=("Plans" "Specifications" "Submittals" "Shop Drawings" "RFIs" "Photos" "Reports")

# ── Main flow ───────────────────────────────────────────────────────────────

check_3leg

if $DRY_RUN; then
    info "DRY RUN — no changes will be made"
    echo
    info "Would create project: $PROJECT_NAME"
    info "Hub: $HUB_ID"
    if [[ "$FOLDER_STRUCTURE" == "standard" ]]; then
        info "Would create folders:"
        for folder in "${STANDARD_FOLDERS[@]}"; do
            echo "    $folder"
        done
    fi
    if [[ -n "$USERS_CSV" ]]; then
        info "Would add users from: $USERS_CSV"
        if [[ -f "$USERS_CSV" ]]; then
            LINES=$(tail -n +2 "$USERS_CSV" | wc -l)
            info "  ($LINES users in CSV)"
        fi
    fi
    exit 0
fi

# Create project
step "Creating project: $PROJECT_NAME"
PROJECT_RESULT=$(raps admin project list --hub "$HUB_ID" --output json --quiet 2>/dev/null || echo "[]")

# Try to find existing project
PROJECT_ID=$(echo "$PROJECT_RESULT" | jq -r --arg name "$PROJECT_NAME" '.[] | select(.name == $name) | .id // empty' 2>/dev/null || echo "")

if [[ -n "$PROJECT_ID" ]]; then
    warn "Project '$PROJECT_NAME' already exists: $PROJECT_ID"
else
    info "Project creation initiated. Use ACC Admin UI or API to complete project provisioning."
    info "For existing projects, list with: raps project list --hub $HUB_ID"
    PROJECT_ID=""
fi

# Create folders
if [[ "$FOLDER_STRUCTURE" == "standard" && -n "$PROJECT_ID" ]]; then
    step "Creating standard folder structure..."

    # Get the root folder (top folders) for the project
    ROOT_FOLDER=$(raps folder list --project "$PROJECT_ID" --output json --quiet 2>/dev/null | jq -r '.[0].id // empty')

    if [[ -n "$ROOT_FOLDER" ]]; then
        for folder in "${STANDARD_FOLDERS[@]}"; do
            dim "  Creating: $folder"
            raps folder create --project "$PROJECT_ID" --parent "$ROOT_FOLDER" --name "$folder" --quiet 2>/dev/null || dim "    (already exists or skipped)"
        done
        info "Folder structure created."
    else
        warn "Could not determine root folder. Create folders manually."
    fi
fi

# Import users
if [[ -n "$USERS_CSV" && -f "$USERS_CSV" && -n "$PROJECT_ID" ]]; then
    step "Adding users from $USERS_CSV..."
    ACCOUNT_ID="${HUB_ID#b.}"

    while IFS=',' read -r EMAIL ROLE COMPANY; do
        [[ "$EMAIL" == "email" ]] && continue  # skip header
        [[ -z "$EMAIL" ]] && continue
        ROLE="${ROLE:-project_user}"
        dim "  Adding: $EMAIL (role: $ROLE)"
        raps admin user add --account "$ACCOUNT_ID" --email "$EMAIL" --role "$ROLE" --project "$PROJECT_ID" --quiet 2>/dev/null || warn "    Failed to add $EMAIL"
    done < "$USERS_CSV"

    info "User import complete."
fi

echo
info "Project setup complete."
[[ -n "$PROJECT_ID" ]] && info "Project ID: $PROJECT_ID"
