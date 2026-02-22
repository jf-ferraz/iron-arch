#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring Fail2ban..."

# Create local jail config
JAIL_LOCAL="/etc/fail2ban/jail.local"
if [[ ! -f "$JAIL_LOCAL" ]]; then
    cat <<'JAIL' | sudo tee "$JAIL_LOCAL" >/dev/null
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 5
banaction = ufw

[sshd]
enabled = true
port = ssh
filter = sshd
maxretry = 3
bantime = 86400

[sshd-ddos]
enabled = true
port = ssh
filter = sshd-ddos
maxretry = 6
bantime = 172800
findtime = 300

[recidive]
enabled = true
filter = recidive
bantime = 604800
findtime = 86400
maxretry = 3
JAIL
    register_rollback "sudo rm -f '$JAIL_LOCAL'"
fi

service_enable_start fail2ban

clear_trap_rollback
log_success "Fail2ban configured"
