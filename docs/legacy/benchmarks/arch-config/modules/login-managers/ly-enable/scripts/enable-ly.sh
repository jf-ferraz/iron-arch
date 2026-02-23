#!/usr/bin/env bash
# Enable Ly display manager

set -euo pipefail

if [ "$EUID" -ne 0 ]; then
  echo "Error: this script must be run as root (sudo)." >&2
  exit 1
fi

echo "Enabling Ly display manager..."

# Disable other common display managers if enabled
for dm in gdm sddm lightdm greetd; do
  if systemctl is-enabled "${dm}.service" &>/dev/null; then
    echo "Disabling ${dm}.service"
    systemctl disable "${dm}.service"
  fi
done

# Remove display-manager.service symlink if it exists and points to something else
if [ -L /etc/systemd/system/display-manager.service ]; then
  current_target=$(readlink /etc/systemd/system/display-manager.service)
  if [ "$current_target" != "/usr/lib/systemd/system/ly.service" ]; then
    echo "Removing existing display-manager.service symlink (points to $current_target)"
    rm -f /etc/systemd/system/display-manager.service
  fi
fi

# Check if ly is installed (binary is named ly-dm)
LY_BIN=""
for path in /usr/bin/ly-dm /usr/local/bin/ly-dm; do
  if [ -x "$path" ]; then
    LY_BIN="$path"
    break
  fi
done

if [ -z "$LY_BIN" ] && command -v ly-dm &>/dev/null; then
  LY_BIN="$(command -v ly-dm)"
fi

if [ -z "$LY_BIN" ]; then
  echo "Error: ly-dm binary not found" >&2
  exit 1
fi

# Create ly service file if it doesn't exist
if [ ! -f /usr/lib/systemd/system/ly.service ]; then
  echo "Creating ly.service file..."
  cat > /usr/lib/systemd/system/ly.service << EOF
[Unit]
Description=TUI display manager
After=systemd-user-sessions.service getty@tty1.service plymouth-quit.service systemd-logind.service

[Service]
Type=simple
Environment="TERM=linux"
Environment="XDG_SESSION_TYPE=tty"
ExecStart=$LY_BIN
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes

[Install]
Alias=display-manager.service
EOF
  systemctl daemon-reload
fi

# Enable Ly
if systemctl is-enabled ly.service &>/dev/null; then
  echo "Ly is already enabled"
else
  systemctl enable --now ly.service
  echo "Ly enabled"
fi

echo "Ly setup complete."
