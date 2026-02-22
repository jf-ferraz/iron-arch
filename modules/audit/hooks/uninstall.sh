#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing audit configuration..."
sudo rm -f /etc/audit/rules.d/99-iron-security.rules
sudo augenrules --load 2>/dev/null || true
log_success "Audit rules removed"
