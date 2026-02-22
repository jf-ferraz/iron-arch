#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring DNS-over-TLS via Stubby..."

# Enable and start stubby
service_enable_start stubby
register_rollback "service_disable_stop stubby"

# Configure resolv.conf to use stubby
RESOLV="/etc/resolv.conf"
backup_file "$RESOLV"
register_rollback "restore_file '$RESOLV'"

# Check if systemd-resolved is in use
if systemctl is-active --quiet systemd-resolved 2>/dev/null; then
    log_info "systemd-resolved detected, configuring as DNS stub..."
    sudo mkdir -p /etc/systemd/resolved.conf.d
    cat <<'RESOLVED' | sudo tee /etc/systemd/resolved.conf.d/iron-stubby.conf >/dev/null
[Resolve]
DNS=127.0.0.1
DNSStubListener=no
RESOLVED
    sudo systemctl restart systemd-resolved
else
    log_info "Configuring resolv.conf for Stubby..."
    echo -e "nameserver 127.0.0.1\noptions edns0" | sudo tee "$RESOLV" >/dev/null
    sudo chattr +i "$RESOLV" 2>/dev/null || true
fi

clear_trap_rollback
log_success "DNS security configured"
