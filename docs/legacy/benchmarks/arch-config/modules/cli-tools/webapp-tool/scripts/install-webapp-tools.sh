#!/usr/bin/env bash
# Install webapp-install and webapp-remove to /usr/local/bin
# Part of arch-config declarative package management

set -euo pipefail

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the arch-config directory
ARCH_CONFIG_DIR="${ARCH_CONFIG_DIR:-/home/${SUDO_USER:-$USER}/.config/arch-config}"

# Check for sudo
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Error: This script must be run with sudo${NC}" >&2
  exit 1
fi

echo -e "${GREEN}Installing webapp tools...${NC}"

# Install webapp-install
WEBAPP_INSTALL_SRC="${ARCH_CONFIG_DIR}/modules/cli-tools/webapp-tool/scripts/webapp-install"
WEBAPP_INSTALL_DEST="/usr/local/bin/webapp-install"

if [ -f "$WEBAPP_INSTALL_SRC" ]; then
  cp "$WEBAPP_INSTALL_SRC" "$WEBAPP_INSTALL_DEST"
  chmod 755 "$WEBAPP_INSTALL_DEST"
  echo -e "  ${GREEN}Installed: ${WEBAPP_INSTALL_DEST}${NC}"
else
  echo -e "${RED}Error: Source file not found: ${WEBAPP_INSTALL_SRC}${NC}" >&2
  exit 1
fi

# Install webapp-remove
WEBAPP_REMOVE_SRC="${ARCH_CONFIG_DIR}/modules/cli-tools/webapp-tool/scripts/webapp-remove"
WEBAPP_REMOVE_DEST="/usr/local/bin/webapp-remove"

if [ -f "$WEBAPP_REMOVE_SRC" ]; then
  cp "$WEBAPP_REMOVE_SRC" "$WEBAPP_REMOVE_DEST"
  chmod 755 "$WEBAPP_REMOVE_DEST"
  echo -e "  ${GREEN}Installed: ${WEBAPP_REMOVE_DEST}${NC}"
else
  echo -e "${RED}Error: Source file not found: ${WEBAPP_REMOVE_SRC}${NC}" >&2
  exit 1
fi

echo ""
echo -e "${GREEN}Webapp tools installed successfully!${NC}"
echo ""
echo "Usage:"
echo "  webapp-install              - Create a new web app (interactive)"
echo "  webapp-install --help       - Show help for creating web apps"
echo "  webapp-remove               - Remove a web app (interactive)"
echo "  webapp-remove --list        - List installed web apps"
echo ""
