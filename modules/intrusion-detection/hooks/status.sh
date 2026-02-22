#!/usr/bin/env bash
set -euo pipefail

checks=0
passed=0

if command -v aide &>/dev/null; then
    checks=$((checks + 1))
    [[ -f /var/lib/aide/aide.db ]] && passed=$((passed + 1))
fi

if command -v rkhunter &>/dev/null; then
    checks=$((checks + 1))
    [[ -f /var/lib/rkhunter/db/rkhunter.dat ]] && passed=$((passed + 1))
fi

timer_active=false
if systemctl is-active --quiet iron-integrity-scan.timer 2>/dev/null; then
    timer_active=true
fi

if [[ $checks -eq 0 ]]; then
    echo "No intrusion detection tools installed"
    exit 1
fi

if [[ $passed -eq $checks ]] && $timer_active; then
    echo "AIDE + rkhunter initialized, weekly scan active"
    exit 0
elif [[ $passed -gt 0 ]]; then
    echo "$passed/$checks tools initialized"
    exit 2
else
    echo "Intrusion detection not configured"
    exit 1
fi
