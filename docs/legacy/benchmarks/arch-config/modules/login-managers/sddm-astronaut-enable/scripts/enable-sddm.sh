#!/usr/bin/env bash
# Enable SDDM display manager with Astronaut pixel_sakura theme
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Enabling SDDM display manager with Astronaut pixel_sakura theme...${NC}"
echo ""

# Check if we have root privileges
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
  exit 1
fi

THEME_NAME="sddm-astronaut-theme"
THEME_REPO="https://github.com/Keyitdev/sddm-astronaut-theme"
THEME_INSTALL_DIR="/usr/share/sddm/themes/${THEME_NAME}"

# Create temporary directory for cloning
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Clone the theme repository
echo -e "${BLUE}Cloning Astronaut theme repository...${NC}"
git clone --depth 1 "$THEME_REPO" "$TEMP_DIR/sddm-astronaut-theme"

if [ ! -d "$TEMP_DIR/sddm-astronaut-theme" ]; then
  echo -e "${RED}Error: Failed to clone theme repository${NC}" >&2
  exit 1
fi

# Install the theme
echo -e "${BLUE}Installing sddm-astronaut-theme...${NC}"

# Create theme directory
mkdir -p /usr/share/sddm/themes

# Remove existing theme if present
if [ -d "$THEME_INSTALL_DIR" ]; then
  echo -e "${YELLOW}Theme already exists, replacing...${NC}"
  rm -rf "$THEME_INSTALL_DIR"
fi

# Copy the entire theme
cp -r "$TEMP_DIR/sddm-astronaut-theme" "$THEME_INSTALL_DIR"

if [ ! -d "$THEME_INSTALL_DIR" ]; then
  echo -e "${RED}Error: Failed to install theme${NC}" >&2
  exit 1
fi

echo -e "${GREEN}Theme installed to ${THEME_INSTALL_DIR}${NC}"

# Install fonts
echo -e "${BLUE}Installing fonts...${NC}"
if [ -d "$THEME_INSTALL_DIR/Fonts" ]; then
  mkdir -p /usr/share/fonts
  cp -r "$THEME_INSTALL_DIR/Fonts/"* /usr/share/fonts/ 2>/dev/null || true
  echo -e "${GREEN}Fonts installed${NC}"
fi

# Configure SDDM to use pixel_sakura theme variant
echo -e "${BLUE}Configuring SDDM to use pixel_sakura theme...${NC}"
mkdir -p /etc/sddm.conf.d

# Update metadata.desktop to use pixel_sakura config
METADATA_FILE="${THEME_INSTALL_DIR}/metadata.desktop"
if [ -f "$METADATA_FILE" ]; then
  sed -i 's|ConfigFile=.*|ConfigFile=Themes/pixel_sakura.conf|' "$METADATA_FILE"
  echo -e "${GREEN}Set theme config to pixel_sakura${NC}"
fi

# Update pixel_sakura.conf to use 12-hour time format
PIXEL_SAKURA_CONF="${THEME_INSTALL_DIR}/Themes/pixel_sakura.conf"
if [ -f "$PIXEL_SAKURA_CONF" ]; then
  echo -e "${BLUE}Configuring 12-hour time format...${NC}"
  sed -i 's|HourFormat="HH:mm"|HourFormat="hh:mm AP"|' "$PIXEL_SAKURA_CONF"
  echo -e "${GREEN}Time format set to 12-hour${NC}"
fi

cat > /etc/sddm.conf.d/theme.conf << EOF
[Theme]
Current=${THEME_NAME}
EOF

# Configure virtual keyboard (required by the theme)
cat > /etc/sddm.conf.d/virtualkbd.conf << EOF
[General]
InputMethod=qtvirtualkeyboard
EOF

echo -e "${GREEN}SDDM configured to use ${THEME_NAME} theme with pixel_sakura variant${NC}"

echo ""
echo -e "${GREEN}Theme installation complete!${NC}"
echo ""
echo -e "${BLUE}Theme: ${NC}sddm-astronaut-theme (pixel_sakura variant)"
echo -e "${BLUE}Note: ${NC}SDDM service is enabled via the module's services configuration"
echo ""
