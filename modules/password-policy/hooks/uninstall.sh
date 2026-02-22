#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing password policy..."
restore_file /etc/security/pwquality.conf
restore_file /etc/security/faillock.conf
log_success "Password policy removed"
