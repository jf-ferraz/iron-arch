#!/usr/bin/env bash
# Enable GDM display manager
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Enabling GDM display manager...${NC}"
echo ""

# Check if we have root privileges
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
  exit 1
fi

# Disable other display managers if they're enabled
for dm in sddm lightdm lxdm; do
  if systemctl is-enabled "${dm}.service" &>/dev/null; then
    echo -e "${YELLOW}Disabling ${dm} display manager...${NC}"
    systemctl disable "${dm}.service"
    echo -e "${GREEN}${dm} disabled${NC}"
  fi
done

echo ""

# Enable GDM
if systemctl is-enabled gdm.service &>/dev/null; then
  echo -e "${YELLOW}GDM is already enabled${NC}"
else
  echo -e "${BLUE}Enabling GDM service...${NC}"
  systemctl enable gdm.service
  echo -e "${GREEN}GDM enabled${NC}"
fi

echo ""
echo -e "${GREEN}GDM configuration complete!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  - Reboot your system to use GDM"
echo "  - Or manually start GDM: sudo systemctl start gdm.service"
echo ""
