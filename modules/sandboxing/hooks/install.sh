#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring Firejail sandboxing..."

# Create Firejail symlinks for common applications
if command_exists firecfg; then
    sudo firecfg 2>/dev/null || true
    register_rollback "sudo firecfg --clean 2>/dev/null || true"
fi

# Configure global settings
FIREJAIL_CONF="/etc/firejail/firejail.config"
if [[ -f "$FIREJAIL_CONF" ]]; then
    backup_file "$FIREJAIL_CONF"
    register_rollback "restore_file '$FIREJAIL_CONF'"
fi

clear_trap_rollback
log_success "Firejail sandboxing configured"
