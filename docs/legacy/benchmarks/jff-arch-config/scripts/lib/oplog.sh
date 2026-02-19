#!/usr/bin/env bash
set -euo pipefail

OPLOG_ROOT=""
OPLOG_MODULE=""
OPLOG_OPERATION=""
OPLOG_ARGS_JSON="[]"
OPLOG_STARTED_MS="0"

_json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

_args_to_json() {
  local out="["
  local first=1
  local arg escaped
  for arg in "$@"; do
    escaped="$(_json_escape "$arg")"
    if [[ "$first" -eq 1 ]]; then
      out="$out\"$escaped\""
      first=0
    else
      out="$out,\"$escaped\""
    fi
  done
  out="$out]"
  printf '%s' "$out"
}

oplog_begin() {
  local root="$1"
  local module="$2"
  local operation="$3"
  shift 3

  OPLOG_ROOT="$root"
  OPLOG_MODULE="$module"
  OPLOG_OPERATION="$operation"
  OPLOG_ARGS_JSON="$(_args_to_json "$@")"
  OPLOG_STARTED_MS="$(date +%s%3N)"
}

oplog_end() {
  local exit_code="$1"
  local now_ms duration_ms result log_dir log_file

  if [[ -z "${OPLOG_ROOT:-}" ]]; then
    return 0
  fi

  now_ms="$(date +%s%3N)"
  duration_ms=$(( now_ms - OPLOG_STARTED_MS ))
  result="error"
  if [[ "$exit_code" -eq 0 ]]; then
    result="ok"
  fi

  log_dir="$OPLOG_ROOT/app/state/logs"
  log_file="$log_dir/operations.jsonl"
  mkdir -p "$log_dir"

  printf '{"timestamp_unix_ms":%s,"module":"%s","operation":"%s","args":%s,"duration_ms":%s,"result":"%s","exit_code":%s}\n' \
    "$now_ms" \
    "$(_json_escape "$OPLOG_MODULE")" \
    "$(_json_escape "$OPLOG_OPERATION")" \
    "$OPLOG_ARGS_JSON" \
    "$duration_ms" \
    "$result" \
    "$exit_code" >> "$log_file"
}
