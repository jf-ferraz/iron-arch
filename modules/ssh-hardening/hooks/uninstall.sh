#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing SSH hardening..."
sudo rm -f /etc/ssh/sshd_config.d/99-iron-hardening.conf
sudo rm -f /etc/ssh/iron-banner.txt
sudo systemctl reload sshd 2>/dev/null || sudo systemctl reload ssh 2>/dev/null || true
log_success "SSH hardening removed"
