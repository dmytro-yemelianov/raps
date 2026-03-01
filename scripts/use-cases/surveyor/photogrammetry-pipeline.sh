#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../common.sh"

USAGE="Photogrammetry Pipeline — Create scene, upload photos, process, and download

End-to-end photogrammetry workflow using Reality Capture API.

Usage:
  $(basename "$0") <photos-dir> [options]

Options:
  --scene-type <type>    Scene type: aerial or object (default: aerial)
  --format <fmt>         Output format: rcm, obj, ortho (default: rcm)
  --scene-name <name>    Scene name (default: photoscene-<timestamp>)
  --upload-to-acc <id>   After processing, upload result to ACC project
  --out-dir <dir>        Output directory for results (default: ./reality-output)
  --help                 Show this help

Examples:
  $(basename "$0") ./site-photos
  $(basename "$0") ./drone-images --scene-type aerial --format obj
  $(basename "$0") ./photos --upload-to-acc PROJ123 --format rcm"

check_help "$@" && show_usage "$USAGE"

# ── Parse args ──────────────────────────────────────────────────────────────

PHOTOS_DIR=""
SCENE_TYPE="aerial"
FORMAT="rcm"
SCENE_NAME=""
ACC_PROJECT=""
OUT_DIR="./reality-output"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --scene-type)     SCENE_TYPE="$2"; shift 2 ;;
        --format)         FORMAT="$2"; shift 2 ;;
        --scene-name)     SCENE_NAME="$2"; shift 2 ;;
        --upload-to-acc)  ACC_PROJECT="$2"; shift 2 ;;
        --out-dir)        OUT_DIR="$2"; shift 2 ;;
        -*)               error "Unknown option: $1"; exit 2 ;;
        *)
            if [[ -z "$PHOTOS_DIR" ]]; then
                PHOTOS_DIR="$1"; shift
            else
                error "Unexpected argument: $1"; exit 2
            fi
            ;;
    esac
done

if [[ -z "$PHOTOS_DIR" ]]; then
    error "Missing required argument: <photos-dir>"
    echo
    echo "$USAGE"
    exit 2
fi

if [[ ! -d "$PHOTOS_DIR" ]]; then
    error "Directory not found: $PHOTOS_DIR"
    exit 1
fi

# Count photos
PHOTO_COUNT=$(find "$PHOTOS_DIR" -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" -o -iname "*.tif" -o -iname "*.tiff" \) | wc -l)
if [[ "$PHOTO_COUNT" -eq 0 ]]; then
    error "No image files found in $PHOTOS_DIR"
    exit 1
fi

# ── Main flow ───────────────────────────────────────────────────────────────

check_auth

if [[ -z "$SCENE_NAME" ]]; then
    SCENE_NAME="photoscene-$(date +%s)"
fi

info "Photos found: $PHOTO_COUNT"
info "Scene type:   $SCENE_TYPE"
info "Output format: $FORMAT"
echo

# Create photoscene
step "Creating photoscene: $SCENE_NAME"
CREATE_RESULT=$(raps reality create --name "$SCENE_NAME" --type "$SCENE_TYPE" --format "$FORMAT" --output json --quiet)
SCENE_ID=$(echo "$CREATE_RESULT" | jq -r '.photosceneId // .id // empty')

if [[ -z "$SCENE_ID" ]]; then
    error "Failed to create photoscene."
    exit 1
fi
info "Photoscene ID: $SCENE_ID"

# Upload photos
step "Uploading $PHOTO_COUNT photos..."
find "$PHOTOS_DIR" -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" -o -iname "*.tif" -o -iname "*.tiff" \) | while IFS= read -r PHOTO; do
    PNAME=$(basename "$PHOTO")
    dim "  Uploading: $PNAME"
    raps reality upload --scene "$SCENE_ID" --file "$PHOTO" --quiet 2>/dev/null || warn "  Failed: $PNAME"
done
info "Photo upload complete."

# Start processing
step "Starting photoscene processing..."
raps reality process --scene "$SCENE_ID" --quiet
info "Processing started."

# Wait for completion
step "Waiting for processing to complete..."
for i in $(seq 1 120); do
    STATUS_RESULT=$(raps reality status --scene "$SCENE_ID" --output json --quiet 2>/dev/null || echo '{}')
    STATUS=$(echo "$STATUS_RESULT" | jq -r '.status // .progress // "unknown"')
    PROGRESS=$(echo "$STATUS_RESULT" | jq -r '.progressPercentage // .progress // ""')

    case "$STATUS" in
        done|complete|completed)
            info "Processing complete."
            break
            ;;
        error|failed)
            error "Processing failed."
            echo "$STATUS_RESULT" | jq . 2>/dev/null
            exit 1
            ;;
        *)
            PROGRESS_TEXT=""
            [[ -n "$PROGRESS" && "$PROGRESS" != "null" ]] && PROGRESS_TEXT=" (${PROGRESS}%)"
            dim "  Status: ${STATUS}${PROGRESS_TEXT} (check $i/120)..."
            sleep 15
            ;;
    esac
done

# Download result
mkdir -p "$OUT_DIR"
step "Downloading result..."
raps reality result --scene "$SCENE_ID" --output-dir "$OUT_DIR" --quiet 2>/dev/null || {
    RESULT_URL=$(raps reality result --scene "$SCENE_ID" --output json --quiet 2>/dev/null | jq -r '.url // .link // empty')
    if [[ -n "$RESULT_URL" ]]; then
        info "Download URL: $RESULT_URL"
        curl -sL "$RESULT_URL" -o "$OUT_DIR/${SCENE_NAME}.${FORMAT}" 2>/dev/null || warn "Download via curl failed."
    else
        warn "Could not download result automatically."
    fi
}

info "Result saved to: $OUT_DIR"

# Upload to ACC if requested
if [[ -n "$ACC_PROJECT" ]]; then
    step "Uploading result to ACC project: $ACC_PROJECT"

    RESULT_FILE=$(find "$OUT_DIR" -type f -newer /tmp -name "*${SCENE_NAME}*" | head -1 || echo "")
    if [[ -z "$RESULT_FILE" ]]; then
        RESULT_FILE=$(find "$OUT_DIR" -type f | head -1 || echo "")
    fi

    if [[ -n "$RESULT_FILE" ]]; then
        BUCKET="raps-reality-$(date +%s)"
        raps bucket create "$BUCKET" --quiet 2>/dev/null || true
        raps object upload "$BUCKET" "$RESULT_FILE" --quiet

        OBJECT_KEY=$(basename "$RESULT_FILE")
        raps item create-from-oss --project "$ACC_PROJECT" --bucket "$BUCKET" --object "$OBJECT_KEY" --quiet 2>/dev/null || {
            warn "Could not create ACC item automatically."
            info "Use: raps item create-from-oss --project $ACC_PROJECT --bucket $BUCKET --object $OBJECT_KEY"
        }
        info "Uploaded to ACC."
    else
        warn "No result file found to upload."
    fi
fi

echo
info "Photogrammetry pipeline complete."
info "Scene ID: $SCENE_ID"
