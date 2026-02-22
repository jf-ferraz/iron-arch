#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

CONF_SRC="$SCRIPT_DIR/../configs/sysctl-security.conf"
CONF_DST="/etc/sysctl.d/99-iron-security.conf"

trap_rollback
register_rollback "sudo rm -f '$CONF_DST'"

log_info "Deploying kernel hardening parameters..."
safe_copy "$CONF_SRC" "$CONF_DST"
sudo sysctl --system >/dev/null 2>&1

clear_trap_rollback
log_success "Kernel hardening applied"
