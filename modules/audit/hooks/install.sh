#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

RULES_SRC="$SCRIPT_DIR/../configs/audit-rules.conf"
RULES_DST="/etc/audit/rules.d/99-iron-security.rules"

log_info "Deploying audit framework..."

# Deploy audit rules
safe_copy "$RULES_SRC" "$RULES_DST"
register_rollback "sudo rm -f '$RULES_DST'"

# Enable and start auditd
service_enable_start auditd
register_rollback "service_disable_stop auditd"

# Load rules
sudo augenrules --load 2>/dev/null || sudo auditctl -R "$RULES_DST" 2>/dev/null || true

clear_trap_rollback
log_success "Audit framework configured"
