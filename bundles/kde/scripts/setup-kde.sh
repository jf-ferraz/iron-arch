#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../../scripts/lib/iron-hooks-common.sh"

log_info "Setting up KDE Plasma..."

# Enable SDDM
sudo systemctl enable sddm.service 2>/dev/null || true

# KWin performance tweaks
KWIN_CFG="$HOME/.config/kwinrc"
mkdir -p "$(dirname "$KWIN_CFG")"
if [[ ! -f "$KWIN_CFG" ]] || ! grep -q "GLCore" "$KWIN_CFG" 2>/dev/null; then
    cat <<'KWIN' >> "$KWIN_CFG"

[Compositing]
OpenGLIsUnsafe=false
GLCore=true
LatencyPolicy=Low

[Wayland]
InputMethod=
EnablePrimarySelection=true
KWIN
fi

# Disable Baloo file indexer
BALOO_CFG="$HOME/.config/baloofilerc"
mkdir -p "$(dirname "$BALOO_CFG")"
cat <<'BALOO' > "$BALOO_CFG"
[Basic Settings]
Indexing-Enabled=false
BALOO
balooctl6 disable 2>/dev/null || balooctl disable 2>/dev/null || true

log_success "KDE Plasma configured"
