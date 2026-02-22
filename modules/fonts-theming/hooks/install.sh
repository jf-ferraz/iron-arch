#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Configuring fonts and theming..."

# Fontconfig
FC_DIR="$HOME/.config/fontconfig/conf.d"
mkdir -p "$FC_DIR"

cat <<'FC' > "$FC_DIR/99-iron-fonts.conf"
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
    <match target="font">
        <edit name="antialias" mode="assign"><bool>true</bool></edit>
        <edit name="hinting" mode="assign"><bool>true</bool></edit>
        <edit name="hintstyle" mode="assign"><const>hintslight</const></edit>
        <edit name="rgba" mode="assign"><const>rgb</const></edit>
        <edit name="lcdfilter" mode="assign"><const>lcddefault</const></edit>
    </match>
    <alias>
        <family>monospace</family>
        <prefer><family>JetBrainsMono Nerd Font</family></prefer>
    </alias>
    <alias>
        <family>sans-serif</family>
        <prefer><family>Inter</family></prefer>
    </alias>
</fontconfig>
FC

# Rebuild font cache
fc-cache -f 2>/dev/null || true

# GTK-3 settings
GTK3_DIR="$HOME/.config/gtk-3.0"
mkdir -p "$GTK3_DIR"
cat <<'GTK' > "$GTK3_DIR/settings.ini"
[Settings]
gtk-icon-theme-name=Papirus-Dark
gtk-cursor-theme-name=Bibata-Modern-Classic
gtk-cursor-theme-size=24
gtk-font-name=Inter 11
gtk-application-prefer-dark-theme=1
GTK

# GTK-4 settings
GTK4_DIR="$HOME/.config/gtk-4.0"
mkdir -p "$GTK4_DIR"
cat <<'GTK' > "$GTK4_DIR/settings.ini"
[Settings]
gtk-icon-theme-name=Papirus-Dark
gtk-cursor-theme-name=Bibata-Modern-Classic
gtk-cursor-theme-size=24
gtk-font-name=Inter 11
gtk-application-prefer-dark-theme=1
GTK

# Qt theming via environment variable
ENV_DIR="$HOME/.config/environment.d"
mkdir -p "$ENV_DIR"
if [[ ! -f "$ENV_DIR/iron-qt.conf" ]]; then
    echo "QT_STYLE_OVERRIDE=kvantum" > "$ENV_DIR/iron-qt.conf"
fi

log_success "Fonts and theming configured"
