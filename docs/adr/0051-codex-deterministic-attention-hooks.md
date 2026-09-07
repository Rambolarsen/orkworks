# Codex deterministic attention hooks

- Status: accepted
- Deciders: Lars-Erik, Codex
- Date: 2026-09-07

## Context

The Codex `SessionStart` integration reliably captures a native session ID,
but it does not describe turn state. Relying on terminal/Peon inference leaves
Codex sessions stale while the tool is actively processing a prompt. Adding a
static attention capability would be unsafe because the hook file may be
missing, untrusted, stale, or disabled for an individual session.

## Decision

Extend the owned Codex hook bundle to `SessionStart`, `UserPromptSubmit`,
`PermissionRequest`, and `Stop`. `SessionStart` remains identity-only;
`UserPromptSubmit` reports `working`; `PermissionRequest` and `Stop` report
`waiting_for_input`. Every command carries one canonical bundle fingerprint,
and attention reports include the event, source, and fingerprint.

The sidecar validates the provenance against the current installed bundle and
the session's Codex harness before accepting it. A valid accepted event
promotes only that live session to hook-authoritative attention. Sessions
without such an event retain terminal/Peon fallback. Codex hook provenance is
stored at the agent-priority tier, while stale or forged reports are rejected
or ignored without mutation.

## Consequences

- Codex turn state can update deterministically without disabling fallback for
  sessions whose hooks are not active.
- Codex installation, probing, repair, and uninstall must treat one owned
  group per event as a bundle and preserve unrelated hooks.
- Codex continues to require the user's native `/hooks` trust approval; OrkWorks
  observes execution but never enables that trust itself.
- `SessionStart` must not create a waiting-for-input state, and Codex patch
  text remains insufficient for canonical plan-path reporting.
