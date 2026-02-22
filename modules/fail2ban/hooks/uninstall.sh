#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Disabling Fail2ban..."
service_disable_stop fail2ban
sudo rm -f /etc/fail2ban/jail.local
log_success "Fail2ban disabled"
