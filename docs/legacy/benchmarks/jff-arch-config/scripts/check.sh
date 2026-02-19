#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "repository" "check" "$@"
trap 'oplog_end $?' EXIT

run_step() {
  local name="$1"
  shift
  echo "==> $name"
  "$@"
}

run_step "Host state" "$ROOT_DIR/scripts/host.sh" show
run_step "Hypr deploy status" "$ROOT_DIR/scripts/deploy-hypr.sh" --status
run_step "Secret Service" "$ROOT_DIR/scripts/check-secrets-service.sh"

if command -v cargo >/dev/null 2>&1; then
  run_step "Manifest validation (Rust CLI)" \
    cargo run -p app-cli --manifest-path "$ROOT_DIR/rust/Cargo.toml" -- validate --root "$ROOT_DIR"
else
  echo "==> Manifest validation (Rust CLI)"
  echo "Skipping: cargo not found in PATH"
fi

echo "All checks completed."
