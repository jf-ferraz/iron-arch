#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../../scripts/lib/iron-hooks-common.sh"

log_info "Setting up COSMIC desktop..."

# Enable COSMIC greeter
sudo systemctl enable cosmic-greeter.service 2>/dev/null || true

# Set XDG defaults
if command_exists xdg-mime; then
    xdg-mime default cosmic-files.desktop inode/directory 2>/dev/null || true
fi

log_success "COSMIC desktop configured"
