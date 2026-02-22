#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing DNS security configuration..."

service_disable_stop stubby

# Remove systemd-resolved override
sudo rm -f /etc/systemd/resolved.conf.d/iron-stubby.conf
if systemctl is-active --quiet systemd-resolved 2>/dev/null; then
    sudo systemctl restart systemd-resolved
fi

# Restore resolv.conf
sudo chattr -i /etc/resolv.conf 2>/dev/null || true
restore_file /etc/resolv.conf

log_success "DNS security removed"
