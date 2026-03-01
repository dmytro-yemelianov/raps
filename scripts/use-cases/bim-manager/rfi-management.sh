#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="RFI Management — Create, list, and update RFIs

Usage:
  $(basename "$0") <command> [options]

Commands:
  create                  Create a single RFI interactively
  bulk-create <csv>       Create multiple RFIs from CSV (columns: title,description,priority)
  overdue-report          List overdue/open RFIs
  answer <rfi-id> <text>  Answer an RFI

Options:
  --project <id>   Project ID (required)
  --help           Show this help

Examples:
  $(basename "$0") create --project abc123
  $(basename "$0") bulk-create rfis.csv --project abc123
  $(basename "$0") overdue-report --project abc123
  $(basename "$0") answer RFI-001 'Use 4-inch pipe per spec 22 05 00' --project abc123"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
CSV_FILE=""
RFI_ID=""
ANSWER_TEXT=""
PROJECT_ID=""

# Parse positional args first
POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        create|bulk-create|overdue-report|answer)
            COMMAND="$1"; shift
            ;;
        --project) PROJECT_ID="$2"; shift 2 ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         POSITIONAL+=("$1"); shift ;;
    esac
done

# Handle positional args per command
case "$COMMAND" in
    bulk-create) CSV_FILE="${POSITIONAL[0]:-}" ;;
    answer)      RFI_ID="${POSITIONAL[0]:-}"; ANSWER_TEXT="${POSITIONAL[1]:-}" ;;
esac

if [[ -z "$COMMAND" ]]; then
    error "Missing command"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ -z "$PROJECT_ID" ]]; then
    error "Missing required option: --project"
    exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

check_3leg

case "$COMMAND" in
    create)
        step "Creating new RFI..."
        echo -n "Title: "; read -r TITLE
        echo -n "Description: "; read -r DESC
        echo -n "Priority (low/medium/high): "; read -r PRIORITY

        raps rfi create --project "$PROJECT_ID" --title "$TITLE" --description "$DESC" --quiet
        info "RFI created."
        ;;

    bulk-create)
        if [[ -z "$CSV_FILE" || ! -f "$CSV_FILE" ]]; then
            error "CSV file required: $CSV_FILE"
            exit 2
        fi

        step "Creating RFIs from $CSV_FILE..."
        CREATED=0
        while IFS=',' read -r TITLE DESC PRIORITY; do
            [[ "$TITLE" == "title" ]] && continue
            [[ -z "$TITLE" ]] && continue
            dim "  Creating: $TITLE"
            if raps rfi create --project "$PROJECT_ID" --title "$TITLE" --description "${DESC:-}" --quiet 2>/dev/null; then
                CREATED=$((CREATED + 1))
            else
                warn "  Failed: $TITLE"
            fi
        done < "$CSV_FILE"
        info "Created $CREATED RFIs."
        ;;

    overdue-report)
        step "RFI overdue report for project: $PROJECT_ID"
        RFIS=$(raps rfi list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$RFIS" | jq 'length')
        OPEN=$(echo "$RFIS" | jq '[.[] | select(.status == "open" or .status == "submitted")] | length')
        ANSWERED=$(echo "$RFIS" | jq '[.[] | select(.status == "answered")] | length')
        CLOSED=$(echo "$RFIS" | jq '[.[] | select(.status == "closed")] | length')

        echo
        echo "RFI Summary"
        echo "─────────────────────────"
        echo "  Total:    $TOTAL"
        echo "  Open:     $OPEN"
        echo "  Answered: $ANSWERED"
        echo "  Closed:   $CLOSED"
        echo

        if [[ "$OPEN" -gt 0 ]]; then
            echo "Open RFIs:"
            echo "$RFIS" | jq -r '.[] | select(.status == "open" or .status == "submitted") | "  [\(.id)] \(.title)"'
        fi
        ;;

    answer)
        if [[ -z "$RFI_ID" || -z "$ANSWER_TEXT" ]]; then
            error "Usage: $(basename "$0") answer <rfi-id> <answer-text> --project <id>"
            exit 2
        fi

        step "Answering RFI: $RFI_ID"
        raps rfi update --project "$PROJECT_ID" --rfi "$RFI_ID" --answer "$ANSWER_TEXT" --quiet
        info "RFI $RFI_ID answered."
        ;;
esac
