#!/usr/bin/env bash
# Iron Common Library
# Shared functions for Iron scripts

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# Check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Ensure running as user (not root)
ensure_user() {
    if [[ $EUID -eq 0 ]]; then
        error "Do not run this script as root"
        exit 1
    fi
}

# Confirm action
confirm() {
    local prompt="${1:-Continue?}"
    read -rp "$prompt [y/N] " response
    [[ "$response" =~ ^[Yy]$ ]]
}

# Create backup of file
backup_file() {
    local file="$1"
    if [[ -f "$file" ]]; then
        cp "$file" "${file}.iron-backup.$(date +%Y%m%d%H%M%S)"
    fi
}

# Detect AUR helper
detect_aur_helper() {
    for helper in paru yay pikaur trizen; do
        if command_exists "$helper"; then
            echo "$helper"
            return 0
        fi
    done
    echo "pacman"
}

# Install packages
install_packages() {
    local helper
    helper=$(detect_aur_helper)
    info "Installing packages with $helper..."
    $helper -S --needed "$@"
}
