#!/usr/bin/env bash
#
# Iron Installation Script
# Installs Iron - Declarative Arch Linux configuration management
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/laraj/iron/main/scripts/install.sh | bash
#   or
#   ./scripts/install.sh [--prefix /usr/local] [--no-completions] [--uninstall]
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Defaults
PREFIX="${PREFIX:-/usr/local}"
INSTALL_COMPLETIONS=true
UNINSTALL=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --no-completions)
            INSTALL_COMPLETIONS=false
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Iron Installation Script"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --prefix DIR       Install prefix (default: /usr/local)"
            echo "  --no-completions   Skip shell completion installation"
            echo "  --uninstall        Uninstall Iron"
            echo "  --verbose, -v      Verbose output"
            echo "  --help, -h         Show this help"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

log() {
    echo -e "${BLUE}==>${NC} $1"
}

success() {
    echo -e "${GREEN}==>${NC} $1"
}

warn() {
    echo -e "${YELLOW}Warning:${NC} $1"
}

error() {
    echo -e "${RED}Error:${NC} $1"
    exit 1
}

check_dependencies() {
    log "Checking dependencies..."

    local missing=()

    if ! command -v cargo &> /dev/null; then
        missing+=("cargo (install via rustup)")
    fi

    if ! command -v git &> /dev/null; then
        missing+=("git")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing dependencies: ${missing[*]}"
    fi

    success "All dependencies satisfied"
}

uninstall() {
    log "Uninstalling Iron..."

    local files=(
        "$PREFIX/bin/iron"
        "/usr/share/bash-completion/completions/iron"
        "/usr/share/zsh/site-functions/_iron"
        "/usr/share/fish/vendor_completions.d/iron.fish"
    )

    for file in "${files[@]}"; do
        if [[ -f "$file" ]]; then
            if [[ -w "$(dirname "$file")" ]]; then
                rm -f "$file"
                log "Removed $file"
            else
                sudo rm -f "$file"
                log "Removed $file (sudo)"
            fi
        fi
    done

    success "Iron uninstalled successfully"
    exit 0
}

build() {
    log "Building Iron..."

    # Check if we're in the iron directory
    if [[ ! -f "Cargo.toml" ]]; then
        error "Please run this script from the Iron repository root"
    fi

    # Build release binary
    cargo build --release --all-features

    if [[ ! -f "target/release/iron" ]]; then
        error "Build failed - binary not found"
    fi

    success "Build complete"
}

install_binary() {
    log "Installing Iron binary to $PREFIX/bin..."

    local bindir="$PREFIX/bin"

    # Create directory if needed
    if [[ ! -d "$bindir" ]]; then
        if [[ -w "$(dirname "$bindir")" ]]; then
            mkdir -p "$bindir"
        else
            sudo mkdir -p "$bindir"
        fi
    fi

    # Install binary
    if [[ -w "$bindir" ]]; then
        install -m755 "target/release/iron" "$bindir/iron"
    else
        sudo install -m755 "target/release/iron" "$bindir/iron"
    fi

    success "Binary installed to $bindir/iron"
}

install_completions() {
    if [[ "$INSTALL_COMPLETIONS" != "true" ]]; then
        log "Skipping shell completions (--no-completions)"
        return
    fi

    log "Installing shell completions..."

    # Generate completions if the command supports it
    if "$PREFIX/bin/iron" completions bash &> /dev/null; then
        # Bash
        local bash_dir="/usr/share/bash-completion/completions"
        if [[ -d "$bash_dir" ]]; then
            if [[ -w "$bash_dir" ]]; then
                "$PREFIX/bin/iron" completions bash > "$bash_dir/iron"
            else
                "$PREFIX/bin/iron" completions bash | sudo tee "$bash_dir/iron" > /dev/null
            fi
            log "Bash completions installed"
        fi

        # Zsh
        local zsh_dir="/usr/share/zsh/site-functions"
        if [[ -d "$zsh_dir" ]]; then
            if [[ -w "$zsh_dir" ]]; then
                "$PREFIX/bin/iron" completions zsh > "$zsh_dir/_iron"
            else
                "$PREFIX/bin/iron" completions zsh | sudo tee "$zsh_dir/_iron" > /dev/null
            fi
            log "Zsh completions installed"
        fi

        # Fish
        local fish_dir="/usr/share/fish/vendor_completions.d"
        if [[ -d "$fish_dir" ]]; then
            if [[ -w "$fish_dir" ]]; then
                "$PREFIX/bin/iron" completions fish > "$fish_dir/iron.fish"
            else
                "$PREFIX/bin/iron" completions fish | sudo tee "$fish_dir/iron.fish" > /dev/null
            fi
            log "Fish completions installed"
        fi

        success "Shell completions installed"
    else
        warn "Shell completions not available (iron completions command not found)"
    fi
}

verify_installation() {
    log "Verifying installation..."

    if ! command -v iron &> /dev/null; then
        # Check if it's in the prefix
        if [[ -x "$PREFIX/bin/iron" ]]; then
            warn "Iron installed but $PREFIX/bin is not in PATH"
            echo "Add this to your shell profile:"
            echo "  export PATH=\"$PREFIX/bin:\$PATH\""
        else
            error "Installation verification failed"
        fi
    else
        local version
        version=$(iron --version 2>/dev/null || echo "unknown")
        success "Iron installed successfully: $version"
    fi
}

main() {
    echo ""
    echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}         Iron Installation Script          ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}   Less is More - Turning Arch into Iron   ${BLUE}║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
    echo ""

    if [[ "$UNINSTALL" == "true" ]]; then
        uninstall
    fi

    check_dependencies
    build
    install_binary
    install_completions
    verify_installation

    echo ""
    success "Installation complete!"
    echo ""
    echo "Get started:"
    echo "  iron init          # Initialize Iron in current directory"
    echo "  iron               # Launch TUI dashboard"
    echo "  iron --help        # Show all commands"
    echo ""
}

main "$@"
