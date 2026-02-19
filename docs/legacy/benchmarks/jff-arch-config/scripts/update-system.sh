#!/usr/bin/env bash
# update-system.sh - Robust safe upgrade workflow for Arch Linux
# Based on best practices for stability, performance, and resilience
#
# Usage: scripts/update-system.sh [options]
#   --dry-run            Preview only (default)
#   --apply              Execute upgrade
#   --non-interactive    Use --noconfirm for automation
#   -h, --help           Show this help

set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/common.sh"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "updates" "update-system" "$@"
trap 'oplog_end $?' EXIT

# =============================================================================
# Configuration
# =============================================================================
POLICY_FILE="$ROOT_DIR/app/manifests/update-policy.toml"
STATE_DIR="$ROOT_DIR/app/state"
RUN_DIR="$STATE_DIR/run"
LOG_DIR="$STATE_DIR/logs"
LOCK_DIR="$STATE_DIR/locks"
LOCK_FILE="$LOCK_DIR/update.lock"
LOG_FILE="$LOG_DIR/update-$(date +%F).log"
PACMAN_OUTPUT_FILE="$RUN_DIR/last-pacman-output.txt"
NEWS_STATE_FILE="$RUN_DIR/last_news_guid"
NEWS_ACK_FILE="$RUN_DIR/news_acknowledged"
BENCH_CSV="$LOG_DIR/bench.csv"

DRY_RUN=1
NON_INTERACTIVE=0

# Policy defaults (can be overridden by policy file)
MODE="manual"
REQUIRE_SNAPSHOT=1
ALLOW_FULL_UPGRADE=1
MAX_PARALLEL_JOBS=1

# Mirror settings
UPDATE_MIRRORS=0
MIRROR_COUNTRY="US"
MIRROR_LATEST=20
MIRROR_SORT="rate"

# Update settings
UPDATE_KEYRING_FIRST=1
UPDATE_AUR=1
AUR_HELPER="auto"
CACHE_KEEP=3
RUN_BENCH=1

# Safety settings
REQUIRE_MIN_DISK_SPACE_GB=2
REQUIRE_MIN_BATTERY_PERCENT=20
REQUIRE_NEWS_ACKNOWLEDGMENT=1
CREATE_SNAPSHOT=0
COLLECT_BENCHMARKS=1

NEWS_FEED_URL="https://archlinux.org/feeds/news/"

# Tracking
PACKAGES_UPGRADED=0
REBOOT_REQUIRED=0
declare -a PACNEW_FILES=()

# =============================================================================
# Usage
# =============================================================================
usage() {
  cat <<USAGE
Usage: scripts/update-system.sh [options]

Robust safe upgrade workflow for Arch Linux.

Options:
  --dry-run          Preview only, no package changes (default)
  --apply            Execute upgrade operations
  --non-interactive  Use --noconfirm for pacman (only with --apply)
  -h, --help         Show this help

Safety Features:
  - Pre-flight checks (network, disk space, battery, pacman lock, time sync)
  - Keyring updated first to prevent signature errors
  - Never partial upgrades (always pacman -Syu)
  - Arch News acknowledgment requirement
  - Reboot detection (kernel, glibc, systemd, microcode)
  - Comprehensive logging

Exit codes:
  0 - Success
  1 - Runtime/pre-flight error
  2 - Usage error
USAGE
}

# =============================================================================
# Helpers
# =============================================================================
read_policy() {
  if [[ ! -f "$POLICY_FILE" ]]; then
    return
  fi

  MODE="$(get_toml_value "$POLICY_FILE" "mode" "$MODE")"
  REQUIRE_SNAPSHOT="$(to_bool "$(get_toml_value "$POLICY_FILE" "require_snapshot" "$REQUIRE_SNAPSHOT")")"
  ALLOW_FULL_UPGRADE="$(to_bool "$(get_toml_value "$POLICY_FILE" "allow_full_upgrade" "$ALLOW_FULL_UPGRADE")")"
  MAX_PARALLEL_JOBS="$(get_toml_value "$POLICY_FILE" "max_parallel_jobs" "$MAX_PARALLEL_JOBS")"

  UPDATE_MIRRORS="$(to_bool "$(get_toml_value "$POLICY_FILE" "update_mirrors" "$UPDATE_MIRRORS")")"
  MIRROR_COUNTRY="$(get_toml_value "$POLICY_FILE" "mirror_country" "$MIRROR_COUNTRY")"
  MIRROR_LATEST="$(get_toml_value "$POLICY_FILE" "mirror_latest" "$MIRROR_LATEST")"
  MIRROR_SORT="$(get_toml_value "$POLICY_FILE" "mirror_sort" "$MIRROR_SORT")"

  UPDATE_KEYRING_FIRST="$(to_bool "$(get_toml_value "$POLICY_FILE" "update_keyring_first" "$UPDATE_KEYRING_FIRST")")"
  UPDATE_AUR="$(to_bool "$(get_toml_value "$POLICY_FILE" "update_aur" "$UPDATE_AUR")")"
  AUR_HELPER="$(get_toml_value "$POLICY_FILE" "aur_helper" "$AUR_HELPER")"
  CACHE_KEEP="$(get_toml_value "$POLICY_FILE" "cache_keep" "$CACHE_KEEP")"
  RUN_BENCH="$(to_bool "$(get_toml_value "$POLICY_FILE" "run_bench" "$RUN_BENCH")")"

  REQUIRE_MIN_DISK_SPACE_GB="$(get_toml_value "$POLICY_FILE" "require_min_disk_space_gb" "$REQUIRE_MIN_DISK_SPACE_GB")"
  REQUIRE_MIN_BATTERY_PERCENT="$(get_toml_value "$POLICY_FILE" "require_min_battery_percent" "$REQUIRE_MIN_BATTERY_PERCENT")"
  REQUIRE_NEWS_ACKNOWLEDGMENT="$(to_bool "$(get_toml_value "$POLICY_FILE" "require_news_acknowledgment" "$REQUIRE_NEWS_ACKNOWLEDGMENT")")"
  CREATE_SNAPSHOT="$(to_bool "$(get_toml_value "$POLICY_FILE" "create_snapshot" "$CREATE_SNAPSHOT")")"
  COLLECT_BENCHMARKS="$(to_bool "$(get_toml_value "$POLICY_FILE" "collect_benchmarks" "$COLLECT_BENCHMARKS")")"
}

# Detect the best AUR helper
detect_aur_helper() {
  if [[ "$AUR_HELPER" == "auto" ]]; then
    if has_cmd paru; then
      AUR_HELPER="paru"
    elif has_cmd yay; then
      AUR_HELPER="yay"
    else
      AUR_HELPER="none"
    fi
  fi
}

# =============================================================================
# Pre-flight checks
# =============================================================================
run_update_preflight() {
  log_info "=== Pre-flight Checks ==="

  local errors=0
  local min_disk_mb=$((REQUIRE_MIN_DISK_SPACE_GB * 1024))

  # Check Arch Linux
  if ! check_arch_linux; then
    log_critical "Not running on Arch Linux"
    ((errors++))
  else
    log_ok "Running on Arch Linux"
  fi

  # Check network
  if ! check_connectivity; then
    log_critical "No network connectivity"
    ((errors++))
  else
    log_ok "Network connectivity verified"
  fi

  # Check disk space
  local avail_mb
  avail_mb="$(get_disk_space_mb /)"
  if ! check_disk_space "/" "$min_disk_mb"; then
    log_critical "Insufficient disk space: ${avail_mb}MB available, ${min_disk_mb}MB required"
    ((errors++))
  else
    log_ok "Disk space OK: ${avail_mb}MB available"
  fi

  # Check battery (warning only for low battery)
  local battery_level
  battery_level="$(get_battery_level)"
  if [[ "$battery_level" == "-1" ]]; then
    log_ok "No battery detected (desktop/server)"
  elif is_on_ac_power; then
    log_ok "On AC power (battery at ${battery_level}%)"
  elif ! check_battery_level "$REQUIRE_MIN_BATTERY_PERCENT"; then
    log_warn "Battery level low: ${battery_level}% (recommended: ${REQUIRE_MIN_BATTERY_PERCENT}%+)"
    log_warn "Consider connecting to AC power before updating."
  else
    log_ok "Battery level OK: ${battery_level}%"
  fi

  # Check pacman lock
  if ! check_pacman_lock; then
    log_critical "Pacman is locked (/var/lib/pacman/db.lck exists)"
    log_info "Another pacman process may be running, or a previous run crashed."
    log_info "If sure no other pacman is running: sudo rm /var/lib/pacman/db.lck"
    ((errors++))
  else
    log_ok "No pacman lock"
  fi

  # Check time sync (warning only)
  if check_time_synced; then
    log_ok "System time synchronized"
  else
    log_warn "System time may not be synchronized (NTP)"
  fi

  if [[ $errors -gt 0 ]]; then
    log_critical "Pre-flight checks failed with $errors error(s)"
    return 1
  fi

  log_ok "All pre-flight checks passed"
  return 0
}

# =============================================================================
# Arch News
# =============================================================================
check_arch_news() {
  log_info "=== Arch News Check ==="

  if ! has_cmd curl; then
    log_warn "curl not found; skipping Arch News check."
    return 0
  fi

  local xml
  if ! xml="$(curl -fsSL --max-time 10 "$NEWS_FEED_URL" 2>/dev/null)"; then
    log_warn "Unable to fetch Arch News feed (network issue or timeout)."
    return 0
  fi

  # Parse the first item from RSS feed
  local latest_guid latest_title latest_link
  latest_guid="$(printf '%s' "$xml" | grep -m1 -oE '<guid[^>]*>[^<]+' | sed 's/^<guid[^>]*>//' || true)"
  latest_title="$(printf '%s' "$xml" | grep -m1 -oE '<title[^>]*>[^<]+' | sed 's/^<title[^>]*>//' || true)"
  latest_link="$(printf '%s' "$xml" | grep -m1 -oE '<link[^>]*>[^<]+' | sed 's/^<link[^>]*>//' || true)"

  if [[ -z "$latest_guid" ]]; then
    log_warn "Could not parse Arch News feed."
    return 0
  fi

  local previous_guid
  previous_guid="$(cat "$NEWS_STATE_FILE" 2>/dev/null || true)"
  local acknowledged_guid
  acknowledged_guid="$(cat "$NEWS_ACK_FILE" 2>/dev/null || true)"

  if [[ "$latest_guid" != "$previous_guid" ]]; then
    log_warn "New Arch News detected:"
    log_info "  Title: ${latest_title:-<title unavailable>}"
    log_info "  Link: ${latest_link:-<link unavailable>}"
    echo ""

    # Save the latest guid
    printf '%s\n' "$latest_guid" > "$NEWS_STATE_FILE"

    if [[ "$REQUIRE_NEWS_ACKNOWLEDGMENT" -eq 1 && "$DRY_RUN" -eq 0 ]]; then
      if [[ "$latest_guid" != "$acknowledged_guid" ]]; then
        if [[ "$NON_INTERACTIVE" -eq 1 ]]; then
          log_warn "News acknowledgment required but running non-interactively."
          log_warn "Please review the news manually and acknowledge with:"
          log_info "  echo '$latest_guid' > '$NEWS_ACK_FILE'"
          return 1
        else
          log_warn "Please review the Arch News before continuing."
          log_info "Open: $latest_link"
          echo ""
          read -p "Have you reviewed the news and confirmed no manual intervention is needed? [y/N] " -r response
          if [[ ! "$response" =~ ^[Yy]$ ]]; then
            log_info "Update cancelled. Please review the news and try again."
            return 1
          fi
          printf '%s\n' "$latest_guid" > "$NEWS_ACK_FILE"
          log_ok "News acknowledged."
        fi
      else
        log_ok "News already acknowledged."
      fi
    fi
  else
    log_ok "No new Arch News since last check."
  fi

  return 0
}

# =============================================================================
# Mirror refresh
# =============================================================================
refresh_mirrors() {
  if [[ "$UPDATE_MIRRORS" != "1" ]]; then
    log_info "Mirror refresh disabled by policy."
    return 0
  fi

  log_info "=== Mirror Refresh ==="

  if ! has_cmd reflector; then
    log_warn "reflector not found; skipping mirror refresh."
    return 0
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would refresh mirrors with reflector"
    log_info "  Country: $MIRROR_COUNTRY"
    log_info "  Latest: $MIRROR_LATEST"
    log_info "  Sort: $MIRROR_SORT"
    return 0
  fi

  log_info "Refreshing mirrors with reflector..."
  if maybe_sudo reflector \
    --country "$MIRROR_COUNTRY" \
    --latest "$MIRROR_LATEST" \
    --protocol https \
    --sort "$MIRROR_SORT" \
    --save /etc/pacman.d/mirrorlist 2>&1 | strip_ansi | tee -a "$LOG_FILE"; then
    log_ok "Mirrors refreshed successfully."
  else
    log_warn "Reflector failed; continuing with existing mirrorlist."
  fi
}

# =============================================================================
# Keyring update
# =============================================================================
update_keyring() {
  if [[ "$UPDATE_KEYRING_FIRST" != "1" ]]; then
    return 0
  fi

  log_info "=== Keyring Update ==="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would update archlinux-keyring first"
    return 0
  fi

  log_info "Updating archlinux-keyring first (prevents signature errors)..."
  if maybe_sudo pacman -Sy --noconfirm archlinux-keyring 2>&1 | strip_ansi | tee -a "$LOG_FILE"; then
    log_ok "Keyring updated successfully."
  else
    log_warn "Keyring update failed; continuing anyway."
  fi
}

# =============================================================================
# Full system upgrade
# =============================================================================
run_full_upgrade() {
  if [[ "$ALLOW_FULL_UPGRADE" != "1" ]]; then
    die "Policy blocks full upgrade (allow_full_upgrade=0)."
  fi

  log_info "=== Full System Upgrade ==="

  : > "$PACMAN_OUTPUT_FILE"

  # Check for pending updates first
  if has_cmd checkupdates; then
    log_info "Checking pending updates..."
    local updates
    updates="$(checkupdates 2>/dev/null | strip_ansi || true)"
    if [[ -n "$updates" ]]; then
      local update_count
      update_count="$(echo "$updates" | wc -l)"
      log_info "$update_count package(s) to upgrade:"
      echo "$updates" | head -20 | tee -a "$LOG_FILE" >/dev/null
      if [[ $update_count -gt 20 ]]; then
        log_info "... and $((update_count - 20)) more"
      fi
      PACKAGES_UPGRADED=$update_count
    else
      log_ok "System is up to date (no pending updates)."
      PACKAGES_UPGRADED=0
    fi
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would execute full upgrade: sudo pacman -Syu"
    return 0
  fi

  log_info "Running full upgrade (pacman -Syu)..."
  local pacman_args=(-Syu)
  if [[ "$NON_INTERACTIVE" -eq 1 ]]; then
    pacman_args+=(--noconfirm)
  fi

  if maybe_sudo pacman "${pacman_args[@]}" 2>&1 | strip_ansi | tee -a "$LOG_FILE" | tee "$PACMAN_OUTPUT_FILE" >/dev/null; then
    log_ok "Full upgrade completed."
  else
    local exit_code=$?
    log_error "Pacman upgrade failed with exit code $exit_code"
    log_info "Troubleshooting suggestions:"
    log_info "  1. Check the log file: $LOG_FILE"
    log_info "  2. If signature errors: sudo pacman -Sy archlinux-keyring"
    log_info "  3. If conflicts: resolve manually or use: sudo pacman -Syu --overwrite '*'"
    return $exit_code
  fi

  # Detect reboot requirement
  if grep -qiE 'upgrading (linux|linux-lts|linux-zen|linux-hardened|glibc|systemd|amd-ucode|intel-ucode)' "$PACMAN_OUTPUT_FILE"; then
    REBOOT_REQUIRED=1
    log_warn "Reboot recommended: kernel/glibc/systemd/microcode updates detected."
  fi
}

# =============================================================================
# AUR updates
# =============================================================================
update_aur() {
  if [[ "$UPDATE_AUR" != "1" ]]; then
    return 0
  fi

  detect_aur_helper

  if [[ "$AUR_HELPER" == "none" ]]; then
    log_info "No AUR helper found (paru/yay). Skipping AUR updates."
    return 0
  fi

  log_info "=== AUR Updates ($AUR_HELPER) ==="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would check AUR updates with $AUR_HELPER"

    # Show pending AUR updates
    local aur_updates
    case "$AUR_HELPER" in
      paru)
        aur_updates="$($AUR_HELPER -Qua 2>/dev/null || true)"
        ;;
      yay)
        aur_updates="$($AUR_HELPER -Qua 2>/dev/null || true)"
        ;;
    esac

    if [[ -n "$aur_updates" ]]; then
      local aur_count
      aur_count="$(echo "$aur_updates" | wc -l)"
      log_info "$aur_count AUR package(s) to upgrade:"
      echo "$aur_updates" | tee -a "$LOG_FILE"
    else
      log_ok "No AUR updates available."
    fi
    return 0
  fi

  log_info "Updating AUR packages with $AUR_HELPER..."
  local aur_args=(-Sua)
  if [[ "$NON_INTERACTIVE" -eq 1 ]]; then
    aur_args+=(--noconfirm)
  fi

  if "$AUR_HELPER" "${aur_args[@]}" 2>&1 | strip_ansi | tee -a "$LOG_FILE"; then
    log_ok "AUR updates completed."
  else
    log_warn "AUR updates completed with warnings (check log)."
  fi
}

# =============================================================================
# Post-upgrade checks
# =============================================================================
scan_pacnew() {
  log_info "=== Scanning for .pacnew/.pacsave ==="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would scan /etc for .pacnew/.pacsave"
    return 0
  fi

  local pacnew_list
  pacnew_list="$(maybe_sudo find /etc -maxdepth 6 -type f \( -name '*.pacnew' -o -name '*.pacsave' \) 2>/dev/null || true)"
  if [[ -n "$pacnew_list" ]]; then
    local pacnew_count
    pacnew_count="$(echo "$pacnew_list" | wc -l)"
    log_warn "$pacnew_count .pacnew/.pacsave file(s) found:"
    echo "$pacnew_list" | tee -a "$LOG_FILE"
    log_info "Tip: run 'sudo pacdiff' to review/merge safely."

    # Store for summary
    while IFS= read -r line; do
      PACNEW_FILES+=("$line")
    done <<< "$pacnew_list"
  else
    log_ok "No .pacnew/.pacsave files found."
  fi
}

check_failed_services() {
  log_info "=== Post-upgrade Service Check ==="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would check for failed services"
    return 0
  fi

  # Failed system services
  local failed_system
  failed_system="$(systemctl --failed --no-legend 2>/dev/null | awk '{print $1}' || true)"
  if [[ -n "$failed_system" ]]; then
    local failed_count
    failed_count="$(echo "$failed_system" | wc -l)"
    log_warn "$failed_count failed system service(s):"
    echo "$failed_system" | tee -a "$LOG_FILE"
  else
    log_ok "No failed system services."
  fi

  # Failed user services
  local failed_user
  failed_user="$(systemctl --user --failed --no-legend 2>/dev/null | awk '{print $1}' || true)"
  if [[ -n "$failed_user" ]]; then
    local failed_count
    failed_count="$(echo "$failed_user" | wc -l)"
    log_warn "$failed_count failed user service(s):"
    echo "$failed_user" | tee -a "$LOG_FILE"
  else
    log_ok "No failed user services."
  fi
}

# =============================================================================
# Benchmarks
# =============================================================================
collect_benchmarks() {
  if [[ "$COLLECT_BENCHMARKS" != "1" ]] || [[ "$RUN_BENCH" != "1" ]]; then
    return 0
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    return 0
  fi

  log_info "=== Collecting Benchmarks ==="

  local now kernel boot_time crit_head
  now="$(date --iso-8601=seconds)"
  kernel="$(uname -r)"
  boot_time="$(systemd-analyze time 2>/dev/null | tr -d '\n' | strip_ansi || true)"
  crit_head="$(systemd-analyze critical-chain 2>/dev/null | head -n 1 | tr -d '\n' | strip_ansi || true)"

  if [[ ! -f "$BENCH_CSV" ]]; then
    echo "timestamp,kernel,boot_time,critical_chain_head" > "$BENCH_CSV"
  fi
  printf '"%s","%s","%s","%s"\n' "$now" "$kernel" "$boot_time" "$crit_head" >> "$BENCH_CSV"
  log_ok "Benchmark row written to $BENCH_CSV"
}

# =============================================================================
# Orphan/cache cleanup reminder
# =============================================================================
suggest_cleanup() {
  log_info "=== Cleanup Suggestions ==="

  # Check orphans
  local orphans
  orphans="$(pacman -Qtdq 2>/dev/null || true)"
  if [[ -n "$orphans" ]]; then
    local orphan_count
    orphan_count="$(echo "$orphans" | wc -l)"
    log_info "$orphan_count orphan package(s) detected. Consider reviewing with:"
    log_info "  app-cli run system-clean --root ."
  fi

  # Check cache size
  local cache_size
  cache_size="$(du -sh /var/cache/pacman/pkg 2>/dev/null | cut -f1 || echo 'unknown')"
  log_info "Package cache size: $cache_size"
  log_info "  To clean: app-cli run system-clean --apply --root ."
}

# =============================================================================
# Summary
# =============================================================================
print_summary() {
  echo ""
  echo "=========================================="
  echo "Update Summary"
  echo "=========================================="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "Mode: DRY-RUN (no changes were made)"
    log_info "Run with --apply to execute upgrade."
  else
    log_info "Mode: APPLY"
    log_info "Packages upgraded: $PACKAGES_UPGRADED"

    if [[ $REBOOT_REQUIRED -eq 1 ]]; then
      echo ""
      log_warn "REBOOT REQUIRED"
      log_warn "Critical system components were updated (kernel/glibc/systemd/microcode)."
      log_warn "Please reboot your system to complete the upgrade."
    fi

    if [[ ${#PACNEW_FILES[@]} -gt 0 ]]; then
      echo ""
      log_warn "${#PACNEW_FILES[@]} .pacnew/.pacsave file(s) need attention."
      log_info "Run: sudo pacdiff"
    fi
  fi

  echo ""
  log_info "Log file: $LOG_FILE"
}

# =============================================================================
# Main
# =============================================================================
for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=1
      ;;
    --apply)
      DRY_RUN=0
      ;;
    --non-interactive)
      NON_INTERACTIVE=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

mkdir -p "$RUN_DIR" "$LOG_DIR" "$LOCK_DIR"

# Acquire lock
if ! acquire_lock "$LOCK_FILE" 9; then
  die "Another update is already in progress (lock: $LOCK_FILE)"
fi

# Read policy
read_policy

# Log startup info
log "=== System Update START ==="
log_info "Root: $ROOT_DIR"
log_info "Mode: $([ $DRY_RUN -eq 1 ] && echo 'DRY-RUN' || echo 'APPLY')"
log_info "Non-interactive: $([ $NON_INTERACTIVE -eq 1 ] && echo 'yes' || echo 'no')"
log_info "Policy mode: $MODE"
log_info "Require snapshot: $REQUIRE_SNAPSHOT"
log_info "Update keyring first: $UPDATE_KEYRING_FIRST"
log_info "Update AUR: $UPDATE_AUR"
echo ""

# Warn about manual mode with non-interactive
if [[ "$DRY_RUN" -eq 0 && "$MODE" == "manual" && "$NON_INTERACTIVE" -eq 1 ]]; then
  log_warn "Policy mode is manual but running non-interactively."
fi

# Snapshot reminder
if [[ "$REQUIRE_SNAPSHOT" == "1" && "$DRY_RUN" -eq 0 ]]; then
  log_warn "Snapshot requirement is enabled by policy."
  log_info "Ensure you have taken a fresh snapshot before continuing."
  if [[ "$CREATE_SNAPSHOT" == "1" ]] && has_cmd snapper; then
    log_info "Creating pre-update snapshot..."
    maybe_sudo snapper create -d "pre-update" 2>&1 | tee -a "$LOG_FILE" || log_warn "Snapshot creation failed."
  fi
  echo ""
fi

# Run pre-flight checks
if ! run_update_preflight; then
  die "Pre-flight checks failed. Please resolve the issues and try again."
fi
echo ""

# Check Arch News
if ! check_arch_news; then
  die "News acknowledgment required. Please review the news and try again."
fi
echo ""

# Refresh mirrors
refresh_mirrors
echo ""

# Update keyring first
update_keyring
echo ""

# Full system upgrade
run_full_upgrade
echo ""

# AUR updates
update_aur
echo ""

# Post-upgrade checks
scan_pacnew
echo ""

check_failed_services
echo ""

# Collect benchmarks
collect_benchmarks
echo ""

# Cleanup suggestions
suggest_cleanup

# Print summary
print_summary

log "=== System Update DONE ==="
exit 0
