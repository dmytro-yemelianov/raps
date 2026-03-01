#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="User Offboarding — Remove user from all projects

Usage:
  $(basename "$0") --email <user> --account <id> [options]

Options:
  --email <user>   User email to remove (required)
  --account <id>   Account ID (required)
  --dry-run        Preview what would be removed without making changes
  --help           Show this help

Examples:
  $(basename "$0") --email jane@example.com --account ACC123 --dry-run
  $(basename "$0") --email departed@example.com --account ACC123"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

EMAIL=""
ACCOUNT_ID=""
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --email)   EMAIL="$2"; shift 2 ;;
        --account) ACCOUNT_ID="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$EMAIL" || -z "$ACCOUNT_ID" ]]; then
    error "Missing required options: --email and --account"
    echo
    echo "$USAGE"
    exit 2
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

step "Looking up user: $EMAIL"
USERS=$(raps admin user list --account "$ACCOUNT_ID" --output json --quiet)

USER_ENTRY=$(echo "$USERS" | jq --arg email "$EMAIL" '[.[] | select(.email == $email)]')
USER_COUNT=$(echo "$USER_ENTRY" | jq 'length')

if [[ "$USER_COUNT" -eq 0 ]]; then
    warn "User not found in account: $EMAIL"
    exit 0
fi

info "Found user in account."

# List projects the user is in
step "Finding user's project memberships..."
PROJECTS=$(raps admin project list --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo "[]")
PROJECT_COUNT=$(echo "$PROJECTS" | jq 'length')

MEMBER_PROJECTS=()
echo "$PROJECTS" | jq -r '.[].id // empty' | while IFS= read -r PID; do
    [[ -z "$PID" ]] && continue
    PNAME=$(echo "$PROJECTS" | jq -r --arg id "$PID" '.[] | select(.id == $id) | .name // "unnamed"')
    PROJECT_USERS=$(raps admin user list --account "$ACCOUNT_ID" --project "$PID" --output json --quiet 2>/dev/null || echo "[]")
    IS_MEMBER=$(echo "$PROJECT_USERS" | jq --arg email "$EMAIL" '[.[] | select(.email == $email)] | length')
    if [[ "$IS_MEMBER" -gt 0 ]]; then
        echo "$PID|$PNAME"
    fi
done > /tmp/raps-offboard-projects-$$.txt || true

FOUND_PROJECTS=$(cat /tmp/raps-offboard-projects-$$.txt 2>/dev/null || echo "")
rm -f /tmp/raps-offboard-projects-$$.txt

if [[ -z "$FOUND_PROJECTS" ]]; then
    info "User is not a member of any projects."
    exit 0
fi

PROJ_COUNT=$(echo "$FOUND_PROJECTS" | grep -c '[^[:space:]]' || echo 0)
info "User is a member of $PROJ_COUNT project(s)."

if $DRY_RUN; then
    info "DRY RUN — no changes will be made"
    echo
    echo "Would remove $EMAIL from:"
    echo "$FOUND_PROJECTS" | while IFS='|' read -r PID PNAME; do
        echo "  - $PNAME ($PID)"
    done
    exit 0
fi

confirm "Remove $EMAIL from $PROJ_COUNT projects?"

step "Removing user from projects..."
REMOVED=0

echo "$FOUND_PROJECTS" | while IFS='|' read -r PID PNAME; do
    dim "  Removing from: $PNAME"
    if raps admin user remove --account "$ACCOUNT_ID" --email "$EMAIL" --project "$PID" --quiet 2>/dev/null; then
        REMOVED=$((REMOVED + 1))
    else
        warn "  Failed to remove from: $PNAME"
    fi
done

echo
echo "Offboarding Summary"
echo "────────────────────"
echo "  User:     $EMAIL"
echo "  Account:  $ACCOUNT_ID"
echo "  Projects removed from: $PROJ_COUNT"
info "Offboarding complete."
