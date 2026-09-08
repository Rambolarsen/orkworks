#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
policy_file="$repo_root/AGENTS.md"

require_policy_text() {
  local text="$1"
  if ! grep -Fq "$text" "$policy_file"; then
    printf 'missing branch-protection guidance in %s: %s\n' "$policy_file" "$text" >&2
    exit 1
  fi
}

require_policy_text 'GitHub rejects approval from the PR author'
require_policy_text 'gh pr review --approve'
require_policy_text 'gh pr merge <PR_NUMBER> --squash --admin'
require_policy_text 'wait for every required status check to pass'
require_policy_text 'scripts/branch-protection-policy-check.sh'

printf 'branch-protection policy guidance is present\n'
