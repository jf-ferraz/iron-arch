#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

trap_rollback

log_info "Applying performance tuning..."

# I/O scheduler udev rules
UDEV_RULE="/etc/udev/rules.d/60-iron-ioscheduler.rules"
cat <<'UDEV' | sudo tee "$UDEV_RULE" >/dev/null
# Iron: I/O scheduler optimization
# SSD/NVMe: mq-deadline, HDD: bfq
ACTION=="add|change", KERNEL=="sd[a-z]*|mmcblk[0-9]*|nvme[0-9]*", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline"
ACTION=="add|change", KERNEL=="sd[a-z]*", ATTR{queue/rotational}=="1", ATTR{queue/scheduler}="bfq"
UDEV
register_rollback "sudo rm -f '$UDEV_RULE'"

# Zram configuration
ZRAM_CONF="/etc/systemd/zram-generator.conf"
if [[ ! -f "$ZRAM_CONF" ]]; then
    cat <<'ZRAM' | sudo tee "$ZRAM_CONF" >/dev/null
[zram0]
zram-size = ram / 2
compression-algorithm = zstd
ZRAM
    register_rollback "sudo rm -f '$ZRAM_CONF'"
fi

# Optimize makepkg
MAKEPKG="/etc/makepkg.conf"
if [[ -f "$MAKEPKG" ]]; then
    backup_file "$MAKEPKG"
    register_rollback "restore_file '$MAKEPKG'"

    # Parallel compilation
    if ! grep -q "^MAKEFLAGS.*-j" "$MAKEPKG"; then
        sudo sed -i "s/^#*MAKEFLAGS=.*/MAKEFLAGS=\"-j\$(nproc)\"/" "$MAKEPKG"
    fi

    # Rust native CPU target
    if ! grep -q "RUSTFLAGS.*target-cpu=native" "$MAKEPKG"; then
        echo 'RUSTFLAGS="-C target-cpu=native"' | sudo tee -a "$MAKEPKG" >/dev/null
    fi
fi

# Add noatime to ext4/btrfs fstab entries
FSTAB="/etc/fstab"
if [[ -f "$FSTAB" ]] && grep -qE "ext4|btrfs" "$FSTAB"; then
    if ! grep -q "noatime" "$FSTAB"; then
        backup_file "$FSTAB"
        register_rollback "restore_file '$FSTAB'"
        sudo sed -i '/ext4\|btrfs/ s/defaults/defaults,noatime/' "$FSTAB"
        log_warn "fstab updated with noatime — takes effect after reboot or remount"
    fi
fi

clear_trap_rollback
log_success "Performance tuning applied"
