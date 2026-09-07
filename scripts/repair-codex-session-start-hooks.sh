#!/usr/bin/env bash

set -euo pipefail

config_path="${1:-.codex/hooks.json}"

if [[ ! -f "$config_path" ]]; then
  exit 0
fi

config_directory="$(dirname "$config_path")"
temporary_path="$(mktemp "$config_directory/.orkworks-codex-hooks.XXXXXX")"
trap 'rm -f "$temporary_path"' EXIT

jq '
  def incompatible_superpowers_session_start:
    if (.command? | type) != "string" then
      false
    else
      (.command
       | contains(".codex/hooks/superpowers/hooks/run-hook.cmd")
       and endswith("session-start"))
    end;

  if (.hooks? | type) != "object" or (.hooks.SessionStart? | type) != "array" then
    .
  else
    .hooks.SessionStart = [
      .hooks.SessionStart[]
      | if (.hooks? | type) != "array" then
          .
        else
          .hooks = [.hooks[] | select(incompatible_superpowers_session_start | not)]
        end
      | select((.hooks? | type) != "array" or (.hooks | length) > 0)
    ]
  end
' "$config_path" > "$temporary_path"

if cmp -s "$temporary_path" "$config_path"; then
  exit 0
fi

mv "$temporary_path" "$config_path"
