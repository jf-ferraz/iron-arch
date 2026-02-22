#!/usr/bin/env bash
set -euo pipefail

checks=0
passed=0

# Check pwquality
checks=$((checks + 1))
if [[ -f /etc/security/pwquality.conf ]] && grep -q "minlen" /etc/security/pwquality.conf 2>/dev/null; then
    minlen=$(grep "^minlen" /etc/security/pwquality.conf 2>/dev/null | awk -F= '{print $2}' | tr -d ' ')
    if [[ "${minlen:-0}" -ge 12 ]]; then
        passed=$((passed + 1))
    fi
fi

# Check faillock
checks=$((checks + 1))
if [[ -f /etc/security/faillock.conf ]] && grep -q "deny" /etc/security/faillock.conf 2>/dev/null; then
    passed=$((passed + 1))
fi

if [[ $passed -eq $checks ]]; then
    echo "Password policy active (minlen=$minlen, faillock enabled)"
    exit 0
elif [[ $passed -gt 0 ]]; then
    echo "$passed/$checks policy components active"
    exit 2
else
    echo "Password policy not configured"
    exit 1
fi
