#!/bin/bash
#
# WinApps Post-Install Setup Hook
# Interactive configuration for WinApps with Docker backend
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

ask() {
    local prompt="$1"
    local default="${2:-Y}"
    local response
    
    if [[ "$default" == "Y" ]]; then
        read -p "$prompt [Y/n]: " response
        response=${response:-Y}
    else
        read -p "$prompt [y/N]: " response
        response=${response:-N}
    fi
    
    [[ "$response" =~ ^[Yy]$ ]]
}

# Banner
echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                  WinApps Setup Wizard                      ║"
echo "║   Seamlessly run Windows applications on Linux via RDP    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# ============================================================================
# STEP 1: System Requirements Check
# ============================================================================

info "Step 1/6: Checking system requirements..."
echo ""

# Check KVM module
if lsmod | grep -q "^kvm"; then
    success "KVM virtualization module loaded"
else
    error "KVM module not loaded - virtualization may not work"
    warn "Check BIOS/UEFI settings to enable Intel VT-x or AMD-V"
fi

# Check iptables modules (required for folder sharing)
IPTABLES_LOADED=true
if ! lsmod | grep -q "ip_tables"; then
    warn "ip_tables module not loaded"
    IPTABLES_LOADED=false
fi

if ! lsmod | grep -q "iptable_nat"; then
    warn "iptable_nat module not loaded"
    IPTABLES_LOADED=false
fi

if [ "$IPTABLES_LOADED" = false ]; then
    echo ""
    warn "Required kernel modules for folder sharing are missing"
    
    if ask "Load and persist iptables modules now? (requires sudo)"; then
        info "Loading kernel modules..."
        sudo modprobe ip_tables
        sudo modprobe iptable_nat
        
        info "Persisting modules for future boots..."
        echo -e "ip_tables\niptable_nat" | sudo tee /etc/modules-load.d/iptables.conf > /dev/null
        
        success "Kernel modules loaded and configured"
    else
        warn "Skipping module setup - you'll need to load them manually:"
        warn "  sudo modprobe ip_tables iptable_nat"
        warn "  echo -e 'ip_tables\niptable_nat' | sudo tee /etc/modules-load.d/iptables.conf"
    fi
fi

echo ""

# ============================================================================
# STEP 2: Docker Group Check
# ============================================================================

info "Step 2/6: Checking Docker permissions..."
echo ""

if groups | grep -q "\bdocker\b"; then
    success "User '$USER' is in docker group"
else
    warn "User '$USER' is NOT in docker group"
    warn "You won't be able to run Docker commands without sudo"
    echo ""
    
    if ask "Add '$USER' to docker group? (requires sudo)"; then
        info "Adding user to docker group..."
        sudo usermod -aG docker "$USER"
        
        success "User added to docker group"
        warn "⚠️  You MUST log out and log back in for group changes to take effect"
        warn "    Or run: newgrp docker"
    else
        warn "Skipping group setup - add yourself manually with:"
        warn "  sudo usermod -aG docker $USER"
    fi
fi

echo ""

# ============================================================================
# STEP 3: Docker Service Check
# ============================================================================

info "Step 3/6: Checking Docker service..."
echo ""

if systemctl is-active --quiet docker.service; then
    success "Docker service is running"
else
    warn "Docker service is not running"
    
    if ask "Start Docker service now?"; then
        info "Starting Docker service..."
        sudo systemctl start docker.service
        success "Docker service started"
    fi
fi

if systemctl is-enabled --quiet docker.service; then
    success "Docker service is enabled (will start on boot)"
else
    warn "Docker service is not enabled"
    
    if ask "Enable Docker service to start on boot?"; then
        sudo systemctl enable docker.service
        success "Docker service enabled"
    fi
fi

echo ""

# ============================================================================
# STEP 4: WinApps Configuration
# ============================================================================

info "Step 4/6: Creating WinApps configuration..."
echo ""

WINAPPS_DIR="$HOME/.config/winapps"
WINAPPS_CONF="$WINAPPS_DIR/winapps.conf"

mkdir -p "$WINAPPS_DIR"

if [ -f "$WINAPPS_CONF" ]; then
    warn "Configuration file already exists: $WINAPPS_CONF"
    
    if ! ask "Overwrite existing configuration?" "N"; then
        info "Keeping existing configuration"
        SKIP_CONFIG=true
    fi
fi

if [ "${SKIP_CONFIG}" != true ]; then
    info "Creating new configuration..."
    
    # Prompt for RDP credentials
    echo ""
    read -p "Windows RDP username: " RDP_USER
    read -sp "Windows RDP password: " RDP_PASS
    echo ""
    
    # Prompt for display scaling
    echo ""
    info "Display scaling options: 100 (default), 140, 180"
    read -p "Display scaling [100]: " RDP_SCALE
    RDP_SCALE=${RDP_SCALE:-100}
    
    # Create config file
    cat > "$WINAPPS_CONF" << EOF
##################################
#   WINAPPS CONFIGURATION FILE   #
##################################

# Windows RDP Credentials
RDP_USER="$RDP_USER"
RDP_PASS="$RDP_PASS"

# Windows Domain (leave empty for local account)
RDP_DOMAIN=""

# Windows IP Address (Docker default)
RDP_IP="127.0.0.1"

# WinApps Backend
WAFLAVOR="docker"

# Display Scaling (100, 140, or 180)
RDP_SCALE="$RDP_SCALE"

# FreeRDP Flags
RDP_FLAGS="/cert:tofu /sound /microphone +home-drive"

# Debug Logging
DEBUG="true"

# Auto-pause Windows when inactive (experimental)
AUTOPAUSE="off"
AUTOPAUSE_TIME="300"

# Timeouts (increase if you experience issues)
PORT_TIMEOUT="5"
RDP_TIMEOUT="30"
APP_SCAN_TIMEOUT="60"
BOOT_TIMEOUT="120"

# Additional settings
HIDEF="on"
REMOVABLE_MEDIA="/run/media"
EOF

    # Secure the config file
    chmod 600 "$WINAPPS_CONF"
    
    success "Configuration file created: $WINAPPS_CONF"
fi

echo ""

# ============================================================================
# STEP 5: Docker Compose Setup
# ============================================================================

info "Step 5/6: Setting up Windows VM configuration..."
echo ""

COMPOSE_FILE="$WINAPPS_DIR/compose.yaml"

# Get recommended resources (from module.lua metadata)
TOTAL_RAM_GB=$(free -g | awk '/^Mem:/{print $2}')
TOTAL_CORES=$(nproc)
RECOMMENDED_RAM=$((TOTAL_RAM_GB * 40 / 100))
RECOMMENDED_CORES=$((TOTAL_CORES / 2))

# Bounds checking
[ "$RECOMMENDED_RAM" -lt 4 ] && RECOMMENDED_RAM=4
[ "$RECOMMENDED_RAM" -gt 16 ] && RECOMMENDED_RAM=16
[ "$RECOMMENDED_CORES" -lt 2 ] && RECOMMENDED_CORES=2
[ "$RECOMMENDED_CORES" -gt 8 ] && RECOMMENDED_CORES=8

info "System resources: ${TOTAL_RAM_GB}GB RAM, ${TOTAL_CORES} CPU cores"
info "Recommended VM: ${RECOMMENDED_RAM}GB RAM, ${RECOMMENDED_CORES} CPU cores"
echo ""

if [ -f "$COMPOSE_FILE" ]; then
    warn "Docker Compose file already exists"
    if ! ask "Overwrite with new configuration?" "N"; then
        info "Keeping existing compose.yaml"
        SKIP_COMPOSE=true
    fi
fi

if [ "${SKIP_COMPOSE}" != true ]; then
    # Ask for confirmation
    read -p "RAM for Windows VM (GB) [$RECOMMENDED_RAM]: " VM_RAM
    VM_RAM=${VM_RAM:-$RECOMMENDED_RAM}
    
    read -p "CPU cores for Windows VM [$RECOMMENDED_CORES]: " VM_CORES
    VM_CORES=${VM_CORES:-$RECOMMENDED_CORES}
    
    info "Creating Docker Compose configuration..."
    
    # Download WinApps compose.yaml template
    if curl -sSL https://raw.githubusercontent.com/winapps-org/winapps/main/compose.yaml -o "$COMPOSE_FILE"; then
        # Modify RAM and CPU settings
        sed -i "s/RAM_SIZE: \"4G\"/RAM_SIZE: \"${VM_RAM}G\"/" "$COMPOSE_FILE"
        sed -i "s/CPU_CORES: \"2\"/CPU_CORES: \"${VM_CORES}\"/" "$COMPOSE_FILE"
        
        success "Docker Compose file created: $COMPOSE_FILE"
        info "VM configuration: ${VM_RAM}GB RAM, ${VM_CORES} CPU cores"
    else
        error "Failed to download compose.yaml template"
        warn "You'll need to create it manually or download from:"
        warn "  https://github.com/winapps-org/winapps/blob/main/compose.yaml"
    fi
fi

echo ""

# ============================================================================
# STEP 6: FreeRDP Certificate Cleanup
# ============================================================================

info "Step 6/6: Cleaning old FreeRDP certificates..."
echo ""

FREERDP_CERTS="$HOME/.config/freerdp/server"
if [ -d "$FREERDP_CERTS" ]; then
    CERT_COUNT=$(find "$FREERDP_CERTS" -name "*127.0.0.1*.pem" 2>/dev/null | wc -l)
    
    if [ "$CERT_COUNT" -gt 0 ]; then
        warn "Found $CERT_COUNT old certificate(s) for 127.0.0.1"
        
        if ask "Remove old certificates? (recommended for fresh setup)"; then
            find "$FREERDP_CERTS" -name "*127.0.0.1*.pem" -delete
            success "Old certificates removed"
        fi
    else
        info "No old certificates found"
    fi
else
    info "FreeRDP certificates directory doesn't exist yet (normal for first setup)"
fi

echo ""

# ============================================================================
# Setup Complete
# ============================================================================

echo "╔════════════════════════════════════════════════════════════╗"
echo "║              WinApps Setup Complete! 🎉                    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

success "All configuration files created successfully!"
echo ""

info "📋 Next Steps:"
echo ""
echo "  1. Start the Windows VM:"
echo "     docker compose -f ~/.config/winapps/compose.yaml up -d"
echo ""
echo "  2. Access Windows setup via web browser:"
echo "     http://127.0.0.1:8006"
echo ""
echo "  3. Complete Windows installation in the browser"
echo "     - Create Windows user matching RDP_USER in config"
echo "     - Set password matching RDP_PASS in config"
echo "     - Professional/Enterprise/Server edition required"
echo ""
echo "  4. Test RDP connection:"
echo "     xfreerdp3 /u:\"<username>\" /p:\"<password>\" /v:127.0.0.1 /cert:tofu"
echo ""
echo "  5. Install WinApps:"
echo "     bash <(curl https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh)"
echo ""

info "📖 Documentation:"
echo "  GitHub: https://github.com/winapps-org/winapps"
echo "  Docker Guide: https://github.com/winapps-org/winapps/blob/main/docs/docker.md"
echo ""

warn "⚠️  Important Reminders:"
echo "  • If you added yourself to docker group, LOG OUT and LOG BACK IN"
echo "  • Windows 'Home' edition does NOT support RDP - use Pro/Enterprise/Server"
echo "  • First RDP connection will prompt to accept the certificate"
echo ""

info "💡 Tip: Manage VM with docker compose:"
echo "  docker compose -f ~/.config/winapps/compose.yaml start|stop|restart|pause"
echo ""
