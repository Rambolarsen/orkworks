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

Status of this amendment: **proposed** (owner approved capture-only
`PostToolUse` and `PreToolUse` diagnostics on 2026-10-01; behavior stays
unchanged until the verification gate below passes). It records a gap in the decision above
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
   `PreToolUse`, `PermissionRequest`, and `PostToolUse` event sequence under
   auto-review and under a manual prompt, recording only key names plus the allowlisted scalars
   `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and any
   per-invocation identifier the payload carries (for example `tool_use_id`). Never record
   `tool_input`, transcript paths, or other free text. This is a deliberate,
   reviewed exception to the reporter's no-payload rule and must stay local,
   bounded, and redacted like the existing reporter diagnostic.
2. **Invocation-scoped `PostToolUse` resolution.** `turn_id` plus `tool_name`
   is not unique: one turn can invoke the same tool repeatedly, and a late or
   duplicate `PostToolUse` for an earlier call could clear a later, real
   prompt. `PermissionRequest` has no documented invocation ID, while
   `PreToolUse` and `PostToolUse` carry `tool_use_id`. A future resolution may
   use that ID only if the gate proves that the captured event ordering
   unambiguously ties the request to the matching pre/post pair. Otherwise no
   `PostToolUse` correlation is adopted and step 2 is withdrawn. Ambiguous
   cases (parallel calls, missing IDs, or uncertain ordering) leave
   `needs_you` in place. A late or duplicate event never reopens a cleared wait.
3. **No terminal-text heuristic.** Terminal text such as "Reviewing approval
   requests" is not an authority source for attention: it is a Peon-tier
   inference and the hook-authority rule keeps it from overriding hook state.
   It may be revisited only if the gate shows `PostToolUse` cannot resolve the
   wait.

### Consequences

- The "generic successful `PostToolUse` must not clear a permission wait"
  sentence in `harness-integration-contracts.md` stays true for generic
  uncorrelated events; this amendment would allow only the exact
  per-invocation-identifier match above, once the gate proves one exists.
- A missed or ambiguous correlation fails safe: Needs You stays until `Stop`,
  the same behavior as today. The remaining cost is occasional false Needs You,
  not a missed real prompt.
- The capture experiment adds `PreToolUse` to the current five-event bundle,
  making six owned events; the bundle fingerprint changes and users must
  re-approve hooks via `/hooks` once.
- If the gate shows the fields are absent, this amendment is withdrawn and the
  issue falls back to documenting the limitation and a user-side workaround.

## Verification update 2026-10-03

The blocking **serial payload capture gate passed** for real automatic and
manual approval sequences. Matching pre/post tool_use_id and event ordering
support serial correlation; PermissionRequest has no invocation ID and the
captures do not prove parallel correlation. The
[investigation record](https://github.com/Rambolarsen/orkworks/issues/690#issuecomment-5973628408)
also records native manual pending-to-active evidence and its limits.

Passing the gate does not accept or implement a resolution rule. PostToolUse
alone cannot avoid false Needs You during long tool execution. The amendment
remains proposed, with the next behavior decision captured in
[proposed ADR 0076](./0076-codex-owned-native-approval-observer.md) and its written
design. PreToolUse and PostToolUse remain capture-only in the current bundle;
PermissionRequest still uses the accepted mapping until that proposal is
reviewed and implemented.
