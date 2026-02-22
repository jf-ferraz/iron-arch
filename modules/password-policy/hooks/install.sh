#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

PWQUALITY_SRC="$SCRIPT_DIR/../configs/pwquality.conf"
PWQUALITY_DST="/etc/security/pwquality.conf"

log_info "Deploying password policy..."

# Deploy pwquality config
backup_file "$PWQUALITY_DST"
register_rollback "restore_file '$PWQUALITY_DST'"
safe_copy "$PWQUALITY_SRC" "$PWQUALITY_DST"

# Configure faillock (account lockout)
FAILLOCK_CONF="/etc/security/faillock.conf"
if [[ -f "$FAILLOCK_CONF" ]]; then
    backup_file "$FAILLOCK_CONF"
    register_rollback "restore_file '$FAILLOCK_CONF'"
fi

cat <<'FAILLOCK' | sudo tee "$FAILLOCK_CONF" >/dev/null
# Iron password policy - account lockout
deny = 5
unlock_time = 600
fail_interval = 900
even_deny_root
root_unlock_time = 600
FAILLOCK

clear_trap_rollback
log_success "Password policy configured"
