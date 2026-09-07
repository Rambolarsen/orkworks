#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
codex_home="${CODEX_HOME:-${HOME:-$repo_root}/.codex}"

if command -v codex >/dev/null 2>&1; then
  codex_version_output="$(codex --version 2>/dev/null || true)"
  codex_version_output="${codex_version_output%%$'\n'*}"
  if [[ "$codex_version_output" =~ ^codex-cli[[:space:]]+([0-9]+\.[0-9]+(\.[0-9]+){0,2})$ ]]; then
    codex_version="version ${BASH_REMATCH[1]}"
  else
    codex_version="available (version output not recognized)"
  fi
else
  codex_version="unavailable (codex is not on PATH)"
fi

if repository="$(git -C "$repo_root" rev-parse --show-toplevel 2>/dev/null)"; then
  branch="$(git -C "$repo_root" branch --show-current 2>/dev/null || true)"
  [ -n "$branch" ] || branch="detached or unavailable"
else
  repository="unavailable"
  branch="unavailable"
fi

presence() {
  local variable_name="$1"
  if [ -n "${!variable_name:-}" ]; then
    printf 'present'
  else
    printf 'absent'
  fi
}

file_state() {
  if [ -f "$1" ]; then
    printf 'present'
  else
    printf 'absent'
  fi
}

printf '%s\n' 'Codex API diagnostics'
printf 'Timestamp (UTC): %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Repository: %s\n' "$repository"
printf 'Branch: %s\n' "$branch"
printf 'Codex CLI: %s\n' "$codex_version"
printf 'CODEX_HOME: %s\n' "$codex_home"
printf 'Project Codex config: %s\n' "$(file_state "$repo_root/.codex/config.toml")"
printf 'User Codex config: %s\n' "$(file_state "$codex_home/config.toml")"
printf 'Codex auth file: %s\n' "$(file_state "$codex_home/auth.json")"
printf 'OPENAI_API_KEY: %s\n' "$(presence OPENAI_API_KEY)"
printf 'CODEX_API_KEY: %s\n' "$(presence CODEX_API_KEY)"
printf 'OPENAI_BASE_URL: %s\n' "$(presence OPENAI_BASE_URL)"
printf '%s\n' 'Next step: preserve the exact error, HTTP status/error code, request ID, timestamp, and timezone; then use docs/agents/codex-api-troubleshooting.md.'
