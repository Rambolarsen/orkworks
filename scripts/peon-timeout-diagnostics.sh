#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

file_state() {
  if [ -f "$1" ]; then
    printf 'present'
  else
    printf 'absent'
  fi
}

setting_state() {
  local variable_name="$1"
  if [ -n "${!variable_name:-}" ]; then
    printf 'set'
  else
    printf 'absent'
  fi
}

probe_sidecar() {
  local label="$1"
  local path="$2"
  local base_url="http://127.0.0.1:${ORKWORKS_PORT}"
  local status

  if ! command -v curl >/dev/null 2>&1; then
    printf '%s: not checked (curl is unavailable)\n' "$label"
    return
  fi

  status="$(
    curl --silent --output /dev/null --write-out '%{http_code}' \
      --connect-timeout 2 --max-time 5 \
      "${base_url}${path}" 2>/dev/null || true
  )"
  if [[ "$status" =~ ^[0-9]{3}$ && "$status" != 000 ]]; then
    printf '%s: HTTP %s\n' "$label" "$status"
  else
    printf '%s: unavailable\n' "$label"
  fi
}

printf '%s\n' 'Peon timeout diagnostics'
printf 'Repository: %s\n' "$repo_root"
printf 'ORKWORKS_PORT: %s\n' "$(setting_state ORKWORKS_PORT)"
printf 'ORKWORKS_SESSION_ID: %s\n' "$(setting_state ORKWORKS_SESSION_ID)"
printf 'PEON_ENABLED: %s\n' "$(setting_state PEON_ENABLED)"
printf 'PEON_INTERVAL: %s\n' "$(setting_state PEON_INTERVAL)"
printf 'PEON_IDLE_TIMEOUT: %s\n' "$(setting_state PEON_IDLE_TIMEOUT)"
if [ -n "${PEON_TIMEOUT:-}" ]; then
  printf '%s\n' 'PEON_TIMEOUT: set (legacy; it does not control session inference)'
else
  printf '%s\n' 'PEON_TIMEOUT: absent (legacy; do not set it for current provider timeouts)'
fi
printf 'Sidecar executable: %s\n' "$(file_state "$repo_root/crates/orkworksd/target/debug/orkworksd")"

if [ -n "${ORKWORKS_PORT:-}" ]; then
  probe_sidecar 'Sidecar health' '/health'
  probe_sidecar 'Provider registry' '/providers'
else
  printf '%s\n' 'Sidecar health: not checked (ORKWORKS_PORT is absent)'
  printf '%s\n' 'Provider registry: not checked (ORKWORKS_PORT is absent)'
fi

printf '%s\n' 'Active provider/model: inspect Settings > Model providers; this is not exposed by environment variables.'
printf '%s\n' 'Next step: docs/agents/peon-timeout-troubleshooting.md'
