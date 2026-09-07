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

temporary_directory="$(mktemp -d)"
trap 'rm -rf "$temporary_directory"' EXIT
fixture_path="$temporary_directory/hooks.json"

jq '.hooks.SessionStart += [
  {
    "hooks": [
      {"type": "command", "command": "\".codex/hooks/superpowers/hooks/run-hook.cmd\" session-start"},
      {"type": "command", "command": "echo unrelated sibling hook"}
    ]
  },
  {
    "hooks": [
      {"type": "command", "command": "echo mentions superpowers/hooks/run-hook.cmd but is unrelated"}
    ]
  }
]' "$config_path" > "$fixture_path"

bash scripts/repair-codex-session-start-hooks.sh "$fixture_path"
fixture_commands="$(jq -r '[.hooks.SessionStart[]?.hooks[]?.command // empty] | .[]' "$fixture_path")"

if grep -Fq '".codex/hooks/superpowers/hooks/run-hook.cmd" session-start' <<<"$fixture_commands"; then
  echo "repair left the incompatible Superpowers hook installed" >&2
  exit 1
fi

grep -Fq 'echo unrelated sibling hook' <<<"$fixture_commands"
grep -Fq 'echo mentions superpowers/hooks/run-hook.cmd but is unrelated' <<<"$fixture_commands"
grep -Fq 'report-harness-event.sh' <<<"$fixture_commands"
grep -Fq 'agent-skills/hooks/session-start.sh' <<<"$fixture_commands"

if jq -e '[.hooks.SessionStart[]? | (.hooks? // []) | length] | any(. == 0)' "$fixture_path" >/dev/null; then
  echo "repair left an empty SessionStart group" >&2
  exit 1
fi

before_second_repair="$(shasum -a 256 "$fixture_path")"
bash scripts/repair-codex-session-start-hooks.sh "$fixture_path"
after_second_repair="$(shasum -a 256 "$fixture_path")"
if [[ "$before_second_repair" != "$after_second_repair" ]]; then
  echo "repair is not idempotent" >&2
  exit 1
fi

echo "Codex SessionStart hook configuration is valid"
