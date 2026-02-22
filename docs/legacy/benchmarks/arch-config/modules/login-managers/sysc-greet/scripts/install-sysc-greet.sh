#!/usr/bin/env bash
# Install sysc-greet configuration and setup
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Running sysc-greet post-install setup...${NC}"

# Check if running with sudo
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
    echo "Usage: sudo $0" >&2
    exit 1
fi

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$MODULE_DIR/config"

echo -e "${BLUE}Applying custom sysc-greet configuration...${NC}"

# Backup existing configs if they exist
if [ -f /etc/greetd/hyprland-greeter-config.conf ]; then
    echo -e "${YELLOW}Backing up existing hyprland-greeter-config.conf${NC}"
    cp /etc/greetd/hyprland-greeter-config.conf /etc/greetd/hyprland-greeter-config.conf.backup
fi

if [ -f /etc/greetd/config.toml ]; then
    echo -e "${YELLOW}Backing up existing config.toml${NC}"
    cp /etc/greetd/config.toml /etc/greetd/config.toml.backup
fi

if [ -f /etc/greetd/kitty.conf ]; then
    echo -e "${YELLOW}Backing up existing kitty.conf${NC}"
    cp /etc/greetd/kitty.conf /etc/greetd/kitty.conf.backup
fi

# Copy configuration files
echo -e "${BLUE}Installing greetd configurations...${NC}"
if [ -f "$CONFIG_DIR/hyprland-greeter-config.conf" ]; then
    cp "$CONFIG_DIR/hyprland-greeter-config.conf" /etc/greetd/hyprland-greeter-config.conf
    chmod 644 /etc/greetd/hyprland-greeter-config.conf
    echo -e "${GREEN}✓ Installed hyprland-greeter-config.conf${NC}"
fi

if [ -f "$CONFIG_DIR/config.toml" ]; then
    cp "$CONFIG_DIR/config.toml" /etc/greetd/config.toml
    chmod 644 /etc/greetd/config.toml
    echo -e "${GREEN}✓ Installed config.toml${NC}"
fi

if [ -f "$CONFIG_DIR/kitty.conf" ]; then
    cp "$CONFIG_DIR/kitty.conf" /etc/greetd/kitty.conf
    chmod 644 /etc/greetd/kitty.conf
    echo -e "${GREEN}✓ Installed kitty.conf${NC}"
fi

# Copy custom catppuccin wallpaper
echo -e "${BLUE}Installing custom catppuccin wallpaper...${NC}"
if [ -f "$CONFIG_DIR/wallpapers/sysc-greet-catppuccin.png" ]; then
    mkdir -p /usr/share/sysc-greet/wallpapers
    cp "$CONFIG_DIR/wallpapers/sysc-greet-catppuccin.png" /usr/share/sysc-greet/wallpapers/sysc-greet-catppuccin.png
    chmod 644 /usr/share/sysc-greet/wallpapers/sysc-greet-catppuccin.png
    echo -e "${GREEN}✓ Installed custom catppuccin wallpaper${NC}"
else
    echo -e "${YELLOW}⚠ Custom wallpaper not found at $CONFIG_DIR/wallpapers/sysc-greet-catppuccin.png${NC}"
    echo -e "${YELLOW}  Using default sysc-greet wallpaper${NC}"
fi

# Update hyprland-greeter-config.conf to use catppuccin theme
echo -e "${BLUE}Configuring catppuccin theme...${NC}"
if grep -q "sysc-greet" /etc/greetd/hyprland-greeter-config.conf; then
    sed -i 's|/usr/local/bin/sysc-greet|/usr/local/bin/sysc-greet -theme catppuccin|g' /etc/greetd/hyprland-greeter-config.conf
    echo -e "${GREEN}✓ Theme set to catppuccin${NC}"
fi

# Enable and start greetd service
echo -e "${BLUE}Enabling greetd service...${NC}"
systemctl enable greetd.service
echo -e "${GREEN}✓ greetd service enabled${NC}"

echo ""
echo -e "${GREEN}✓ sysc-greet setup completed successfully!${NC}"
echo -e "${YELLOW}NOTE: greetd will start on next boot. To test now, run: systemctl start greetd${NC}"
echo -e "${YELLOW}      Current display manager will be replaced on next boot.${NC}"
echo ""
