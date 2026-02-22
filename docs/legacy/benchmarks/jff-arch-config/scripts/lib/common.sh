#!/usr/bin/env bash
# common.sh - Shared utility functions for system maintenance scripts
# Provides logging, pre-flight checks, and utilities
#
# Usage: source "$ROOT_DIR/scripts/lib/common.sh"

# Prevent double-sourcing
[[ -n "${JFF_COMMON_SOURCED:-}" ]] && return 0
readonly JFF_COMMON_SOURCED=1

# =============================================================================
# Color codes and icons
# =============================================================================
readonly COLOR_RED='\033[0;31m'
readonly COLOR_GREEN='\033[0;32m'
readonly COLOR_YELLOW='\033[1;33m'
readonly COLOR_BLUE='\033[0;34m'
readonly COLOR_MAGENTA='\033[0;35m'
readonly COLOR_CYAN='\033[0;36m'
readonly COLOR_GRAY='\033[0;90m'
readonly COLOR_RESET='\033[0m'
readonly COLOR_BOLD='\033[1m'

# =============================================================================
# Logging with severity levels
# =============================================================================

# Internal: print with color and optional prefix
_log_print() {
  local color="$1" prefix="$2" msg="$3" logfile="${4:-}"
  local timestamp
  timestamp="$(date '+%F %T')"

  # Print to stdout with color
  printf "${color}[%s] %s${COLOR_RESET} %s\n" "$timestamp" "$prefix" "$msg"

  # Append to log file if specified (without color)
  if [[ -n "$logfile" && -w "$(dirname "$logfile")" ]]; then
    printf "[%s] %s %s\n" "$timestamp" "$prefix" "$msg" >> "$logfile"
  fi
}

log_ok() {
  _log_print "$COLOR_GREEN" "OK" "$1" "${LOG_FILE:-}"
}

log_info() {
  _log_print "$COLOR_CYAN" "INFO" "$1" "${LOG_FILE:-}"
}

log_warn() {
  _log_print "$COLOR_YELLOW" "WARN" "$1" "${LOG_FILE:-}"
}

log_error() {
  _log_print "$COLOR_RED" "ERROR" "$1" "${LOG_FILE:-}"
}

log_critical() {
  _log_print "$COLOR_RED$COLOR_BOLD" "CRITICAL" "$1" "${LOG_FILE:-}"
}

log_debug() {
  if [[ "${DEBUG:-0}" == "1" ]]; then
    _log_print "$COLOR_GRAY" "DEBUG" "$1" "${LOG_FILE:-}"
  fi
}

# Simple log without severity (for compatibility)
log() {
  local logfile="${LOG_FILE:-}"
  local timestamp
  timestamp="$(date '+%F %T')"
  printf "[%s] %s\n" "$timestamp" "$*"
  if [[ -n "$logfile" && -w "$(dirname "$logfile")" ]]; then
    printf "[%s] %s\n" "$timestamp" "$*" >> "$logfile"
  fi
}

# Die with error message
die() {
  log_critical "$*"
  exit 1
}

# =============================================================================
# Pre-flight checks
# =============================================================================

# Check if running on Arch Linux
check_arch_linux() {
  if [[ -f /etc/arch-release ]]; then
    return 0
  fi
  return 1
}

# Check network connectivity
check_connectivity() {
  # Try curl first (more reliable behind proxies)
  if command -v curl >/dev/null 2>&1; then
    if curl -fsSI --max-time 5 https://archlinux.org >/dev/null 2>&1; then
      return 0
    fi
  fi
  # Fallback to ping
  if command -v ping >/dev/null 2>&1; then
    if ping -c 1 -W 2 1.1.1.1 >/dev/null 2>&1; then
      return 0
    fi
    if ping -c 1 -W 2 8.8.8.8 >/dev/null 2>&1; then
      return 0
    fi
  fi
  return 1
}

# Check disk space on a path
# Usage: check_disk_space <path> <min_mb>
# Returns 0 if available space >= min_mb, 1 otherwise
check_disk_space() {
  local path="${1:-/}"
  local min_mb="${2:-1024}"

  if ! command -v df >/dev/null 2>&1; then
    return 1
  fi

  local avail_kb
  avail_kb="$(df -Pk "$path" 2>/dev/null | awk 'NR==2 {print $4}')"
  if [[ -z "$avail_kb" ]]; then
    return 1
  fi

  local avail_mb=$((avail_kb / 1024))
  if [[ $avail_mb -ge $min_mb ]]; then
    return 0
  fi
  return 1
}

# Get available disk space in MB
get_disk_space_mb() {
  local path="${1:-/}"
  local avail_kb
  avail_kb="$(df -Pk "$path" 2>/dev/null | awk 'NR==2 {print $4}')"
  if [[ -n "$avail_kb" ]]; then
    echo $((avail_kb / 1024))
  else
    echo "0"
  fi
}

# Get disk usage percentage
get_disk_usage_percent() {
  local path="${1:-/}"
  df -P "$path" 2>/dev/null | awk 'NR==2 {gsub(/%/,""); print $5}'
}

# Check battery level (for laptops)
# Returns 0 if on AC or battery >= threshold, 1 if below threshold
check_battery_level() {
  local min_percent="${1:-20}"

  # Check if this is a laptop with battery
  local bat_path=""
  for p in /sys/class/power_supply/BAT0 /sys/class/power_supply/BAT1; do
    if [[ -d "$p" ]]; then
      bat_path="$p"
      break
    fi
  done

  # No battery = not a laptop, return OK
  if [[ -z "$bat_path" ]]; then
    return 0
  fi

  # Check if on AC power
  local ac_status=""
  for ac in /sys/class/power_supply/AC*/online /sys/class/power_supply/ADP*/online; do
    if [[ -f "$ac" ]]; then
      ac_status="$(cat "$ac" 2>/dev/null)"
      break
    fi
  done

  # On AC power = OK
  if [[ "$ac_status" == "1" ]]; then
    return 0
  fi

  # Check battery level
  local capacity=""
  if [[ -f "$bat_path/capacity" ]]; then
    capacity="$(cat "$bat_path/capacity" 2>/dev/null)"
  fi

  if [[ -n "$capacity" && "$capacity" -ge "$min_percent" ]]; then
    return 0
  fi

  return 1
}

# Get battery level percentage (or -1 if no battery)
get_battery_level() {
  local bat_path=""
  for p in /sys/class/power_supply/BAT0 /sys/class/power_supply/BAT1; do
    if [[ -d "$p" ]]; then
      bat_path="$p"
      break
    fi
  done

  if [[ -z "$bat_path" ]]; then
    echo "-1"
    return
  fi

  if [[ -f "$bat_path/capacity" ]]; then
    cat "$bat_path/capacity" 2>/dev/null || echo "-1"
  else
    echo "-1"
  fi
}

# Check if on AC power
is_on_ac_power() {
  for ac in /sys/class/power_supply/AC*/online /sys/class/power_supply/ADP*/online; do
    if [[ -f "$ac" ]]; then
      if [[ "$(cat "$ac" 2>/dev/null)" == "1" ]]; then
        return 0
      fi
    fi
  done
  return 1
}

# Check if pacman is locked
check_pacman_lock() {
  if [[ -f /var/lib/pacman/db.lck ]]; then
    return 1
  fi
  return 0
}

# Check if system time is synced
check_time_synced() {
  if command -v timedatectl >/dev/null 2>&1; then
    if timedatectl show --property=NTPSynchronized --value 2>/dev/null | grep -q "yes"; then
      return 0
    fi
  fi
  return 1
}

# Run all pre-flight checks for updates
# Usage: run_preflight_checks <min_disk_mb> <min_battery_percent>
run_preflight_checks() {
  local min_disk_mb="${1:-2048}"
  local min_battery="${2:-20}"
  local errors=0

  log_info "Running pre-flight checks..."

  # Arch Linux check
  if check_arch_linux; then
    log_ok "Running on Arch Linux"
  else
    log_error "Not running on Arch Linux (/etc/arch-release not found)"
    ((errors++))
  fi

  # Network connectivity
  if check_connectivity; then
    log_ok "Network connectivity verified"
  else
    log_error "No network connectivity"
    ((errors++))
  fi

  # Disk space
  local avail_mb
  avail_mb="$(get_disk_space_mb /)"
  if check_disk_space "/" "$min_disk_mb"; then
    log_ok "Disk space OK (${avail_mb}MB available, ${min_disk_mb}MB required)"
  else
    log_error "Insufficient disk space (${avail_mb}MB available, ${min_disk_mb}MB required)"
    ((errors++))
  fi

  # Battery level (warning only, not an error)
  local battery_level
  battery_level="$(get_battery_level)"
  if [[ "$battery_level" == "-1" ]]; then
    log_ok "No battery detected (desktop/server)"
  elif is_on_ac_power; then
    log_ok "On AC power (battery at ${battery_level}%)"
  elif check_battery_level "$min_battery"; then
    log_ok "Battery level OK (${battery_level}%)"
  else
    log_warn "Battery level low (${battery_level}%, recommended ${min_battery}%+)"
  fi

  # Pacman lock
  if check_pacman_lock; then
    log_ok "No pacman lock detected"
  else
    log_error "Pacman is locked (/var/lib/pacman/db.lck exists)"
    ((errors++))
  fi

  # Time sync (warning only)
  if check_time_synced; then
    log_ok "System time is synchronized"
  else
    log_warn "System time may not be synchronized (NTP)"
  fi

  return $errors
}

# =============================================================================
# Utility functions
# =============================================================================

# Convert bytes to human-readable format
bytes_to_human() {
  local bytes="${1:-0}"

  if [[ $bytes -lt 1024 ]]; then
    echo "${bytes}B"
  elif [[ $bytes -lt 1048576 ]]; then
    echo "$((bytes / 1024))KB"
  elif [[ $bytes -lt 1073741824 ]]; then
    echo "$((bytes / 1048576))MB"
  else
    echo "$((bytes / 1073741824))GB"
  fi
}

# Convert human-readable size to bytes
human_to_bytes() {
  local size="$1"
  local num unit

  num="${size//[^0-9.]/}"
  unit="${size//[0-9.]/}"
  unit="${unit^^}"  # uppercase

  case "$unit" in
    K|KB|KIB) echo "$((${num%.*} * 1024))" ;;
    M|MB|MIB) echo "$((${num%.*} * 1048576))" ;;
    G|GB|GIB) echo "$((${num%.*} * 1073741824))" ;;
    T|TB|TIB) echo "$((${num%.*} * 1099511627776))" ;;
    *) echo "${num%.*}" ;;
  esac
}

# Check if a command exists
require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "Missing required command: $1"
  fi
}

# Check if command exists (returns 0/1 without dying)
has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

# Strip ANSI escape sequences
strip_ansi() {
  sed -r 's/\x1B\[[0-9;]*[mK]//g'
}

# Run command and log output (stdout+stderr), stripping ANSI
run_logged() {
  "$@" 2>&1 | strip_ansi | tee -a "${LOG_FILE:-/dev/null}"
}

# Get a value from a TOML file (basic implementation)
# Usage: get_toml_value <file> <key> <default>
get_toml_value() {
  local file="$1"
  local key="$2"
  local default="$3"

  if [[ ! -f "$file" ]]; then
    printf "%s" "$default"
    return
  fi

  local raw
  raw="$(sed -n "s/^[[:space:]]*$key[[:space:]]*=[[:space:]]*\(.*\)[[:space:]]*$/\1/p" "$file" | head -n 1)"
  if [[ -z "$raw" ]]; then
    printf "%s" "$default"
    return
  fi

  # Strip quotes
  raw="${raw%\"}"
  raw="${raw#\"}"
  printf "%s" "$raw"
}

# Convert various boolean representations to 0 or 1
to_bool() {
  local value="${1:-}"
  case "${value,,}" in
    1|true|yes|on) echo "1" ;;
    0|false|no|off|"") echo "0" ;;
    *) echo "$value" ;;
  esac
}

# Get system memory info in MB
get_memory_total_mb() {
  awk '/MemTotal/ {print int($2/1024)}' /proc/meminfo 2>/dev/null || echo "0"
}

get_memory_available_mb() {
  awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo 2>/dev/null || echo "0"
}

get_memory_used_percent() {
  local total avail
  total="$(awk '/MemTotal/ {print $2}' /proc/meminfo 2>/dev/null)"
  avail="$(awk '/MemAvailable/ {print $2}' /proc/meminfo 2>/dev/null)"
  if [[ -n "$total" && -n "$avail" && "$total" -gt 0 ]]; then
    echo "$((100 - (avail * 100 / total)))"
  else
    echo "0"
  fi
}

# Get load average (1 minute)
get_load_average() {
  awk '{print $1}' /proc/loadavg 2>/dev/null || echo "0"
}

# Get number of CPU cores
get_cpu_count() {
  nproc 2>/dev/null || grep -c ^processor /proc/cpuinfo 2>/dev/null || echo "1"
}

# JSON output helpers
json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# Create a simple JSON object from key-value pairs
# Usage: json_object key1 value1 key2 value2 ...
json_object() {
  local out="{"
  local first=1
  while [[ $# -ge 2 ]]; do
    local key="$1" value="$2"
    shift 2
    if [[ $first -eq 1 ]]; then
      first=0
    else
      out="$out,"
    fi
    out="$out\"$(json_escape "$key")\":\"$(json_escape "$value")\""
  done
  out="$out}"
  printf '%s' "$out"
}

# Acquire a lock file (flock-based)
# Usage: acquire_lock <lock_file> <fd_num>
acquire_lock() {
  local lock_file="$1"
  local fd_num="${2:-9}"

  mkdir -p "$(dirname "$lock_file")"
  eval "exec $fd_num>\"$lock_file\""
  if ! flock -n "$fd_num"; then
    return 1
  fi
  return 0
}

# Check if running as root
is_root() {
  [[ $EUID -eq 0 ]]
}

# Safe sudo wrapper that checks if already root
maybe_sudo() {
  if is_root; then
    "$@"
  else
    sudo "$@"
  fi
}
