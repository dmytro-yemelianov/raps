#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Monitor Workitems — Poll DA workitem status and download results

Usage:
  $(basename "$0") [options]

Options:
  --active           List only active (in-progress) workitems
  --wait <id>        Wait for a specific workitem to complete
  --download <id>    Download outputs from a completed workitem
  --out-dir <dir>    Output directory for downloads (default: ./da-output)
  --timeout <secs>   Wait timeout in seconds (default: 600)
  --help             Show this help

Examples:
  $(basename "$0") --active
  $(basename "$0") --wait WI-ABC123
  $(basename "$0") --download WI-ABC123 --out-dir ./results"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

ACTIVE=false
WAIT_ID=""
DOWNLOAD_ID=""
OUT_DIR="./da-output"
TIMEOUT=600

while [[ $# -gt 0 ]]; do
    case "$1" in
        --active)   ACTIVE=true; shift ;;
        --wait)     WAIT_ID="$2"; shift 2 ;;
        --download) DOWNLOAD_ID="$2"; shift 2 ;;
        --out-dir)  OUT_DIR="$2"; shift 2 ;;
        --timeout)  TIMEOUT="$2"; shift 2 ;;
        -*)         error "Unknown option: $1"; exit 2 ;;
        *)          error "Unexpected argument: $1"; exit 2 ;;
    esac
done

# Default: list all if no specific action
if ! $ACTIVE && [[ -z "$WAIT_ID" ]] && [[ -z "$DOWNLOAD_ID" ]]; then
    ACTIVE=true
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

if $ACTIVE || [[ -z "$WAIT_ID" && -z "$DOWNLOAD_ID" ]]; then
    step "Workitem status:"
    WORKITEMS=$(raps da workitems --output json --quiet)

    if $ACTIVE; then
        FILTERED=$(echo "$WORKITEMS" | jq '[.[] | select(.status == "pending" or .status == "inprogress" or .status == "in_progress")]')
    else
        FILTERED="$WORKITEMS"
    fi

    COUNT=$(echo "$FILTERED" | jq 'length')
    if [[ "$COUNT" -eq 0 ]]; then
        info "No ${ACTIVE:+active }workitems found."
    else
        info "Found $COUNT workitem(s):"
        echo "$FILTERED" | jq -r '.[] | "  [\(.id)] \(.status // "unknown") — \(.activityId // "unknown activity")"'
    fi
fi

if [[ -n "$WAIT_ID" ]]; then
    step "Waiting for workitem: $WAIT_ID (timeout: ${TIMEOUT}s)"
    ELAPSED=0
    INTERVAL=10

    while [[ $ELAPSED -lt $TIMEOUT ]]; do
        STATUS=$(raps da workitems --output json --quiet 2>/dev/null | jq -r --arg id "$WAIT_ID" '.[] | select(.id == $id) | .status // "unknown"')

        case "$STATUS" in
            success|completed)
                info "Workitem completed successfully."
                break
                ;;
            failed|cancelled)
                error "Workitem $STATUS."
                exit 1
                ;;
            "")
                error "Workitem not found: $WAIT_ID"
                exit 1
                ;;
            *)
                dim "  Status: $STATUS (${ELAPSED}s elapsed)..."
                sleep $INTERVAL
                ELAPSED=$((ELAPSED + INTERVAL))
                ;;
        esac
    done

    if [[ $ELAPSED -ge $TIMEOUT ]]; then
        error "Timeout waiting for workitem: $WAIT_ID"
        exit 1
    fi
fi

if [[ -n "$DOWNLOAD_ID" ]]; then
    step "Downloading outputs for workitem: $DOWNLOAD_ID"
    mkdir -p "$OUT_DIR"

    # Get workitem details to find output URLs
    WORKITEM=$(raps da workitems --output json --quiet 2>/dev/null | jq --arg id "$DOWNLOAD_ID" '.[] | select(.id == $id)')

    if [[ -z "$WORKITEM" || "$WORKITEM" == "null" ]]; then
        error "Workitem not found: $DOWNLOAD_ID"
        exit 1
    fi

    STATUS=$(echo "$WORKITEM" | jq -r '.status // "unknown"')
    if [[ "$STATUS" != "success" && "$STATUS" != "completed" ]]; then
        warn "Workitem status is '$STATUS'. Output may not be available."
    fi

    info "Check $OUT_DIR for downloaded files."
    info "If outputs are in an OSS bucket, use: raps object download <bucket> <key> --output-dir $OUT_DIR"
fi
