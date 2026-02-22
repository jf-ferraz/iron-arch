#!/usr/bin/env bash
set -euo pipefail

if ! command -v fail2ban-client &>/dev/null; then
    echo "Fail2ban not installed"
    exit 1
fi

if ! systemctl is-active --quiet fail2ban 2>/dev/null; then
    echo "Fail2ban service not running"
    exit 1
fi

jails=$(sudo fail2ban-client status 2>/dev/null | grep "Jail list" | sed 's/.*:\s*//' | tr -d ' ')
if [[ -n "$jails" ]]; then
    count=$(echo "$jails" | tr ',' '\n' | wc -l)
    echo "Fail2ban active with $count jails"
    exit 0
else
    echo "Fail2ban running but no jails configured"
    exit 2
fi
