# Copilot icon design

## Problem

Sessions that use GitHub Copilot currently fall back to the generic terminal-prompt glyph in the Sessions list. That makes Copilot rows look like an unknown tool instead of a first-class integration.

## Goal

Show the correct Copilot brand mark anywhere OrkWorks renders a harness icon.

## Proposed change

1. Add a Copilot SVG mark to `apps/desktop/src/harnessIcons.ts`.
2. Map both `gh-copilot` and `copilot` to that mark so legacy session data and the current provider id resolve the same way.
3. Extend `apps/desktop/tests/harnessIcon.test.ts` so the Copilot id/name pair is covered alongside the other built-ins.

## Non-goals

- No changes to session sorting, labels, or provider wiring.
- No visual redesign beyond the Copilot icon itself.

## Testing

- Unit test the key normalization and icon lookup.
- Verify the Sessions list no longer shows the fallback glyph for Copilot sessions.
