#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
skill="$repo_root/skills/starting-work/SKILL.md"
root_instructions="$repo_root/AGENTS.md"

grep -Fq 'launch another coding harness from an active session' "$skill"
grep -Fq 'terminal state (merged or closed)' "$skill"
grep -Fq 'ScheduleWakeup' "$skill"
grep -Fq '2-hour wall-clock budget' "$skill"
grep -Fq 'let the user start' "$skill"
grep -Fq 'separate session from the appropriate repository root' "$skill"
grep -Fq 'sibling-worktree path' "$skill"
grep -Fq "terminal state (merged or closed), not just through opening it" "$root_instructions"

echo 'Starting-work nested-session guard passed'
