#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="User Onboarding — Bulk add users from CSV

Usage:
  $(basename "$0") --csv <file> --account <id> [options]

Options:
  --csv <file>       CSV file with user data (required, columns: email,role,company)
  --account <id>     Account ID (required)
  --project <id>     Add users to a specific project (optional)
  --role <role>      Default role: project_admin or project_user (default: project_user)
  --dry-run          Preview what would happen without making changes
  --help             Show this help

CSV Format:
  email,role,company
  jane@example.com,project_admin,ACME Corp
  bob@example.com,project_user,BuildCo

Examples:
  $(basename "$0") --csv team.csv --account ACC123 --dry-run
  $(basename "$0") --csv team.csv --account ACC123 --project PROJ456
  $(basename "$0") --csv new-hires.csv --account ACC123 --role project_user"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

CSV_FILE=""
ACCOUNT_ID=""
PROJECT_ID=""
DEFAULT_ROLE="project_user"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --csv)     CSV_FILE="$2"; shift 2 ;;
        --account) ACCOUNT_ID="$2"; shift 2 ;;
        --project) PROJECT_ID="$2"; shift 2 ;;
        --role)    DEFAULT_ROLE="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$CSV_FILE" || -z "$ACCOUNT_ID" ]]; then
    error "Missing required options: --csv and --account"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -f "$CSV_FILE" ]]; then
    error "CSV file not found: $CSV_FILE"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

# Count users
TOTAL=$(tail -n +2 "$CSV_FILE" | grep -c '[^[:space:]]' || echo 0)
info "Found $TOTAL users in $CSV_FILE"

if $DRY_RUN; then
    info "DRY RUN — no changes will be made"
    echo
    echo "Users to onboard:"
    echo "─────────────────────────────────────────"
    printf "  %-30s %-15s %s\n" "EMAIL" "ROLE" "COMPANY"
    echo "  ──────────────────────────── ─────────────── ─────────────"
    while IFS=',' read -r EMAIL ROLE COMPANY; do
        [[ "$EMAIL" == "email" ]] && continue
        [[ -z "$EMAIL" ]] && continue
        ROLE="${ROLE:-$DEFAULT_ROLE}"
        printf "  %-30s %-15s %s\n" "$EMAIL" "$ROLE" "${COMPANY:-N/A}"
    done < "$CSV_FILE"
    echo
    info "Would add $TOTAL users to account $ACCOUNT_ID"
    [[ -n "$PROJECT_ID" ]] && info "Would add to project: $PROJECT_ID"
    exit 0
fi

confirm "Add $TOTAL users to account $ACCOUNT_ID?"

step "Onboarding users..."
ADDED=0
FAILED=0

while IFS=',' read -r EMAIL ROLE COMPANY; do
    [[ "$EMAIL" == "email" ]] && continue
    [[ -z "$EMAIL" ]] && continue
    ROLE="${ROLE:-$DEFAULT_ROLE}"

    dim "  Adding: $EMAIL (role: $ROLE)"

    ADD_ARGS=(raps admin user add --account "$ACCOUNT_ID" --email "$EMAIL" --role "$ROLE")
    [[ -n "$PROJECT_ID" ]] && ADD_ARGS+=(--project "$PROJECT_ID")
    ADD_ARGS+=(--quiet)

    if "${ADD_ARGS[@]}" 2>/dev/null; then
        ADDED=$((ADDED + 1))
    else
        warn "  Failed to add: $EMAIL"
        FAILED=$((FAILED + 1))
    fi
done < "$CSV_FILE"

echo
echo "Onboarding Summary"
echo "───────────────────"
echo "  Added:   $ADDED"
echo "  Failed:  $FAILED"
echo "  Total:   $TOTAL"
info "Onboarding complete."
