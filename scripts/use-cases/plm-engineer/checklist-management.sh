#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Checklist Management — Create checklists from templates and assign

Usage:
  $(basename "$0") <command> [options]

Commands:
  templates                       List available checklist templates
  create-from-template <tmpl-id>  Create a checklist from a template
  assign <checklist-id> <email>   Assign a checklist to a user
  status                          Show checklist status summary

Options:
  --project <id>   Project ID (required)
  --help           Show this help

Examples:
  $(basename "$0") templates --project abc123
  $(basename "$0") create-from-template TMPL001 --project abc123
  $(basename "$0") status --project abc123"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
PROJECT_ID=""
TEMPLATE_ID=""
CHECKLIST_ID=""
ASSIGNEE=""

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        templates|status)  COMMAND="$1"; shift ;;
        create-from-template)
            COMMAND="create-from-template"
            TEMPLATE_ID="${2:-}"
            [[ -n "$TEMPLATE_ID" && "$TEMPLATE_ID" != -* ]] && shift 2 || shift
            ;;
        assign)
            COMMAND="assign"
            CHECKLIST_ID="${2:-}"
            ASSIGNEE="${3:-}"
            shift
            [[ -n "$CHECKLIST_ID" && "$CHECKLIST_ID" != -* ]] && shift
            [[ -n "$ASSIGNEE" && "$ASSIGNEE" != -* ]] && shift
            ;;
        --project) PROJECT_ID="$2"; shift 2 ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         POSITIONAL+=("$1"); shift ;;
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
    templates)
        step "Available checklist templates for project: $PROJECT_ID"
        raps acc checklist list --project "$PROJECT_ID" --output json --quiet | \
            jq -r '.[] | "  [\(.id)] \(.title // .name // "unnamed") — \(.status // "unknown")"'
        ;;

    create-from-template)
        if [[ -z "$TEMPLATE_ID" ]]; then
            error "Usage: $(basename "$0") create-from-template <template-id> --project <id>"
            exit 2
        fi

        step "Creating checklist from template: $TEMPLATE_ID"
        raps acc checklist create --project "$PROJECT_ID" --quiet
        info "Checklist created."
        ;;

    assign)
        if [[ -z "$CHECKLIST_ID" || -z "$ASSIGNEE" ]]; then
            error "Usage: $(basename "$0") assign <checklist-id> <email> --project <id>"
            exit 2
        fi

        step "Assigning checklist $CHECKLIST_ID to $ASSIGNEE"
        raps acc checklist update --project "$PROJECT_ID" --checklist "$CHECKLIST_ID" --quiet
        info "Checklist assigned to $ASSIGNEE."
        ;;

    status)
        step "Checklist status for project: $PROJECT_ID"
        CHECKLISTS=$(raps acc checklist list --project "$PROJECT_ID" --output json --quiet)

        TOTAL=$(echo "$CHECKLISTS" | jq 'length')
        echo
        echo "Checklist Summary"
        echo "─────────────────────────"
        echo "  Total: $TOTAL"
        echo
        echo "By status:"
        echo "$CHECKLISTS" | jq -r '
            group_by(.status // "unknown")
            | .[]
            | "  \(.[0].status // "unknown"): \(length)"
        '
        echo
        echo "Checklists:"
        echo "$CHECKLISTS" | jq -r '.[] | "  [\(.id)] \(.title // .name // "unnamed") — \(.status // "unknown")"'
        ;;
esac
