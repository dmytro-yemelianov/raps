#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Portfolio Health Report — Cross-project summary of issues, RFIs, and submittals

Usage:
  $(basename "$0") --account <id> [options]

Options:
  --account <id>    Account ID (required)
  --since <date>    Filter by date (default: 30 days ago)
  --output <file>   Save report to JSON file
  --help            Show this help

Examples:
  $(basename "$0") --account ACC123
  $(basename "$0") --account ACC123 --since 2026-01-01
  $(basename "$0") --account ACC123 --output portfolio-report.json"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

ACCOUNT_ID=""
SINCE=""
OUTPUT_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --account) ACCOUNT_ID="$2"; shift 2 ;;
        --since)   SINCE="$2"; shift 2 ;;
        --output)  OUTPUT_FILE="$2"; shift 2 ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         error "Unexpected argument: $1"; exit 2 ;;
    esac
done

if [[ -z "$ACCOUNT_ID" ]]; then
    error "Missing required option: --account"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ -z "$SINCE" ]]; then
    SINCE=$(date -d "-30 days" +%Y-%m-%d 2>/dev/null || date -v-30d +%Y-%m-%d 2>/dev/null || echo "2026-01-01")
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_3leg

echo
echo "Portfolio Health Report"
echo "══════════════════════════════════════════════════"
echo "  Account: $ACCOUNT_ID"
echo "  Since:   $SINCE"
echo "  Date:    $(date +%Y-%m-%d)"
echo

# Gather reports
step "Fetching issues summary..."
ISSUES_REPORT=$(raps report issues-summary --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo '{"total":0}')

step "Fetching RFI summary..."
RFI_REPORT=$(raps report rfi-summary --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo '{"total":0}')

step "Fetching submittals summary..."
SUBMITTALS_REPORT=$(raps report submittals-summary --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo '{"total":0}')

step "Fetching checklists summary..."
CHECKLISTS_REPORT=$(raps report checklists-summary --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo '{"total":0}')

step "Fetching assets summary..."
ASSETS_REPORT=$(raps report assets-summary --account "$ACCOUNT_ID" --output json --quiet 2>/dev/null || echo '{"total":0}')

# Display
echo
echo "────────────────────────────────────────"
echo "  Issues"
echo "────────────────────────────────────────"
echo "$ISSUES_REPORT" | jq -r '
    if type == "array" then
        "  Total: \(length)"
    elif type == "object" then
        to_entries[] | "  \(.key): \(.value)"
    else
        "  \(.)"
    end
' 2>/dev/null || echo "  (no data)"

echo
echo "────────────────────────────────────────"
echo "  RFIs"
echo "────────────────────────────────────────"
echo "$RFI_REPORT" | jq -r '
    if type == "array" then
        "  Total: \(length)"
    elif type == "object" then
        to_entries[] | "  \(.key): \(.value)"
    else
        "  \(.)"
    end
' 2>/dev/null || echo "  (no data)"

echo
echo "────────────────────────────────────────"
echo "  Submittals"
echo "────────────────────────────────────────"
echo "$SUBMITTALS_REPORT" | jq -r '
    if type == "array" then
        "  Total: \(length)"
    elif type == "object" then
        to_entries[] | "  \(.key): \(.value)"
    else
        "  \(.)"
    end
' 2>/dev/null || echo "  (no data)"

echo
echo "────────────────────────────────────────"
echo "  Checklists"
echo "────────────────────────────────────────"
echo "$CHECKLISTS_REPORT" | jq -r '
    if type == "array" then
        "  Total: \(length)"
    elif type == "object" then
        to_entries[] | "  \(.key): \(.value)"
    else
        "  \(.)"
    end
' 2>/dev/null || echo "  (no data)"

echo
echo "────────────────────────────────────────"
echo "  Assets"
echo "────────────────────────────────────────"
echo "$ASSETS_REPORT" | jq -r '
    if type == "array" then
        "  Total: \(length)"
    elif type == "object" then
        to_entries[] | "  \(.key): \(.value)"
    else
        "  \(.)"
    end
' 2>/dev/null || echo "  (no data)"

# Save combined report
if [[ -n "$OUTPUT_FILE" ]]; then
    step "Saving combined report to $OUTPUT_FILE..."
    jq -n \
        --arg account "$ACCOUNT_ID" \
        --arg since "$SINCE" \
        --arg date "$(date +%Y-%m-%d)" \
        --argjson issues "$ISSUES_REPORT" \
        --argjson rfis "$RFI_REPORT" \
        --argjson submittals "$SUBMITTALS_REPORT" \
        --argjson checklists "$CHECKLISTS_REPORT" \
        --argjson assets "$ASSETS_REPORT" \
        '{
            account: $account,
            since: $since,
            generated: $date,
            issues: $issues,
            rfis: $rfis,
            submittals: $submittals,
            checklists: $checklists,
            assets: $assets
        }' > "$OUTPUT_FILE"
    info "Report saved: $OUTPUT_FILE"
fi

echo
info "Portfolio health report complete."
