#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring intrusion detection..."

# Initialize AIDE database
if command_exists aide; then
    log_info "Initializing AIDE database (this may take a while)..."
    sudo aide --init 2>/dev/null || true
    if [[ -f /var/lib/aide/aide.db.new ]]; then
        sudo mv /var/lib/aide/aide.db.new /var/lib/aide/aide.db
    fi
fi

# Update rkhunter signatures
if command_exists rkhunter; then
    log_info "Updating rkhunter signatures..."
    sudo rkhunter --update 2>/dev/null || true
    sudo rkhunter --propupd 2>/dev/null || true
fi

# Create weekly scan timer
sudo mkdir -p /etc/systemd/system
cat <<'TIMER' | sudo tee /etc/systemd/system/iron-integrity-scan.timer >/dev/null
[Unit]
Description=Iron weekly integrity scan

[Timer]
OnCalendar=weekly
Persistent=true
RandomizedDelaySec=3600

[Install]
WantedBy=timers.target
TIMER

cat <<'SERVICE' | sudo tee /etc/systemd/system/iron-integrity-scan.service >/dev/null
[Unit]
Description=Iron integrity scan (AIDE + rkhunter)

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'aide --check 2>/dev/null; rkhunter --check --skip-keypress 2>/dev/null'
SERVICE

sudo systemctl daemon-reload
service_enable iron-integrity-scan.timer

clear_trap_rollback
log_success "Intrusion detection configured"
