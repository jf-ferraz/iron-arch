#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Configuring PipeWire audio and Bluetooth..."

# Enable PipeWire user services
systemctl --user enable --now pipewire.socket 2>/dev/null || true
systemctl --user enable --now pipewire-pulse.socket 2>/dev/null || true
systemctl --user enable --now wireplumber.service 2>/dev/null || true

# Low-latency PipeWire config
PW_CONF_DIR="$HOME/.config/pipewire/pipewire.conf.d"
mkdir -p "$PW_CONF_DIR"

cat <<'PWCONF' > "$PW_CONF_DIR/99-iron-lowlatency.conf"
context.properties = {
    default.clock.rate          = 48000
    default.clock.quantum       = 256
    default.clock.min-quantum   = 32
    default.clock.max-quantum   = 2048
}
PWCONF

# Enable Bluetooth
sudo systemctl enable --now bluetooth.service 2>/dev/null || true

log_success "Audio and Bluetooth configured"
