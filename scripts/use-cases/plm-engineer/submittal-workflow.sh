#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Submittal Workflow — Create submittals from CSV and track status

Usage:
  $(basename "$0") <command> [options]

Commands:
  create-from-csv <file>  Create submittals from CSV (columns: title,description,spec_section)
  status-report           Show submittal status summary
  overdue                 List overdue submittals

Options:
  --project <id>   Project ID (required)
  --help           Show this help

Examples:
  $(basename "$0") create-from-csv submittals.csv --project abc123
  $(basename "$0") status-report --project abc123
  $(basename "$0") overdue --project abc123"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
CSV_FILE=""
PROJECT_ID=""

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        create-from-csv)
            COMMAND="create-from-csv"
            CSV_FILE="${2:-}"
            [[ -n "$CSV_FILE" && "$CSV_FILE" != -* ]] && shift 2 || shift
            ;;
        status-report) COMMAND="status-report"; shift ;;
        overdue)       COMMAND="overdue"; shift ;;
        --project)     PROJECT_ID="$2"; shift 2 ;;
        -*)            error "Unknown option: $1"; exit 2 ;;
        *)             POSITIONAL+=("$1"); shift ;;
    esac
done

if [[ -z "$COMMAND" ]]; then
    error "Missing command"; echo; echo "$USAGE"; exit 2
fi
if [[ -z "$PROJECT_ID" ]]; then
    error "Missing required option: --project"; exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

check_3leg

case "$COMMAND" in
    create-from-csv)
        if [[ -z "$CSV_FILE" || ! -f "$CSV_FILE" ]]; then
            error "CSV file required and must exist: $CSV_FILE"
            exit 2
        fi

        step "Creating submittals from $CSV_FILE..."
        CREATED=0
        while IFS=',' read -r TITLE DESC SPEC; do
            [[ "$TITLE" == "title" ]] && continue
            [[ -z "$TITLE" ]] && continue
            dim "  Creating: $TITLE"
            if raps acc submittal create --project "$PROJECT_ID" --title "$TITLE" --description "${DESC:-}" --quiet 2>/dev/null; then
                CREATED=$((CREATED + 1))
            else
                warn "  Failed: $TITLE"
            fi
        done < "$CSV_FILE"
        info "Created $CREATED submittals."
        ;;

    status-report)
        step "Submittal status report for project: $PROJECT_ID"
        SUBMITTALS=$(raps acc submittal list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$SUBMITTALS" | jq 'length')
        echo
        echo "Submittal Summary"
        echo "─────────────────────────"
        echo "  Total: $TOTAL"
        echo
        echo "By status:"
        echo "$SUBMITTALS" | jq -r '
            group_by(.status // "unknown")
            | .[]
            | "  \(.[0].status // "unknown"): \(length)"
        '
        ;;

    overdue)
        step "Overdue submittals for project: $PROJECT_ID"
        SUBMITTALS=$(raps acc submittal list --project "$PROJECT_ID" --output json --quiet)

        TODAY=$(date +%Y-%m-%d)
        OVERDUE=$(echo "$SUBMITTALS" | jq --arg today "$TODAY" '[
            .[] | select(
                (.status != "closed" and .status != "completed") and
                (.dueDate // "" | . != "" and . < $today)
            )
        ]')

        OVERDUE_COUNT=$(echo "$OVERDUE" | jq 'length')
        if [[ "$OVERDUE_COUNT" -eq 0 ]]; then
            info "No overdue submittals."
        else
            warn "Found $OVERDUE_COUNT overdue submittals:"
            echo "$OVERDUE" | jq -r '.[] | "  [\(.id)] \(.title // "unnamed") — due: \(.dueDate // "unknown")"'
        fi
        ;;
esac
