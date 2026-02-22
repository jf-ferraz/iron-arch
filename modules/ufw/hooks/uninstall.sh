#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Disabling UFW firewall..."
sudo ufw disable 2>/dev/null || true
service_disable_stop ufw
log_success "UFW firewall disabled"
