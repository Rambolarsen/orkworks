#!/usr/bin/env bash

set -euo pipefail

config_path="${1:-.codex/hooks.json}"

if [[ ! -f "$config_path" ]]; then
  echo "Codex hook config not found: $config_path" >&2
  exit 1
fi

commands="$(jq -r '[.hooks.SessionStart[]?.hooks[]?.command // empty] | .[]' "$config_path")"

if grep -Fq '.codex/hooks/superpowers/hooks/run-hook.cmd' <<<"$commands"; then
  echo "Codex SessionStart must not register Superpowers' non-Codex hook output." >&2
  exit 1
fi

grep -Fq 'report-harness-event.sh' <<<"$commands"
grep -Fq 'agent-skills/hooks/session-start.sh' <<<"$commands"

echo "Codex SessionStart hook configuration is valid"
