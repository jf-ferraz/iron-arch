#!/usr/bin/env bash
# Install Archcraft BSPWM configuration
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Determine the user (handles both sudo and non-sudo cases)
TARGET_USER="${SUDO_USER:-$USER}"
TARGET_HOME="/home/${TARGET_USER}"
CONFIG_DIR="${TARGET_HOME}/.config"
BSPWM_CONFIG_DIR="${CONFIG_DIR}/bspwm"
ARCH_CONFIG_DIR="${ARCH_CONFIG_DIR:-${TARGET_HOME}/.config/arch-config}"
SOURCE_DIR="${ARCH_CONFIG_DIR}/dotfiles/bspwm"

echo -e "${BLUE}Installing Archcraft BSPWM configuration...${NC}"
echo ""

# Check if source directory exists
if [ ! -d "$SOURCE_DIR" ]; then
  echo -e "${RED}Error: Source directory not found: $SOURCE_DIR${NC}" >&2
  exit 1
fi

# Create .config directory if it doesn't exist
if [ ! -d "$CONFIG_DIR" ]; then
  echo -e "${YELLOW}Creating config directory: $CONFIG_DIR${NC}"
  mkdir -p "$CONFIG_DIR"
  chown "$TARGET_USER:$TARGET_USER" "$CONFIG_DIR"
fi

# Backup existing BSPWM configuration if it exists
if [ -d "$BSPWM_CONFIG_DIR" ]; then
  BACKUP_DIR="${BSPWM_CONFIG_DIR}.backup.$(date +%Y%m%d_%H%M%S)"
  echo -e "${YELLOW}Backing up existing BSPWM configuration...${NC}"
  echo -e "${BLUE}  From: $BSPWM_CONFIG_DIR${NC}"
  echo -e "${BLUE}  To:   $BACKUP_DIR${NC}"

  # Create backup as the target user
  if [ "$EUID" -eq 0 ]; then
    sudo -u "$TARGET_USER" cp -r "$BSPWM_CONFIG_DIR" "$BACKUP_DIR"
  else
    cp -r "$BSPWM_CONFIG_DIR" "$BACKUP_DIR"
  fi

  echo -e "${GREEN}✓ Backup created successfully${NC}"
  echo ""
fi

# Copy BSPWM configuration
echo -e "${BLUE}Copying Archcraft BSPWM configuration...${NC}"
echo -e "${BLUE}  From: $SOURCE_DIR${NC}"
echo -e "${BLUE}  To:   $BSPWM_CONFIG_DIR${NC}"

# Remove existing config directory if present
if [ -d "$BSPWM_CONFIG_DIR" ]; then
  rm -rf "$BSPWM_CONFIG_DIR"
fi

# Copy configuration as the target user
if [ "$EUID" -eq 0 ]; then
  sudo -u "$TARGET_USER" cp -r "$SOURCE_DIR" "$BSPWM_CONFIG_DIR"
else
  cp -r "$SOURCE_DIR" "$BSPWM_CONFIG_DIR"
fi

# Ensure bspwmrc is executable
if [ -f "${BSPWM_CONFIG_DIR}/bspwmrc" ]; then
  chmod +x "${BSPWM_CONFIG_DIR}/bspwmrc"
  echo -e "${GREEN}✓ Made bspwmrc executable${NC}"
fi

# Make all scripts in the scripts directory executable
if [ -d "${BSPWM_CONFIG_DIR}/scripts" ]; then
  chmod +x "${BSPWM_CONFIG_DIR}/scripts"/*
  echo -e "${GREEN}✓ Made scripts executable${NC}"
fi

echo ""
echo -e "${GREEN}✓ Archcraft BSPWM configuration installed successfully!${NC}"
echo ""
echo -e "${BLUE}Installed to:${NC}"
echo "  - $BSPWM_CONFIG_DIR"

if [ -n "${BACKUP_DIR:-}" ]; then
  echo ""
  echo -e "${BLUE}Backup location:${NC}"
  echo "  - $BACKUP_DIR"
fi

echo ""
echo -e "${BLUE}Note:${NC} To use the new configuration:"
echo "  1. Log out of your current session"
echo "  2. Select 'bspwm' from your display manager"
echo "  3. Log in to start using Archcraft BSPWM"
echo ""
echo -e "${YELLOW}Tip:${NC} To restore the backup if needed:"
echo "  rm -rf $BSPWM_CONFIG_DIR"
echo "  mv $BACKUP_DIR $BSPWM_CONFIG_DIR"
echo ""
