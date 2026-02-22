#!/usr/bin/env bash
set -euo pipefail

if ! command -v stubby &>/dev/null; then
    echo "Stubby not installed"
    exit 1
fi

if ! systemctl is-active --quiet stubby 2>/dev/null; then
    echo "Stubby service not running"
    exit 1
fi

# Test DNS resolution through stubby
if dig +short +timeout=3 @127.0.0.1 example.com &>/dev/null; then
    echo "DNS-over-TLS active via Stubby"
    exit 0
else
    echo "Stubby running but DNS resolution failed"
    exit 2
fi
