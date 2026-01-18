#!/bin/bash
# RAPS Install Script
# https://rapscli.xyz
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash
#
# Environment Variables:
#   RAPS_VERSION       - Specific version to install (default: latest)
#   RAPS_INSTALL_DIR   - Installation directory (default: ~/.raps/bin)
#   RAPS_NO_MODIFY_PATH - Skip PATH modification if set

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# GitHub repository
REPO="dmytro-yemelianov/raps"
GITHUB_API="https://api.github.com/repos/${REPO}"
GITHUB_RELEASES="https://github.com/${REPO}/releases"

# Default install directory
DEFAULT_INSTALL_DIR="${HOME}/.raps/bin"
INSTALL_DIR="${RAPS_INSTALL_DIR:-${DEFAULT_INSTALL_DIR}}"

# Version to install
VERSION="${RAPS_VERSION:-latest}"

# Print banner
print_banner() {
    echo -e "${CYAN}"
    echo '     ____  ___    ____  _____'
    echo '    / __ \/ _ |  / __ \/ ___/'
    echo '   / /_/ / __ | / /_/ (__  ) '
    echo '  / _, _/ /_/ |/ ____/____/  '
    echo ' /_/ |_/_/ |_/_/             '
    echo -e "${NC}"
}

# Print colored output
info() {
    echo -e "${BLUE}→${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1" >&2
}

# Show help
show_help() {
    echo "RAPS Install Script"
    echo ""
    echo "Usage:"
    echo "  curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash"
    echo "  ./install.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --help        Show this help message"
    echo "  --uninstall   Remove RAPS installation"
    echo ""
    echo "Environment Variables:"
    echo "  RAPS_VERSION         Specific version to install (default: latest)"
    echo "  RAPS_INSTALL_DIR     Installation directory (default: ~/.raps/bin)"
    echo "  RAPS_NO_MODIFY_PATH  Skip PATH modification if set"
    echo ""
    echo "Examples:"
    echo "  # Install latest version"
    echo "  curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash"
    echo ""
    echo "  # Install specific version"
    echo "  RAPS_VERSION=3.10.0 curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash"
    echo ""
    echo "  # Install to custom directory"
    echo "  RAPS_INSTALL_DIR=/opt/raps/bin curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash"
    echo ""
    echo "For more information, visit: https://rapscli.xyz"
}

# Detect OS (returns cargo-dist target OS component)
detect_os() {
    local os
    os="$(uname -s)"
    case "${os}" in
        Linux*)  echo "unknown-linux-gnu" ;;
        Darwin*) echo "apple-darwin" ;;
        *)       echo "unsupported" ;;
    esac
}

# Detect architecture (returns cargo-dist target arch component)
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "${arch}" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             echo "unsupported" ;;
    esac
}

# Check for required commands
check_dependencies() {
    local missing=()

    # Check for curl or wget
    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        missing+=("curl or wget")
    fi

    # Check for tar
    if ! command -v tar &> /dev/null; then
        missing+=("tar")
    fi

    # Check for xz (needed for .tar.xz extraction)
    if ! command -v xz &> /dev/null; then
        missing+=("xz")
    fi

    # Check for sha256sum or shasum
    if ! command -v sha256sum &> /dev/null && ! command -v shasum &> /dev/null; then
        missing+=("sha256sum or shasum")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        error "Missing required dependencies: ${missing[*]}"
        exit 1
    fi
}

# Download file using curl or wget
download() {
    local url="$1"
    local output="$2"

    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found"
        exit 1
    fi
}

# Get latest version from GitHub API
get_latest_version() {
    local api_url="${GITHUB_API}/releases/latest"
    local response

    if command -v curl &> /dev/null; then
        response=$(curl -fsSL "$api_url")
    elif command -v wget &> /dev/null; then
        response=$(wget -qO- "$api_url")
    fi

    # Extract tag_name from JSON response
    echo "$response" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/"tag_name": *"\([^"]*\)"/\1/' | sed 's/^v//'
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local expected_checksum="$2"
    local actual_checksum

    if command -v sha256sum &> /dev/null; then
        actual_checksum=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum &> /dev/null; then
        actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "Cannot verify checksum: sha256sum/shasum not found"
        return 0
    fi

    if [ "$actual_checksum" = "$expected_checksum" ]; then
        return 0
    else
        error "Checksum verification failed"
        error "Expected: $expected_checksum"
        error "Actual:   $actual_checksum"
        return 1
    fi
}

# Detect shell and config file
detect_shell_config() {
    local shell_name
    shell_name=$(basename "$SHELL")

    case "$shell_name" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                echo "$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                echo "$HOME/.bash_profile"
            else
                echo "$HOME/.bashrc"
            fi
            ;;
        zsh)
            echo "$HOME/.zshrc"
            ;;
        fish)
            echo "$HOME/.config/fish/config.fish"
            ;;
        *)
            # Default to bashrc
            echo "$HOME/.bashrc"
            ;;
    esac
}

# Update PATH in shell config
update_path() {
    local config_file
    config_file=$(detect_shell_config)
    local shell_name
    shell_name=$(basename "$SHELL")

    # Check if PATH is already updated
    if grep -q "\.raps/bin" "$config_file" 2>/dev/null; then
        success "PATH already configured in $config_file"
        return 0
    fi

    info "Updating PATH in $config_file..."

    # Create config file if it doesn't exist
    mkdir -p "$(dirname "$config_file")"

    # Add PATH based on shell
    case "$shell_name" in
        fish)
            echo "" >> "$config_file"
            echo "# RAPS" >> "$config_file"
            echo "fish_add_path $INSTALL_DIR" >> "$config_file"
            ;;
        *)
            echo "" >> "$config_file"
            echo "# RAPS" >> "$config_file"
            echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$config_file"
            ;;
    esac

    success "PATH updated in $config_file"
}

# Uninstall RAPS
uninstall() {
    print_banner
    info "Uninstalling RAPS..."

    local binary_path="${INSTALL_DIR}/raps"

    if [ -f "$binary_path" ]; then
        rm -f "$binary_path"
        success "Removed binary from $INSTALL_DIR"
    else
        warn "Binary not found at $binary_path"
    fi

    # Check if install dir is empty and remove it
    if [ -d "$INSTALL_DIR" ] && [ -z "$(ls -A "$INSTALL_DIR")" ]; then
        rmdir "$INSTALL_DIR"
        success "Removed empty directory $INSTALL_DIR"
    fi

    # Check if .raps directory is empty and remove it
    local raps_dir="${HOME}/.raps"
    if [ -d "$raps_dir" ] && [ -z "$(ls -A "$raps_dir")" ]; then
        rmdir "$raps_dir"
        success "Removed empty directory $raps_dir"
    fi

    echo ""
    warn "Note: PATH entry in shell config was not removed."
    warn "You may want to remove the RAPS entry from your shell config file."
    echo ""
    success "RAPS has been uninstalled."
}

# Main installation function
install() {
    print_banner

    # Detect platform
    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)

    if [ "$os" = "unsupported" ]; then
        error "Unsupported operating system: $(uname -s)"
        error "Supported: Linux, macOS"
        exit 1
    fi

    if [ "$arch" = "unsupported" ]; then
        error "Unsupported architecture: $(uname -m)"
        error "Supported: x86_64/amd64, aarch64/arm64"
        exit 1
    fi

    # Check dependencies
    check_dependencies

    # Get version
    if [ "$VERSION" = "latest" ]; then
        info "Fetching latest version..."
        VERSION=$(get_latest_version)
        if [ -z "$VERSION" ]; then
            error "Failed to fetch latest version from GitHub"
            exit 1
        fi
    fi

    # Build target triple for cargo-dist naming
    local target="${arch}-${os}"
    echo -e "Installing RAPS ${BOLD}v${VERSION}${NC} for ${BOLD}${target}${NC}..."
    echo ""

    # Construct download URLs (cargo-dist naming convention)
    local archive_name="raps-cli-${target}.tar.xz"
    local download_url="${GITHUB_RELEASES}/download/v${VERSION}/${archive_name}"
    local checksum_url="${GITHUB_RELEASES}/download/v${VERSION}/${archive_name}.sha256"

    # Create temp directory
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    # Download binary archive
    info "Downloading ${archive_name}..."
    if ! download "$download_url" "${tmp_dir}/${archive_name}"; then
        error "Failed to download RAPS. Check your internet connection."
        error "URL: $download_url"
        exit 1
    fi
    success "Downloaded"

    # Download and verify checksum
    info "Verifying checksum..."
    if download "$checksum_url" "${tmp_dir}/${archive_name}.sha256" 2>/dev/null; then
        local expected_checksum
        # cargo-dist .sha256 files contain just the hash followed by filename
        expected_checksum=$(awk '{print $1}' "${tmp_dir}/${archive_name}.sha256")
        if [ -n "$expected_checksum" ]; then
            if ! verify_checksum "${tmp_dir}/${archive_name}" "$expected_checksum"; then
                error "Checksum verification failed. The download may be corrupted."
                exit 1
            fi
            success "Checksum verified"
        else
            warn "Checksum file empty, skipping verification"
        fi
    else
        warn "Could not download checksum file, skipping verification"
    fi

    # Create install directory
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    if [ ! -w "$INSTALL_DIR" ]; then
        error "Cannot write to ${INSTALL_DIR}. Check permissions or specify RAPS_INSTALL_DIR"
        exit 1
    fi

    # Extract binary (.tar.xz format)
    tar -xJf "${tmp_dir}/${archive_name}" -C "$tmp_dir"

    # Find and move binary
    local binary_path
    binary_path=$(find "$tmp_dir" -name "raps" -type f -executable 2>/dev/null | head -1)
    if [ -z "$binary_path" ]; then
        # Try without executable check (might not work in all environments)
        binary_path=$(find "$tmp_dir" -name "raps" -type f 2>/dev/null | head -1)
    fi

    if [ -z "$binary_path" ]; then
        error "Could not find raps binary in archive"
        exit 1
    fi

    mv "$binary_path" "${INSTALL_DIR}/raps"
    chmod +x "${INSTALL_DIR}/raps"
    success "Installed"

    # Update PATH
    if [ -z "${RAPS_NO_MODIFY_PATH}" ]; then
        info "Updating PATH..."
        update_path
    else
        warn "Skipping PATH modification (RAPS_NO_MODIFY_PATH is set)"
    fi

    # Verify installation
    echo ""
    info "Verifying installation..."
    export PATH="${INSTALL_DIR}:${PATH}"
    local installed_version
    if installed_version=$("${INSTALL_DIR}/raps" --version 2>&1); then
        success "raps ${installed_version} installed successfully!"
    else
        error "Installation verification failed"
        error "Binary may be corrupted or incompatible with your system"
        exit 1
    fi

    # Print success message
    echo ""
    echo -e "${GREEN}${BOLD}Installation complete!${NC}"
    echo ""
    echo "To get started, run:"
    echo -e "  ${CYAN}raps --help${NC}"
    echo ""

    local shell_name
    shell_name=$(basename "$SHELL")
    local config_file
    config_file=$(detect_shell_config)

    if [ -z "${RAPS_NO_MODIFY_PATH}" ]; then
        echo "Note: You may need to restart your shell or run:"
        echo -e "  ${CYAN}source ${config_file}${NC}"
    else
        echo "Note: Add the following to your shell config:"
        case "$shell_name" in
            fish)
                echo -e "  ${CYAN}fish_add_path ${INSTALL_DIR}${NC}"
                ;;
            *)
                echo -e "  ${CYAN}export PATH=\"${INSTALL_DIR}:\$PATH\"${NC}"
                ;;
        esac
    fi
}

# Parse arguments
main() {
    case "${1:-}" in
        --help|-h)
            show_help
            exit 0
            ;;
        --uninstall)
            uninstall
            exit 0
            ;;
        "")
            install
            ;;
        *)
            error "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
}

main "$@"
