#!/usr/bin/env bash
# Install MangoWC dotfiles and theming
# Part of arch-config declarative package management

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the actual user (not root if run with sudo)
ACTUAL_USER="${SUDO_USER:-$USER}"
ACTUAL_HOME=$(eval echo "~$ACTUAL_USER")

# Get the module directory (parent of scripts directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_DIR="$(dirname "$SCRIPT_DIR")"
DOTFILES_MANGOWC="${MODULE_DIR}/dotfiles/mango"
DOTFILES_THEMES="${MODULE_DIR}/themes-mango"
USER_CONFIG_DIR="${ACTUAL_HOME}/.config"

echo -e "${BLUE}Installing MangoWC dotfiles and theming...${NC}"
echo ""

# Check if dotfiles sources exist
if [ ! -d "$DOTFILES_MANGOWC" ]; then
  echo -e "${RED}Error: MangoWC dotfiles directory not found: $DOTFILES_MANGOWC${NC}" >&2
  exit 1
fi

if [ ! -d "$DOTFILES_THEMES" ]; then
  echo -e "${RED}Error: Themes directory not found: $DOTFILES_THEMES${NC}" >&2
  exit 1
fi

# Fix permissions on dotfiles (git doesn't track directory permissions)
echo -e "${BLUE}Fixing dotfiles permissions...${NC}"
if [ -d "${DOTFILES_THEMES}/fish" ]; then
  chmod 755 "${DOTFILES_THEMES}/fish"
  if [ -f "${DOTFILES_THEMES}/fish/fish_variables" ]; then
    chmod 644 "${DOTFILES_THEMES}/fish/fish_variables"
  fi
  echo -e "${GREEN}✓ Fixed fish directory permissions${NC}"
fi
echo ""

# Backup timestamp for all backups
BACKUP_SUFFIX=".bak-$(date +%Y%m%d-%H%M%S)"

# MangoWC-specific configs
MANGOWC_CONFIGS=(
  "mango"
)

# Shared theming configs
THEME_CONFIGS=(
  "kitty"
  "fastfetch"
  "DankMaterialShell"
  "gtk-3.0"
  "gtk-4.0"
  "qt5ct"
  "qt6ct"
)

# Shared configs in home directory (not .config)
HOME_CONFIGS=(
  ".crystal-dock-2"
)

# Optional configs (only install if they don't already exist)
OPTIONAL_CONFIGS=(
  "fish"
)

# Backup existing configurations
echo -e "${BLUE}Checking for existing configurations...${NC}"
BACKED_UP=false

# Check .config configs
ALL_CONFIGS=("${MANGOWC_CONFIGS[@]}" "${THEME_CONFIGS[@]}")
for config in "${ALL_CONFIGS[@]}"; do
  TARGET="${USER_CONFIG_DIR}/${config}"
  if [ -d "$TARGET" ] || [ -f "$TARGET" ]; then
    if [ "$BACKED_UP" = false ]; then
      echo ""
      BACKED_UP=true
    fi
    echo -e "${YELLOW}Backing up existing ${config} configuration...${NC}"
    BACKUP="${TARGET}${BACKUP_SUFFIX}"
    sudo -u "$ACTUAL_USER" mv "$TARGET" "$BACKUP"
    echo -e "${GREEN}✓ Backed up to: $BACKUP${NC}"
  fi
done

# Check home directory configs
for config in "${HOME_CONFIGS[@]}"; do
  TARGET="${ACTUAL_HOME}/${config}"
  if [ -d "$TARGET" ] || [ -f "$TARGET" ]; then
    if [ "$BACKED_UP" = false ]; then
      echo ""
      BACKED_UP=true
    fi
    echo -e "${YELLOW}Backing up existing ${config} configuration...${NC}"
    BACKUP="${TARGET}${BACKUP_SUFFIX}"
    sudo -u "$ACTUAL_USER" mv "$TARGET" "$BACKUP"
    echo -e "${GREEN}✓ Backed up to: $BACKUP${NC}"
  fi
done

if [ "$BACKED_UP" = true ]; then
  echo ""
fi

# Install MangoWC-specific dotfiles
echo -e "${BLUE}Installing MangoWC-specific dotfiles...${NC}"
echo ""

for config in "${MANGOWC_CONFIGS[@]}"; do
  SOURCE="${DOTFILES_MANGOWC}/${config}"
  TARGET="${USER_CONFIG_DIR}/${config}"

  if [ -d "$SOURCE" ]; then
    echo -e "${BLUE}Installing ${config}...${NC}"
    sudo -u "$ACTUAL_USER" cp -r "$SOURCE" "$TARGET"
    chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
    echo -e "${GREEN}✓ Installed ${config}${NC}"
  else
    echo -e "${YELLOW}⚠ Skipping ${config} (not found in dotfiles)${NC}"
  fi
done

echo ""

# Install shared theming
echo -e "${BLUE}Installing shared theming...${NC}"
echo ""

for config in "${THEME_CONFIGS[@]}"; do
  SOURCE="${DOTFILES_THEMES}/${config}"
  TARGET="${USER_CONFIG_DIR}/${config}"

  if [ -d "$SOURCE" ]; then
    echo -e "${BLUE}Installing ${config}...${NC}"
    sudo -u "$ACTUAL_USER" cp -r "$SOURCE" "$TARGET"
    chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
    echo -e "${GREEN}✓ Installed ${config}${NC}"
  else
    echo -e "${YELLOW}⚠ Skipping ${config} (not found in themes)${NC}"
  fi
done

echo ""

# Install home directory configs
echo -e "${BLUE}Installing home directory configs...${NC}"
echo ""

for config in "${HOME_CONFIGS[@]}"; do
  # Crystal-dock has WM-specific subdirectories (mango/, niri/)
  if [[ "$config" == ".crystal-dock-2" ]]; then
    SOURCE="${DOTFILES_THEMES}/${config}/mango"
    TARGET="${ACTUAL_HOME}/${config}"

    if [ -d "$SOURCE" ]; then
      echo -e "${BLUE}Installing ${config} (MangoWC-specific)...${NC}"
      # Create target directory if it doesn't exist
      sudo -u "$ACTUAL_USER" mkdir -p "$TARGET"
      # Copy contents from mango subdirectory
      sudo -u "$ACTUAL_USER" cp -r "$SOURCE"/* "$TARGET/"
      chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
      echo -e "${GREEN}✓ Installed ${config}${NC}"
    else
      echo -e "${YELLOW}⚠ Skipping ${config} (mango config not found in themes)${NC}"
    fi
  else
    SOURCE="${DOTFILES_THEMES}/${config}"
    TARGET="${ACTUAL_HOME}/${config}"

    if [ -d "$SOURCE" ]; then
      echo -e "${BLUE}Installing ${config}...${NC}"
      sudo -u "$ACTUAL_USER" cp -r "$SOURCE" "$TARGET"
      chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
      echo -e "${GREEN}✓ Installed ${config}${NC}"
    else
      echo -e "${YELLOW}⚠ Skipping ${config} (not found in themes)${NC}"
    fi
  fi
done

echo ""

# Handle optional configs with user prompts
echo -e "${BLUE}Handling optional configurations...${NC}"
echo ""

for config in "${OPTIONAL_CONFIGS[@]}"; do
  SOURCE="${DOTFILES_THEMES}/${config}"
  TARGET="${USER_CONFIG_DIR}/${config}"

  if [ ! -d "$SOURCE" ]; then
    echo -e "${YELLOW}⚠ Skipping ${config} (not found in themes)${NC}"
    continue
  fi

  # Check if config already exists
  if [ -d "$TARGET" ] || [ -f "$TARGET" ]; then
    echo -e "${YELLOW}Existing ${config} configuration detected${NC}"

    # Check if running interactively
    if [ -t 0 ]; then
      echo -e "${BLUE}What would you like to do?${NC}"
      echo "  1) Keep existing ${config} config (recommended)"
      echo "  2) Replace with new ${config} config (backup will be created)"
      echo ""
      read -r -p "Enter your choice [1/2]: " choice || choice=1
    else
      # Non-interactive mode: default to keeping existing config
      choice=1
      echo -e "${BLUE}Non-interactive mode: keeping existing ${config} configuration${NC}"
    fi

    case $choice in
      2)
        echo -e "${YELLOW}Backing up existing ${config} configuration...${NC}"
        BACKUP="${TARGET}${BACKUP_SUFFIX}"
        sudo -u "$ACTUAL_USER" mv "$TARGET" "$BACKUP"
        echo -e "${GREEN}✓ Backed up to: $BACKUP${NC}"

        echo -e "${BLUE}Installing new ${config}...${NC}"
        sudo -u "$ACTUAL_USER" cp -r "$SOURCE" "$TARGET"
        chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
        echo -e "${GREEN}✓ Installed ${config}${NC}"
        ;;
      1|*)
        echo -e "${GREEN}✓ Keeping existing ${config} configuration${NC}"
        ;;
    esac
    echo ""
  else
    # No existing config, just install
    echo -e "${BLUE}Installing ${config}...${NC}"
    sudo -u "$ACTUAL_USER" cp -r "$SOURCE" "$TARGET"
    chown -R "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
    echo -e "${GREEN}✓ Installed ${config}${NC}"
    echo ""
  fi
done

echo -e "${GREEN}✓ MangoWC dotfiles installed successfully!${NC}"
echo ""
echo -e "${BLUE}Installed configurations:${NC}"
echo "  • MangoWC compositor settings (mango)"
echo "  • Kitty terminal theme"
echo "  • Fastfetch system info styling"
echo "  • DankMaterialShell components"
echo "  • GTK 3/4 theming"
echo "  • Qt5/Qt6 configuration"
echo "  • Crystal Dock application dock"
echo ""
echo -e "${BLUE}Installation location:${NC}"
echo "  ${USER_CONFIG_DIR}/"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Log out and select MangoWC as your session"
echo "  2. Your custom theming will be automatically applied"
echo "  3. Launch Kitty to see your themed terminal"
echo "  4. Run ${YELLOW}fastfetch${NC} to see your styled system info"
echo ""
if [ "$BACKED_UP" = true ]; then
  echo -e "${BLUE}Note:${NC}"
  echo "  Your previous configurations were backed up with suffix: ${BACKUP_SUFFIX}"
  echo "  You can restore them by removing the new configs and renaming the backups"
  echo ""
fi
