#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Webhook Setup — Create, test, list, and clean up webhooks

Usage:
  $(basename "$0") <command> [options]

Commands:
  create <url> <event>   Create a webhook subscription
  test <url>             Test webhook endpoint connectivity
  list                   List all webhook subscriptions
  events                 List available webhook events
  cleanup                Delete all inactive webhooks

Options:
  --scope <scope>    Webhook scope (e.g., workflow, data, folder)
  --dry-run          Preview cleanup without deleting
  --help             Show this help

Examples:
  $(basename "$0") create https://hooks.example.com/aps dm.version.added
  $(basename "$0") test https://hooks.example.com/aps
  $(basename "$0") list
  $(basename "$0") events
  $(basename "$0") cleanup --dry-run"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
URL=""
EVENT=""
SCOPE=""
DRY_RUN=false

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        create|test|list|events|cleanup) COMMAND="$1"; shift ;;
        --scope)   SCOPE="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        -*)        error "Unknown option: $1"; exit 2 ;;
        *)         POSITIONAL+=("$1"); shift ;;
    esac
done

case "$COMMAND" in
    create) URL="${POSITIONAL[0]:-}"; EVENT="${POSITIONAL[1]:-}" ;;
    test)   URL="${POSITIONAL[0]:-}" ;;
esac

if [[ -z "$COMMAND" ]]; then
    error "Missing command"; echo; echo "$USAGE"; exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

check_auth

case "$COMMAND" in
    create)
        if [[ -z "$URL" || -z "$EVENT" ]]; then
            error "Usage: $(basename "$0") create <url> <event>"
            exit 2
        fi

        step "Creating webhook..."
        dim "  URL:   $URL"
        dim "  Event: $EVENT"

        CREATE_ARGS=(raps webhook create --url "$URL" --event "$EVENT")
        [[ -n "$SCOPE" ]] && CREATE_ARGS+=(--scope "$SCOPE")
        CREATE_ARGS+=(--output json --quiet)

        RESULT=$("${CREATE_ARGS[@]}")
        HOOK_ID=$(echo "$RESULT" | jq -r '.hookId // .id // empty')

        info "Webhook created: $HOOK_ID"
        ;;

    test)
        if [[ -z "$URL" ]]; then
            error "Usage: $(basename "$0") test <url>"
            exit 2
        fi

        step "Testing webhook endpoint: $URL"
        if raps webhook test --url "$URL" --quiet 2>/dev/null; then
            info "Endpoint is reachable and responded correctly."
        else
            error "Endpoint test failed. Check URL and server."
            exit 1
        fi
        ;;

    list)
        step "Webhook subscriptions:"
        raps webhook list --output json --quiet | jq -r '
            .[] | "  [\(.hookId // .id)] \(.event // "unknown") → \(.callbackUrl // .url // "unknown") (\(.status // "unknown"))"
        '
        ;;

    events)
        step "Available webhook events:"
        raps webhook events --quiet
        ;;

    cleanup)
        step "Finding inactive webhooks..."
        HOOKS=$(raps webhook list --output json --quiet)

        INACTIVE=$(echo "$HOOKS" | jq '[.[] | select(.status == "inactive" or .status == "failed")]')
        INACTIVE_COUNT=$(echo "$INACTIVE" | jq 'length')

        if [[ "$INACTIVE_COUNT" -eq 0 ]]; then
            info "No inactive webhooks found."
            exit 0
        fi

        warn "Found $INACTIVE_COUNT inactive webhook(s):"
        echo "$INACTIVE" | jq -r '.[] | "  [\(.hookId // .id)] \(.event // "unknown") → \(.callbackUrl // .url // "unknown")"'

        if $DRY_RUN; then
            info "DRY RUN — no webhooks deleted."
            exit 0
        fi

        confirm "Delete $INACTIVE_COUNT inactive webhooks?"

        echo "$INACTIVE" | jq -r '.hookId // .id // empty' | while IFS= read -r HID; do
            [[ -z "$HID" ]] && continue
            dim "  Deleting: $HID"
            raps webhook delete --hook "$HID" --quiet 2>/dev/null || warn "  Failed to delete: $HID"
        done
        info "Cleanup complete."
        ;;
esac
