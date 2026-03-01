#!/usr/bin/env bash
# common.sh — Shared helpers for RAPS use-case scripts
# Source at top of every script: source "$(dirname "$0")/../common.sh"

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────────────

if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    DIM='\033[2m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' DIM='' RESET=''
fi

# ── Output helpers ──────────────────────────────────────────────────────────

info()  { echo -e "${GREEN}[INFO]${RESET}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${RESET}  $*" >&2; }
error() { echo -e "${RED}[ERROR]${RESET} $*" >&2; }
step()  { echo -e "${BLUE}[STEP]${RESET}  ${BOLD}$*${RESET}"; }
dim()   { echo -e "${DIM}$*${RESET}"; }

# ── Precondition checks ────────────────────────────────────────────────────

require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" &>/dev/null; then
        error "'$cmd' is required but not found on PATH."
        error "Install it and try again."
        exit 1
    fi
}

check_auth() {
    step "Checking 2-legged authentication..."
    if ! raps auth test --quiet 2>/dev/null; then
        error "2-legged authentication failed."
        error "Run: raps auth test   — to diagnose."
        error "Set APS_CLIENT_ID and APS_CLIENT_SECRET, or run: raps config set client_id <id>"
        exit 1
    fi
    info "Authentication OK."
}

check_3leg() {
    step "Checking 3-legged authentication..."
    if ! raps auth status --quiet 2>/dev/null; then
        error "3-legged authentication required but not active."
        error "Run: raps auth login   — to authenticate via browser."
        exit 1
    fi
    info "3-legged auth OK."
}

# ── User interaction ────────────────────────────────────────────────────────

confirm() {
    local prompt="${1:-Continue?}"
    echo -en "${YELLOW}${prompt} [y/N]${RESET} "
    read -r answer
    case "$answer" in
        [yY]|[yY][eE][sS]) return 0 ;;
        *) info "Aborted."; exit 0 ;;
    esac
}

# ── JSON helpers ────────────────────────────────────────────────────────────

extract_json() {
    local json="$1"
    local query="$2"
    local result
    result=$(echo "$json" | jq -r "$query" 2>/dev/null) || {
        error "Failed to parse JSON with query: $query"
        return 1
    }
    if [[ "$result" == "null" || -z "$result" ]]; then
        error "JSON query returned empty result: $query"
        return 1
    fi
    echo "$result"
}

# ── Usage helper ────────────────────────────────────────────────────────────

# Call from scripts: show_usage "$USAGE" when --help is passed
show_usage() {
    echo "$1"
    exit 0
}

check_help() {
    for arg in "$@"; do
        if [[ "$arg" == "--help" || "$arg" == "-h" ]]; then
            return 0
        fi
    done
    return 1
}

# ── Prereqs ─────────────────────────────────────────────────────────────────

require_cmd raps
require_cmd jq
