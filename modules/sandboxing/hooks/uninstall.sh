#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Removing Firejail configuration..."
if command_exists firecfg; then
    sudo firecfg --clean 2>/dev/null || true
fi
restore_file "/etc/firejail/firejail.config"
log_success "Firejail sandboxing removed"
