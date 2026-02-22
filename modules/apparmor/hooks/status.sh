#!/usr/bin/env bash
set -euo pipefail

if ! command -v aa-status &>/dev/null; then
    echo "AppArmor not installed"
    exit 1
fi

if ! systemctl is-active --quiet apparmor 2>/dev/null; then
    echo "AppArmor service not running"
    exit 1
fi

profiles=$(sudo aa-status 2>/dev/null | grep "profiles are loaded" | awk '{print $1}' || echo "0")
enforced=$(sudo aa-status 2>/dev/null | grep "profiles are in enforce" | awk '{print $1}' || echo "0")

if [[ "$profiles" -gt 0 ]]; then
    echo "$enforced/$profiles profiles enforced"
    [[ "$enforced" -eq "$profiles" ]] && exit 0 || exit 2
else
    echo "No profiles loaded"
    exit 2
fi
