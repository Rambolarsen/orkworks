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
`waiting_for_input` and `idle`, respectively. Every command carries one canonical bundle fingerprint,
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

## Amendment 2026-10-01 (proposed): `PermissionRequest` is not proof of a human prompt

Status of this amendment: **proposed**. It records a gap in the decision above
and the verification gate for closing it; it changes no behavior until the gate
passes and the amendment is marked accepted. Tracked by
[#690](https://github.com/Rambolarsen/orkworks/issues/690).

### Context

Codex runs the `PermissionRequest` hook "when Codex is about to ask for
approval", including when `approvals_reviewer = "auto_review"` resolves the
request without any user prompt. The decision above treats the event as a direct
prompt signal, and the only clears are a user keystroke or click, a later
`UserPromptSubmit`, or `Stop`. Under auto-review none of those occurs mid-turn,
so a working session latches `needs_you` until the turn ends (observed on a
live session on 2026-10-01). The documented payload (`session_id`, `cwd`,
`hook_event_name`, `model`, `permission_mode`, `turn_id`, `tool_name`,
`tool_input`) has no field that separates auto-reviewed from manual approvals.

### Decision (proposed)

Hook authority stands; the gap is the missing resolution signal, not the
authority rule. Add a conservative resolution path, in this order:

1. **Verification gate (blocking).** Before any behavior change, capture a real
   `PermissionRequest` and `PostToolUse` payload pair under auto-review and
   under a manual prompt, recording only key names plus the allowlisted scalars
   `hook_event_name`, `permission_mode`, `turn_id`, and `tool_name`. Never record
   `tool_input`, transcript paths, or other free text. This is a deliberate,
   reviewed exception to the reporter's no-payload rule and must stay local,
   bounded, and redacted like the existing reporter diagnostic.
2. **Turn-scoped `PostToolUse` resolution.** If the gate shows `PostToolUse`
   carries the same `turn_id` and `tool_name`, install it as a fifth owned
   event. The sidecar records `turn_id` and `tool_name` from the accepted
   `PermissionRequest` and clears the wait only when a `PostToolUse` matches
   both and no other `PermissionRequest` is outstanding for that turn.
   Ambiguous cases (parallel calls, missing or mismatched `turn_id`) leave
   `needs_you` in place. A late or duplicate event never reopens a cleared wait.
3. **No terminal-text heuristic.** Terminal text such as "Reviewing approval
   requests" is not an authority source for attention: it is a Peon-tier
   inference and the hook-authority rule keeps it from overriding hook state.
   It may be revisited only if the gate shows `PostToolUse` cannot resolve the
   wait.

### Consequences

- The "generic successful `PostToolUse` must not clear a permission wait"
  sentence in `harness-integration-contracts.md` stays true for generic
  uncorrelated events; this amendment would allow only the turn-and-tool
  correlated case above, once the gate proves the fields exist.
- A missed or ambiguous correlation fails safe: Needs You stays until `Stop`,
  the same behavior as today. The remaining cost is occasional false Needs You,
  not a missed real prompt.
- The Codex bundle grows from four to five owned events, so the bundle
  fingerprint changes and users must re-approve hooks via `/hooks` once.
- If the gate shows the fields are absent, this amendment is withdrawn and the
  issue falls back to documenting the limitation and a user-side workaround.
