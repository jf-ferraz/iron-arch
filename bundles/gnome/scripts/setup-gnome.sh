#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../../scripts/lib/iron-hooks-common.sh"

log_info "Setting up GNOME..."

# Enable GDM
sudo systemctl enable gdm.service 2>/dev/null || true

# Disable tracker miners for better performance
if command_exists gsettings; then
    gsettings set org.freedesktop.Tracker3.Miner.Files crawling-interval -2 2>/dev/null || true
    gsettings set org.freedesktop.Tracker3.Miner.Files enable-monitors false 2>/dev/null || true
fi

# Mask tracker systemd services
systemctl --user mask tracker-miner-fs-3.service 2>/dev/null || true
systemctl --user mask tracker-extract-3.service 2>/dev/null || true

# Ensure GDM uses Wayland
GDM_CONF="/etc/gdm/custom.conf"
if [[ -f "$GDM_CONF" ]]; then
    if grep -q "#WaylandEnable=false" "$GDM_CONF"; then
        sudo sed -i 's/#WaylandEnable=false/WaylandEnable=true/' "$GDM_CONF"
    fi
fi

log_success "GNOME configured"
