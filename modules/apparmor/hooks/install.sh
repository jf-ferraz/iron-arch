#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Configuring AppArmor..."

# Enable AppArmor service
service_enable_start apparmor

# Add kernel parameters to GRUB if not present
GRUB_CFG="/etc/default/grub"
if [[ -f "$GRUB_CFG" ]]; then
    if ! grep -q "apparmor=1" "$GRUB_CFG"; then
        backup_file "$GRUB_CFG"
        register_rollback "restore_file '$GRUB_CFG'"
        sudo sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\(.*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 lsm=landlock,lockdown,yama,integrity,apparmor,bpf apparmor=1"/' "$GRUB_CFG"
        sudo grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true
        log_warn "Reboot required for AppArmor kernel parameters"
    fi
fi

# Set profiles to enforce mode
if command_exists aa-enforce; then
    sudo aa-enforce /etc/apparmor.d/* 2>/dev/null || true
fi

clear_trap_rollback
log_success "AppArmor configured"
