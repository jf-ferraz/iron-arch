#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "hyprland" "deploy-hypr" "$@"
trap 'oplog_end $?' EXIT

STOW_DIR="$ROOT_DIR/modules/configs/hyprland"
PKG_NAME="stow"
TARGET_DIR="$HOME"
HOSTS_MANIFEST="$ROOT_DIR/app/manifests/hosts.toml"
ACTIVE_HOST_FILE="$ROOT_DIR/app/state/run/active-host"
CLI_HOST_ID=""
DRY_RUN=0
ROLLBACK=0
ADOPT=0
STATUS=0

usage() {
  cat <<USAGE
Usage: scripts/deploy-hypr.sh [options]

Options:
  --host <id>      Host id (default resolution: --host > HOST_ID > active-host > hosts.toml default_host)
  --target <path>  Deployment target for stow (default: \$HOME)
  --dry-run        Simulate deploy only
  --rollback       Remove stow-managed symlinks
  --adopt          Adopt existing files into the stow package
  --status         Check deployment status only
  -h, --help       Show this help
USAGE
}

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
  elif [[ -n "${HOST_ID:-}" ]]; then
    resolved="$HOST_ID"
  else
    resolved="$(read_active_host || true)"
    if [[ -z "$resolved" ]]; then
      resolved="$(read_default_host || true)"
    fi
  fi

  if [[ -z "$resolved" ]]; then
    echo "Unable to resolve host id. Set one with app-cli host set <id> or pass --host." >&2
    exit 1
  fi

  echo "$resolved"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      CLI_HOST_ID="${2:-}"
      if [[ -z "$CLI_HOST_ID" ]]; then
        echo "--host requires a value" >&2
        exit 2
      fi
      shift 2
      ;;
    --target)
      TARGET_DIR="${2:-}"
      if [[ -z "$TARGET_DIR" ]]; then
        echo "--target requires a value" >&2
        exit 2
      fi
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --rollback)
      ROLLBACK=1
      shift
      ;;
    --adopt)
      ADOPT=1
      shift
      ;;
    --status)
      STATUS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if ! command -v stow >/dev/null 2>&1; then
  echo "stow is required but not found in PATH" >&2
  echo "Arch hint: sudo pacman -S stow" >&2
  exit 1
fi

if [[ "$STATUS" -eq 1 ]]; then
  if [[ -L "$TARGET_DIR/.config/hypr/hyprland.conf" || -f "$TARGET_DIR/.config/hypr/hyprland.conf" ]]; then
    echo "Hyprland config present at $TARGET_DIR/.config/hypr/hyprland.conf"
    exit 0
  fi
  echo "Hyprland config not found at $TARGET_DIR/.config/hypr/hyprland.conf" >&2
  exit 1
fi

if [[ "$ROLLBACK" -eq 1 ]]; then
  echo "Rollback: stow -D"
  stow -d "$STOW_DIR" -t "$TARGET_DIR" -D "$PKG_NAME"
  exit 0
fi

SELECTED_HOST="$(resolve_host_id)"
echo "Selected host: $SELECTED_HOST"
"$ROOT_DIR/scripts/hypr-sync-host-overlay.sh" "$SELECTED_HOST"

SIM_ARGS=( -n -v -d "$STOW_DIR" -t "$TARGET_DIR" )
APPLY_ARGS=( -d "$STOW_DIR" -t "$TARGET_DIR" )

if [[ "$ADOPT" -eq 1 ]]; then
  SIM_ARGS+=( --adopt )
  APPLY_ARGS+=( --adopt )
fi

SIM_ARGS+=( "$PKG_NAME" )
APPLY_ARGS+=( "$PKG_NAME" )

echo "Preflight: stow simulation"
if ! stow "${SIM_ARGS[@]}"; then
  echo "Stow simulation failed. Resolve conflicts or rerun with --adopt if appropriate." >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry-run successful; no changes applied."
  exit 0
fi

echo "Applying deploy"
stow "${APPLY_ARGS[@]}"

echo "Deploy complete: $TARGET_DIR/.config/hypr"
