---
type: Design
title: Session startup diagnostics
description: Proposed bounded failure details for admitted session launches and resumes, with safe desktop and HTTP presentation.
tags: [sessions, startup, diagnostics, desktop, api]
status: draft
workflow_status: proposed
---

# Session startup diagnostics

Proposed contract for [#689](https://github.com/Rambolarsen/orkworks/issues/689).
Owner approval is pending. This document records a proposal, not implemented
behavior or a published-installer claim. It requires one approval of this
contract before dependent runtime, metadata, API or desktop implementation.

## Problem and boundary

An admitted new launch returns `creating` before its detached startup task runs.
If that task fails, the sidecar logs the reason and publishes `error`; the desktop
shows a generic toast. Resume awaits startup, but converts its failure into a
generic application error and an empty HTTP 500. Users cannot distinguish a
failed Codex compatibility check from an ordinary process-spawn failure.

Keep asynchronous new-session admission, exact resume identity, existing resume
strategy selection, process ownership, cancellation and lifecycle vocabulary.
Add one bounded diagnostic to the current startup attempt. It grants no retry,
resume, process adoption, input, metadata-reporting or native integration authority.
Do not change native qualification gates, storage activation or resource limits.
No hosted model calls or live coding-tool probes are needed for this change.

This proposal extends the [MVP lifecycle scope](../../../specs/orkworks-mvp.md#session-lifetime).
Implementation must update that spec, the domain/API references and user-facing
session troubleshooting documentation to describe the approved contract.

## Record and safe vocabulary

Add optional `startupFailure` to persisted `SessionMetadata` and every complete
`SessionInfo` projection, including REST, workspace update snapshots and changed
rows, remembered sessions and reloads. Absence means no recorded failure for the
current attempt; it does not prove successful startup.

```json
{
  "startupFailure": {
    "code": "codex_help_timeout",
    "operation": "resume",
    "occurredAt": "2026-11-07T12:00:00.000Z"
  }
}
```

`operation` is exactly `launch` or `resume`; all admitted resume strategies use
`resume`, including exact resume. `occurredAt` is sidecar-generated UTC RFC 3339
with milliseconds. There is one record, no history or output attachment. The
serialized object is at most 256 UTF-8 bytes. Missing fields, unknown codes,
invalid timestamps/operations or oversized objects are treated as absent on
persisted reads without rejecting the surrounding session. No free-text field
is accepted or retained inside this object.

The following fixed English catalog is the first supported vocabulary. Codes
are stable API values; messages are display text and may later be localized.
The same catalog is tested across server and desktop.

| Code | Fixed message and next step |
| --- | --- |
| `codex_help_failed` | Could not check Codex session isolation support. Check the configured executable and retry. |
| `codex_help_timeout` | The Codex session isolation check timed out. Check that the configured executable responds to --help, then retry. |
| `codex_help_oversized` | The Codex session isolation check returned too much output. Check the configured executable and retry. |
| `pty_open_failed` | Could not open a terminal for this session. Check available system resources and retry. |
| `process_spawn_failed` | Could not start the coding tool. Check its configured executable and working directory, then retry. |
| `terminal_io_failed` | Could not connect the session terminal. Check available system resources and retry. |
| `instruction_setup_failed` | Could not prepare session instructions. Check workspace access and retry. |
| `native_startup_failed` | Could not prepare the coding-tool integration. Check its integration status in Settings and retry. |
| `startup_internal_failed` | Session startup failed. Check sidecar diagnostics before retrying. |

Classification happens at typed error origins, never by matching arbitrary
error strings. Codex help resolution/spawn/read/wait/nonzero-exit errors map to
`codex_help_failed`; its existing three-second deadline and 64 KiB per-stream
limit map to the two specific codes. PTY open, spawn and reader/writer acquisition
map to their corresponding codes. Instruction preparation and native startup
retain their existing fail-closed behavior and map to the corresponding codes.
Unexpected startup task failure, capability generation or authority preparation
failure maps to `startup_internal_failed`. Preserve existing internal error
logging; this proposal does not broaden logging or copy internal errors to UI.

Never serialize executable output, OS error strings, paths, argv, environment,
tokens, native conversation IDs or exception text into the diagnostic or its
message. Truncating/redacting arbitrary output is not the safety mechanism.
Cancellation, deletion or replacement produces no startup-failure diagnostic.
A process that starts successfully and later exits unsuccessfully remains an
ordinary runtime outcome, outside this field's scope. Startup includes setup
through successful completion of the existing runtime startup future, including
terminal I/O setup after spawn; observing `running` alone is not that boundary.

## Ownership, transitions and persistence

`SessionApplication` owns publication. Carry a typed safe failure separately
from internal diagnostic context through the startup future. Preserve the
existing captured runtime generation and check it at the application boundary
before publishing a failure or clearing one. Do not expose runtime generations
as new public authority or weaken existing cleanup guards.

1. Pre-admission validation/conflict/not-found failures retain existing responses
   and the previous record. They do not become startup failures.
2. On admission, clear the previous record in the same stored mutation that
   establishes `creating`. Cancellation of an uncommitted resume admission must
   restore the previous record with its existing rollback state. New creation
   begins without a record.
3. A current attempt's actual startup failure publishes the diagnostic with its
   `error` transition, under the existing transaction lane and lifecycle guards.
   Keep the physical `ending` transition and finalization order described in
   [transition ownership](../../agents/domain-entities.md#transition-ownership-and-observation-windows).
   Consolidate existing early error transitions where necessary so finalization
   cannot discard or race the diagnostic. Never add a late unguarded write.
4. Finalization preserves the diagnostic through `ending` and `ended`. Archive,
   restore, unrelated metadata writes and orphan reconciliation preserve it.
   A later admitted attempt clears it; Forget/retention removes it with its
   existing session record. Success cannot inherit a previous attempt's failure.
5. Concurrent deletion/replacement takes precedence. Recheck cancellation and
   generation after awaited work; a stale task cannot recreate a record, alter
   a newer attempt or publish a failure toast through a stale response.

Use the retained metadata owner's scoped transaction, revision advancement and
post-commit projection signals. Do not add an independent diagnostic file or
writer. Storage failure cannot guarantee durability: retain the current safe
failure in the owned live handle when possible, return it for the failed resume,
and log the persistence failure without claiming it survived restart. Existing
failed-finalization retry semantics remain; no automatic retry is introduced.
If no owned failure can be published because deletion/replacement won, return
HTTP 409 with an empty body for that superseded/cancelled admitted resume,
rather than this startup-failure body. Pre-admission responses remain unchanged.

The field is sidecar-written. Exclude it from authenticated agent/user patch
allowlists, Peon inference and source-priority arbitration; other scoped writes
must preserve it. Legacy direct-JSON writers are unsupported producers of this
field and should preserve it when updating records. Existing legacy JSON access
is not tamper-proof: a local writer can forge an allowed code, but cannot make
this presentation path display arbitrary text. A diagnostic never proves trusted
execution or qualifies native attention. The pending
[ordinary storage contract](../../../specs/session-storage.md#preservation-and-authority)
must preserve the field when its separately qualified producer/cutover work
lands; this change does not activate format 2 or change its authority.

## HTTP and desktop behavior

New-session creation keeps its existing admitted `creating` response. Failure
arrives in the authoritative session projections, without holding admission open.
An admitted resume startup failure retains HTTP 500 and returns JSON:

```json
{
  "error": "session_startup_failed",
  "message": "The Codex session isolation check timed out. Check that the configured executable responds to --help, then retry.",
  "startupFailure": {
    "code": "codex_help_timeout",
    "operation": "resume",
    "occurredAt": "2026-11-07T12:00:00.000Z"
  }
}
```

The entire failure body is at most 1 KiB. `message` is derived only from the fixed
catalog. Introduce a specific application error for this response; do not expose
all generic `Internal` errors. Other HTTP errors and successful resume payloads
retain their existing contracts. Older clients still see a failed request;
new clients accept an absent/unknown diagnostic and show their generic fallback.

The desktop validates the structured object and renders its local catalog text,
never server-supplied arbitrary message/response text for this new error shape.
Bound resume failure-body reads to 1 KiB before parsing. Preserve compatibility
for existing non-startup error responses under their current bounded policy.
Show “Session could not start” plus the fixed message in the selected session's
Terminal surface, alongside available replay, including remembered failed
sessions after reload. This remains a single central context. Do not create a
new panel, show a diagnostic as terminal output or steal selection/focus.

For a tracked newly created session, replace the current generic failure toast
with the catalog message once when pending-create reconciliation observes the
failure. For resume, use the failed request's typed detail once, then refresh the
authoritative row even on failure. The row remains the durable presentation;
normal polling, reconnect/reload and session selection must not replay toasts.
Retain existing workspace/admission/foreground guards so old responses cannot
change another workspace's sessions or toasts. No automatic retry button/action
is introduced; existing explicit resume controls keep all eligibility checks.

## Delivery and acceptance

After owner approval, implement one bounded slice across the existing owners:

1. Add the safe type/catalog, additive metadata and DTO projections; cover old
   records, invalid/unknown values, bounded serialization and unrelated writes.
2. Classify runtime origins and publish/clear under the current generation and
   admission rollback rules; preserve cleanup, cancellation and finalization.
3. Add the specific resume HTTP error and desktop parsing, inline presentation
   and existing-toast integration; update canonical and user documentation.

Required tests use synthetic executables/fault injection, never hosted inference:
failed/nonzero, timed-out and oversized Codex help; missing spawn executable;
PTY-open and terminal-I/O failures; unexpected startup failure; successful older
Codex help without the option; async launch returning `creating`; exact resume
preserving its requested native ID/argv; failed resume response and row detail;
successful readmission clearing the old detail; rejected/rolled-back admission
preserving it; deletion/replacement races; persistence failure; finalization and
reload; malformed/unknown responses; stale workspace responses; and toast
non-repetition. Inject secret-looking bytes and ANSI into executable errors and
assert they appear in neither persisted diagnostics, HTTP body nor desktop text.

Run affected Rust and desktop checks plus `bash scripts/verify-repo.sh`, docs
checks/build and the repository close-out hooks. Complete the required current
code review before a product-code merge. Live native qualification is outside
this slice. #689 stays open until implementation, tests and documentation land.

## Source baseline and review

Source inspection baseline: `de180b5d685b80329ce808e74147cff9817db609`.
Relevant owners: `session_application.rs` launch/resume workflows;
`runtime/session_runtime.rs` startup and `runtime/codex_launch.rs` help isolation;
`metadata.rs`, `session_types.rs`, `http/session_handlers.rs`;
`apps/desktop/src/api.ts` and `workspaceSessionController.ts`.

Author choices are the fixed catalog, one optional record, 256-byte/1-KiB bounds,
HTTP 500 compatibility, and inline Terminal presentation. Owner approval covers
these behavior and metadata/API choices together. A fresh read-only builder check on 2026-11-07 returned “nothing important left.”
The author separately clarified the cancellation response as empty HTTP 409;
this preserves cancellation precedence without exposing internal error strings.
Builder review does not substitute for owner approval.
