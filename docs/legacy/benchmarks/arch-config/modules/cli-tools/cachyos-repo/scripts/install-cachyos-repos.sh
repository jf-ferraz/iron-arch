#!/usr/bin/env bash
# Install CachyOS repositories (includes Chaotic AUR)
# Part of arch-config declarative package management
# https://mirror.cachyos.org/

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PACMAN_CONF="/etc/pacman.conf"
TEMP_DIR=$(mktemp -d)

echo -e "${BLUE}Setting up CachyOS repositories...${NC}"

# Check if CachyOS repos are already configured
if grep -q "^\[cachyos\]" "$PACMAN_CONF"; then
    echo -e "${YELLOW}CachyOS repositories are already configured in pacman.conf${NC}"
    echo -e "${GREEN}✓ Nothing to do${NC}"
    exit 0
fi

# Cleanup function
cleanup() {
    echo -e "${BLUE}Cleaning up temporary files...${NC}"
    cd /
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

cd "$TEMP_DIR"

echo -e "${BLUE}Downloading CachyOS repository installer...${NC}"
curl -LO https://mirror.cachyos.org/cachyos-repo.tar.xz

echo -e "${BLUE}Extracting installer...${NC}"
tar xvf cachyos-repo.tar.xz

cd cachyos-repo

echo -e "${BLUE}Running CachyOS repository installer...${NC}"
sudo ./cachyos-repo.sh

echo ""
echo -e "${GREEN}✓ CachyOS repositories installed successfully!${NC}"
echo ""
echo -e "${BLUE}You can now install packages from CachyOS repos using pacman.${NC}"
echo "Example: sudo pacman -S <package-name>"
echo ""
