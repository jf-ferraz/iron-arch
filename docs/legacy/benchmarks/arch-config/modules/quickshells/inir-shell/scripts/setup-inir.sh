#!/usr/bin/env bash
# iNiR Shell Setup Script
# This script clones iNiR repo and prints instructions for completing setup

set -e

# Get the actual user (not root when running via sudo)
if [[ -n "$SUDO_USER" ]]; then
    REAL_USER="$SUDO_USER"
    REAL_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
else
    REAL_USER="$USER"
    REAL_HOME="$HOME"
fi

INIR_DIR="${REAL_HOME}/.config/quickshell/ii"
INIR_REPO="https://github.com/snowarch/inir.git"

echo ""
echo "==============================================================="
echo "  iNiR Shell - Setup"
echo "==============================================================="
echo ""
echo "User: $REAL_USER"
echo "Home: $REAL_HOME"
echo ""

# Check if iNiR is already installed
if [[ -d "$INIR_DIR" ]]; then
    echo "iNiR directory already exists at: $INIR_DIR"
    echo ""
    read -p "Update existing installation? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Updating iNiR..."
        # Run git as the real user to avoid permission issues
        if [[ -n "$SUDO_USER" ]]; then
            sudo -u "$SUDO_USER" git -C "$INIR_DIR" pull --ff-only origin main || {
                echo "Git pull failed. You may have local changes."
                echo "Resolve manually: cd $INIR_DIR && git status"
                exit 1
            }
        else
            git -C "$INIR_DIR" pull --ff-only origin main || {
                echo "Git pull failed. You may have local changes."
                echo "Resolve manually: cd $INIR_DIR && git status"
                exit 1
            }
        fi
        echo ""
        echo "iNiR repository updated."
    else
        echo "Skipping iNiR update."
    fi
else
    echo "Cloning iNiR to: $INIR_DIR"
    mkdir -p "$(dirname "$INIR_DIR")"
    
    # Run git clone as the real user
    if [[ -n "$SUDO_USER" ]]; then
        # Ensure directory is owned by user
        chown "$SUDO_USER:$SUDO_USER" "$(dirname "$INIR_DIR")"
        sudo -u "$SUDO_USER" git clone "$INIR_REPO" "$INIR_DIR"
    else
        git clone "$INIR_REPO" "$INIR_DIR"
    fi
    echo ""
    echo "iNiR repository cloned."
fi

echo ""
echo "==============================================================="
echo "  iNiR Shell - Next Steps"
echo "==============================================================="
echo ""
echo "iNiR is now cloned at: $INIR_DIR"
echo ""
echo "Your existing niri config was NOT modified."
echo ""
echo "To complete the iNiR setup, run the following command AS YOUR USER:"
echo ""
echo "    cd $INIR_DIR && ./setup install-deps -y"
echo ""
echo "NOTE: The iNiR setup script cannot run as root, so run it"
echo "      as your normal user after this dcli sync completes."
echo ""
echo "---------------------------------------------------------------"
echo ""
echo "When you're ready to use iNiR, add this to your niri startup:"
echo ""
echo "    spawn-at-startup \"qs\" \"-c\" \"ii\""
echo ""
echo "Or integrate it with your shell-switcher system."
echo ""
echo "To test iNiR manually without adding to startup:"
echo ""
echo "    qs -c ii"
echo ""
echo "==============================================================="
