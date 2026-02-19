#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "repository" "lint" "$@"
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

require_cmd() {
  local cmd="$1"
  local hint="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required tool: $cmd" >&2
    echo "Arch install hint: $hint" >&2
    FAILURES=$((FAILURES + 1))
    return 1
  fi
  return 0
}

cd "$ROOT_DIR"

require_cmd shellcheck "sudo pacman -S shellcheck" && \
  run_step "Shellcheck scripts" sh -c 'shellcheck scripts/*.sh scripts/lib/*.sh'

require_cmd shfmt "sudo pacman -S shfmt" && \
  run_step "shfmt check" sh -c 'shfmt -d scripts/*.sh scripts/lib/*.sh'

if command -v cargo >/dev/null 2>&1; then
  run_step "cargo fmt --check" \
    cargo fmt --manifest-path rust/Cargo.toml --all -- --check
  run_step "cargo clippy" \
    cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --all-features -- -D warnings
else
  echo "Missing required tool: cargo" >&2
  echo "Arch install hint: sudo pacman -S rust" >&2
  FAILURES=$((FAILURES + 1))
fi

if [[ "$FAILURES" -gt 0 ]]; then
  echo "lint.sh finished with $FAILURES failure(s)." >&2
  exit 1
fi

echo "lint.sh passed."
