#!/usr/bin/env bash
# Install XeroLinux Cosmic Desktop Environment
# Part of arch-config declarative package management
# https://xerolinux.xyz/

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_URL="https://xerolinux.xyz/script/cosmic.sh"
TEMP_SCRIPT="/tmp/cosmic-install-$$.sh"

echo -e "${BLUE}Installing XeroLinux Cosmic Desktop Environment...${NC}"

# Check if running with sudo
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
    echo "Usage: sudo $0" >&2
    exit 1
fi

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo -e "${RED}Error: curl is not installed${NC}" >&2
    echo "Please install curl first: sudo pacman -S curl" >&2
    exit 1
fi

echo -e "${BLUE}Downloading XeroLinux Cosmic installation script...${NC}"
echo -e "${YELLOW}Script URL: ${SCRIPT_URL}${NC}"

# Download the script
if ! curl -fsSL "${SCRIPT_URL}" -o "${TEMP_SCRIPT}"; then
    echo -e "${RED}Error: Failed to download installation script${NC}" >&2
    exit 1
fi

# Make script executable
chmod +x "${TEMP_SCRIPT}"

# Patch out the problematic xero-fix-scripts package
echo -e "${YELLOW}Patching script to remove unavailable 'xero-fix-scripts' package...${NC}"
sed -i 's/xero-fix-scripts//g' "${TEMP_SCRIPT}"

echo -e "${BLUE}Executing installation script...${NC}"
echo ""

# Execute the patched script
if bash "${TEMP_SCRIPT}"; then
    echo ""
    echo -e "${GREEN}✓ XeroLinux Cosmic Desktop Environment installed successfully!${NC}"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "  1. Reboot your system"
    echo "  2. Select 'COSMIC' from your display manager session list"
    echo "  3. Log in to enjoy your new desktop environment"
    echo ""

    # Cleanup
    rm -f "${TEMP_SCRIPT}"
    exit 0
else
    echo ""
    echo -e "${RED}✗ Installation failed!${NC}" >&2
    echo -e "${YELLOW}Please check the error messages above and try again.${NC}" >&2

    # Cleanup
    rm -f "${TEMP_SCRIPT}"
    exit 1
fi
