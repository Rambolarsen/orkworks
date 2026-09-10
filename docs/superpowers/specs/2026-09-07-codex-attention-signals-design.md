# Codex deterministic attention signals

## Status

Design approved in conversation; implementation pending spec review and an
implementation plan.

## Problem

OrkWorks currently installs only a Codex `SessionStart` hook. That hook is
useful for capturing the native Codex session ID, but it does not describe the
state of a turn. The shared reporter deliberately suppresses all attention
POSTs for the Codex marker, so a Codex session can continue producing output
while its last Peon inference remains `idle`.

The current runtime also treats `active_work_hook` as a launch-time static
capability. Accepting a Codex `working` report therefore requires either
predeclaring every Codex session authoritative or changing the session to
hook-authoritative only after a valid Codex hook event is observed. The former
would disable the terminal/Peon fallback when the hook file is missing,
untrusted, stale, or not actually enabled by Codex.

This change makes Codex turn state deterministic when its explicitly approved
hooks are active while preserving the observer-first fallback contract.

## Goals

- Install and reconcile the Codex turn events that have a verified payload:
  `SessionStart`, `UserPromptSubmit`, `PermissionRequest`, and `Stop`.
- Capture `session_id` from every useful Codex event, retaining the existing
  `codex_hook` source and hook-fingerprint verification.
- Map prompt submission to `working`, permission requests to
  `waiting_for_input`, and completed turns to `idle`.
- Make `UserPromptSubmit` clear a previously stale or waiting attention state
  promptly.
- Promote an individual live session to hook-authoritative scheduling only
  after an accepted, correctly correlated Codex event.
- Preserve Peon and terminal inference when Codex hooks are not installed,
  not trusted, not enabled, or cannot be correlated to the OrkWorks session.
- Reject or ignore malformed, unknown, and stale events without allowing them
  to overwrite newer user, agent, or hook state.
- Preserve unrelated entries in `.codex/hooks.json` and keep install, probe,
  repair, and uninstall ownership-safe.

## Non-goals

- Automatically enabling Codex hooks or bypassing Codex's `/hooks` trust flow.
- Treating Codex `SessionStart` as an attention signal. It captures identity
  only; startup, resume, clear, and compaction must not become
  `waiting_for_input`.
- Adding session lifecycle semantics from `Stop`. Codex `Stop` marks the end
  of a turn, not termination of the interactive session.
- Inferring a canonical plan path from Codex patch text.
- Replacing Peon for sessions without an active deterministic signal source.
- Adding `Interrupt` or `SessionEnd` support without a separately verified
  upstream contract and product decision.

## Contract evidence

The existing harness capability design records the verified Codex event set
and normalized semantics in
`docs/superpowers/specs/2026-07-22-harness-capability-system-design.md`.
The event payloads and lifecycle semantics are also covered by the current
Codex hook documentation:
<https://learn.chatgpt.com/docs/hooks>.

The contract is intentionally limited to fields needed here:

| Event | Required payload | OrkWorks effect | Attention? |
| --- | --- | --- | --- |
| `SessionStart` | string `session_id` | Report native session ID and hook fingerprint | No |
| `UserPromptSubmit` | string `session_id`, event timestamp | Report native session ID; report `working`; clear prior attention | Yes |
| `PermissionRequest` | string `session_id`, event timestamp | Report native session ID; report `waiting_for_input` | Yes |
| `Stop` | string `session_id`, event timestamp | Report native session ID; report `idle` | Yes |

Unknown event names, missing or non-string session IDs, malformed JSON, and
invalid timestamps are successful no-ops at the reporter boundary and do not
mutate session metadata.

## Design

### 1. Codex hook installation

The Codex integration handler will own one OrkWorks command fragment in each
of the four event arrays under the top-level `hooks` object. Each generated
command will identify its event explicitly, so status mapping does not depend
on a mutable payload field or on shell parsing of arbitrary Codex input.

The generated fragments will:

- invoke the stable OrkWorks reporter path;
- carry the Codex marker and event name;
- carry the current hook fingerprint;
- use the existing command hook shape (`type: "command"`); and
- omit a matcher so all eligible sources for that event are covered.

`probe` reports `Installed` only when the complete owned set is present and
exact. A missing, changed, duplicated, or narrowed owned fragment is
`Drifted` or `Ambiguous` according to the existing ownership rules. The
handler must continue preserving unrelated user hooks, including unrelated
entries in the same event arrays.

The fingerprint must identify the installed OrkWorks Codex hook bundle, not
just the `SessionStart` entry. The implementation must keep the existing
observation-file behavior: Settings can say Codex is active only when a
reported fingerprint matches the currently installed definition. A report
from a stale command must not promote a session to deterministic authority.

Uninstall removes only the complete, exact OrkWorks-owned fragments. It must
not delete a user-edited fragment silently and must retain unrelated hooks.

### 2. Event-aware reporter

The POSIX and PowerShell reporters will accept an explicit Codex event
argument. For the Codex marker they will use this mapping:

| Reporter event | Attention payload | Harness-session payload |
| --- | --- | --- |
| `SessionStart` | none | `session_id`, source `codex_hook`, matching fingerprint |
| `UserPromptSubmit` | `status: "working"` | `session_id`, source, matching fingerprint |
| `PermissionRequest` | `status: "waiting_for_input"` | `session_id`, source, matching fingerprint |
| `Stop` | `status: "idle"` | `session_id`, source, matching fingerprint |

The reporter remains best-effort: missing OrkWorks environment variables,
invalid JSON, an empty session ID, or an unavailable sidecar must not block or
alter Codex. It should avoid making a harness-session request when no native
session ID is present, and it should not post generic attention for
`SessionStart`.

Existing Claude, Gemini, Copilot, and Aider behavior must remain unchanged.

### 3. Attention protocol and validation

The attention request needs enough provenance for the sidecar to distinguish a
deterministic Codex event from a generic agent report. The protocol will add
optional event/source fields, retaining backward compatibility for existing
clients. Codex attention reports identify:

- source `codex_hook`;
- one of the four verified event names; and
- the hook fingerprint observed by the reporter.

The sidecar validates that the event is legal for the session's harness and
that the fingerprint matches the current Codex installation observation. A
report that cannot be correlated to the live OrkWorks session, has the wrong
harness, or has a stale/unknown fingerprint is ignored or rejected without a
metadata mutation. Existing non-Codex attention callers keep the current
validation and source behavior.

The normalized write uses source `agent` only for the existing generic agent
path. Codex hook writes use source `codex_hook`, confidence `1.0`, and the
existing metadata priority rules. The precise public response remains the
current best-effort success behavior for accepted or ignored hook reports;
malformed requests remain a client error where the current route already
does so.

### 4. Dynamic hook authority

The launch-time capability registry may advertise that the Codex binding has a
deterministic attention contract, but that advertisement must not by itself
set every Codex session to `active_work_hook`.

For each live session, the sidecar keeps hook authority inactive until it
accepts a valid Codex event from the current installed/trusted hook bundle.
Acceptance of `UserPromptSubmit`, `PermissionRequest`, or `Stop` promotes that
session's runtime authority. `SessionStart` captures identity but does not
promote attention authority by itself.

The launch resolver must therefore stop deriving the initial
`active_work_hook` value solely from `CapabilityName::Attention`. It needs a
binding-aware activation policy (or equivalent explicit capability metadata)
that preserves the existing behavior for integrations whose authority is
known at launch while starting Codex sessions with authority disabled. This
prevents the new static Codex capability from disabling fallback before any
trusted Codex event has actually arrived.

After promotion:

- hook `UserPromptSubmit` may write `working` and clears pending/idle
  attention;
- hook `PermissionRequest` may write `waiting_for_input`, while `Stop` writes
  `idle` because it marks a completed turn rather than an explicit user
  request;
- Peon must not downgrade the session while deterministic hook authority is
  active; and
- terminal input/output inference must not race a newer accepted hook event
  into a lower-confidence state.

If no qualifying hook event is accepted, `active_work_hook` remains false and
the current terminal/Peon fallback stays in force. This is required for
manual Codex launches, untrusted hook files, stale installations, and users
who have not approved the Codex hooks.

The transition to hook authority and the attention metadata merge must be one
ordered operation for an event. A rejected event must not partially promote a
session.

### 5. Ordering and stale events

Codex hook timestamps are compared with the existing accepted-input and last
hook-attention timestamps. A late `Stop` from turn N must not overwrite a
newer `UserPromptSubmit` from turn N+1. The accepted event ordering is:

1. Validate JSON shape, event, session correlation, harness, fingerprint, and
   timestamp.
2. Reject an event at or before the newer accepted input/hook timestamp.
3. Atomically merge the normalized status and promote authority when the event
   is eligible.
4. Update the last accepted hook timestamp only after a successful metadata
   merge.

The existing user-over-agent priority remains authoritative. Hook events do
not erase newer user-authored attention or mutate a terminal session.

### 6. Capability and integration metadata

The Codex binding should advertise `NativeSessionId` and `Attention` once its
contract fixtures and handler tests pass. It should not advertise `Lifecycle`
for this change. The static capability is a description of supported
integration behavior; the launch resolver must treat Codex's capability as
requiring observed activation, as described above.

The existing Codex adapter note remains valid and must be expanded with the
four-event contract:

- harness ID: `codex`;
- adapter/integration ID: `codex` workspace hooks plus the shared reporter;
- launch: `codex`;
- exact resume: `codex resume {harnessSessionId}`;
- latest fallback: `codex resume --last` for the latest repository strategy;
- native session ID: hook JSON `session_id`, source `codex_hook`, confidence
  `0.98` for the reporter's harness-session capture;
- approval: explicit OrkWorks integration install/repair and Codex `/hooks`
  trust approval;
- label reset commands: `/clear` and `/new`, as declared in the builtin
  harness resource;
- test surfaces: Codex integration fixtures, reporter scripts, capability
  registry, attention HTTP/application tests, and Peon/runtime authority
  tests.

## Testing strategy

Tests will be written before implementation changes and will cover:

1. Codex integration merge/probe/remove for all four event arrays, including
   preservation of unrelated hooks, drift, duplicate ownership, matcher
   narrowing, and fingerprint changes.
2. POSIX and PowerShell reporter mapping for all four events, with malformed
   payloads, missing environment, empty IDs, and sidecar-unavailable cases.
3. Capability registry evidence: Codex has `NativeSessionId` and `Attention`,
   but not `Lifecycle`.
4. HTTP/application validation of event/source/fingerprint correlation and
   no mutation on rejected reports.
5. A session accepts `UserPromptSubmit` as `working` even when it launched
   without static `active_work_hook`, then becomes hook-authoritative.
6. A session with no accepted Codex event continues to accept the existing
   Peon/terminal fallback.
7. `PermissionRequest` produces waiting state, `Stop` produces idle state, and
   `SessionStart` does not write attention.
8. A stale `Stop` cannot overwrite a newer prompt submission, and a later
   prompt clears waiting/idle state.
9. Unrelated non-Codex attention behavior and existing Codex session-ID
   capture remain unchanged.

## Files expected to change during implementation

- `crates/orkworksd/src/harness/integrations/codex.rs`
- `crates/orkworksd/src/harness/registry.rs`
- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/session_application.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs` and/or the shared runtime
  authority seam, as required by tests
- `crates/orkworksd/scripts/report-harness-event.sh`
- `crates/orkworksd/scripts/report-harness-event.ps1`
- relevant Rust and script fixtures/tests
- `docs/agents/harness-integration-contracts.md` and the related authoritative
  harness documentation if the final protocol boundary requires it

The implementation plan must resolve the exact fingerprint representation and
the least invasive runtime seam before code is edited. No automatic hook
installation or unrelated harness changes are part of this design.
