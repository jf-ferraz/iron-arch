#!/usr/bin/env bash
# Iron Hooks Common Library
# Shared functions for iron-arch module hook scripts
#
# Source this from hook scripts via:
#   source "$(dirname "$0")/../../scripts/lib/iron-hooks-common.sh"

set -euo pipefail

# =========================================================================
# Logging (stdout, captured by iron-arch)
# =========================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }

# =========================================================================
# Package management (pacman wrappers)
# =========================================================================

pkg_is_installed() {
    pacman -Q "$1" &>/dev/null
}

pkg_install() {
    local missing=()
    for pkg in "$@"; do
        if ! pkg_is_installed "$pkg"; then
            missing+=("$pkg")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_info "Installing: ${missing[*]}"
        sudo pacman -S --needed --noconfirm "${missing[@]}"
    fi
}

pkg_remove() {
    local installed=()
    for pkg in "$@"; do
        if pkg_is_installed "$pkg"; then
            installed+=("$pkg")
        fi
    done
    if [[ ${#installed[@]} -gt 0 ]]; then
        log_info "Removing: ${installed[*]}"
        sudo pacman -Rns --noconfirm "${installed[@]}" || true
    fi
}

# =========================================================================
# Service management (systemctl wrappers)
# =========================================================================

service_enable() {
    sudo systemctl enable "$1" 2>/dev/null || true
}

service_start() {
    sudo systemctl start "$1" 2>/dev/null || true
}

service_stop() {
    sudo systemctl stop "$1" 2>/dev/null || true
}

service_disable() {
    sudo systemctl disable "$1" 2>/dev/null || true
}

service_is_active() {
    systemctl is-active --quiet "$1" 2>/dev/null
}

service_enable_start() {
    service_enable "$1"
    service_start "$1"
}

service_disable_stop() {
    service_stop "$1"
    service_disable "$1"
}

# =========================================================================
# File backup / restore
# =========================================================================

backup_file() {
    local file="$1"
    if [[ -f "$file" ]]; then
        local backup="${file}.iron-backup"
        if [[ ! -f "$backup" ]]; then
            sudo cp "$file" "$backup"
            log_info "Backed up: $file"
        fi
    fi
}

restore_file() {
    local file="$1"
    local backup="${file}.iron-backup"
    if [[ -f "$backup" ]]; then
        sudo mv "$backup" "$file"
        log_info "Restored: $file"
    fi
}

# =========================================================================
# Rollback mechanism
# =========================================================================

_ROLLBACK_COMMANDS=()

register_rollback() {
    _ROLLBACK_COMMANDS+=("$*")
}

execute_rollback() {
    log_warn "Executing rollback..."
    for ((i=${#_ROLLBACK_COMMANDS[@]}-1; i>=0; i--)); do
        log_info "Rollback: ${_ROLLBACK_COMMANDS[$i]}"
        eval "${_ROLLBACK_COMMANDS[$i]}" || true
    done
    _ROLLBACK_COMMANDS=()
}

trap_rollback() {
    trap 'execute_rollback; exit 1' ERR INT TERM
}

clear_trap_rollback() {
    trap - ERR INT TERM
    _ROLLBACK_COMMANDS=()
}

# =========================================================================
# Utilities
# =========================================================================

command_exists() {
    command -v "$1" &>/dev/null
}

ensure_dir() {
    [[ -d "$1" ]] || mkdir -p "$1"
}

# Idempotent line insertion
ensure_line_in_file() {
    local file="$1"
    local line="$2"
    if ! grep -qF "$line" "$file" 2>/dev/null; then
        echo "$line" | sudo tee -a "$file" >/dev/null
    fi
}

# Safe file copy with backup
safe_copy() {
    local src="$1"
    local dst="$2"
    backup_file "$dst"
    sudo cp "$src" "$dst"
    log_info "Deployed: $dst"
}
