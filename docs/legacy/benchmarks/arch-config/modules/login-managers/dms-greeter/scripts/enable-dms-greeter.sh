#!/usr/bin/env bash
# Enable DankGreeter (greetd-based display manager)
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Enabling DankGreeter display manager...${NC}"
echo ""

# Check if we have root privileges
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
  exit 1
fi

# Disable other display managers if they're enabled
echo -e "${BLUE}Checking for conflicting display managers...${NC}"
for dm in gdm sddm lightdm lxdm; do
  if systemctl is-enabled "${dm}.service" &>/dev/null; then
    echo -e "${YELLOW}Disabling ${dm} display manager...${NC}"
    systemctl disable "${dm}.service"
    echo -e "${GREEN}${dm} disabled${NC}"
  fi
done

echo ""

# Enable greetd service
if systemctl is-enabled greetd.service &>/dev/null; then
  echo -e "${YELLOW}greetd is already enabled${NC}"
else
  echo -e "${BLUE}Enabling greetd service...${NC}"
  systemctl enable greetd.service
  echo -e "${GREEN}greetd enabled${NC}"
fi

# Configure greetd to use DankGreeter
echo ""
echo -e "${BLUE}Configuring greetd to use DankGreeter...${NC}"

mkdir -p /etc/greetd

cat > /etc/greetd/config.toml << 'GREETDEOF'
[terminal]
vt = 1

[default_session]
command = "dms-greeter --command niri"
user = "greeter"
GREETDEOF

echo -e "${GREEN}greetd configuration created at /etc/greetd/config.toml${NC}"

echo ""
echo -e "${GREEN}DankGreeter configuration complete!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  - Reboot your system to use DankGreeter"
echo "  - Or manually restart greetd: sudo systemctl restart greetd.service"
echo ""
echo -e "${YELLOW}Note:${NC} The greeter is configured to use niri compositor."
echo "      To use a different compositor, edit the --command argument in /etc/greetd/config.toml"
echo "      Available compositors: niri, hyprland, sway, scroll, or mangowc"
echo ""
