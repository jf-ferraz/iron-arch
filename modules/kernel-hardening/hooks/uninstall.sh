#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

CONF_DST="/etc/sysctl.d/99-iron-security.conf"

log_info "Removing kernel hardening..."
sudo rm -f "$CONF_DST"
sudo sysctl --system >/dev/null 2>&1
log_success "Kernel hardening removed"
