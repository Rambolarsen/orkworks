#!/usr/bin/env bash
# Runs at end of each Claude Code session.
# Checks whether doc files need updating based on what changed.

CHANGED=$(git diff --name-only HEAD 2>/dev/null; git diff --cached --name-only 2>/dev/null) || exit 0
[ -z "$CHANGED" ] && exit 0

needs=()

# docs/agents/architecture.md — new routes, new Rust modules, IPC boundary, API client, or deps
NEW_RUST=$(git diff --name-status HEAD 2>/dev/null | grep -E '^A.*crates/orkworksd/src/' | wc -l)
NEW_ROUTES=$(git diff HEAD -- crates/orkworksd/src/main.rs 2>/dev/null | grep -cE '^\+.*\.route\(')
if echo "$CHANGED" | grep -qE 'apps/desktop/src/api\.ts|apps/desktop/electron/(main|preload)|apps/desktop/package\.json|Cargo\.toml' \
    || [ "$NEW_RUST" -gt 0 ] || [ "$NEW_ROUTES" -gt 0 ]; then
  echo "$CHANGED" | grep -q 'docs/agents/architecture\.md' || \
    needs+=("docs/agents/architecture.md  (Rust modules, preload API, endpoints, or deps changed)")
fi

# docs/agents/apm.md
if echo "$CHANGED" | grep -qE 'orkworks/apm\.yml|opencode\.json'; then
  echo "$CHANGED" | grep -q 'docs/agents/apm\.md' || \
    needs+=("docs/agents/apm.md  (APM config or OpenCode plugins changed)")
fi

# AGENTS.md
if echo "$CHANGED" | grep -qE '^skills/|orkworks/apm\.yml|apps/desktop/package\.json|Cargo\.toml'; then
  echo "$CHANGED" | grep -q '^AGENTS\.md' || \
    needs+=("AGENTS.md  (skills, deps, or APM targets changed)")
fi

# README.md — new ADRs or spec changes only
if echo "$CHANGED" | grep -qE '^docs/adr/|^specs/'; then
  echo "$CHANGED" | grep -q '^README\.md' || \
    needs+=("README.md  (architecture, milestones, or ADRs changed)")
fi

[ ${#needs[@]} -eq 0 ] && exit 0

# Plain stdout (no "decision":"block") so this surfaces as a warning, not a
# Stop-hook block -- doc drift is a nudge, not a gate.
printf '\n[doc-check] Consider updating before closing:\n'
for f in "${needs[@]}"; do
  printf '  • %s\n' "$f"
done
printf '\n'
