#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Weekly Access Audit — Report on user roles and flag stale accounts

Usage:
  $(basename "$0") --account <id> [options]

Options:
  --account <id>    Account ID (required)
  --output <file>   Save report to JSON file
  --warn-days <n>   Flag admins with no activity in N days (default: 90)
  --help            Show this help

Examples:
  $(basename "$0") --account ACC123
  $(basename "$0") --account ACC123 --output audit-2026-02.json
  $(basename "$0") --account ACC123 --warn-days 60"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

ACCOUNT_ID=""
OUTPUT_FILE=""
WARN_DAYS=90

while [[ $# -gt 0 ]]; do
    case "$1" in
        --account)   ACCOUNT_ID="$2"; shift 2 ;;
        --output)    OUTPUT_FILE="$2"; shift 2 ;;
        --warn-days) WARN_DAYS="$2"; shift 2 ;;
        -*)          error "Unknown option: $1"; exit 2 ;;
        *)           error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$ACCOUNT_ID" ]]; then
    error "Missing required option: --account"
    echo
    echo "$USAGE"
    exit 2
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

step "Fetching users for account: $ACCOUNT_ID"
USERS=$(raps admin user list --account "$ACCOUNT_ID" --output json --quiet)

TOTAL=$(echo "$USERS" | jq 'length')
info "Found $TOTAL users."

echo
echo "Access Audit Report"
echo "════════════════════════════════════════"
echo "  Account:    $ACCOUNT_ID"
echo "  Date:       $(date +%Y-%m-%d)"
echo "  Total users: $TOTAL"
echo

# Group by role
step "Users by role:"
echo "$USERS" | jq -r '
    group_by(.role // "unknown")
    | .[]
    | "  \(.[0].role // "unknown"): \(length)"
'

# List admins
ADMINS=$(echo "$USERS" | jq '[.[] | select(.role == "account_admin" or .role == "project_admin")]')
ADMIN_COUNT=$(echo "$ADMINS" | jq 'length')

echo
step "Admin accounts ($ADMIN_COUNT):"
echo "$ADMINS" | jq -r '.[] | "  \(.email // .autodeskId // "unknown") — \(.role // "unknown")"'

# Flag stale admins (those with lastSignIn older than WARN_DAYS)
CUTOFF_DATE=$(date -d "-${WARN_DAYS} days" +%Y-%m-%dT%H:%M:%S 2>/dev/null || date -v-${WARN_DAYS}d +%Y-%m-%dT%H:%M:%S 2>/dev/null || echo "")

if [[ -n "$CUTOFF_DATE" ]]; then
    echo
    step "Stale admin accounts (no activity in $WARN_DAYS days):"
    STALE=$(echo "$ADMINS" | jq -r --arg cutoff "$CUTOFF_DATE" '
        [.[] | select(
            (.lastSignIn // "1970-01-01T00:00:00") < $cutoff
        )]
    ')
    STALE_COUNT=$(echo "$STALE" | jq 'length')

    if [[ "$STALE_COUNT" -gt 0 ]]; then
        warn "Found $STALE_COUNT stale admin(s):"
        echo "$STALE" | jq -r '.[] | "  ⚠ \(.email // .autodeskId // "unknown") — last sign-in: \(.lastSignIn // "never")"'
    else
        info "No stale admin accounts found."
    fi
fi

# Companies
echo
step "Companies:"
echo "$USERS" | jq -r '
    group_by(.companyName // "unspecified")
    | .[]
    | "  \(.[0].companyName // "unspecified"): \(length) users"
'

# Save report
if [[ -n "$OUTPUT_FILE" ]]; then
    step "Saving report to $OUTPUT_FILE..."
    jq -n \
        --arg account "$ACCOUNT_ID" \
        --arg date "$(date +%Y-%m-%d)" \
        --argjson total "$TOTAL" \
        --argjson admin_count "$ADMIN_COUNT" \
        --argjson users "$USERS" \
        --argjson admins "$ADMINS" \
        '{
            account: $account,
            date: $date,
            total_users: $total,
            admin_count: $admin_count,
            users_by_role: ($users | group_by(.role // "unknown") | map({role: .[0].role, count: length})),
            admins: $admins,
            all_users: $users
        }' > "$OUTPUT_FILE"
    info "Report saved: $OUTPUT_FILE"
fi

echo
info "Audit complete."
