#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="CI Auth Check — Token validation for CI pipelines

Validates authentication and checks token expiry. Returns CI-friendly exit codes.

Usage:
  $(basename "$0") [options]

Options:
  --warn-seconds <n>   Warn if token expires within N seconds (default: 300)
  --quiet              Only output exit code, no messages
  --help               Show this help

Exit Codes:
  0 — Authentication valid, token not expiring soon
  1 — Authentication failed
  3 — Authentication valid but token expiring soon

Examples:
  $(basename "$0")
  $(basename "$0") --warn-seconds 600
  $(basename "$0") --quiet && echo 'Auth OK' || echo 'Auth failed'"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

WARN_SECONDS=300
QUIET=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --warn-seconds) WARN_SECONDS="$2"; shift 2 ;;
        --quiet)        QUIET=true; shift ;;
        -*)             error "Unknown option: $1"; exit 2 ;;
        *)              error "Unexpected argument: $1"; exit 2 ;;
    esac
done

# ── Main flow ───────────────────────────────────────────────────────────────

log() {
    if ! $QUIET; then
        echo "$@"
    fi
}

# Test 2-legged auth
if ! raps auth test --quiet 2>/dev/null; then
    $QUIET || error "2-legged authentication failed."
    $QUIET || error "Ensure APS_CLIENT_ID and APS_CLIENT_SECRET are set."
    exit 1
fi

$QUIET || info "2-legged authentication: OK"

# Inspect token expiry
INSPECT_OUTPUT=$(raps auth inspect --output json --quiet 2>/dev/null || echo '{}')
EXPIRES_IN=$(echo "$INSPECT_OUTPUT" | jq -r '.expires_in // .expiresIn // .ttl // empty' 2>/dev/null || echo "")

if [[ -n "$EXPIRES_IN" && "$EXPIRES_IN" =~ ^[0-9]+$ ]]; then
    $QUIET || info "Token expires in: ${EXPIRES_IN}s"

    if [[ "$EXPIRES_IN" -lt "$WARN_SECONDS" ]]; then
        $QUIET || warn "Token expiring soon (< ${WARN_SECONDS}s remaining)."
        $QUIET || warn "Consider refreshing credentials before the next pipeline step."
        exit 3
    fi
else
    $QUIET || dim "  (could not determine token expiry)"
fi

$QUIET || info "Auth check passed."
exit 0
