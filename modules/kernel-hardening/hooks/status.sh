#!/usr/bin/env bash
# Exit codes: 0=installed, 1=not installed, 2=partial
set -euo pipefail

checks=0
passed=0

check_param() {
    local param="$1" expected="$2"
    checks=$((checks + 1))
    local val
    val=$(sysctl -n "$param" 2>/dev/null || echo "")
    [[ "$val" == "$expected" ]] && passed=$((passed + 1))
}

check_param "kernel.randomize_va_space" "2"
check_param "kernel.kptr_restrict" "2"
check_param "kernel.dmesg_restrict" "1"
check_param "net.ipv4.tcp_syncookies" "1"
check_param "net.ipv4.conf.all.rp_filter" "1"

if [[ $passed -eq $checks ]]; then
    echo "All $checks kernel parameters verified"
    exit 0
elif [[ $passed -gt 0 ]]; then
    echo "$passed/$checks parameters active"
    exit 2
else
    echo "Kernel hardening not active"
    exit 1
fi
