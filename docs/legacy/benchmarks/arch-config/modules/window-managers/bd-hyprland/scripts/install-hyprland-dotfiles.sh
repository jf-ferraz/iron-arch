#!/usr/bin/env bash
# Post-install hook for bdots-hypr module
# Dotfiles are now handled by dcli's symlink system
# This script only handles additional configuration that can't be symlinked

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Determine the user (handles both sudo and non-sudo cases)
TARGET_USER="${SUDO_USER:-$USER}"
TARGET_HOME="/home/${TARGET_USER}"
CONFIG_DIR="${TARGET_HOME}/.config"
ARCH_CONFIG_DIR="${ARCH_CONFIG_DIR:-${TARGET_HOME}/.config/arch-config}"

echo -e "${BLUE}Configuring Hyprland environment...${NC}"
echo ""
echo -e "${YELLOW}Note: Dotfiles are now managed via symlinks by dcli${NC}"
echo ""

# Apply GTK theme settings with gsettings
echo -e "${BLUE}Applying GTK theme settings...${NC}"

# Run gsettings as the target user
apply_gsettings() {
  if [ "$EUID" -eq 0 ]; then
    sudo -u "$TARGET_USER" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u $TARGET_USER)/bus" gsettings "$@" 2>/dev/null || true
  else
    gsettings "$@" 2>/dev/null || true
  fi
}

# Apply theme settings
apply_gsettings set org.gnome.desktop.interface gtk-theme 'catppuccin-mocha-mauve-standard+default'
apply_gsettings set org.gnome.desktop.interface icon-theme 'Tela-purple-dark'
apply_gsettings set org.gnome.desktop.interface cursor-theme 'Bibata-Modern-Ice'
apply_gsettings set org.gnome.desktop.interface font-name 'Inter Variable 10'
apply_gsettings set org.gnome.desktop.interface cursor-size 24

echo -e "${GREEN}GTK theme settings applied via gsettings${NC}"
echo ""

# Create .gtkrc-2.0 for GTK2 applications
echo -e "${BLUE}Creating GTK2 configuration...${NC}"
cat > "${TARGET_HOME}/.gtkrc-2.0" << 'EOF'
gtk-theme-name="catppuccin-mocha-mauve-standard+default"
gtk-icon-theme-name="Tela-purple-dark"
gtk-font-name="Inter Variable 10"
gtk-cursor-theme-name="Bibata-Modern-Ice"
gtk-cursor-theme-size=24
EOF

if [ "$EUID" -eq 0 ]; then
  chown "$TARGET_USER:$TARGET_USER" "${TARGET_HOME}/.gtkrc-2.0"
fi

echo -e "${GREEN}GTK2 configuration created${NC}"
echo ""

# Set default cursor theme
echo -e "${BLUE}Setting default cursor theme...${NC}"
mkdir -p "${TARGET_HOME}/.icons/default"
cat > "${TARGET_HOME}/.icons/default/index.theme" << 'EOF'
[Icon Theme]
Inherits=Bibata-Modern-Ice
EOF

if [ "$EUID" -eq 0 ]; then
  chown -R "$TARGET_USER:$TARGET_USER" "${TARGET_HOME}/.icons"
fi

echo -e "${GREEN}Default cursor theme set${NC}"
echo ""

# Set cursor in Xresources
echo -e "${BLUE}Creating Xresources for cursor...${NC}"
cat > "${TARGET_HOME}/.Xresources" << 'EOF'
Xcursor.theme: Bibata-Modern-Ice
Xcursor.size: 24
EOF

if [ "$EUID" -eq 0 ]; then
  chown "$TARGET_USER:$TARGET_USER" "${TARGET_HOME}/.Xresources"
fi

echo -e "${GREEN}Xresources created${NC}"
echo ""

# Set up wallpaper directory and default wallpaper
echo -e "${BLUE}Setting up wallpaper...${NC}"
HYPR_CONFIG="${CONFIG_DIR}/hypr"
WALLPAPER_DIR="${HYPR_CONFIG}/wallpapers"
DEFAULT_WALLPAPER="${ARCH_CONFIG_DIR}/wallpapers/37.png"

# Create wallpapers directory if it doesn't exist
mkdir -p "$WALLPAPER_DIR"

# Create symlink to default wallpaper if it exists
if [ -f "$DEFAULT_WALLPAPER" ]; then
  ln -sf "$DEFAULT_WALLPAPER" "${WALLPAPER_DIR}/wallpaper.png"
  if [ "$EUID" -eq 0 ]; then
    chown -h "$TARGET_USER:$TARGET_USER" "${WALLPAPER_DIR}/wallpaper.png"
    chown "$TARGET_USER:$TARGET_USER" "$WALLPAPER_DIR"
  fi
  echo -e "${GREEN}Wallpaper symlink created${NC}"
else
  echo -e "${YELLOW}Warning: Default wallpaper not found${NC}"
fi
echo ""

echo -e "${GREEN}Hyprland environment configuration complete!${NC}"
echo ""
echo -e "${BLUE}Dotfiles are symlinked from arch-config/packages/modules/bdots-hypr/dotfiles/${NC}"
echo ""
echo -e "${BLUE}To apply changes:${NC}"
echo "  - Reload Hyprland: SUPER+SHIFT+R or 'hyprctl reload'"
echo "  - For full effect, log out and log back in"
echo ""
