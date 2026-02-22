#!/usr/bin/env bash
# Enable SDDM display manager with Catppuccin theme
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Enabling SDDM display manager with Catppuccin theme...${NC}"
echo ""

# Check if we have root privileges
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
  exit 1
fi

# Get the module directory (where this script is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_DIR="$(dirname "$SCRIPT_DIR")"
THEME_ZIP="${MODULE_DIR}/catppuccin-frappe-mauve-sddm.zip"
THEME_NAME="catppuccin-frappe-mauve"
THEME_INSTALL_DIR="/usr/share/sddm/themes/${THEME_NAME}"

# Install Catppuccin theme
if [ ! -f "$THEME_ZIP" ]; then
  echo -e "${RED}Error: Theme file not found at ${THEME_ZIP}${NC}" >&2
  exit 1
fi

echo -e "${BLUE}Installing Catppuccin theme...${NC}"

# Create temporary directory for extraction
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract theme
unzip -q "$THEME_ZIP" -d "$TEMP_DIR"

# Fix Clock.qml to remove invalid format property
CLOCK_QML="${TEMP_DIR}/${THEME_NAME}/Components/Clock.qml"
if [ -f "$CLOCK_QML" ]; then
  echo -e "${BLUE}Fixing Clock.qml component...${NC}"
  cat > "$CLOCK_QML" << 'EOF'
import QtQuick 2.15
import SddmComponents 2.0

Clock {
  id: time
  color: "#C6D0F5"
  timeFont.family: config.Font
  dateFont.family: config.Font
  anchors {
    margins: 10
    top: parent.top
    right: parent.right
  }
}
EOF
  echo -e "${GREEN}Clock component fixed${NC}"
fi

# Install theme to system directory
mkdir -p /usr/share/sddm/themes
if [ -d "$THEME_INSTALL_DIR" ]; then
  echo -e "${YELLOW}Theme already exists, replacing...${NC}"
  rm -rf "$THEME_INSTALL_DIR"
fi

cp -r "${TEMP_DIR}/${THEME_NAME}" "$THEME_INSTALL_DIR"
echo -e "${GREEN}Theme installed to ${THEME_INSTALL_DIR}${NC}"

# Configure SDDM to use the theme
echo -e "${BLUE}Configuring SDDM to use Catppuccin theme...${NC}"
mkdir -p /etc/sddm.conf.d

cat > /etc/sddm.conf.d/theme.conf << EOF
[Theme]
Current=${THEME_NAME}
EOF

echo -e "${GREEN}SDDM configured to use ${THEME_NAME} theme${NC}"

# Disable other display managers if they're enabled
echo ""
for dm in gdm lightdm lxdm; do
  if systemctl is-enabled "${dm}.service" &>/dev/null; then
    echo -e "${YELLOW}Disabling ${dm} display manager...${NC}"
    systemctl disable "${dm}.service"
    echo -e "${GREEN}${dm} disabled${NC}"
  fi
done

echo ""

# Enable SDDM
if systemctl is-enabled sddm.service &>/dev/null; then
  echo -e "${YELLOW}SDDM is already enabled${NC}"
else
  echo -e "${BLUE}Enabling SDDM service...${NC}"
  systemctl enable sddm.service
  echo -e "${GREEN}SDDM enabled${NC}"
fi

echo ""
echo -e "${GREEN}SDDM configuration complete!${NC}"
echo ""
echo -e "${BLUE}Theme: ${NC}Catppuccin Frappe Mauve (12-hour time)"
echo -e "${BLUE}Next steps:${NC}"
echo "  - Reboot your system to use SDDM with the new theme"
echo "  - Or manually restart SDDM: sudo systemctl restart sddm.service"
echo ""
