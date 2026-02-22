#!/usr/bin/env bash
set -euo pipefail

if ! command -v firejail &>/dev/null; then
    echo "Firejail not installed"
    exit 1
fi

sandboxed=$(firejail --list 2>/dev/null | wc -l || echo "0")
symlinks=$(find /usr/local/bin -lname '*/firejail' 2>/dev/null | wc -l || echo "0")

echo "Firejail active: $sandboxed sandboxes, $symlinks symlinks"
[[ "$symlinks" -gt 0 ]] && exit 0 || exit 2
