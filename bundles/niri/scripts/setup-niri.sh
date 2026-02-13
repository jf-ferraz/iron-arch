#!/usr/bin/env bash
#
# Iron Post-Install: Niri Setup
# Configures Niri-specific settings after bundle installation

set -euo pipefail

echo "Setting up Niri bundle..."

# Enable user services
systemctl --user enable --now pipewire.service
systemctl --user enable --now pipewire-pulse.service
systemctl --user enable --now wireplumber.service

# Create default config directory
mkdir -p ~/.config/niri

# Set XDG environment variables
if ! grep -q "XDG_CURRENT_DESKTOP=niri" ~/.profile 2>/dev/null; then
    cat >> ~/.profile << 'EOF'

# Niri environment
export XDG_CURRENT_DESKTOP=niri
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=niri
EOF
fi

echo "Niri setup complete!"
echo "Log out and select Niri from your display manager."
