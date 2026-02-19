#!/usr/bin/env bash
# clean-system.sh - Aggressive resource reclamation for Arch Linux
# Reclaims disk space and memory for system performance
#
# Usage: scripts/clean-system.sh [options]
#   --dry-run         Preview what would be cleaned (default)
#   --apply           Execute cleanup operations
#   --aggressive      Include opt-in cleanups (browser, dev caches)
#   --category=X      Run specific category only
#   -h, --help        Show this help

set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/lib/common.sh"
source "$ROOT_DIR/scripts/lib/oplog.sh"
oplog_begin "$ROOT_DIR" "clean" "clean-system" "$@"
trap 'oplog_end $?' EXIT

# =============================================================================
# Configuration
# =============================================================================
POLICY_FILE="$ROOT_DIR/app/manifests/update-policy.toml"
STATE_DIR="$ROOT_DIR/app/state"
LOG_DIR="$STATE_DIR/logs"
LOCK_DIR="$STATE_DIR/locks"
LOCK_FILE="$LOCK_DIR/clean.lock"
LOG_FILE="$LOG_DIR/clean-$(date +%F).log"

DRY_RUN=1
AGGRESSIVE=0
CATEGORY_FILTER=""

# Policy defaults
CACHE_KEEP=3
REMOVE_UNINSTALLED=1
USER_CACHE_MAX_AGE_DAYS=30
JOURNAL_MAX_SIZE="500M"
JOURNAL_MAX_AGE="2weeks"
CLEAN_THUMBNAILS=1
CLEAN_BROWSER_CACHES=0
CLEAN_DEV_CACHES=0
SHOW_MEMORY_SUGGESTIONS=1

# Tracking space reclaimed
declare -A SPACE_RECLAIMED
TOTAL_RECLAIMED=0

# =============================================================================
# Usage
# =============================================================================
usage() {
  cat <<USAGE
Usage: scripts/clean-system.sh [options]

Aggressive resource reclamation for Arch Linux.

Options:
  --dry-run         Preview what would be cleaned (default)
  --apply           Execute cleanup operations
  --aggressive      Include opt-in cleanups (browser, dev caches)
  --category=X      Run specific category only:
                    packages, journal, user-cache, thumbnails, logs, memory, browser, dev
  -h, --help        Show this help

Categories:
  packages      Package cache and orphan packages
  journal       Systemd journal vacuum
  user-cache    User cache files older than N days
  thumbnails    Thumbnail cache cleanup
  logs          Old application logs
  memory        Memory usage report and suggestions
  browser       Browser caches (Firefox, Chrome) - opt-in with --aggressive
  dev           Developer caches (npm, yarn, pip, cargo) - opt-in with --aggressive

Exit codes:
  0 - Success
  1 - Runtime error
  2 - Usage error
USAGE
}

# =============================================================================
# Helpers
# =============================================================================
read_policy() {
  if [[ ! -f "$POLICY_FILE" ]]; then
    log "Policy file not found, using defaults."
    return 0
  fi

  CACHE_KEEP="$(get_toml_value "$POLICY_FILE" "cache_keep" "$CACHE_KEEP")"
  REMOVE_UNINSTALLED="$(to_bool "$(get_toml_value "$POLICY_FILE" "remove_uninstalled" "$REMOVE_UNINSTALLED")")"
  USER_CACHE_MAX_AGE_DAYS="$(get_toml_value "$POLICY_FILE" "user_cache_max_age_days" "$USER_CACHE_MAX_AGE_DAYS")"
  JOURNAL_MAX_SIZE="$(get_toml_value "$POLICY_FILE" "journal_max_size" "$JOURNAL_MAX_SIZE")"
  JOURNAL_MAX_AGE="$(get_toml_value "$POLICY_FILE" "journal_max_age" "$JOURNAL_MAX_AGE")"
  CLEAN_THUMBNAILS="$(to_bool "$(get_toml_value "$POLICY_FILE" "clean_thumbnails" "$CLEAN_THUMBNAILS")")"
  CLEAN_BROWSER_CACHES="$(to_bool "$(get_toml_value "$POLICY_FILE" "clean_browser_caches" "$CLEAN_BROWSER_CACHES")")"
  CLEAN_DEV_CACHES="$(to_bool "$(get_toml_value "$POLICY_FILE" "clean_dev_caches" "$CLEAN_DEV_CACHES")")"
  SHOW_MEMORY_SUGGESTIONS="$(to_bool "$(get_toml_value "$POLICY_FILE" "show_memory_suggestions" "$SHOW_MEMORY_SUGGESTIONS")")"
}

should_run_category() {
  local category="$1"
  if [[ -z "$CATEGORY_FILTER" ]]; then
    return 0
  fi
  [[ "$CATEGORY_FILTER" == "$category" ]]
}

# Get size of a directory in bytes
get_dir_size_bytes() {
  local dir="$1"
  if [[ -d "$dir" ]]; then
    du -sb "$dir" 2>/dev/null | cut -f1 || echo "0"
  else
    echo "0"
  fi
}

# Record space reclaimed
record_reclaimed() {
  local category="$1"
  local bytes="$2"
  SPACE_RECLAIMED["$category"]="${SPACE_RECLAIMED[$category]:-0}"
  SPACE_RECLAIMED["$category"]=$((SPACE_RECLAIMED["$category"] + bytes))
  TOTAL_RECLAIMED=$((TOTAL_RECLAIMED + bytes))
}

# =============================================================================
# Package cache cleanup
# =============================================================================
clean_package_cache() {
  should_run_category "packages" || return 0

  log_info "=== Package Cache Cleanup ==="

  if ! has_cmd paccache; then
    log_warn "paccache not found (install pacman-contrib). Skipping cache cleanup."
    return 0
  fi

  local cache_dir="/var/cache/pacman/pkg"
  local before_size
  before_size="$(get_dir_size_bytes "$cache_dir")"
  local before_human
  before_human="$(du -sh "$cache_dir" 2>/dev/null | cut -f1 || echo 'unknown')"
  log_info "Current cache size: $before_human"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would run paccache -rk${CACHE_KEEP} (keep ${CACHE_KEEP} versions)"
    paccache -dk"$CACHE_KEEP" 2>&1 | tee -a "$LOG_FILE" || true

    if [[ "$REMOVE_UNINSTALLED" -eq 1 ]]; then
      log_info "DRY-RUN: would remove cached packages of uninstalled packages"
      paccache -duk0 2>&1 | tee -a "$LOG_FILE" || true
    fi
  else
    log_info "Pruning package cache (keeping ${CACHE_KEEP} versions)..."
    maybe_sudo paccache -rk"$CACHE_KEEP" 2>&1 | tee -a "$LOG_FILE"

    if [[ "$REMOVE_UNINSTALLED" -eq 1 ]]; then
      log_info "Removing cached packages of uninstalled packages..."
      maybe_sudo paccache -ruk0 2>&1 | tee -a "$LOG_FILE"
    fi

    local after_size
    after_size="$(get_dir_size_bytes "$cache_dir")"
    local after_human
    after_human="$(du -sh "$cache_dir" 2>/dev/null | cut -f1 || echo 'unknown')"
    local freed=$((before_size - after_size))
    if [[ $freed -gt 0 ]]; then
      record_reclaimed "packages" "$freed"
      log_ok "Cache size after cleanup: $after_human (freed $(bytes_to_human $freed))"
    else
      log_ok "Cache size after cleanup: $after_human"
    fi
  fi
}

# =============================================================================
# Orphan packages
# =============================================================================
remove_orphans() {
  should_run_category "packages" || return 0

  log_info "=== Orphan Package Removal ==="

  local orphans
  orphans="$(pacman -Qtdq 2>/dev/null || true)"

  if [[ -z "$orphans" ]]; then
    log_ok "No orphan packages found."
    return 0
  fi

  local count
  count="$(echo "$orphans" | wc -l)"
  log_info "Found $count orphan package(s):"
  echo "$orphans" | tee -a "$LOG_FILE"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would run pacman -Rns on $count orphan(s)"
  else
    log_info "Removing orphan packages..."
    echo "$orphans" | maybe_sudo pacman -Rns --noconfirm - 2>&1 | tee -a "$LOG_FILE"
    log_ok "Orphan removal complete."
    record_reclaimed "orphans" 0  # Hard to calculate exact size
  fi
}

# =============================================================================
# Journal vacuum
# =============================================================================
vacuum_journal() {
  should_run_category "journal" || return 0

  log_info "=== Systemd Journal Vacuum ==="

  if ! has_cmd journalctl; then
    log_warn "journalctl not found. Skipping journal vacuum."
    return 0
  fi

  local disk_usage
  disk_usage="$(journalctl --disk-usage 2>/dev/null | head -1 || echo 'unknown')"
  log_info "Current journal usage: $disk_usage"

  local before_bytes
  before_bytes="$(journalctl --disk-usage 2>/dev/null | grep -oE '[0-9]+(\.[0-9]+)?[KMGT]?' | head -1)"
  before_bytes="$(human_to_bytes "$before_bytes")"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would vacuum journals older than $JOURNAL_MAX_AGE and limit to $JOURNAL_MAX_SIZE"
  else
    log_info "Vacuuming journals older than $JOURNAL_MAX_AGE..."
    maybe_sudo journalctl --vacuum-time="$JOURNAL_MAX_AGE" 2>&1 | tee -a "$LOG_FILE"

    log_info "Limiting journal size to $JOURNAL_MAX_SIZE..."
    maybe_sudo journalctl --vacuum-size="$JOURNAL_MAX_SIZE" 2>&1 | tee -a "$LOG_FILE"

    local after_usage
    after_usage="$(journalctl --disk-usage 2>/dev/null | head -1 || echo 'unknown')"
    local after_bytes
    after_bytes="$(journalctl --disk-usage 2>/dev/null | grep -oE '[0-9]+(\.[0-9]+)?[KMGT]?' | head -1)"
    after_bytes="$(human_to_bytes "$after_bytes")"

    local freed=$((before_bytes - after_bytes))
    if [[ $freed -gt 0 ]]; then
      record_reclaimed "journal" "$freed"
      log_ok "Journal usage after vacuum: $after_usage (freed $(bytes_to_human $freed))"
    else
      log_ok "Journal usage after vacuum: $after_usage"
    fi
  fi
}

# =============================================================================
# User cache cleanup
# =============================================================================
clean_user_cache() {
  should_run_category "user-cache" || return 0

  log_info "=== User Cache Cleanup ==="

  local user_cache="${HOME}/.cache"
  if [[ ! -d "$user_cache" ]]; then
    log_info "No user cache directory found."
    return 0
  fi

  local before_size
  before_size="$(get_dir_size_bytes "$user_cache")"
  local before_human
  before_human="$(du -sh "$user_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
  log_info "User cache (~/.cache) size: $before_human"

  local old_files old_count
  old_files="$(find "$user_cache" -type f -atime +"$USER_CACHE_MAX_AGE_DAYS" 2>/dev/null || true)"
  if [[ -z "$old_files" ]]; then
    old_count=0
  else
    old_count="$(printf '%s\n' "$old_files" | wc -l)"
  fi
  log_info "Files older than $USER_CACHE_MAX_AGE_DAYS days: $old_count"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would remove $old_count old files from ~/.cache"
    if [[ "$old_count" -gt 0 ]]; then
      local old_size
      old_size="$(echo "$old_files" | xargs du -cb 2>/dev/null | tail -1 | cut -f1 || echo "0")"
      log_info "  Space that would be freed: $(bytes_to_human "$old_size")"
    fi
  else
    if [[ "$old_count" -gt 0 ]]; then
      log_info "Removing cache files older than $USER_CACHE_MAX_AGE_DAYS days..."
      find "$user_cache" -type f -atime +"$USER_CACHE_MAX_AGE_DAYS" -delete 2>&1 | tee -a "$LOG_FILE" || true
      find "$user_cache" -type d -empty -delete 2>&1 | tee -a "$LOG_FILE" || true

      local after_size
      after_size="$(get_dir_size_bytes "$user_cache")"
      local after_human
      after_human="$(du -sh "$user_cache" 2>/dev/null | cut -f1 || echo 'unknown')"

      local freed=$((before_size - after_size))
      if [[ $freed -gt 0 ]]; then
        record_reclaimed "user-cache" "$freed"
        log_ok "User cache after cleanup: $after_human (freed $(bytes_to_human $freed))"
      else
        log_ok "User cache after cleanup: $after_human"
      fi
    else
      log_ok "No old cache files to remove."
    fi
  fi
}

# =============================================================================
# Thumbnail cache cleanup
# =============================================================================
clean_thumbnails() {
  should_run_category "thumbnails" || return 0
  [[ "$CLEAN_THUMBNAILS" -eq 1 ]] || return 0

  log_info "=== Thumbnail Cache Cleanup ==="

  local thumb_dir="${HOME}/.cache/thumbnails"
  if [[ ! -d "$thumb_dir" ]]; then
    log_info "No thumbnail cache directory found."
    return 0
  fi

  local before_size
  before_size="$(get_dir_size_bytes "$thumb_dir")"
  local before_human
  before_human="$(du -sh "$thumb_dir" 2>/dev/null | cut -f1 || echo 'unknown')"
  log_info "Thumbnail cache size: $before_human"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "DRY-RUN: would clear thumbnail cache"
  else
    log_info "Clearing thumbnail cache..."
    rm -rf "${thumb_dir:?}"/* 2>&1 | tee -a "$LOG_FILE" || true

    record_reclaimed "thumbnails" "$before_size"
    log_ok "Thumbnail cache cleared (freed $(bytes_to_human $before_size))"
  fi
}

# =============================================================================
# Application logs cleanup
# =============================================================================
clean_app_logs() {
  should_run_category "logs" || return 0

  log_info "=== Application Logs Cleanup ==="

  local total_freed=0

  # Clean old logs from app state directory
  local app_log_dir="$LOG_DIR"
  if [[ -d "$app_log_dir" ]]; then
    local log_count
    log_count="$(find "$app_log_dir" -name '*.log' -type f 2>/dev/null | wc -l || echo '0')"
    log_info "Total log files in $app_log_dir: $log_count"

    local old_logs old_log_count
    old_logs="$(find "$app_log_dir" -name '*.log' -type f -mtime +30 2>/dev/null || true)"
    if [[ -z "$old_logs" ]]; then
      old_log_count=0
    else
      old_log_count="$(printf '%s\n' "$old_logs" | wc -l)"
    fi

    if [[ "$old_log_count" -gt 0 ]]; then
      local old_size
      old_size="$(echo "$old_logs" | xargs du -cb 2>/dev/null | tail -1 | cut -f1 || echo "0")"

      if [[ "$DRY_RUN" -eq 1 ]]; then
        log_info "DRY-RUN: would remove $old_log_count old log file(s) ($(bytes_to_human "$old_size"))"
      else
        log_info "Removing $old_log_count old log files..."
        echo "$old_logs" | xargs rm -f 2>&1 | tee -a "$LOG_FILE" || true
        total_freed=$((total_freed + old_size))
        log_ok "Removed $old_log_count old log files (freed $(bytes_to_human "$old_size"))"
      fi
    else
      log_ok "No log files older than 30 days."
    fi
  fi

  # Clean ~/.local/state old logs
  local state_dir="${HOME}/.local/state"
  if [[ -d "$state_dir" ]]; then
    local state_logs
    state_logs="$(find "$state_dir" -name '*.log' -type f -mtime +30 2>/dev/null || true)"
    local state_log_count
    if [[ -z "$state_logs" ]]; then
      state_log_count=0
    else
      state_log_count="$(printf '%s\n' "$state_logs" | wc -l)"
    fi

    if [[ "$state_log_count" -gt 0 ]]; then
      local state_size
      state_size="$(echo "$state_logs" | xargs du -cb 2>/dev/null | tail -1 | cut -f1 || echo "0")"

      if [[ "$DRY_RUN" -eq 1 ]]; then
        log_info "DRY-RUN: would remove $state_log_count old state logs ($(bytes_to_human "$state_size"))"
      else
        log_info "Removing old state logs..."
        echo "$state_logs" | xargs rm -f 2>&1 | tee -a "$LOG_FILE" || true
        total_freed=$((total_freed + state_size))
      fi
    fi
  fi

  if [[ "$DRY_RUN" -eq 0 && $total_freed -gt 0 ]]; then
    record_reclaimed "logs" "$total_freed"
  fi
}

# =============================================================================
# Memory report
# =============================================================================
report_memory() {
  should_run_category "memory" || return 0
  [[ "$SHOW_MEMORY_SUGGESTIONS" -eq 1 ]] || return 0

  log_info "=== Memory Usage Report ==="

  local mem_total mem_avail mem_used_percent
  mem_total="$(get_memory_total_mb)"
  mem_avail="$(get_memory_available_mb)"
  mem_used_percent="$(get_memory_used_percent)"

  log_info "Total memory: ${mem_total}MB"
  log_info "Available memory: ${mem_avail}MB"
  log_info "Used: ${mem_used_percent}%"

  # Show top memory consumers
  log_info ""
  log_info "Top memory consumers:"
  ps aux --sort=-%mem 2>/dev/null | head -6 | tail -5 | while read -r line; do
    local user pid mem cmd
    user="$(echo "$line" | awk '{print $1}')"
    pid="$(echo "$line" | awk '{print $2}')"
    mem="$(echo "$line" | awk '{print $4}')"
    cmd="$(echo "$line" | awk '{print $11}')"
    printf "  PID %s (%s): %s%% - %s\n" "$pid" "$user" "$mem" "$cmd"
  done

  if [[ "$mem_used_percent" -ge 90 ]]; then
    log_warn "Memory pressure is high. Consider closing unused applications."
    log_info "Hint: Use 'kill <PID>' to terminate a process, or 'systemctl --user stop <service>' for user services."
  fi
}

# =============================================================================
# Browser cache cleanup (opt-in)
# =============================================================================
clean_browser_caches() {
  should_run_category "browser" || return 0

  if [[ "$AGGRESSIVE" -eq 0 && "$CLEAN_BROWSER_CACHES" -eq 0 ]]; then
    return 0
  fi

  log_info "=== Browser Cache Cleanup ==="

  local total_freed=0

  # Firefox cache
  local firefox_cache="${HOME}/.cache/mozilla/firefox"
  if [[ -d "$firefox_cache" ]]; then
    local ff_size
    ff_size="$(get_dir_size_bytes "$firefox_cache")"
    local ff_human
    ff_human="$(du -sh "$firefox_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
    log_info "Firefox cache: $ff_human"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      log_info "DRY-RUN: would clear Firefox cache"
    else
      # Only clear cache directories, not profile data
      find "$firefox_cache" -type d -name "cache2" -exec rm -rf {}/* \; 2>/dev/null || true
      find "$firefox_cache" -type d -name "cache" -exec rm -rf {}/* \; 2>/dev/null || true
      local ff_after
      ff_after="$(get_dir_size_bytes "$firefox_cache")"
      local ff_freed=$((ff_size - ff_after))
      if [[ $ff_freed -gt 0 ]]; then
        total_freed=$((total_freed + ff_freed))
        log_ok "Firefox cache cleaned (freed $(bytes_to_human $ff_freed))"
      fi
    fi
  fi

  # Chrome/Chromium cache
  for chrome_dir in "${HOME}/.cache/google-chrome" "${HOME}/.cache/chromium"; do
    if [[ -d "$chrome_dir" ]]; then
      local chrome_size
      chrome_size="$(get_dir_size_bytes "$chrome_dir")"
      local chrome_human
      chrome_human="$(du -sh "$chrome_dir" 2>/dev/null | cut -f1 || echo 'unknown')"
      log_info "$(basename "$chrome_dir") cache: $chrome_human"

      if [[ "$DRY_RUN" -eq 1 ]]; then
        log_info "DRY-RUN: would clear $(basename "$chrome_dir") cache"
      else
        # Clear cache directories
        find "$chrome_dir" -type d -name "Cache" -exec rm -rf {}/* \; 2>/dev/null || true
        find "$chrome_dir" -type d -name "Code Cache" -exec rm -rf {}/* \; 2>/dev/null || true
        local chrome_after
        chrome_after="$(get_dir_size_bytes "$chrome_dir")"
        local chrome_freed=$((chrome_size - chrome_after))
        if [[ $chrome_freed -gt 0 ]]; then
          total_freed=$((total_freed + chrome_freed))
          log_ok "$(basename "$chrome_dir") cache cleaned (freed $(bytes_to_human $chrome_freed))"
        fi
      fi
    fi
  done

  if [[ "$DRY_RUN" -eq 0 && $total_freed -gt 0 ]]; then
    record_reclaimed "browser" "$total_freed"
  fi
}

# =============================================================================
# Developer cache cleanup (opt-in)
# =============================================================================
clean_dev_caches() {
  should_run_category "dev" || return 0

  if [[ "$AGGRESSIVE" -eq 0 && "$CLEAN_DEV_CACHES" -eq 0 ]]; then
    return 0
  fi

  log_info "=== Developer Cache Cleanup ==="

  local total_freed=0

  # npm cache
  if has_cmd npm; then
    local npm_cache
    npm_cache="$(npm config get cache 2>/dev/null || echo "${HOME}/.npm")"
    if [[ -d "$npm_cache" ]]; then
      local npm_size
      npm_size="$(get_dir_size_bytes "$npm_cache")"
      local npm_human
      npm_human="$(du -sh "$npm_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
      log_info "npm cache: $npm_human"

      if [[ "$DRY_RUN" -eq 1 ]]; then
        log_info "DRY-RUN: would run npm cache clean --force"
      else
        npm cache clean --force 2>&1 | tee -a "$LOG_FILE" || true
        local npm_after
        npm_after="$(get_dir_size_bytes "$npm_cache")"
        local npm_freed=$((npm_size - npm_after))
        if [[ $npm_freed -gt 0 ]]; then
          total_freed=$((total_freed + npm_freed))
          log_ok "npm cache cleaned (freed $(bytes_to_human $npm_freed))"
        fi
      fi
    fi
  fi

  # yarn cache
  if has_cmd yarn; then
    local yarn_cache
    yarn_cache="$(yarn cache dir 2>/dev/null || echo "${HOME}/.cache/yarn")"
    if [[ -d "$yarn_cache" ]]; then
      local yarn_size
      yarn_size="$(get_dir_size_bytes "$yarn_cache")"
      local yarn_human
      yarn_human="$(du -sh "$yarn_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
      log_info "yarn cache: $yarn_human"

      if [[ "$DRY_RUN" -eq 1 ]]; then
        log_info "DRY-RUN: would run yarn cache clean"
      else
        yarn cache clean 2>&1 | tee -a "$LOG_FILE" || true
        local yarn_after
        yarn_after="$(get_dir_size_bytes "$yarn_cache")"
        local yarn_freed=$((yarn_size - yarn_after))
        if [[ $yarn_freed -gt 0 ]]; then
          total_freed=$((total_freed + yarn_freed))
          log_ok "yarn cache cleaned (freed $(bytes_to_human $yarn_freed))"
        fi
      fi
    fi
  fi

  # pip cache
  local pip_cache="${HOME}/.cache/pip"
  if [[ -d "$pip_cache" ]]; then
    local pip_size
    pip_size="$(get_dir_size_bytes "$pip_cache")"
    local pip_human
    pip_human="$(du -sh "$pip_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
    log_info "pip cache: $pip_human"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      log_info "DRY-RUN: would clear pip cache"
    else
      if has_cmd pip; then
        pip cache purge 2>&1 | tee -a "$LOG_FILE" || rm -rf "$pip_cache"/* 2>/dev/null || true
      else
        rm -rf "$pip_cache"/* 2>/dev/null || true
      fi
      local pip_after
      pip_after="$(get_dir_size_bytes "$pip_cache")"
      local pip_freed=$((pip_size - pip_after))
      if [[ $pip_freed -gt 0 ]]; then
        total_freed=$((total_freed + pip_freed))
        log_ok "pip cache cleaned (freed $(bytes_to_human $pip_freed))"
      fi
    fi
  fi

  # cargo cache (registry and target dirs are separate)
  local cargo_cache="${HOME}/.cargo/registry"
  if [[ -d "$cargo_cache" ]]; then
    local cargo_size
    cargo_size="$(get_dir_size_bytes "$cargo_cache")"
    local cargo_human
    cargo_human="$(du -sh "$cargo_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
    log_info "cargo registry cache: $cargo_human"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      log_info "DRY-RUN: would clear cargo cache"
    else
      if has_cmd cargo-cache; then
        cargo-cache --remove-dir all 2>&1 | tee -a "$LOG_FILE" || true
      else
        # Only clear cache, not src (downloads are in cache)
        rm -rf "${HOME}/.cargo/registry/cache"/* 2>/dev/null || true
      fi
      local cargo_after
      cargo_after="$(get_dir_size_bytes "$cargo_cache")"
      local cargo_freed=$((cargo_size - cargo_after))
      if [[ $cargo_freed -gt 0 ]]; then
        total_freed=$((total_freed + cargo_freed))
        log_ok "cargo cache cleaned (freed $(bytes_to_human $cargo_freed))"
      fi
    fi
  fi

  # Go module cache
  local go_cache="${HOME}/go/pkg/mod/cache"
  if [[ -d "$go_cache" ]]; then
    local go_size
    go_size="$(get_dir_size_bytes "$go_cache")"
    local go_human
    go_human="$(du -sh "$go_cache" 2>/dev/null | cut -f1 || echo 'unknown')"
    log_info "Go module cache: $go_human"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      log_info "DRY-RUN: would run go clean -modcache"
    else
      if has_cmd go; then
        go clean -modcache 2>&1 | tee -a "$LOG_FILE" || true
        local go_after
        go_after="$(get_dir_size_bytes "$go_cache")"
        local go_freed=$((go_size - go_after))
        if [[ $go_freed -gt 0 ]]; then
          total_freed=$((total_freed + go_freed))
          log_ok "Go module cache cleaned (freed $(bytes_to_human $go_freed))"
        fi
      fi
    fi
  fi

  if [[ "$DRY_RUN" -eq 0 && $total_freed -gt 0 ]]; then
    record_reclaimed "dev" "$total_freed"
  fi
}

# =============================================================================
# Summary
# =============================================================================
print_summary() {
  echo ""
  echo "=========================================="
  echo "Cleanup Summary"
  echo "=========================================="

  if [[ "$DRY_RUN" -eq 1 ]]; then
    log_info "Mode: DRY-RUN (no changes were made)"
    log_info "Run with --apply to execute cleanup."
  else
    log_info "Mode: APPLY"

    if [[ ${#SPACE_RECLAIMED[@]} -gt 0 ]]; then
      echo ""
      echo "Space reclaimed by category:"
      for category in "${!SPACE_RECLAIMED[@]}"; do
        printf "  %-15s %s\n" "$category:" "$(bytes_to_human "${SPACE_RECLAIMED[$category]}")"
      done
      echo ""
      echo "Total space reclaimed: $(bytes_to_human $TOTAL_RECLAIMED)"
    else
      echo ""
      echo "No space was reclaimed (already clean or nothing to clean)."
    fi
  fi

  echo ""
  log_ok "Cleanup complete."
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
    --aggressive)
      AGGRESSIVE=1
      ;;
    --category=*)
      CATEGORY_FILTER="${arg#--category=}"
      case "$CATEGORY_FILTER" in
        packages|journal|user-cache|thumbnails|logs|memory|browser|dev) ;;
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

mkdir -p "$LOG_DIR" "$LOCK_DIR"

# Acquire lock
if ! acquire_lock "$LOCK_FILE" 9; then
  die "Another cleanup is already in progress (lock: $LOCK_FILE)"
fi

# Read policy
read_policy

log "=== System Cleanup Started ==="
log_info "Root: $ROOT_DIR"
log_info "Mode: $([ $DRY_RUN -eq 1 ] && echo 'DRY-RUN' || echo 'APPLY')"
log_info "Aggressive: $([ $AGGRESSIVE -eq 1 ] && echo 'yes' || echo 'no')"
if [[ -n "$CATEGORY_FILTER" ]]; then
  log_info "Category: $CATEGORY_FILTER"
fi
echo ""

# Run cleanup operations
clean_package_cache
echo ""
remove_orphans
echo ""
vacuum_journal
echo ""
clean_user_cache
echo ""
clean_thumbnails
echo ""
clean_app_logs
echo ""
report_memory
echo ""
clean_browser_caches
echo ""
clean_dev_caches

# Print summary
print_summary

log "=== System Cleanup Complete ==="
exit 0
