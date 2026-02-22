#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

SSHD_CONF="/etc/ssh/sshd_config.d/99-iron-hardening.conf"
BANNER="/etc/ssh/iron-banner.txt"

log_info "Deploying SSH hardening configuration..."

# Deploy hardened sshd config
safe_copy "$SCRIPT_DIR/../configs/sshd-hardening.conf" "$SSHD_CONF"
register_rollback "sudo rm -f '$SSHD_CONF'"

# Deploy SSH banner
safe_copy "$SCRIPT_DIR/../configs/ssh-banner.txt" "$BANNER"
register_rollback "sudo rm -f '$BANNER'"

# Validate configuration
if sshd -t 2>/dev/null; then
    log_success "SSH configuration validated"
else
    log_error "SSH configuration invalid, rolling back"
    execute_rollback
    exit 1
fi

# Restart sshd
sudo systemctl reload sshd 2>/dev/null || sudo systemctl reload ssh 2>/dev/null || true

clear_trap_rollback
log_success "SSH hardening applied"
