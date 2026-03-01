#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Issue Tracker — List, create, and manage ACC issues

Usage:
  $(basename "$0") <command> [options]

Commands:
  create-from-csv <file>  Create issues from CSV (columns: title,description,status)
  status-report           Show issue status summary for a project
  close-resolved          Transition resolved issues to closed

Options:
  --project <id>   Project ID (required)
  --hub <hub-id>   Hub ID (required for some operations)
  --dry-run        Preview actions without executing
  --help           Show this help

Examples:
  $(basename "$0") create-from-csv issues.csv --project abc123
  $(basename "$0") status-report --project abc123
  $(basename "$0") close-resolved --project abc123 --dry-run"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
CSV_FILE=""
PROJECT_ID=""
HUB_ID=""
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        create-from-csv)
            COMMAND="create-from-csv"
            CSV_FILE="${2:-}"
            [[ -n "$CSV_FILE" && "$CSV_FILE" != -* ]] && shift 2 || { shift; }
            ;;
        status-report)  COMMAND="status-report"; shift ;;
        close-resolved) COMMAND="close-resolved"; shift ;;
        --project)      PROJECT_ID="$2"; shift 2 ;;
        --hub)          HUB_ID="$2"; shift 2 ;;
        --dry-run)      DRY_RUN=true; shift ;;
        -*)             error "Unknown option: $1"; exit 2 ;;
        *)              error "Unknown command: $1"; echo; echo "$USAGE"; exit 2 ;;
    esac
done

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
    create-from-csv)
        if [[ -z "$CSV_FILE" || ! -f "$CSV_FILE" ]]; then
            error "CSV file required and must exist: $CSV_FILE"
            exit 2
        fi

        step "Creating issues from $CSV_FILE..."
        CREATED=0
        FAILED=0

        while IFS=',' read -r TITLE DESCRIPTION STATUS; do
            [[ "$TITLE" == "title" ]] && continue  # skip header
            [[ -z "$TITLE" ]] && continue

            if $DRY_RUN; then
                dim "  [DRY RUN] Would create: $TITLE"
            else
                dim "  Creating: $TITLE"
                if raps issue create --project "$PROJECT_ID" --title "$TITLE" --description "${DESCRIPTION:-}" --quiet 2>/dev/null; then
                    CREATED=$((CREATED + 1))
                else
                    warn "  Failed to create: $TITLE"
                    FAILED=$((FAILED + 1))
                fi
            fi
        done < "$CSV_FILE"

        if $DRY_RUN; then
            info "Dry run complete."
        else
            info "Created: $CREATED issues, Failed: $FAILED"
        fi
        ;;

    status-report)
        step "Issue status report for project: $PROJECT_ID"
        ISSUES=$(raps issue list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$ISSUES" | jq 'length')
        echo
        echo "Total issues: $TOTAL"
        echo
        echo "By status:"
        echo "$ISSUES" | jq -r 'group_by(.status // "unknown") | .[] | "  \(.[0].status // "unknown"): \(length)"'
        echo
        echo "By type:"
        echo "$ISSUES" | jq -r 'group_by(.issueType // .type // "unknown") | .[] | "  \(.[0].issueType // .[0].type // "unknown"): \(length)"'
        ;;

    close-resolved)
        step "Finding resolved issues to close..."
        ISSUES=$(raps issue list --project "$PROJECT_ID" --output json --quiet)

        RESOLVED=$(echo "$ISSUES" | jq -r '.[] | select(.status == "answered" or .status == "resolved") | .id')

        if [[ -z "$RESOLVED" ]]; then
            info "No resolved issues found to close."
            exit 0
        fi

        COUNT=$(echo "$RESOLVED" | wc -l)
        info "Found $COUNT resolved issues."

        if $DRY_RUN; then
            echo "$RESOLVED" | while IFS= read -r ID; do
                dim "  [DRY RUN] Would close: $ID"
            done
        else
            confirm "Close $COUNT resolved issues?"
            echo "$RESOLVED" | while IFS= read -r ID; do
                dim "  Closing: $ID"
                raps issue transition --project "$PROJECT_ID" --issue "$ID" --status closed --quiet 2>/dev/null || warn "  Failed to close: $ID"
            done
            info "Done."
        fi
        ;;
esac
