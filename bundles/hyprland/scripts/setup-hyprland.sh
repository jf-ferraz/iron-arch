#!/usr/bin/env bash
#
# Iron Post-Install: Hyprland Setup
# Configures Hyprland-specific settings after bundle installation

set -euo pipefail

echo "Setting up Hyprland bundle..."

# Enable user services
systemctl --user enable --now pipewire.service
systemctl --user enable --now pipewire-pulse.service
systemctl --user enable --now wireplumber.service

# Create default config directory if not exists
mkdir -p ~/.config/hypr

# Set XDG environment variables if not already set
if ! grep -q "XDG_CURRENT_DESKTOP=Hyprland" ~/.profile 2>/dev/null; then
    cat >> ~/.profile << 'EOF'

# Hyprland environment
export XDG_CURRENT_DESKTOP=Hyprland
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=Hyprland
EOF
fi

# Configure NVIDIA if present
if lspci | grep -i nvidia &>/dev/null; then
    echo "NVIDIA GPU detected, configuring for Hyprland..."
    if ! grep -q "LIBVA_DRIVER_NAME=nvidia" ~/.profile 2>/dev/null; then
        cat >> ~/.profile << 'EOF'

# NVIDIA Wayland settings
export LIBVA_DRIVER_NAME=nvidia
export GBM_BACKEND=nvidia-drm
export __GLX_VENDOR_LIBRARY_NAME=nvidia
export WLR_NO_HARDWARE_CURSORS=1
EOF
    fi
fi

echo "Hyprland setup complete!"
echo "Log out and select Hyprland from your display manager."
