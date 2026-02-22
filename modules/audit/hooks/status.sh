#!/usr/bin/env bash
set -euo pipefail

if ! command -v auditctl &>/dev/null; then
    echo "Audit not installed"
    exit 1
fi

if ! systemctl is-active --quiet auditd 2>/dev/null; then
    echo "Auditd service not running"
    exit 1
fi

rules=$(sudo auditctl -l 2>/dev/null | wc -l || echo "0")
if [[ "$rules" -gt 0 ]]; then
    echo "Auditd active with $rules rules"
    exit 0
else
    echo "Auditd running but no rules loaded"
    exit 2
fi
