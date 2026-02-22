#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Disabling AppArmor..."
service_disable_stop apparmor

# Remove kernel params from GRUB
GRUB_CFG="/etc/default/grub"
if [[ -f "$GRUB_CFG" ]]; then
    restore_file "$GRUB_CFG"
    sudo grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true
fi

log_success "AppArmor disabled"
