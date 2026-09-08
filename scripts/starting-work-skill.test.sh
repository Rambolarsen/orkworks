#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
skill="$repo_root/skills/starting-work/SKILL.md"
root_instructions="$repo_root/AGENTS.md"

grep -Fq 'launch another coding harness from an active session' "$skill"
grep -Fq 'wakeup or `/loop` continuation' "$skill"
grep -Fq 'stop and let the user start a' "$skill"
grep -Fq 'separate session from the appropriate repository root' "$skill"
grep -Fq 'sibling-worktree path' "$skill"
grep -Fq 'Each coding session owns one task through verification' "$root_instructions"

echo 'Starting-work nested-session guard passed'
