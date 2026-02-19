#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "repository" "test" "$@"
trap 'oplog_end $?' EXIT

FAILURES=0

run_step() {
  local name="$1"
  shift
  echo "==> $name"
  if "$@"; then
    echo "PASS: $name"
  else
    echo "FAIL: $name" >&2
    FAILURES=$((FAILURES + 1))
  fi
}

cd "$ROOT_DIR"

if command -v cargo >/dev/null 2>&1; then
  run_step "cargo test" cargo test --manifest-path rust/Cargo.toml --workspace
  run_step "app-cli validate" \
    cargo run -p app-cli --manifest-path rust/Cargo.toml -- validate --root .
  run_step "app-cli plan (hyprland)" \
    cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --module hyprland --root .
else
  echo "Missing required tool: cargo" >&2
  echo "Arch install hint: sudo pacman -S rust" >&2
  FAILURES=$((FAILURES + 1))
fi

if [[ "$FAILURES" -gt 0 ]]; then
  echo "test.sh finished with $FAILURES failure(s)." >&2
  exit 1
fi

echo "test.sh passed."
