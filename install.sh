#!/usr/bin/env bash
# Fluvio install script
# This script downloads and installs the Fluvio Version Manager (FVM) and Fluvio CLI

set -e

# Constants
readonly FVM_HOME="${FVM_HOME:-$HOME/.fvm}"
readonly FLUVIO_HOME="${FLUVIO_HOME:-$HOME/.fluvio}"
readonly FVM_BIN="${FVM_HOME}/bin"
readonly FLUVIO_BIN="${FLUVIO_HOME}/bin"
readonly GITHUB_REPO="${GITHUB_REPO:-fluvio-community/fluvio}"
readonly GITHUB_API="https://api.github.com"
readonly GITHUB_RELEASES="https://github.com/${GITHUB_REPO}/releases"

# Color output functions
color_print() {
    local color=$1
    shift
    if [ -t 1 ]; then
        echo -e "\033[${color}m$@\033[0m"
    else
        echo "$@"
    fi
}

info() { color_print "34" "ℹ $@"; }
success() { color_print "32" "✓ $@"; }
error() { color_print "31" "✗ $@"; }

# Detect OS and architecture
detect_platform() {
    local os
    local arch

    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="apple-darwin"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Windows is not directly supported. Please use WSL2"
            exit 1
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        armv7l)
            arch="armv7"
            ;;
        armv6l)
            error "armv6l architecture is not supported"
            exit 1
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    # Construct target triple
    if [ "$os" = "linux" ]; then
        echo "${arch}-unknown-linux-musl"
    else
        echo "${arch}-${os}"
    fi
}

# Get the latest release tag from GitHub
get_latest_release() {
    local tag

    # Try to use GitHub API to get latest release
    if command -v curl >/dev/null 2>&1; then
        tag=$(curl -fsSL "${GITHUB_API}/repos/${GITHUB_REPO}/releases/latest" \
            | grep '"tag_name":' \
            | sed -E 's/.*"tag_name": *"v?([^"]+)".*/\1/' 2>/dev/null)
    fi

    # If that failed, try using git ls-remote
    if [ -z "$tag" ] && command -v git >/dev/null 2>&1; then
        tag=$(git ls-remote --tags --refs --sort='v:refname' \
            "https://github.com/${GITHUB_REPO}.git" \
            | tail -n1 \
            | sed 's/.*refs\/tags\/v\?//' 2>/dev/null)
    fi

    # Fallback to 'latest' if we couldn't determine a version
    if [ -z "$tag" ]; then
        echo "latest"
    else
        echo "$tag"
    fi
}

# Download a file from GitHub releases
download_file() {
    local url=$1
    local output=$2

    info "Downloading from ${url}"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Extract binary from zip archive
extract_binary() {
    local zip_file=$1
    local output_dir=$2
    local binary_name=$3

    if command -v unzip >/dev/null 2>&1; then
        unzip -q -o "$zip_file" -d "$output_dir"
    else
        error "unzip not found. Please install unzip."
        exit 1
    fi

    # Find and move the binary
    local binary_path=$(find "$output_dir" -name "$binary_name" -type f | head -n 1)
    if [ -n "$binary_path" ]; then
        mv "$binary_path" "$output_dir/$binary_name"
        chmod +x "$output_dir/$binary_name"
    else
        error "Binary $binary_name not found in archive"
        exit 1
    fi
}

# Install FVM and Fluvio
install_fluvio() {
    local version="${VERSION:-$(get_latest_release)}"
    local target=$(detect_platform)
    local fluvio_version="${FLUVIO_VERSION:-stable}"

    info "Installing FVM from version: ${version}"
    info "Target platform: ${target}"

    # Create directories
    mkdir -p "${FLUVIO_HOME}"

    # Download FVM from GitHub releases
    local fvm_url="${GITHUB_RELEASES}/download/v${version}/fvm-${target}.zip"
    local fvm_zip="${FLUVIO_HOME}/fvm.zip"

    info "Downloading Fluvio Version Manager (FVM)..."
    if ! download_file "$fvm_url" "$fvm_zip"; then
        error "Failed to download FVM from ${fvm_url}"
        error "Please check if version ${version} exists for your platform"
        exit 1
    fi

    info "Extracting FVM..."
    if ! extract_binary "$fvm_zip" "${FLUVIO_HOME}" "fvm"; then
        error "Failed to extract FVM"
        exit 1
    fi
    rm -f "$fvm_zip"

    # Run fvm self install
    info "Installing FVM..."
    "${FLUVIO_HOME}/fvm" self install

    success "FVM installed successfully"

    # Install Fluvio using FVM
    info "Installing Fluvio ${fluvio_version} using FVM..."
    "${FVM_BIN}/fvm" install "${fluvio_version}"

    success "Fluvio installed successfully"
}

# Add Fluvio to PATH
setup_path() {
    local shell_rc=""

    # Detect shell configuration file
    if [ -n "$BASH_VERSION" ]; then
        if [ -f "$HOME/.bashrc" ]; then
            shell_rc="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            shell_rc="$HOME/.bash_profile"
        fi
    elif [ -n "$ZSH_VERSION" ]; then
        shell_rc="$HOME/.zshrc"
    elif [ -f "$HOME/.profile" ]; then
        shell_rc="$HOME/.profile"
    fi

    # Check if already in PATH
    if echo "$PATH" | grep -q "$FLUVIO_BIN"; then
        return 0
    fi

    # Add to PATH in shell config
    if [ -n "$shell_rc" ]; then
        if ! grep -q "FLUVIO_HOME" "$shell_rc"; then
            echo '' >> "$shell_rc"
            echo '# Fluvio' >> "$shell_rc"
            echo "export FVM_HOME=\"${FVM_HOME}\"" >> "$shell_rc"
            echo "export FLUVIO_HOME=\"${FLUVIO_HOME}\"" >> "$shell_rc"
            echo 'export PATH="$FVM_HOME/bin:$FLUVIO_HOME/bin:$PATH"' >> "$shell_rc"
            info "Added Fluvio to PATH in ${shell_rc}"
        fi
    fi

    # Add to current session
    export PATH="${FVM_BIN}:${FLUVIO_BIN}:$PATH"
}

# Print post-installation message
print_success_message() {
    echo ""
    success "Fluvio has been installed successfully!"
    echo ""
    info "FVM is installed in: ${FVM_HOME}"
    info "Fluvio is installed in: ${FLUVIO_HOME}"
    echo ""
    info "To get started, run:"
    echo ""
    echo "  # Add Fluvio to your PATH (if not done automatically)"
    echo "  export PATH=\"\$HOME/.fvm/bin:\$HOME/.fluvio/bin:\$PATH\""
    echo ""
    echo "  # Start a local Fluvio cluster"
    echo "  fluvio cluster start"
    echo ""
    echo "  # Create a topic"
    echo "  fluvio topic create hello-fluvio"
    echo ""
    echo "  # Produce messages"
    echo "  fluvio produce hello-fluvio"
    echo ""
    echo "  # Consume messages (in another terminal)"
    echo "  fluvio consume hello-fluvio -B"
    echo ""
}

# Main installation flow
main() {
    info "Starting Fluvio installation..."
    echo ""

    # Check for required commands
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if ! command -v unzip >/dev/null 2>&1; then
        error "unzip not found. Please install unzip."
        exit 1
    fi

    # Install Fluvio
    install_fluvio

    # Setup PATH
    setup_path

    # Print success message
    print_success_message
}

# Run main
main
