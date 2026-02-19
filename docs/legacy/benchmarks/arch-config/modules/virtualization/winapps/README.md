# WinApps Module

Seamlessly run Windows applications on Linux using Docker and RDP.

## Overview

This module sets up everything needed to run Windows applications natively integrated into your Linux desktop using the [WinApps project](https://github.com/winapps-org/winapps).

**Your System:**
- RAM: 31 GB
- CPU: Intel (24 cores)
- **Recommended VM:** 12GB RAM, 8 CPU cores

## Features

- ✅ **Hardware Detection** - Automatically checks CPU virtualization and RAM
- ✅ **Docker Backend** - Automated Windows VM in Docker container
- ✅ **Interactive Setup** - Guided configuration wizard
- ✅ **Kernel Module Management** - Handles iptables requirements
- ✅ **Smart Defaults** - Calculates optimal VM resources
- ✅ **Secure Configuration** - Proper permissions for credentials

## Installation

### 1. Enable the Module

```bash
dcli module enable winapps
```

### 2. Install Packages

```bash
dcli sync
```

This will install:
- `freerdp` - RDP client (v3+)
- `docker` & `docker-compose` - Container runtime
- `curl`, `dialog`, `git` - Utilities
- `iproute2`, `libnotify`, `openbsd-netcat` - Networking
- `iptables-nft` - For folder sharing

### 3. Run Interactive Setup

```bash
dcli hooks run winapps
```

Or manually:
```bash
dcli module run-hook winapps
```

The setup wizard will:
1. ✅ Check system requirements (KVM, iptables modules)
2. ✅ Add you to docker group (with prompt)
3. ✅ Enable Docker service
4. ✅ Create WinApps configuration file
5. ✅ Set up Docker Compose with smart VM resources
6. ✅ Clean old FreeRDP certificates

### 4. Start Windows VM

```bash
docker compose -f ~/.config/winapps/compose.yaml up -d
```

### 5. Complete Windows Setup

Open in browser: http://127.0.0.1:8006

**Important:**
- Install Windows 10/11 **Pro, Enterprise, or Server** (NOT Home edition)
- Create Windows user matching your `RDP_USER` from config
- Set password matching your `RDP_PASS` from config

### 6. Test RDP Connection

```bash
xfreerdp3 /u:"YourUsername" /p:"YourPassword" /v:127.0.0.1 /cert:tofu
```

Accept the certificate when prompted.

### 7. Install WinApps

```bash
bash <(curl https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh)
```

## Configuration Files

After setup, you'll have:

- `~/.config/winapps/winapps.conf` - WinApps configuration (secure, 600 permissions)
- `~/.config/winapps/compose.yaml` - Docker Compose VM configuration

## Managing the Windows VM

```bash
# Start VM
docker compose -f ~/.config/winapps/compose.yaml start

# Stop VM (graceful shutdown)
docker compose -f ~/.config/winapps/compose.yaml stop

# Restart VM
docker compose -f ~/.config/winapps/compose.yaml restart

# Pause VM (suspend)
docker compose -f ~/.config/winapps/compose.yaml pause

# Unpause VM (resume)
docker compose -f ~/.config/winapps/compose.yaml unpause

# Force stop VM
docker compose -f ~/.config/winapps/compose.yaml kill

# Remove VM (keeps data)
docker compose -f ~/.config/winapps/compose.yaml down
```

## System Requirements

### Minimum:
- **RAM:** 8GB+ total (4GB for VM)
- **CPU:** 2+ cores with VT-x/AMD-V virtualization
- **Disk:** 20GB+ free space
- **OS:** Arch Linux with KVM support

### Recommended (Your System):
- **RAM:** 16GB+ total (your system: 31GB ✅)
- **CPU:** 4+ cores (your system: 24 cores ✅)
- **Windows:** Pro/Enterprise/Server edition

## Kernel Modules Required

The setup script will check and optionally load:

- `ip_tables` - iptables support
- `iptable_nat` - NAT for container networking
- `kvm` (or `kvm_intel`/`kvm_amd`) - Virtualization

## Troubleshooting

### "Permission denied" when running docker commands

**Solution:** You need to log out and log back in after being added to docker group.

Or temporarily:
```bash
newgrp docker
```

### "KVM not available" error

**Solution:** Enable virtualization in BIOS/UEFI settings:
- Intel: Enable "VT-x" or "Intel Virtualization Technology"
- AMD: Enable "AMD-V" or "SVM Mode"

### Folder sharing not working

**Solution:** Ensure iptables modules are loaded:
```bash
sudo modprobe ip_tables iptable_nat
```

### RDP connection fails

**Solution:**
1. Check Windows VM is running: `docker ps`
2. Verify credentials in `~/.config/winapps/winapps.conf`
3. Remove old certificates: `rm ~/.config/freerdp/server/*127.0.0.1*.pem`
4. Ensure Windows user account exists (not just PIN)

### Performance issues

**Solution:** Adjust VM resources in `~/.config/winapps/compose.yaml`:
```yaml
environment:
  RAM_SIZE: "12G"    # Increase if you have RAM to spare
  CPU_CORES: "8"     # Increase for better performance
```

Then recreate container:
```bash
docker compose -f ~/.config/winapps/compose.yaml down
docker compose -f ~/.config/winapps/compose.yaml up -d
```

## Documentation

- **Official WinApps:** https://github.com/winapps-org/winapps
- **Docker Setup Guide:** https://github.com/winapps-org/winapps/blob/main/docs/docker.md
- **Troubleshooting:** https://github.com/winapps-org/winapps/wiki

## Module Structure

```
modules/winapps/
├── module.lua              # Main module with Lua hardware detection
├── packages.lua            # Package reference (managed by module.lua)
├── scripts/
│   └── setup-winapps.sh   # Interactive post-install hook
└── README.md              # This file
```

## What Gets Installed

The module will run Windows in a Docker container and expose applications via FreeRDP. You can then:

- Launch Windows apps from your Linux application menu
- Right-click files to open in Windows applications
- Seamlessly use Microsoft Office, Adobe apps, etc.
- Access your Linux home directory from Windows (`\\tsclient\home`)

## Notes

- ⚠️ **First launch takes time** - Windows needs to install and configure
- ⚠️ **Windows Home edition does NOT work** - RDP requires Pro/Enterprise/Server
- ⚠️ **Re-login required** after docker group addition
- ✅ Your Linux `/home` directory is accessible in Windows
- ✅ Docker containers are isolated and secure
- ✅ No dual-boot needed - both OSes run simultaneously

## Support

If you encounter issues:

1. Check the [WinApps GitHub Issues](https://github.com/winapps-org/winapps/issues)
2. Verify all system requirements are met
3. Review logs: `~/.local/share/winapps/winapps.log` (if DEBUG=true)
4. Check Docker logs: `docker compose -f ~/.config/winapps/compose.yaml logs`

## Uninstalling

To remove WinApps:

```bash
# Stop and remove Windows VM
docker compose -f ~/.config/winapps/compose.yaml down --volumes

# Disable module
dcli module disable winapps

# Optional: Remove configuration
rm -rf ~/.config/winapps
```

## License

This module follows the WinApps project license. See [WinApps LICENSE](https://github.com/winapps-org/winapps/blob/main/LICENSE.md).
