#!/usr/bin/env bash
# doctor-system.sh - Comprehensive system health verification
# Checks packages, storage, services, stability, performance, and security
#
# Usage: scripts/doctor-system.sh [options]
#   --dry-run         Read-only checks (default)
#   --apply           Apply safe auto-fixes (e.g., sync keyring)
#   --json            Machine-readable JSON output
#   --strict          Exit non-zero on warnings
#   --category=X      Run specific category only (packages|storage|services|stability|performance|security)
#   -h, --help        Show this help

set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/common.sh"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "doctor" "doctor-system" "$@"
trap 'oplog_end $?' EXIT

# =============================================================================
# Configuration
# =============================================================================
POLICY_FILE="$ROOT_DIR/app/manifests/update-policy.toml"
STATE_DIR="$ROOT_DIR/app/state"
LOG_DIR="$STATE_DIR/logs"
LOG_FILE="$LOG_DIR/doctor-$(date +%F).log"

DRY_RUN=1
JSON_OUTPUT=0
STRICT_MODE=0
CATEGORY_FILTER=""

# Counters
CRITICAL_COUNT=0
ERROR_COUNT=0
WARNING_COUNT=0
INFO_COUNT=0

# JSON results array
declare -a JSON_RESULTS=()

# Policy defaults
AUTO_FIX_KEYRING=1
DISK_WARN_PERCENT=80
DISK_CRITICAL_PERCENT=90
JOURNAL_MAX_SIZE="500M"
KEYRING_MAX_AGE_DAYS=30

# =============================================================================
# Usage
# =============================================================================
usage() {
  cat <<USAGE
Usage: scripts/doctor-system.sh [options]

Comprehensive system health verification for Arch Linux.

Options:
  --dry-run         Read-only checks (default)
  --apply           Apply safe auto-fixes (e.g., sync keyring)
  --json            Machine-readable JSON output
  --strict          Exit non-zero on warnings
  --category=X      Run specific category only:
                    packages, storage, services, stability, performance, security
  -h, --help        Show this help

Categories:
  packages      Orphans, foreign packages, database integrity, file conflicts
  storage       Package cache, journal size, disk usage, user cache
  services      Failed system/user services, portal health
  stability     .pacnew/.pacsave files, broken symlinks, kernel errors
  performance   Boot time, memory pressure, load average
  security      Keyring freshness, open ports

Exit codes:
  0 - All checks passed (no errors)
  1 - Runtime/execution error
  2 - Usage error
  3 - Health checks found errors (or warnings in strict mode)
USAGE
}

# =============================================================================
# Helpers
# =============================================================================
read_policy() {
  if [[ ! -f "$POLICY_FILE" ]]; then
    return
  fi

  AUTO_FIX_KEYRING="$(to_bool "$(get_toml_value "$POLICY_FILE" "auto_fix_keyring" "$AUTO_FIX_KEYRING")")"
  STRICT_MODE="$(to_bool "$(get_toml_value "$POLICY_FILE" "strict_mode" "$STRICT_MODE")")"
  DISK_WARN_PERCENT="$(get_toml_value "$POLICY_FILE" "disk_warn_percent" "$DISK_WARN_PERCENT")"
  DISK_CRITICAL_PERCENT="$(get_toml_value "$POLICY_FILE" "disk_critical_percent" "$DISK_CRITICAL_PERCENT")"
  JOURNAL_MAX_SIZE="$(get_toml_value "$POLICY_FILE" "journal_max_size" "$JOURNAL_MAX_SIZE")"
  KEYRING_MAX_AGE_DAYS="$(get_toml_value "$POLICY_FILE" "keyring_max_age_days" "$KEYRING_MAX_AGE_DAYS")"
}

# Record a check result
# Usage: record_result <category> <check> <severity> <message> [<details>]
record_result() {
  local category="$1"
  local check="$2"
  local severity="$3"
  local message="$4"
  local details="${5:-}"

  case "$severity" in
    CRITICAL) ((++CRITICAL_COUNT)) ;;
    ERROR)    ((++ERROR_COUNT)) ;;
    WARNING)  ((++WARNING_COUNT)) ;;
    INFO)     ((++INFO_COUNT)) ;;
    OK)       ;;
  esac

  if [[ "$JSON_OUTPUT" -eq 1 ]]; then
    local json
    json="{\"category\":\"$(json_escape "$category")\",\"check\":\"$(json_escape "$check")\",\"severity\":\"$severity\",\"message\":\"$(json_escape "$message")\""
    if [[ -n "$details" ]]; then
      json="$json,\"details\":\"$(json_escape "$details")\""
    fi
    json="$json}"
    JSON_RESULTS+=("$json")
  else
    case "$severity" in
      CRITICAL) log_critical "[$category] $check: $message" ;;
      ERROR)    log_error "[$category] $check: $message" ;;
      WARNING)  log_warn "[$category] $check: $message" ;;
      INFO)     log_info "[$category] $check: $message" ;;
      OK)       log_ok "[$category] $check: $message" ;;
    esac
    if [[ -n "$details" && "$JSON_OUTPUT" -eq 0 ]]; then
      echo "  Details: $details"
    fi
  fi
}

should_run_category() {
  local category="$1"
  if [[ -z "$CATEGORY_FILTER" ]]; then
    return 0
  fi
  [[ "$CATEGORY_FILTER" == "$category" ]]
}

# =============================================================================
# Package checks
# =============================================================================
check_packages() {
  should_run_category "packages" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== Package Health ==="

  # Orphan packages
  local orphans orphan_count
  orphans="$(pacman -Qtdq 2>/dev/null || true)"
  if [[ -z "$orphans" ]]; then
    orphan_count=0
  else
    orphan_count="$(printf '%s\n' "$orphans" | wc -l)"
  fi
  if [[ "$orphan_count" -gt 0 ]]; then
    record_result "packages" "orphans" "WARNING" "$orphan_count orphan package(s) found" "$orphans"
  else
    record_result "packages" "orphans" "OK" "No orphan packages"
  fi

  # Foreign packages (AUR/manual)
  local foreign foreign_count
  foreign="$(pacman -Qmq 2>/dev/null || true)"
  if [[ -z "$foreign" ]]; then
    foreign_count=0
  else
    foreign_count="$(printf '%s\n' "$foreign" | wc -l)"
  fi
  if [[ "$foreign_count" -gt 50 ]]; then
    record_result "packages" "foreign" "WARNING" "$foreign_count foreign packages (AUR/manual)" "Consider reviewing for unmaintained packages"
  else
    record_result "packages" "foreign" "INFO" "$foreign_count foreign packages (AUR/manual)"
  fi

  # Database integrity
  if pacman -Dk >/dev/null 2>&1; then
    record_result "packages" "database" "OK" "Package database integrity verified"
  else
    record_result "packages" "database" "ERROR" "Package database integrity check failed"
  fi

  # File conflicts check (check if any packages own the same files)
  # Note: pacman -Qkk is slow on large systems, so we use a timeout
  local conflicts
  if has_cmd timeout; then
    conflicts="$(timeout 10 pacman -Qkk 2>&1 | grep -E 'warning:|error:' | head -5 || true)"
  else
    conflicts="$(pacman -Qkk 2>&1 | grep -E 'warning:|error:' | head -5 || true)"
  fi
  if [[ -n "$conflicts" ]]; then
    record_result "packages" "files" "WARNING" "File ownership issues detected" "$conflicts"
  else
    record_result "packages" "files" "OK" "No file ownership conflicts"
  fi

  # Partially installed packages
  local partial
  partial="$(pacman -Qqd 2>/dev/null | sort | uniq -d || true)"
  if [[ -n "$partial" ]]; then
    record_result "packages" "partial" "ERROR" "Partially installed packages detected" "$partial"
  fi
}

# =============================================================================
# Storage checks
# =============================================================================
check_storage() {
  should_run_category "storage" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== Storage Health ==="

  # Disk usage on /
  local usage_percent
  usage_percent="$(get_disk_usage_percent /)"
  if [[ "$usage_percent" -ge "$DISK_CRITICAL_PERCENT" ]]; then
    record_result "storage" "disk-root" "CRITICAL" "Root filesystem at ${usage_percent}% (critical threshold: ${DISK_CRITICAL_PERCENT}%)"
  elif [[ "$usage_percent" -ge "$DISK_WARN_PERCENT" ]]; then
    record_result "storage" "disk-root" "WARNING" "Root filesystem at ${usage_percent}% (warn threshold: ${DISK_WARN_PERCENT}%)"
  else
    record_result "storage" "disk-root" "OK" "Root filesystem at ${usage_percent}%"
  fi

  # Package cache size
  local cache_size cache_count
  if [[ -d /var/cache/pacman/pkg ]]; then
    cache_size="$(du -sh /var/cache/pacman/pkg 2>/dev/null | cut -f1 | head -1 || true)"
    cache_size="${cache_size:-unknown}"
    cache_count="$(find /var/cache/pacman/pkg -name '*.pkg.tar.*' 2>/dev/null | wc -l || true)"
    cache_count="${cache_count:-0}"
    local cache_bytes
    cache_bytes="$(du -sb /var/cache/pacman/pkg 2>/dev/null | cut -f1 | head -1 || true)"
    cache_bytes="${cache_bytes:-0}"
    if [[ "$cache_bytes" -gt 10737418240 ]]; then  # 10GB
      record_result "storage" "pkg-cache" "WARNING" "Package cache: $cache_size ($cache_count packages)" "Consider running: paccache -rk3"
    else
      record_result "storage" "pkg-cache" "INFO" "Package cache: $cache_size ($cache_count packages)"
    fi
  fi

  # Journal size
  if has_cmd journalctl; then
    local journal_size
    journal_size="$(journalctl --disk-usage 2>/dev/null | grep -oE '[0-9]+(\.[0-9]+)?[KMGT]?' | head -1 || echo 'unknown')"
    local journal_bytes
    journal_bytes="$(human_to_bytes "$journal_size")"
    local max_bytes
    max_bytes="$(human_to_bytes "$JOURNAL_MAX_SIZE")"
    if [[ "$journal_bytes" -gt "$max_bytes" ]]; then
      record_result "storage" "journal" "WARNING" "Journal size: $journal_size (max recommended: $JOURNAL_MAX_SIZE)"
    else
      record_result "storage" "journal" "OK" "Journal size: $journal_size"
    fi
  fi

  # User cache (~/.cache)
  if [[ -d "$HOME/.cache" ]]; then
    local user_cache_size
    user_cache_size="$(du -sh "$HOME/.cache" 2>/dev/null | cut -f1 | head -1 || true)"
    user_cache_size="${user_cache_size:-unknown}"
    local user_cache_bytes
    user_cache_bytes="$(du -sb "$HOME/.cache" 2>/dev/null | cut -f1 | head -1 || true)"
    user_cache_bytes="${user_cache_bytes:-0}"
    if [[ "$user_cache_bytes" -gt 5368709120 ]]; then  # 5GB
      record_result "storage" "user-cache" "WARNING" "User cache (~/.cache): $user_cache_size" "Consider cleaning old cache files"
    else
      record_result "storage" "user-cache" "INFO" "User cache (~/.cache): $user_cache_size"
    fi
  fi

  # /boot partition (important for kernel updates)
  if mountpoint -q /boot 2>/dev/null; then
    local boot_usage
    boot_usage="$(get_disk_usage_percent /boot)"
    if [[ "$boot_usage" -ge 90 ]]; then
      record_result "storage" "disk-boot" "ERROR" "/boot partition at ${boot_usage}%" "May cause kernel update failures"
    elif [[ "$boot_usage" -ge 80 ]]; then
      record_result "storage" "disk-boot" "WARNING" "/boot partition at ${boot_usage}%"
    else
      record_result "storage" "disk-boot" "OK" "/boot partition at ${boot_usage}%"
    fi
  fi
}

# =============================================================================
# Service checks
# =============================================================================
check_services() {
  should_run_category "services" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== Service Health ==="

  # Failed system services
  local failed_system
  failed_system="$(systemctl --failed --no-legend 2>/dev/null | awk '{print $1}' || true)"
  local failed_system_count
  if [[ -z "$failed_system" ]]; then
    failed_system_count=0
  else
    failed_system_count="$(printf '%s\n' "$failed_system" | wc -l)"
  fi
  if [[ "$failed_system_count" -gt 0 ]]; then
    record_result "services" "failed-system" "ERROR" "$failed_system_count failed system service(s)" "$failed_system"
  else
    record_result "services" "failed-system" "OK" "No failed system services"
  fi

  # Failed user services
  local failed_user
  failed_user="$(systemctl --user --failed --no-legend 2>/dev/null | awk '{print $1}' || true)"
  local failed_user_count
  if [[ -z "$failed_user" ]]; then
    failed_user_count=0
  else
    failed_user_count="$(printf '%s\n' "$failed_user" | wc -l)"
  fi
  if [[ "$failed_user_count" -gt 0 ]]; then
    record_result "services" "failed-user" "ERROR" "$failed_user_count failed user service(s)" "$failed_user"
  else
    record_result "services" "failed-user" "OK" "No failed user services"
  fi

  # XDG desktop portal health (important for Wayland)
  local portal_status=""
  if systemctl --user is-active xdg-desktop-portal.service >/dev/null 2>&1; then
    portal_status="active"
  elif systemctl --user is-enabled xdg-desktop-portal.service >/dev/null 2>&1; then
    portal_status="enabled but not running"
  else
    portal_status="not enabled"
  fi

  if [[ "$portal_status" == "active" ]]; then
    record_result "services" "portal" "OK" "XDG desktop portal is active"
  elif [[ "$portal_status" == "not enabled" ]]; then
    record_result "services" "portal" "INFO" "XDG desktop portal not enabled"
  else
    record_result "services" "portal" "WARNING" "XDG desktop portal: $portal_status"
  fi

  # D-Bus session bus
  if [[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]]; then
    record_result "services" "dbus-session" "OK" "D-Bus session bus available"
  else
    record_result "services" "dbus-session" "WARNING" "D-Bus session bus not detected"
  fi
}

# =============================================================================
# Stability checks
# =============================================================================
check_stability() {
  should_run_category "stability" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== System Stability ==="

  # .pacnew/.pacsave files
  local pacnew_list pacnew_count
  pacnew_list="$(sudo find /etc -maxdepth 6 -type f \( -name '*.pacnew' -o -name '*.pacsave' \) 2>/dev/null || true)"
  if [[ -z "$pacnew_list" ]]; then
    pacnew_count=0
  else
    pacnew_count="$(printf '%s\n' "$pacnew_list" | wc -l)"
  fi
  if [[ "$pacnew_count" -gt 0 ]]; then
    record_result "stability" "pacnew" "WARNING" "$pacnew_count .pacnew/.pacsave file(s) need attention" "Run: sudo pacdiff"
  else
    record_result "stability" "pacnew" "OK" "No .pacnew/.pacsave files pending"
  fi

  # Broken symlinks in /etc
  local broken_etc broken_count
  broken_etc="$(find /etc -maxdepth 3 -xtype l 2>/dev/null | head -10 || true)"
  if [[ -z "$broken_etc" ]]; then
    broken_count=0
  else
    broken_count="$(printf '%s\n' "$broken_etc" | wc -l)"
  fi
  if [[ "$broken_count" -gt 0 ]]; then
    record_result "stability" "symlinks-etc" "WARNING" "$broken_count broken symlink(s) in /etc" "$broken_etc"
  else
    record_result "stability" "symlinks-etc" "OK" "No broken symlinks in /etc"
  fi

  # Kernel errors in dmesg (last boot)
  local kernel_errors
  kernel_errors="$(dmesg --level=err,crit,alert,emerg 2>/dev/null | tail -5 || true)"
  if [[ -n "$kernel_errors" ]]; then
    local error_count
    error_count="$(dmesg --level=err,crit,alert,emerg 2>/dev/null | wc -l || echo "0")"
    record_result "stability" "kernel-errors" "WARNING" "$error_count kernel error(s) in dmesg" "$(echo "$kernel_errors" | head -3)"
  else
    record_result "stability" "kernel-errors" "OK" "No critical kernel errors"
  fi

  # Check if running kernel matches installed
  local running_kernel installed_kernel
  running_kernel="$(uname -r)"
  installed_kernel="$(pacman -Q linux 2>/dev/null | awk '{print $2}' || true)"
  # Also check linux-lts
  local installed_lts
  installed_lts="$(pacman -Q linux-lts 2>/dev/null | awk '{print $2}' || true)"

  local kernel_match=0
  if [[ "$running_kernel" == *"$installed_kernel"* ]] || [[ -z "$installed_kernel" ]]; then
    kernel_match=1
  fi
  if [[ "$running_kernel" == *"$installed_lts"* ]] || [[ -z "$installed_lts" ]]; then
    kernel_match=1
  fi

  if [[ $kernel_match -eq 1 ]]; then
    record_result "stability" "kernel-version" "OK" "Running kernel: $running_kernel"
  else
    record_result "stability" "kernel-version" "WARNING" "Running kernel ($running_kernel) differs from installed" "Reboot recommended"
  fi

  # systemd-analyze verify (check for broken unit files)
  if has_cmd systemd-analyze; then
    local unit_errors
    unit_errors="$(systemd-analyze verify --user default.target 2>&1 | grep -E 'error|Error' | head -3 || true)"
    if [[ -n "$unit_errors" ]]; then
      record_result "stability" "unit-files" "WARNING" "systemd unit file issues detected" "$unit_errors"
    else
      record_result "stability" "unit-files" "OK" "systemd unit files valid"
    fi
  fi
}

# =============================================================================
# Performance checks
# =============================================================================
check_performance() {
  should_run_category "performance" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== Performance Health ==="

  # Boot time
  if has_cmd systemd-analyze; then
    local boot_time
    boot_time="$(systemd-analyze time 2>/dev/null | grep -oE 'reached after [0-9]+(\.[0-9]+)?s' | grep -oE '[0-9]+(\.[0-9]+)?s' || echo 'unknown')"
    local boot_secs="${boot_time%s}"
    if [[ "$boot_secs" != "unknown" ]]; then
      if (( $(echo "$boot_secs > 60" | bc -l 2>/dev/null || echo 0) )); then
        record_result "performance" "boot-time" "WARNING" "Boot time: $boot_time (>60s)" "Run: systemd-analyze blame"
      elif (( $(echo "$boot_secs > 30" | bc -l 2>/dev/null || echo 0) )); then
        record_result "performance" "boot-time" "INFO" "Boot time: $boot_time"
      else
        record_result "performance" "boot-time" "OK" "Boot time: $boot_time"
      fi
    fi
  fi

  # Memory pressure
  local mem_used_percent
  mem_used_percent="$(get_memory_used_percent)"
  local mem_total mem_avail
  mem_total="$(get_memory_total_mb)"
  mem_avail="$(get_memory_available_mb)"

  if [[ "$mem_used_percent" -ge 90 ]]; then
    record_result "performance" "memory" "WARNING" "Memory usage: ${mem_used_percent}% (${mem_avail}MB available of ${mem_total}MB)"
  else
    record_result "performance" "memory" "OK" "Memory usage: ${mem_used_percent}% (${mem_avail}MB available)"
  fi

  # Swap usage
  local swap_total swap_used swap_percent
  swap_total="$(awk '/SwapTotal/ {print $2}' /proc/meminfo 2>/dev/null || echo "0")"
  swap_used="$(awk '/SwapTotal/ {t=$2} /SwapFree/ {f=$2} END {print t-f}' /proc/meminfo 2>/dev/null || echo "0")"
  if [[ "$swap_total" -gt 0 ]]; then
    swap_percent="$((swap_used * 100 / swap_total))"
    if [[ "$swap_percent" -ge 80 ]]; then
      record_result "performance" "swap" "WARNING" "Swap usage: ${swap_percent}%"
    else
      record_result "performance" "swap" "OK" "Swap usage: ${swap_percent}%"
    fi
  else
    record_result "performance" "swap" "INFO" "No swap configured"
  fi

  # Load average
  local load_avg cpu_count load_ratio
  load_avg="$(get_load_average)"
  cpu_count="$(get_cpu_count)"
  if has_cmd bc; then
    load_ratio="$(echo "scale=2; $load_avg / $cpu_count" | bc 2>/dev/null || echo "0")"
  else
    load_ratio="0"
  fi

  if (( $(echo "$load_ratio > 2" | bc -l 2>/dev/null || echo 0) )); then
    record_result "performance" "load" "WARNING" "Load average: $load_avg (${cpu_count} cores, ratio: ${load_ratio})"
  else
    record_result "performance" "load" "OK" "Load average: $load_avg (${cpu_count} cores)"
  fi

  # I/O wait
  if has_cmd iostat; then
    local iowait
    iowait="$(iostat -c 1 2 2>/dev/null | tail -1 | awk '{print $4}' || echo "0")"
    if (( $(echo "$iowait > 20" | bc -l 2>/dev/null || echo 0) )); then
      record_result "performance" "iowait" "WARNING" "I/O wait: ${iowait}%"
    fi
  fi
}

# =============================================================================
# Security checks
# =============================================================================
check_security() {
  should_run_category "security" || return 0

  [[ "$JSON_OUTPUT" -eq 0 ]] && echo "" && log_info "=== Security Health ==="

  # Keyring freshness
  local keyring_pkg keyring_date keyring_age_days
  keyring_pkg="$(LC_ALL=C pacman -Qi archlinux-keyring 2>/dev/null || true)"
  if [[ -n "$keyring_pkg" ]]; then
    keyring_date="$(echo "$keyring_pkg" | grep "Install Date" | cut -d: -f2- | xargs)"
    if [[ -n "$keyring_date" ]]; then
      local keyring_epoch now_epoch
      keyring_epoch="$(date -d "$keyring_date" +%s 2>/dev/null || echo "0")"
      now_epoch="$(date +%s)"
      keyring_age_days="$(( (now_epoch - keyring_epoch) / 86400 ))"

      if [[ "$keyring_age_days" -gt "$KEYRING_MAX_AGE_DAYS" ]]; then
        record_result "security" "keyring" "WARNING" "archlinux-keyring is ${keyring_age_days} days old" "Consider: sudo pacman -Sy archlinux-keyring"
        if [[ "$DRY_RUN" -eq 0 && "$AUTO_FIX_KEYRING" -eq 1 ]]; then
          log_info "Auto-fixing: updating archlinux-keyring..."
          if sudo pacman -Sy --noconfirm archlinux-keyring 2>&1 | tee -a "$LOG_FILE"; then
            record_result "security" "keyring-fix" "OK" "archlinux-keyring updated successfully"
          else
            record_result "security" "keyring-fix" "ERROR" "Failed to update archlinux-keyring"
          fi
        fi
      else
        record_result "security" "keyring" "OK" "archlinux-keyring is ${keyring_age_days} days old"
      fi
    fi
  else
    record_result "security" "keyring" "ERROR" "archlinux-keyring not installed"
  fi

  # Listening ports (informational)
  if has_cmd ss; then
    local listening_ports
    listening_ports="$(ss -tlnp 2>/dev/null | grep -c LISTEN || true)"
    listening_ports="${listening_ports:-0}"
    record_result "security" "ports" "INFO" "$listening_ports listening TCP port(s)"
  fi

  # SUID binaries count (informational)
  local suid_count
  suid_count="$(find /usr/bin /usr/sbin -perm -4000 2>/dev/null | wc -l || true)"
  suid_count="${suid_count:-0}"
  record_result "security" "suid" "INFO" "$suid_count SUID binaries in /usr/bin, /usr/sbin"

  # Failed login attempts
  if has_cmd journalctl; then
    local failed_logins
    failed_logins="$(journalctl -q --since "24 hours ago" 2>/dev/null | grep -c "authentication failure" || true)"
    failed_logins="${failed_logins:-0}"
    if [[ "$failed_logins" -gt 10 ]]; then
      record_result "security" "auth-failures" "WARNING" "$failed_logins authentication failures in last 24h"
    elif [[ "$failed_logins" -gt 0 ]]; then
      record_result "security" "auth-failures" "INFO" "$failed_logins authentication failures in last 24h"
    fi
  fi

  # World-writable files in /etc (security concern)
  local world_writable
  world_writable="$(find /etc -maxdepth 2 -perm -002 -type f 2>/dev/null | head -5 || true)"
  if [[ -n "$world_writable" ]]; then
    record_result "security" "world-writable" "WARNING" "World-writable files in /etc" "$world_writable"
  fi
}

# =============================================================================
# Output functions
# =============================================================================
print_summary() {
  echo ""
  echo "=========================================="
  echo "Doctor Summary"
  echo "=========================================="
  echo "Critical: $CRITICAL_COUNT"
  echo "Errors:   $ERROR_COUNT"
  echo "Warnings: $WARNING_COUNT"
  echo "Info:     $INFO_COUNT"
  echo ""

  if [[ $CRITICAL_COUNT -gt 0 || $ERROR_COUNT -gt 0 ]]; then
    log_error "System health: ISSUES DETECTED"
  elif [[ $WARNING_COUNT -gt 0 ]]; then
    log_warn "System health: OK with warnings"
  else
    log_ok "System health: GOOD"
  fi
}

print_json_output() {
  echo "{"
  echo "  \"timestamp\": \"$(date --iso-8601=seconds)\","
  echo "  \"summary\": {"
  echo "    \"critical\": $CRITICAL_COUNT,"
  echo "    \"errors\": $ERROR_COUNT,"
  echo "    \"warnings\": $WARNING_COUNT,"
  echo "    \"info\": $INFO_COUNT"
  echo "  },"
  echo "  \"results\": ["

  local first=1
  for result in "${JSON_RESULTS[@]}"; do
    if [[ $first -eq 1 ]]; then
      echo "    $result"
      first=0
    else
      echo "    ,$result"
    fi
  done

  echo "  ]"
  echo "}"
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
    --json)
      JSON_OUTPUT=1
      ;;
    --strict)
      STRICT_MODE=1
      ;;
    --category=*)
      CATEGORY_FILTER="${arg#--category=}"
      case "$CATEGORY_FILTER" in
        packages|storage|services|stability|performance|security) ;;
        *)
          echo "Unknown category: $CATEGORY_FILTER" >&2
          usage >&2
          exit 2
          ;;
      esac
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

mkdir -p "$LOG_DIR"

# Read policy
read_policy

if [[ "$JSON_OUTPUT" -eq 0 ]]; then
  echo "=========================================="
  echo "System Doctor - Comprehensive Health Check"
  echo "=========================================="
  echo "Root: $ROOT_DIR"
  echo "Mode: $([ $DRY_RUN -eq 1 ] && echo 'dry-run' || echo 'apply')"
  echo "Strict: $([ $STRICT_MODE -eq 1 ] && echo 'yes' || echo 'no')"
  if [[ -n "$CATEGORY_FILTER" ]]; then
    echo "Category: $CATEGORY_FILTER"
  fi
fi

# Run checks
check_packages
check_storage
check_services
check_stability
check_performance
check_security

# Output results
if [[ "$JSON_OUTPUT" -eq 1 ]]; then
  print_json_output
else
  print_summary
fi

# Determine exit code
if [[ $CRITICAL_COUNT -gt 0 || $ERROR_COUNT -gt 0 ]]; then
  exit 3
fi
if [[ $STRICT_MODE -eq 1 && $WARNING_COUNT -gt 0 ]]; then
  exit 3
fi
exit 0
