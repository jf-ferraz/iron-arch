#!/usr/bin/env bash
# Flutter SDK Installation Script
# Downloads and configures Flutter with Linux, Android, and Web support
# Reference: https://docs.flutter.dev/platform-integration/linux/setup

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the actual user (even when running with sudo)
if [[ -n "${SUDO_USER:-}" ]]; then
    ACTUAL_USER="$SUDO_USER"
    ACTUAL_HOME="$(getent passwd "$SUDO_USER" | cut -d: -f6)"
else
    ACTUAL_USER="${USER:-$(whoami)}"
    ACTUAL_HOME="$HOME"
fi

# Configuration
FLUTTER_VERSION="stable"
FLUTTER_INSTALL_DIR="$ACTUAL_HOME/development"
FLUTTER_DIR="$FLUTTER_INSTALL_DIR/flutter"
ANDROID_STUDIO_DIR="/opt/android-studio"
SHELL_CONFIG=""

# Helper functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect shell configuration file
detect_shell_config() {
    # Get the shell from the actual user's passwd entry
    local current_shell=$(basename "$(getent passwd "$ACTUAL_USER" | cut -d: -f7)")
    
    case "$current_shell" in
        bash)
            if [[ -f "$ACTUAL_HOME/.bashrc" ]]; then
                SHELL_CONFIG="$ACTUAL_HOME/.bashrc"
            elif [[ -f "$ACTUAL_HOME/.bash_profile" ]]; then
                SHELL_CONFIG="$ACTUAL_HOME/.bash_profile"
            fi
            ;;
        zsh)
            SHELL_CONFIG="$ACTUAL_HOME/.zshrc"
            ;;
        fish)
            SHELL_CONFIG="$ACTUAL_HOME/.config/fish/config.fish"
            # Create directory if it doesn't exist
            mkdir -p "$ACTUAL_HOME/.config/fish"
            ;;
        *)
            # Default to bashrc
            SHELL_CONFIG="$ACTUAL_HOME/.bashrc"
            ;;
    esac
    
    info "Detected shell: $current_shell"
    info "Shell config: $SHELL_CONFIG"
}

# Check if Flutter is already installed
check_flutter() {
    if command -v flutter &> /dev/null; then
        info "Flutter is already installed"
        flutter --version
        
        read -p "Reinstall Flutter? [y/N]: " response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            success "Keeping existing Flutter installation"
            return 1
        fi
    fi
    return 0
}

# Download and install Flutter SDK
install_flutter() {
    info "Installing Flutter SDK..."
    
    # Create development directory with correct ownership
    mkdir -p "$FLUTTER_INSTALL_DIR"
    chown "$ACTUAL_USER:$ACTUAL_USER" "$FLUTTER_INSTALL_DIR"
    
    # Remove old installation if exists
    if [[ -d "$FLUTTER_DIR" ]]; then
        warn "Removing existing Flutter installation..."
        rm -rf "$FLUTTER_DIR"
    fi
    
    # Download Flutter
    info "Downloading Flutter $FLUTTER_VERSION..."
    cd "$FLUTTER_INSTALL_DIR"
    
    git clone https://github.com/flutter/flutter.git -b "$FLUTTER_VERSION" --depth 1
    
    # Ensure correct ownership of Flutter directory
    chown -R "$ACTUAL_USER:$ACTUAL_USER" "$FLUTTER_DIR"
    
    success "Flutter SDK downloaded to $FLUTTER_DIR"
}

# Add Flutter to PATH
configure_path() {
    info "Configuring PATH..."
    
    local flutter_bin="$FLUTTER_DIR/bin"
    local chromium_bin="/usr/lib/chromium"
    local shell_name=$(basename "$(getent passwd "$ACTUAL_USER" | cut -d: -f7)")
    
    # Check if PATH already contains Flutter
    if grep -q "flutter/bin" "$SHELL_CONFIG" 2>/dev/null; then
        warn "Flutter PATH entry already exists in $SHELL_CONFIG"
    else
        info "Adding Flutter to PATH in $SHELL_CONFIG"
        
        case "$shell_name" in
            fish)
                echo "" >> "$SHELL_CONFIG"
                echo "# Flutter SDK" >> "$SHELL_CONFIG"
                echo "set -gx PATH $flutter_bin \$PATH" >> "$SHELL_CONFIG"
                ;;
            *)
                echo "" >> "$SHELL_CONFIG"
                echo "# Flutter SDK" >> "$SHELL_CONFIG"
                echo "export PATH=\"$flutter_bin:\$PATH\"" >> "$SHELL_CONFIG"
                ;;
        esac
        
        success "Flutter added to PATH"
    fi
    
    # Add Chromium to PATH for debugging if not already present
    if ! grep -q "chromium" "$SHELL_CONFIG" 2>/dev/null; then
        info "Adding Chromium to PATH for Flutter web debugging..."
        
        case "$shell_name" in
            fish)
                echo "" >> "$SHELL_CONFIG"
                echo "# Chromium for Flutter web debugging" >> "$SHELL_CONFIG"
                echo "set -gx CHROME_EXECUTABLE /usr/bin/chromium" >> "$SHELL_CONFIG"
                ;;
            *)
                echo "" >> "$SHELL_CONFIG"
                echo "# Chromium for Flutter web debugging" >> "$SHELL_CONFIG"
                echo "export CHROME_EXECUTABLE=/usr/bin/chromium" >> "$SHELL_CONFIG"
                ;;
        esac
        
        success "Chromium configured for Flutter web debugging"
    fi
    
    # Add Android Studio to PATH if installed
    if [[ -d "$ANDROID_STUDIO_DIR" ]]; then
        if ! grep -q "android-studio" "$SHELL_CONFIG" 2>/dev/null; then
            info "Adding Android Studio to PATH..."
            
            case "$shell_name" in
                fish)
                    echo "" >> "$SHELL_CONFIG"
                    echo "# Android Studio" >> "$SHELL_CONFIG"
                    echo "set -gx PATH $ANDROID_STUDIO_DIR/bin \$PATH" >> "$SHELL_CONFIG"
                    ;;
                *)
                    echo "" >> "$SHELL_CONFIG"
                    echo "# Android Studio" >> "$SHELL_CONFIG"
                    echo "export PATH=\"$ANDROID_STUDIO_DIR/bin:\$PATH\"" >> "$SHELL_CONFIG"
                    ;;
            esac
            
            success "Android Studio added to PATH"
        fi
    fi
}

# Run flutter doctor to verify installation
verify_installation() {
    info "Verifying Flutter installation..."
    
    # Source the shell config to get updated PATH
    export PATH="$FLUTTER_DIR/bin:$PATH"
    
    # Run flutter precache as the actual user (not root)
    info "Running flutter precache..."
    su - "$ACTUAL_USER" -c "export PATH=\"$FLUTTER_DIR/bin:\$PATH\" && $FLUTTER_DIR/bin/flutter precache --linux --android --web"
    
    # Run flutter doctor as the actual user (not root)
    info "Running flutter doctor..."
    su - "$ACTUAL_USER" -c "export PATH=\"$FLUTTER_DIR/bin:\$PATH\" && $FLUTTER_DIR/bin/flutter doctor -v" || true
    
    success "Flutter installation complete!"
}

# Setup Android Studio permissions
configure_android_studio() {
    if [[ -d "$ANDROID_STUDIO_DIR" ]]; then
        info "Configuring Android Studio..."
        
        # Check if user is in kvm group (required for emulator)
        if ! groups "$ACTUAL_USER" | grep -q "\bkvm\b"; then
            warn "User $ACTUAL_USER is not in the kvm group"
            warn "You may need to add yourself to kvm group for Android emulator:"
            warn "  sudo usermod -aG kvm $ACTUAL_USER"
            warn "Then log out and log back in."
        fi
        
        success "Android Studio is installed at $ANDROID_STUDIO_DIR"
        info "You can launch it with: studio.sh"
    fi
}

# Main installation flow
main() {
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║          Flutter SDK Installation Wizard                   ║"
    echo "║    Linux Desktop, Android, and Web Development Ready       ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    detect_shell_config
    
    if check_flutter; then
        install_flutter
    fi
    
    configure_path
    verify_installation
    configure_android_studio
    
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║           Flutter Setup Complete! 🎉                       ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    success "Flutter SDK is ready to use!"
    echo ""
    
    info "📋 Quick Start Guide:"
    echo ""
    echo "  1. Reload your shell configuration:"
    echo "     source $SHELL_CONFIG"
    echo ""
    echo "  2. Verify installation:"
    echo "     flutter doctor"
    echo ""
    echo "  3. Create a new project:"
    echo "     flutter create my_app"
    echo ""
    echo "  4. Run on different platforms:"
    echo "     cd my_app"
    echo "     flutter run -d linux      # Linux desktop"
    echo "     flutter run -d chrome     # Web (requires CHROME_EXECUTABLE)"
    echo "     flutter run               # Android (requires connected device/emulator)"
    echo ""
    
    info "🔧 Development Tools:"
    echo ""
    echo "  • Android Studio: studio.sh (or find it in your app menu)"
    echo "  • Chromium: /usr/bin/chromium (for web debugging)"
    echo ""
    
    warn "⚠️  Important Notes:"
    echo ""
    echo "  • You may need to LOG OUT and LOG BACK IN for PATH changes to take effect"
    echo "  • For Android emulator: Add yourself to kvm group:"
    echo "      sudo usermod -aG kvm $ACTUAL_USER && sudo usermod -aG libvirt $ACTUAL_USER"
    echo "  • Run 'flutter config --android-studio-dir=/opt/android-studio' if needed"
    echo "  • Accept Android licenses: flutter doctor --android-licenses"
    echo ""
    
    info "📚 Documentation:"
    echo "  Flutter: https://docs.flutter.dev"
    echo "  Linux setup: https://docs.flutter.dev/platform-integration/linux/setup"
    echo ""
}

# Run main function
main "$@"
