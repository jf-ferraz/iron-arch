#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "systemd-user" "sync-user-units" "$@"
trap 'oplog_end $?' EXIT

SRC_DIR="$ROOT_DIR/modules/services/systemd-user/units"
DST_DIR="$HOME/.config/systemd/user"

mkdir -p "$DST_DIR"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "Source units directory not found: $SRC_DIR" >&2
  exit 1
fi

for unit in "$SRC_DIR"/*; do
  [[ -f "$unit" ]] || continue
  ln -sf "$unit" "$DST_DIR/$(basename "$unit")"
done

systemctl --user daemon-reload

echo "Synced user units from $SRC_DIR to $DST_DIR"
