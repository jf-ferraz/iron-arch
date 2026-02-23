#!/bin/bash
# =============================================================
# BSPWM Pre-Install Script
# Description: Sets up required repositories for gh0stzk dotfiles
# - Adds gh0stzk-dotfiles repository
# - Adds chaotic-aur repository with GPG keys
# - Updates pacman database
#
# Copyright (C) 2021-2025 gh0stzk <z0mbi3.zk@protonmail.com>
# Licensed under GPL-3.0 license
# =============================================================

# Colors
CRE='\033[0;31m'    # Red
CYE='\033[1;33m'    # Yellow
CGR='\033[0;32m'    # Green
CBL='\033[0;34m'    # Blue
BLD='\033[1m'       # Bold
CNC='\033[0m'       # Reset colors

# Check if running as root
if [ "$(id -u)" = 0 ]; then
    echo -e "${BLD}${CRE}ERROR:${CNC} This script MUST NOT be run as root user."
    exit 1
fi

echo -e "${BLD}${CBL}BSPWM Pre-Install Script${CNC}"
echo -e "${CYE}Setting up required repositories...${CNC}"
echo

# Function to add gh0stzk-dotfiles repository
add_gh0stzk_repo() {
    echo -e "${BLD}${CYE}→ Adding gh0stzk-dotfiles repository...${CNC}"
    
    local repo_name="gh0stzk-dotfiles"
    
    # Check if repository already exists
    if grep -q "\[${repo_name}\]" /etc/pacman.conf; then
        echo -e "${CGR}  ✓ Repository already exists${CNC}"
        return 0
    fi
    
    # Add repository to pacman.conf
    echo -e "\n[${repo_name}]" | sudo tee -a /etc/pacman.conf > /dev/null
    echo "SigLevel = Optional TrustAll" | sudo tee -a /etc/pacman.conf > /dev/null
    echo "Server = http://gh0stzk.github.io/pkgs/x86_64" | sudo tee -a /etc/pacman.conf > /dev/null
    
    echo -e "${CGR}  ✓ gh0stzk-dotfiles repository added${CNC}"
}

# Function to add chaotic-aur repository
add_chaotic_repo() {
    echo -e "${BLD}${CYE}→ Adding chaotic-aur repository...${CNC}"
    
    local repo_chaotic="chaotic-aur"
    local key_id="3056513887B78AEB"
    
    # Check if already configured
    if grep -q "\[${repo_chaotic}\]" /etc/pacman.conf; then
        echo -e "${CGR}  ✓ Repository already exists${CNC}"
        return 0
    fi
    
    # Install GPG key if not present
    if ! pacman-key -f "$key_id" > /dev/null 2>&1; then
        echo -e "${CYE}  → Adding GPG key...${CNC}"
        sudo pacman-key --recv-key "$key_id" --keyserver keyserver.ubuntu.com
        sudo pacman-key --lsign-key "$key_id"
    else
        echo -e "${CGR}  ✓ GPG key already exists${CNC}"
    fi
    
    # Install chaotic keyring and mirrorlist
    echo -e "${CYE}  → Installing chaotic keyring and mirrorlist...${CNC}"
    sudo pacman -U --noconfirm --needed \
        'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst' \
        'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'
    
    # Add repository to pacman.conf
    echo -e "\n[${repo_chaotic}]" | sudo tee -a /etc/pacman.conf > /dev/null
    echo "Include = /etc/pacman.d/chaotic-mirrorlist" | sudo tee -a /etc/pacman.conf > /dev/null
    
    echo -e "${CGR}  ✓ chaotic-aur repository added${CNC}"
}

# Function to install AUR helper
install_aur_helper() {
    echo -e "${BLD}${CYE}→ Checking for AUR helper...${CNC}"
    
    # Check if paru or yay is already installed
    if command -v paru > /dev/null 2>&1; then
        echo -e "${CGR}  ✓ paru is already installed${CNC}"
        return 0
    elif command -v yay > /dev/null 2>&1; then
        echo -e "${CGR}  ✓ yay is already installed${CNC}"
        return 0
    fi
    
    # Install paru
    echo -e "${CYE}  → Installing paru (AUR helper)...${CNC}"
    
    # Create temp directory
    local temp_dir=$(mktemp -d)
    cd "$temp_dir" || exit 1
    
    # Clone and build paru
    git clone https://aur.archlinux.org/paru-bin.git
    cd paru-bin || exit 1
    makepkg -si --noconfirm
    
    # Cleanup
    cd "$HOME" || exit 1
    rm -rf "$temp_dir"
    
    echo -e "${CGR}  ✓ paru installed successfully${CNC}"
}

# Main execution
main() {
    echo
    
    # Add repositories
    add_gh0stzk_repo
    echo
    add_chaotic_repo
    echo
    
    # Update pacman database
    echo -e "${BLD}${CYE}→ Updating pacman database...${CNC}"
    sudo pacman -Syy
    echo -e "${CGR}  ✓ Database updated${CNC}"
    echo
    
    # Install AUR helper
    install_aur_helper
    echo
    
    # Summary
    echo -e "${BLD}${CGR}✓ Pre-installation complete!${CNC}"
    echo
    echo -e "${CYE}The following repositories are now enabled:${CNC}"
    echo -e "  • ${CBL}gh0stzk-dotfiles${CNC} - Custom themes, icons, and packages"
    echo -e "  • ${CBL}chaotic-aur${CNC} - Pre-built AUR packages"
    echo
    echo -e "${CYE}You can now install BSPWM packages with:${CNC}"
    echo -e "  ${BLD}./apply-config.sh${CNC} or your config management tool"
    echo
}

# Run main function
main
