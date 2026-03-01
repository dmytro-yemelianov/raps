#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Project Lifecycle — Create, list, and archive projects

Usage:
  $(basename "$0") <command> [options]

Commands:
  create          Create a new project
  list-active     List active projects
  archive <id>    Archive a project
  status-report   Summary of all projects by status

Options:
  --account <id>   Account ID (required)
  --name <name>    Project name (for create)
  --type <type>    Project type (for create, default: acc)
  --dry-run        Preview without executing (for archive)
  --help           Show this help

Examples:
  $(basename "$0") create --account ACC123 --name 'Highway Expansion Phase 3'
  $(basename "$0") list-active --account ACC123
  $(basename "$0") archive PROJ456 --account ACC123 --dry-run
  $(basename "$0") status-report --account ACC123"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
ACCOUNT_ID=""
PROJECT_NAME=""
PROJECT_TYPE="acc"
PROJECT_ID=""
DRY_RUN=false

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        create|list-active|status-report)
            COMMAND="$1"; shift
            ;;
        archive)
            COMMAND="archive"
            PROJECT_ID="${2:-}"
            [[ -n "$PROJECT_ID" && "$PROJECT_ID" != -* ]] && shift 2 || shift
            ;;
        --account) ACCOUNT_ID="$2"; shift 2 ;;
        --name)    PROJECT_NAME="$2"; shift 2 ;;
        --type)    PROJECT_TYPE="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         POSITIONAL+=("$1"); shift ;;
    esac
done

if [[ -z "$COMMAND" ]]; then
    error "Missing command"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ -z "$ACCOUNT_ID" ]]; then
    error "Missing required option: --account"
    exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

check_auth

case "$COMMAND" in
    create)
        if [[ -z "$PROJECT_NAME" ]]; then
            error "Missing required option: --name"
            exit 2
        fi

        step "Creating project: $PROJECT_NAME"
        raps admin project list --account "$ACCOUNT_ID" --output json --quiet >/dev/null 2>&1

        info "To create a new ACC project, use the ACC Admin UI or template-based creation:"
        info "  raps template list --account $ACCOUNT_ID"
        info "  raps template create --account $ACCOUNT_ID --name '$PROJECT_NAME' --type $PROJECT_TYPE"
        ;;

    list-active)
        step "Active projects for account: $ACCOUNT_ID"
        PROJECTS=$(raps admin project list --account "$ACCOUNT_ID" --output json --quiet)

        ACTIVE=$(echo "$PROJECTS" | jq '[.[] | select(.status == "active")]')
        ACTIVE_COUNT=$(echo "$ACTIVE" | jq 'length')

        info "Found $ACTIVE_COUNT active projects."
        echo
        echo "$ACTIVE" | jq -r '.[] | "  [\(.id)] \(.name // "unnamed") — \(.status // "unknown")"'
        ;;

    archive)
        if [[ -z "$PROJECT_ID" ]]; then
            error "Missing project ID"
            exit 2
        fi

        if $DRY_RUN; then
            info "DRY RUN — would archive project: $PROJECT_ID"
            exit 0
        fi

        confirm "Archive project $PROJECT_ID? This cannot be easily undone."

        step "Archiving project: $PROJECT_ID"
        raps admin project list --account "$ACCOUNT_ID" --output json --quiet | \
            jq -r --arg id "$PROJECT_ID" '.[] | select(.id == $id) | "  Name: \(.name // "unnamed")\n  Status: \(.status // "unknown")"'

        warn "Project archival via API requires ACC Admin permissions."
        info "Use: raps api patch /construction/admin/v1/projects/$PROJECT_ID --body '{\"status\": \"archived\"}'"
        ;;

    status-report)
        step "Project status report for account: $ACCOUNT_ID"
        PROJECTS=$(raps admin project list --account "$ACCOUNT_ID" --output json --quiet)

        TOTAL=$(echo "$PROJECTS" | jq 'length')
        echo
        echo "Project Status Report"
        echo "═════════════════════════════"
        echo "  Account: $ACCOUNT_ID"
        echo "  Date:    $(date +%Y-%m-%d)"
        echo "  Total:   $TOTAL"
        echo
        echo "By status:"
        echo "$PROJECTS" | jq -r '
            group_by(.status // "unknown")
            | .[]
            | "  \(.[0].status // "unknown"): \(length)"
        '
        echo
        echo "Projects:"
        echo "$PROJECTS" | jq -r '.[] | "  [\(.status // "?")] \(.name // "unnamed") (\(.id))"'
        ;;
esac
