#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Pipeline Runner — Validate and run YAML/JSON pipelines

Usage:
  $(basename "$0") <pipeline-file> [options]

Options:
  --dry-run            Validate and preview without executing
  --ignore-failure     Continue pipeline even if a step fails
  --help               Show this help

Examples:
  $(basename "$0") pipeline.yaml --dry-run
  $(basename "$0") deploy-pipeline.yaml
  $(basename "$0") batch-process.json --ignore-failure"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

PIPELINE_FILE=""
DRY_RUN=false
CONTINUE_ON_ERROR=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)           DRY_RUN=true; shift ;;
        --ignore-failure) CONTINUE_ON_ERROR=true; shift ;;
        -*)                  error "Unknown option: $1"; exit 2 ;;
        *)
            if [[ -z "$PIPELINE_FILE" ]]; then
                PIPELINE_FILE="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$PIPELINE_FILE" ]]; then
    error "Missing required argument: <pipeline-file>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -f "$PIPELINE_FILE" ]]; then
    error "Pipeline file not found: $PIPELINE_FILE"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

# Validate
step "Validating pipeline: $PIPELINE_FILE"
if ! raps pipeline validate "$PIPELINE_FILE" --quiet 2>/dev/null; then
    error "Pipeline validation failed."
    error "Fix errors and try again."
    exit 1
fi
info "Pipeline is valid."

if $DRY_RUN; then
    step "Dry run preview:"
    raps pipeline validate "$PIPELINE_FILE"
    echo
    info "DRY RUN — pipeline not executed."
    exit 0
fi

# Confirm and run
confirm "Execute pipeline: $PIPELINE_FILE?"

step "Running pipeline..."
RUN_ARGS=(raps pipeline run "$PIPELINE_FILE")
$CONTINUE_ON_ERROR && RUN_ARGS+=(--ignore-failure)

if "${RUN_ARGS[@]}"; then
    info "Pipeline completed successfully."
else
    EXIT_CODE=$?
    if $CONTINUE_ON_ERROR; then
        warn "Pipeline completed with errors (exit code: $EXIT_CODE)."
    else
        error "Pipeline failed (exit code: $EXIT_CODE)."
    fi
    exit $EXIT_CODE
fi
