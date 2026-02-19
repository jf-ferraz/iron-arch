#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "systemd-user" "check-secrets-service" "$@"
trap 'oplog_end $?' EXIT

if ! command -v gnome-keyring-daemon >/dev/null 2>&1; then
  echo "ERROR: gnome-keyring-daemon not found. Install package: gnome-keyring" >&2
  exit 1
fi

if ! command -v busctl >/dev/null 2>&1; then
  echo "ERROR: busctl not found. Install package providing systemd user tools." >&2
  exit 1
fi

if busctl --user --list | grep -q '^org[.]freedesktop[.]secrets[[:space:]]'; then
  echo "OK: org.freedesktop.secrets is available on the user bus"
  exit 0
fi

echo "ERROR: org.freedesktop.secrets not found on user bus" >&2
echo "Hint: relogin into Hyprland session after enabling gnome-keyring autostart" >&2
exit 1
