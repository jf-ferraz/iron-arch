#!/usr/bin/env bash
# Setup Tailscale for Sunshine remote gaming
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the arch-config directory
ARCH_CONFIG_DIR="${ARCH_CONFIG_DIR:-/home/don/.config/arch-config}"
SUNSHINE_MODULE="${ARCH_CONFIG_DIR}/modules/gaming/sunshine.lua"

echo -e "${BLUE}Setting up Tailscale for Sunshine remote gaming...${NC}"
echo ""

# Check if Tailscale is installed
if ! command -v tailscale &> /dev/null; then
    echo -e "${RED}Error: Tailscale is not installed${NC}" >&2
    echo "This script should be run as a post-install hook after tailscale package is installed"
    exit 1
fi

# Start Tailscale service (enable is handled declaratively in the Lua module)
echo -e "${BLUE}Starting Tailscale service...${NC}"
if ! systemctl is-active --quiet tailscaled; then
    sudo systemctl start tailscaled
    echo -e "${GREEN}✓ Tailscale service started${NC}"
else
    echo -e "${YELLOW}Tailscale service already running${NC}"
fi

echo ""

# Check if Tailscale is authenticated
echo -e "${BLUE}Checking Tailscale authentication status...${NC}"
if ! tailscale status &> /dev/null; then
    echo -e "${YELLOW}Tailscale is not authenticated${NC}"
    echo ""
    echo -e "${BLUE}Please authenticate Tailscale:${NC}"
    echo "Run the following command and follow the link to authenticate:"
    echo ""
    echo -e "${GREEN}  sudo tailscale up${NC}"
    echo ""
    echo "After authentication, run this script again or manually update the tailscale_ip in:"
    echo "  $SUNSHINE_MODULE"
    echo ""
    exit 0
fi

echo -e "${GREEN}✓ Tailscale is authenticated${NC}"
echo ""

# Get Tailscale IP address
echo -e "${BLUE}Retrieving Tailscale IP address...${NC}"
TAILSCALE_IP=$(tailscale ip -4 | head -n1)

if [ -z "$TAILSCALE_IP" ]; then
    echo -e "${RED}Error: Could not retrieve Tailscale IP address${NC}" >&2
    echo "Please ensure Tailscale is properly connected"
    exit 1
fi

echo -e "${GREEN}✓ Tailscale IP: ${TAILSCALE_IP}${NC}"
echo ""

# Display the Tailscale IP
echo -e "${BLUE}Tailscale IP for this host:${NC} ${GREEN}${TAILSCALE_IP}${NC}"
echo ""
echo -e "${YELLOW}Note:${NC} The tailscale_ip in the module metadata is a reference value."
echo "The actual current IP is displayed above."

echo ""
echo -e "${GREEN}✓ Tailscale setup complete!${NC}"
echo ""
echo -e "${BLUE}Tailscale Network Information:${NC}"
tailscale status
echo ""
echo -e "${BLUE}Your Tailscale IP for Sunshine:${NC} ${GREEN}${TAILSCALE_IP}${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Configure Sunshine to listen on your Tailscale IP"
echo "  2. Connect to Sunshine using: ${TAILSCALE_IP}"
echo "  3. Enable Sunshine service: sudo systemctl enable --now sunshine"
echo ""
echo -e "${YELLOW}Note:${NC} For best performance, consider running:"
echo "  ${GREEN}sudo tailscale up --accept-routes${NC}"
echo ""
