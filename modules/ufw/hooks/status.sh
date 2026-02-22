#!/usr/bin/env bash
set -euo pipefail

if ! command -v ufw &>/dev/null; then
    echo "UFW not installed"
    exit 1
fi

status=$(sudo ufw status 2>/dev/null | head -1)
if echo "$status" | grep -q "active"; then
    rules=$(sudo ufw status numbered 2>/dev/null | grep -c '^\[' || echo "0")
    echo "UFW active with $rules rules"
    exit 0
else
    echo "UFW installed but inactive"
    exit 1
fi
