#!/bin/bash

# Post-install hook for bdots-kde module
# Copies KDE configuration files and applies Catppuccin Mocha Mauve theme

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_DIR="$(dirname "$SCRIPT_DIR")"
DOTFILES_DIR="$MODULE_DIR/dotfiles/kde"

echo "==> Installing bdots-kde configuration files..."

# Ensure .config directory exists
mkdir -p ~/.config

# Copy KDE configuration files
echo "  -> Copying KDE RC files to ~/.config/"
cp -v "$DOTFILES_DIR/kdeglobals" ~/.config/
cp -v "$DOTFILES_DIR/kwinrc" ~/.config/
cp -v "$DOTFILES_DIR/plasmashellrc" ~/.config/
cp -v "$DOTFILES_DIR/dolphinrc" ~/.config/
cp -v "$DOTFILES_DIR/konsolerc" ~/.config/

# Copy kdedefaults directory
echo "  -> Copying kdedefaults configuration..."
mkdir -p ~/.config/kdedefaults
cp -rv "$DOTFILES_DIR/kdedefaults/"* ~/.config/kdedefaults/

# Apply theme settings using kwriteconfig
echo "  -> Applying Catppuccin Mocha Mauve theme settings..."

# Set global theme
kwriteconfig6 --file kdeglobals --group "KDE" --key "LookAndFeelPackage" "Catppuccin-Mocha-Mauve"
kwriteconfig6 --file kdeglobals --group "Icons" --key "Theme" "Tela-purple-dark"
kwriteconfig6 --file kdeglobals --group "General" --key "ColorScheme" "CatppuccinMocha"

# Set cursor theme
kwriteconfig6 --file kcminputrc --group "Mouse" --key "cursorTheme" "Catppuccin-Mocha-Mauve-Cursors"

# Set splash screen
kwriteconfig6 --file ksplashrc --group "KSplash" --key "Engine" "KSplashQML"
kwriteconfig6 --file ksplashrc --group "KSplash" --key "Theme" "Catppuccin-Mocha-Mauve"

# Set window decorations (Layan theme)
kwriteconfig6 --file kwinrc --group "org.kde.kdecoration2" --key "theme" "__aurorae__svg__Layan"

echo "  -> Configuration files installed successfully!"

# Restart plasmashell to apply changes
echo "  -> Restarting plasmashell to apply theme changes..."
if pgrep -x "plasmashell" > /dev/null; then
    killall plasmashell 2>/dev/null || true
    sleep 1
    nohup plasmashell > /dev/null 2>&1 &
    echo "  -> Plasmashell restarted!"
else
    echo "  -> Plasmashell not running, skipping restart"
fi

echo ""
echo "==> bdots-kde installation complete!"
echo "    Theme: Catppuccin Mocha Mauve"
echo "    Icons: Tela-purple-dark"
echo "    Cursor: Catppuccin-Mocha-Mauve-Cursors"
echo "    Window Decorations: Layan"
echo ""
echo "    Note: If running Wayland, you may need to log out and back in for all changes to take effect."
