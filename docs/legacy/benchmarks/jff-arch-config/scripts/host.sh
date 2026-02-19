#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "hosts" "host.${1:-unknown}" "$@"
trap 'oplog_end $?' EXIT

HOSTS_MANIFEST="$ROOT_DIR/app/manifests/hosts.toml"
ACTIVE_HOST_FILE="$ROOT_DIR/app/state/run/active-host"

usage() {
  cat <<USAGE
Usage: scripts/host.sh <command> [args]

Commands:
  list           List host ids from hosts manifest
  show           Show active and default host
  set <id>       Set active host in app/state/run/active-host
USAGE
}

if [[ $# -lt 1 ]]; then
  usage
  exit 2
fi

read_default_host() {
  sed -n 's/^default_host[[:space:]]*=[[:space:]]*"\([^"]*\)"[[:space:]]*$/\1/p' "$HOSTS_MANIFEST" | head -n 1
}

list_hosts() {
  sed -n 's/^id[[:space:]]*=[[:space:]]*"\([^"]*\)"[[:space:]]*$/\1/p' "$HOSTS_MANIFEST"
}

host_exists() {
  local candidate="$1"
  list_hosts | grep -Fx "$candidate" >/dev/null 2>&1
}

case "$1" in
  list)
    if [[ ! -f "$HOSTS_MANIFEST" ]]; then
      echo "hosts manifest not found: $HOSTS_MANIFEST" >&2
      exit 1
    fi
    list_hosts
    ;;
  show)
    default_host=""
    active_host=""

    if [[ -f "$HOSTS_MANIFEST" ]]; then
      default_host="$(read_default_host || true)"
    fi
    if [[ -f "$ACTIVE_HOST_FILE" ]]; then
      active_host="$(tr -d '[:space:]' < "$ACTIVE_HOST_FILE")"
    fi

    echo "active=${active_host:-<unset>}"
    echo "default=${default_host:-<unset>}"
    ;;
  set)
    host_id="${2:-}"
    if [[ -z "$host_id" ]]; then
      echo "set requires <id>" >&2
      exit 2
    fi
    if [[ ! -f "$HOSTS_MANIFEST" ]]; then
      echo "hosts manifest not found: $HOSTS_MANIFEST" >&2
      exit 1
    fi
    if ! host_exists "$host_id"; then
      echo "unknown host id: $host_id" >&2
      echo "known hosts:" >&2
      list_hosts >&2
      exit 1
    fi

    mkdir -p "$(dirname "$ACTIVE_HOST_FILE")"
    printf '%s\n' "$host_id" > "$ACTIVE_HOST_FILE"
    echo "active host set to '$host_id'"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown command: $1" >&2
    usage
    exit 2
    ;;
esac
