#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Multi-Profile Switch — Set up and switch between client profiles

Usage:
  $(basename "$0") <command> [options]

Commands:
  setup <name> <client-id> <secret>   Create a new profile
  switch <name>                       Switch to a profile
  list                                List all profiles
  current                             Show active profile
  test-all                            Test auth on all profiles

Options:
  --help   Show this help

Examples:
  $(basename "$0") setup staging CLIENT_ID_HERE SECRET_HERE
  $(basename "$0") switch staging
  $(basename "$0") list
  $(basename "$0") test-all"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

COMMAND=""
PROFILE_NAME=""
CLIENT_ID=""
CLIENT_SECRET=""

POSITIONAL=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        setup|switch|list|current|test-all) COMMAND="$1"; shift ;;
        -*) error "Unknown option: $1"; exit 2 ;;
        *)  POSITIONAL+=("$1"); shift ;;
    esac
done

case "$COMMAND" in
    setup)
        PROFILE_NAME="${POSITIONAL[0]:-}"
        CLIENT_ID="${POSITIONAL[1]:-}"
        CLIENT_SECRET="${POSITIONAL[2]:-}"
        ;;
    switch)
        PROFILE_NAME="${POSITIONAL[0]:-}"
        ;;
esac

if [[ -z "$COMMAND" ]]; then
    error "Missing command"; echo; echo "$USAGE"; exit 2
fi

# ── Commands ────────────────────────────────────────────────────────────────

case "$COMMAND" in
    setup)
        if [[ -z "$PROFILE_NAME" || -z "$CLIENT_ID" || -z "$CLIENT_SECRET" ]]; then
            error "Usage: $(basename "$0") setup <name> <client-id> <secret>"
            exit 2
        fi

        step "Creating profile: $PROFILE_NAME"
        raps config profile create "$PROFILE_NAME" --quiet 2>/dev/null || dim "  (profile may already exist)"
        raps config profile use "$PROFILE_NAME" --quiet
        raps config set client_id "$CLIENT_ID" --quiet
        raps config set client_secret "$CLIENT_SECRET" --quiet

        info "Profile '$PROFILE_NAME' created and configured."

        # Test the new profile
        dim "  Testing authentication..."
        if raps auth test --quiet 2>/dev/null; then
            info "Authentication test passed."
        else
            warn "Authentication test failed. Check credentials."
        fi
        ;;

    switch)
        if [[ -z "$PROFILE_NAME" ]]; then
            error "Usage: $(basename "$0") switch <name>"
            exit 2
        fi

        step "Switching to profile: $PROFILE_NAME"
        raps config profile use "$PROFILE_NAME" --quiet
        info "Active profile: $PROFILE_NAME"

        dim "  Testing authentication..."
        if raps auth test --quiet 2>/dev/null; then
            info "Authentication OK."
        else
            warn "Authentication failed for profile '$PROFILE_NAME'."
        fi
        ;;

    list)
        step "Configured profiles:"
        raps config profile list
        ;;

    current)
        step "Active profile:"
        raps config profile current
        ;;

    test-all)
        step "Testing all profiles..."
        PROFILES=$(raps config profile list --output json --quiet 2>/dev/null || echo '[]')

        # Save current profile to restore later
        CURRENT=$(raps config profile current --quiet 2>/dev/null || echo "default")

        PASSED=0
        FAILED=0

        echo "$PROFILES" | jq -r '.[] | .name // .' 2>/dev/null | while IFS= read -r PNAME; do
            [[ -z "$PNAME" ]] && continue
            raps config profile use "$PNAME" --quiet 2>/dev/null
            if raps auth test --quiet 2>/dev/null; then
                info "  $PNAME: OK"
            else
                error "  $PNAME: FAILED"
            fi
        done

        # Restore original profile
        raps config profile use "$CURRENT" --quiet 2>/dev/null || true
        info "Restored active profile: $CURRENT"
        ;;
esac
