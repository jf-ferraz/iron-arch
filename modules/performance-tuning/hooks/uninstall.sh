#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Reverting performance tuning..."
sudo rm -f /etc/udev/rules.d/60-iron-ioscheduler.rules
sudo rm -f /etc/systemd/zram-generator.conf
restore_file /etc/makepkg.conf
restore_file /etc/fstab
log_success "Performance tuning reverted"
