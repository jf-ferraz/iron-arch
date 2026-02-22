#!/usr/bin/env bash
set -euo pipefail

SSHD_CONF="/etc/ssh/sshd_config.d/99-iron-hardening.conf"

if [[ ! -f "$SSHD_CONF" ]]; then
    echo "SSH hardening config not deployed"
    exit 1
fi

checks=0
passed=0

check_sshd() {
    local key="$1" expected="$2"
    checks=$((checks + 1))
    if sshd -T 2>/dev/null | grep -qi "^${key} ${expected}"; then
        passed=$((passed + 1))
    fi
}

check_sshd "permitrootlogin" "no"
check_sshd "maxauthtries" "3"
check_sshd "x11forwarding" "no"
check_sshd "permitemptypasswords" "no"

if [[ $passed -eq $checks ]]; then
    echo "All $checks SSH settings verified"
    exit 0
elif [[ $passed -gt 0 ]]; then
    echo "$passed/$checks SSH settings active"
    exit 2
else
    echo "SSH hardening not effective"
    exit 1
fi
