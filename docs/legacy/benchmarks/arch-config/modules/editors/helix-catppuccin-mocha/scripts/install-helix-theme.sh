#!/usr/bin/env bash
# Install Catppuccin Mocha theme for Helix editor
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Setting up Catppuccin Mocha theme for Helix editor...${NC}"

# Determine the actual user (not root if running with sudo)
if [ -n "${SUDO_USER:-}" ]; then
    ACTUAL_USER="$SUDO_USER"
    USER_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
else
    ACTUAL_USER="$USER"
    USER_HOME="$HOME"
fi

echo -e "${BLUE}Installing theme for user: ${ACTUAL_USER}${NC}"

# Helix config directories
HELIX_CONFIG_DIR="${USER_HOME}/.config/helix"
HELIX_THEMES_DIR="${HELIX_CONFIG_DIR}/themes"
HELIX_CONFIG_FILE="${HELIX_CONFIG_DIR}/config.toml"

# Module theme location
MODULE_DIR="${ARCH_CONFIG_DIR:-/home/${ACTUAL_USER}/.config/arch-config}/modules/editors/helix-catppuccin-mocha"
THEME_SOURCE="${MODULE_DIR}/themes/catppuccin_mocha.toml"

# Check if theme source exists
if [ ! -f "$THEME_SOURCE" ]; then
    echo -e "${RED}Error: Theme file not found at ${THEME_SOURCE}${NC}" >&2
    exit 1
fi

# Create Helix config directories if they don't exist
echo -e "${BLUE}Creating Helix configuration directories...${NC}"
sudo -u "$ACTUAL_USER" mkdir -p "$HELIX_THEMES_DIR"

# Copy theme file
echo -e "${BLUE}Installing Catppuccin Mocha theme...${NC}"
sudo -u "$ACTUAL_USER" cp "$THEME_SOURCE" "$HELIX_THEMES_DIR/"

# Update or create config.toml with theme setting
if [ -f "$HELIX_CONFIG_FILE" ]; then
    # Check if theme line already exists
    if grep -q "^theme = " "$HELIX_CONFIG_FILE"; then
        echo -e "${YELLOW}Updating existing theme setting in config.toml...${NC}"
        sudo -u "$ACTUAL_USER" sed -i 's/^theme = .*/theme = "catppuccin_mocha"/' "$HELIX_CONFIG_FILE"
    else
        echo -e "${YELLOW}Adding theme setting to existing config.toml...${NC}"
        # Add theme at the top of the file
        sudo -u "$ACTUAL_USER" sh -c "echo 'theme = \"catppuccin_mocha\"' | cat - '$HELIX_CONFIG_FILE' > '$HELIX_CONFIG_FILE.tmp' && mv '$HELIX_CONFIG_FILE.tmp' '$HELIX_CONFIG_FILE'"
    fi
else
    echo -e "${BLUE}Creating new config.toml with Catppuccin Mocha theme...${NC}"
    sudo -u "$ACTUAL_USER" tee "$HELIX_CONFIG_FILE" > /dev/null << 'EOF'
theme = "catppuccin_mocha"

[editor]
line-number = "relative"
cursorline = true
color-modes = true

[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"
EOF
fi

# Set proper permissions
sudo -u "$ACTUAL_USER" chmod 644 "$HELIX_THEMES_DIR/catppuccin_mocha.toml"
sudo -u "$ACTUAL_USER" chmod 644 "$HELIX_CONFIG_FILE"

echo ""
echo -e "${GREEN}✓ Catppuccin Mocha theme installed successfully!${NC}"
echo ""
echo -e "${BLUE}Theme location:${NC} ${HELIX_THEMES_DIR}/catppuccin_mocha.toml"
echo -e "${BLUE}Config file:${NC} ${HELIX_CONFIG_FILE}"
echo ""
echo -e "${YELLOW}Note:${NC} The theme has been set in your config.toml"
echo -e "      Open Helix and the Catppuccin Mocha theme will be active!"
echo ""
