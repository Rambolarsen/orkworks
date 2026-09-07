#!/usr/bin/env bash

set -euo pipefail

config_path="${1:-.codex/hooks.json}"

if [[ ! -f "$config_path" ]]; then
  exit 0
fi

temporary_path="$(mktemp "${TMPDIR:-/tmp}/orkworks-codex-hooks.XXXXXX")"
trap 'rm -f "$temporary_path"' EXIT

jq '
  if (.hooks? | type) != "object" or (.hooks.SessionStart? | type) != "array" then
    .
  else
    .hooks.SessionStart = [
      .hooks.SessionStart[]
      | select(
          ([.hooks[]?.command // empty] | any(contains("superpowers/hooks/run-hook.cmd")))
          | not
        )
    ]
  end
' "$config_path" > "$temporary_path"

if cmp -s "$temporary_path" "$config_path"; then
  exit 0
fi

mv "$temporary_path" "$config_path"
