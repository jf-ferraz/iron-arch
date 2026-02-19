#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "repository" "ci-check" "$@"
trap 'oplog_end $?' EXIT

cd "$ROOT_DIR"

echo "==> Running lint suite"
"$ROOT_DIR/scripts/lint.sh"

echo "==> Running test suite"
"$ROOT_DIR/scripts/test.sh"

echo "ci-check.sh passed."
