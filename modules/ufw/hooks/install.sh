#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring UFW firewall..."

# Set default policies
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw default deny routed

# Rate-limit SSH
sudo ufw limit ssh comment "Iron: rate-limit SSH"

# Enable UFW
sudo ufw --force enable
register_rollback "sudo ufw disable"

service_enable_start ufw

clear_trap_rollback
log_success "UFW firewall configured"
