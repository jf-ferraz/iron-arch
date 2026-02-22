#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "hyprland" "hypr-sync-host-overlay" "$@"
trap 'oplog_end $?' EXIT

HOSTS_MANIFEST="$ROOT_DIR/app/manifests/hosts.toml"
ACTIVE_HOST_FILE="$ROOT_DIR/app/state/run/active-host"
CLI_HOST_ID="${1:-}"
ENV_HOST_ID="${HOST_ID:-}"

read_default_host() {
  if [[ ! -f "$HOSTS_MANIFEST" ]]; then
    return 1
  fi

  sed -n 's/^default_host[[:space:]]*=[[:space:]]*"\([^"]*\)"[[:space:]]*$/\1/p' "$HOSTS_MANIFEST" | head -n 1
}

read_active_host() {
  if [[ -f "$ACTIVE_HOST_FILE" ]]; then
    tr -d '[:space:]' < "$ACTIVE_HOST_FILE"
  fi
}

resolve_host_id() {
  local resolved=""
  if [[ -n "$CLI_HOST_ID" ]]; then
    resolved="$CLI_HOST_ID"
  elif [[ -n "$ENV_HOST_ID" ]]; then
    resolved="$ENV_HOST_ID"
  else
    resolved="$(read_active_host || true)"
    if [[ -z "$resolved" ]]; then
      resolved="$(read_default_host || true)"
    fi
  fi

  if [[ -z "$resolved" ]]; then
    echo "Unable to resolve host id. Set one via: app-cli host set <id> or pass --host." >&2
    exit 1
  fi

  echo "$resolved"
}

HOST_ID="$(resolve_host_id)"
SRC="$ROOT_DIR/hosts/$HOST_ID/hyprland/overrides.conf"
DST="$ROOT_DIR/modules/configs/hyprland/stow/.config/hypr/host-overrides.conf"

if [[ ! -f "$SRC" ]]; then
  echo "Missing host override: $SRC" >&2
  exit 1
fi

cat > "$DST" <<EOF2
# Generated from hosts/$HOST_ID/hyprland/overrides.conf
# Do not edit manually.
EOF2
cat "$SRC" >> "$DST"

echo "Rendered host override for '$HOST_ID' -> $DST"
