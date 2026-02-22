#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing intrusion detection..."
sudo systemctl disable --now iron-integrity-scan.timer 2>/dev/null || true
sudo rm -f /etc/systemd/system/iron-integrity-scan.timer
sudo rm -f /etc/systemd/system/iron-integrity-scan.service
sudo systemctl daemon-reload
log_success "Intrusion detection removed"
