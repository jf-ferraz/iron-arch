#!/usr/bin/env bash
# Install Catppuccin Mocha theme for Zen Browser
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

# Zen Browser profile directory
ZEN_PROFILE_DIR="${ACTUAL_HOME}/.zen"

echo -e "${BLUE}Installing Catppuccin Mocha theme for Zen Browser...${NC}"
echo ""

# Check if Zen Browser is installed
if ! command -v zen-browser &> /dev/null; then
  echo -e "${YELLOW}Warning: zen-browser command not found${NC}"
  echo -e "${YELLOW}The theme will be installed, but you may need to install zen-browser first${NC}"
  echo ""
fi

# Find Zen Browser profile directory
if [ ! -d "$ZEN_PROFILE_DIR" ]; then
  echo -e "${YELLOW}Zen Browser profile directory not found at: $ZEN_PROFILE_DIR${NC}"
  echo -e "${YELLOW}Please run Zen Browser at least once to create the profile, then re-run this script${NC}"
  exit 0
fi

# Find the default profile (usually ends with .default-release or .Default)
PROFILE_PATH=""
# Try multiple patterns for different Zen Browser profile naming conventions
for profile in "$ZEN_PROFILE_DIR"/*.default* "$ZEN_PROFILE_DIR"/*.default-release "$ZEN_PROFILE_DIR"/*.Default*; do
  if [ -d "$profile" ]; then
    PROFILE_PATH="$profile"
    break
  fi
done

if [ -z "$PROFILE_PATH" ]; then
  echo -e "${RED}Error: Could not find Zen Browser profile${NC}" >&2
  echo -e "${YELLOW}Available profiles in $ZEN_PROFILE_DIR:${NC}"
  ls -la "$ZEN_PROFILE_DIR" || true
  exit 1
fi

echo -e "${GREEN}Found Zen Browser profile: $PROFILE_PATH${NC}"
echo ""

# Create chrome directory
CHROME_DIR="${PROFILE_PATH}/chrome"
sudo -u "$ACTUAL_USER" mkdir -p "$CHROME_DIR"

# Create temporary directory for downloading theme
TEMP_DIR=$(sudo -u "$ACTUAL_USER" mktemp -d)
trap "rm -rf '$TEMP_DIR'" EXIT

echo -e "${BLUE}Downloading Catppuccin Mocha theme...${NC}"

# Clone the repository
if ! sudo -u "$ACTUAL_USER" git clone --depth 1 https://github.com/catppuccin/zen-browser.git "$TEMP_DIR/catppuccin-zen"; then
  echo -e "${RED}Error: Failed to clone Catppuccin repository${NC}" >&2
  exit 1
fi

# Copy Mocha theme files
# Try multiple possible directory structures
if [ -d "$TEMP_DIR/catppuccin-zen/themes/mocha" ]; then
  MOCHA_DIR="$TEMP_DIR/catppuccin-zen/themes/mocha"
elif [ -d "$TEMP_DIR/catppuccin-zen/Mocha" ]; then
  MOCHA_DIR="$TEMP_DIR/catppuccin-zen/Mocha"
elif [ -d "$TEMP_DIR/catppuccin-zen/themes" ]; then
  # Check what's in the themes directory
  echo -e "${YELLOW}Checking themes directory structure...${NC}"
  ls -la "$TEMP_DIR/catppuccin-zen/themes/" || true

  # Try to find mocha in subdirectories (case-insensitive)
  MOCHA_FOUND=$(find "$TEMP_DIR/catppuccin-zen/themes" -maxdepth 2 -type d -iname "*mocha*" | head -1)
  if [ -n "$MOCHA_FOUND" ]; then
    MOCHA_DIR="$MOCHA_FOUND"
    echo -e "${GREEN}Found Mocha theme at: $MOCHA_DIR${NC}"
  else
    echo -e "${RED}Error: Mocha theme directory not found${NC}" >&2
    echo -e "${YELLOW}Available content in themes/:${NC}"
    find "$TEMP_DIR/catppuccin-zen/themes" -maxdepth 2 -type d || true
    exit 1
  fi
else
  echo -e "${RED}Error: Could not find themes directory${NC}" >&2
  echo -e "${YELLOW}Repository structure:${NC}"
  ls -la "$TEMP_DIR/catppuccin-zen/" || true
  exit 1
fi

echo -e "${BLUE}Installing theme files to: $CHROME_DIR${NC}"

# Check what files are available in the theme directory
echo -e "${YELLOW}Available files in theme directory:${NC}"
ls -la "$MOCHA_DIR/" || true
echo ""

# Copy theme files (check for different possible filenames)
COPIED_FILES=0

# Try to copy userChrome.css (or alternatives)
if [ -f "$MOCHA_DIR/userChrome.css" ]; then
  sudo -u "$ACTUAL_USER" cp "$MOCHA_DIR/userChrome.css" "$CHROME_DIR/"
  echo -e "${GREEN}✓ Copied userChrome.css${NC}"
  COPIED_FILES=$((COPIED_FILES + 1))
elif [ -f "$MOCHA_DIR/chrome/userChrome.css" ]; then
  sudo -u "$ACTUAL_USER" cp "$MOCHA_DIR/chrome/userChrome.css" "$CHROME_DIR/"
  echo -e "${GREEN}✓ Copied userChrome.css from chrome/ subdirectory${NC}"
  COPIED_FILES=$((COPIED_FILES + 1))
else
  echo -e "${YELLOW}Warning: userChrome.css not found${NC}"
fi

# Try to copy userContent.css (or alternatives)
if [ -f "$MOCHA_DIR/userContent.css" ]; then
  sudo -u "$ACTUAL_USER" cp "$MOCHA_DIR/userContent.css" "$CHROME_DIR/"
  echo -e "${GREEN}✓ Copied userContent.css${NC}"
  COPIED_FILES=$((COPIED_FILES + 1))
elif [ -f "$MOCHA_DIR/chrome/userContent.css" ]; then
  sudo -u "$ACTUAL_USER" cp "$MOCHA_DIR/chrome/userContent.css" "$CHROME_DIR/"
  echo -e "${GREEN}✓ Copied userContent.css from chrome/ subdirectory${NC}"
  COPIED_FILES=$((COPIED_FILES + 1))
fi

# Try to copy logo
if [ -f "$MOCHA_DIR/zen-logo.svg" ]; then
  sudo -u "$ACTUAL_USER" cp "$MOCHA_DIR/zen-logo.svg" "$CHROME_DIR/"
  echo -e "${GREEN}✓ Copied zen-logo.svg${NC}"
  COPIED_FILES=$((COPIED_FILES + 1))
fi

# Copy all CSS and SVG files if specific files weren't found
if [ $COPIED_FILES -eq 0 ]; then
  echo -e "${YELLOW}Attempting to copy all CSS and SVG files...${NC}"
  find "$MOCHA_DIR" -maxdepth 2 -type f \( -name "*.css" -o -name "*.svg" \) -exec sudo -u "$ACTUAL_USER" cp {} "$CHROME_DIR/" \;
  COPIED_FILES=$(find "$CHROME_DIR" -type f \( -name "*.css" -o -name "*.svg" \) | wc -l)
  echo -e "${GREEN}✓ Copied $COPIED_FILES theme files${NC}"
fi

if [ $COPIED_FILES -eq 0 ]; then
  echo -e "${RED}Error: No theme files were copied${NC}" >&2
  exit 1
fi

echo -e "${GREEN}✓ Theme files installed successfully${NC}"
echo ""

# Replace blue accent color with mauve
echo -e "${BLUE}Customizing theme: Replacing blue accent with mauve...${NC}"
BLUE_COLOR="#89b4fa"
MAUVE_COLOR="#cba6f7"

# Replace in userChrome.css if it exists
if [ -f "$CHROME_DIR/userChrome.css" ]; then
  sudo -u "$ACTUAL_USER" sed -i "s/$BLUE_COLOR/$MAUVE_COLOR/g" "$CHROME_DIR/userChrome.css"
  echo -e "${GREEN}✓ Updated userChrome.css to use mauve accent${NC}"
fi

# Replace in userContent.css if it exists
if [ -f "$CHROME_DIR/userContent.css" ]; then
  sudo -u "$ACTUAL_USER" sed -i "s/$BLUE_COLOR/$MAUVE_COLOR/g" "$CHROME_DIR/userContent.css"
  echo -e "${GREEN}✓ Updated userContent.css to use mauve accent${NC}"
fi

echo ""

# Update user.js to enable legacy stylesheets
USER_JS="${PROFILE_PATH}/user.js"
PREF_LINE='user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);'

if [ -f "$USER_JS" ]; then
  if grep -q "toolkit.legacyUserProfileCustomizations.stylesheets" "$USER_JS"; then
    echo -e "${BLUE}Legacy stylesheet preference already configured in user.js${NC}"
  else
    echo -e "${BLUE}Adding legacy stylesheet preference to user.js${NC}"
    sudo -u "$ACTUAL_USER" bash -c "echo '' >> '$USER_JS'"
    sudo -u "$ACTUAL_USER" bash -c "echo '// Enable custom CSS (added by zen-catppuccin module)' >> '$USER_JS'"
    sudo -u "$ACTUAL_USER" bash -c "echo '$PREF_LINE' >> '$USER_JS'"
  fi
else
  echo -e "${BLUE}Creating user.js with legacy stylesheet preference${NC}"
  sudo -u "$ACTUAL_USER" bash -c "echo '// Zen Browser user preferences' > '$USER_JS'"
  sudo -u "$ACTUAL_USER" bash -c "echo '// Enable custom CSS (added by zen-catppuccin module)' >> '$USER_JS'"
  sudo -u "$ACTUAL_USER" bash -c "echo '$PREF_LINE' >> '$USER_JS'"
fi

echo ""
echo -e "${GREEN}✓ Catppuccin Mocha theme installed successfully!${NC}"
echo ""
echo -e "${BLUE}Installation complete:${NC}"
echo "  Profile: $PROFILE_PATH"
echo "  Chrome directory: $CHROME_DIR"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. ${YELLOW}Restart Zen Browser${NC} for the theme to take effect"
echo "  2. Enable ${YELLOW}dark mode${NC} in Zen Browser settings (Mocha requires dark mode)"
echo "  3. Alternatively, verify the setting in about:config:"
echo "     toolkit.legacyUserProfileCustomizations.stylesheets = true"
echo ""
echo -e "${BLUE}Theme details:${NC}"
echo "  Flavor: Catppuccin Mocha (dark)"
echo "  Repository: https://github.com/catppuccin/zen-browser"
echo ""
