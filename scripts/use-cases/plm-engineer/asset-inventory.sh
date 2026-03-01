#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Asset Inventory — List, create, update, and export assets

Usage:
  $(basename "$0") <command> [options]

Commands:
  list                          List all assets
  create <description> <barcode> Create a new asset
  update <id>                   Update an asset (interactive)
  export                        Export all assets to JSON

Options:
  --project <id>   Project ID (required)
  --output <file>  Output file for export (default: assets-export.json)
  --help           Show this help

Examples:
  $(basename "$0") list --project abc123
  $(basename "$0") create 'HVAC Unit 3rd Floor' 'BC-HVAC-003' --project abc123
  $(basename "$0") export --project abc123 --output inventory.json"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
PROJECT_ID=""
OUTPUT_FILE="assets-export.json"
DESCRIPTION=""
BARCODE=""
ASSET_ID=""

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        list|export)  COMMAND="$1"; shift ;;
        create)       COMMAND="create"; shift ;;
        update)
            COMMAND="update"
            ASSET_ID="${2:-}"
            [[ -n "$ASSET_ID" && "$ASSET_ID" != -* ]] && shift 2 || shift
            ;;
        --project) PROJECT_ID="$2"; shift 2 ;;
        --output)  OUTPUT_FILE="$2"; shift 2 ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         POSITIONAL+=("$1"); shift ;;
    esac
done

# Handle positional args for create
if [[ "$COMMAND" == "create" ]]; then
    DESCRIPTION="${POSITIONAL[0]:-}"
    BARCODE="${POSITIONAL[1]:-}"
fi

if [[ -z "$COMMAND" ]]; then
    error "Missing command"; echo; echo "$USAGE"; exit 2
fi
if [[ -z "$PROJECT_ID" ]]; then
    error "Missing required option: --project"; exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

check_3leg

case "$COMMAND" in
    list)
        step "Assets for project: $PROJECT_ID"
        ASSETS=$(raps acc asset list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$ASSETS" | jq 'length')
        info "Found $TOTAL assets."
        echo
        echo "$ASSETS" | jq -r '.[] | "  [\(.id)] \(.description // .name // "unnamed") — \(.barcode // .serialNumber // "no barcode")"'
        ;;

    create)
        if [[ -z "$DESCRIPTION" ]]; then
            error "Usage: $(basename "$0") create <description> <barcode> --project <id>"
            exit 2
        fi

        step "Creating asset: $DESCRIPTION"
        raps acc asset create --project "$PROJECT_ID" --description "$DESCRIPTION" --quiet
        info "Asset created."
        ;;

    update)
        if [[ -z "$ASSET_ID" ]]; then
            error "Usage: $(basename "$0") update <asset-id> --project <id>"
            exit 2
        fi

        step "Current asset details:"
        raps acc asset get --project "$PROJECT_ID" --asset "$ASSET_ID" --output json --quiet | jq .

        echo -n "New description (or Enter to skip): "
        read -r NEW_DESC

        if [[ -n "$NEW_DESC" ]]; then
            raps acc asset update --project "$PROJECT_ID" --asset "$ASSET_ID" --description "$NEW_DESC" --quiet
            info "Asset updated."
        else
            info "No changes made."
        fi
        ;;

    export)
        step "Exporting assets to $OUTPUT_FILE..."
        ASSETS=$(raps acc asset list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$ASSETS" | jq 'length')
        jq -n \
            --arg project "$PROJECT_ID" \
            --arg date "$(date +%Y-%m-%d)" \
            --argjson count "$TOTAL" \
            --argjson assets "$ASSETS" \
            '{project: $project, exported: $date, count: $count, assets: $assets}' > "$OUTPUT_FILE"

        info "Exported $TOTAL assets to $OUTPUT_FILE"
        ;;
esac
