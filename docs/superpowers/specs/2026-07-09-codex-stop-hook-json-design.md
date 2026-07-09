# Codex Stop Hook JSON Design

## Goal

Fix Codex sessions that end with `Stop hook (failed)` because the repo's Codex stop hook emits plain text instead of valid stop-hook JSON.

## Scope

- Keep the existing doc-diff logic in `.codex/hooks/doc-check.sh`.
- Add a Codex-specific wrapper that translates the doc-check result into valid stop-hook JSON.
- Repoint `.codex/hooks.json` so Codex runs the wrapper instead of the plain-text script.
- Add a focused regression test for the wrapper output contract.

## Design

- The wrapper runs `.codex/hooks/doc-check.sh` and captures its stdout/stderr.
- If the doc check produces no message, the wrapper emits `{}`.
- If the doc check produces a message, the wrapper emits JSON with `systemMessage`.
- If the doc check exits non-zero, the wrapper still emits valid JSON so Codex never sees invalid stop-hook output from this path.

## Non-Goals

- No change to Claude's `.claude/hooks/doc-check.sh` behavior
- No APM regeneration work
- No change to the doc-check diff rules themselves

## Verification

- Add a Node test that executes the wrapper with controlled input and asserts valid JSON output.
- Verify `.codex/hooks.json` points Stop to the wrapper.
