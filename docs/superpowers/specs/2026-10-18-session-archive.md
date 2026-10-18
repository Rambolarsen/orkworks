---
type: Design
title: Active, Closed and Archived sessions
description: Proposed session archive state and close-first context-menu actions for issue 439.
tags: [orkworks, sessions, archive, lifecycle]
status: draft
---

# Active, Closed and Archived sessions

Status: proposed for owner review. Issue: [#439](https://github.com/Rambolarsen/orkworks/issues/439).
This document does not authorize runtime implementation until the concrete design is approved.

## User outcome

Sessions have three user-facing states: **Active**, **Closed**, **Archived**.
Archive is an explicit durable state, not a synonym for a successful result,
acknowledgement, deletion, or a hidden-list flag. The selected workspace owns
this state; native coding-tool archives are not changed by this increment.

The owner's latest direction permits Archive and Delete on Active sessions,
but **only through Close first**. Actual permanent deletion is allowed only
once the session is Closed or Archived. If Close fails, times out, or cannot
establish completed cleanup, Archive/Delete does not run.

## State and action contract

| Current state | Context-menu actions | Successful destination |
| --- | --- | --- |
| Active | Close; Archive; Delete | Closed; Closed then Archived; Closed then deleted |
| Closed | Resume when supported; Archive; Delete | Active; Archived; deleted |
| Archived | Restore to Closed; Delete | Closed; deleted |

Close includes stopping a live session through existing owned runtime cleanup;
creating/stopping/finalizing sessions remain Active for action eligibility until
that cleanup and the persisted ending transition are complete. A status string
such as killed alone is not sufficient proof. An absent runtime after restart
uses existing orphan reconciliation, never an inferred live-process adoption.
Restore changes state only; it never starts/resumes a tool. Resume requires
Closed state and the existing tool/resume/ownership checks. Archive preserves
metadata, task history, replay, plan reference and native resume identity.
Archived history and Review remain readable; independent review requests stay
live-session-only. There is no Mark handled action or work acceptance here.

Delete confirmation is shown **before** closing anything and explains that an
Active session will be closed and its OrkWorks record/history permanently
removed. It does not delete the underlying tool's native saved session or Git
worktree. Cancelling the confirmation has no effects. Archive needs no extra
confirmation, but its Active menu label/help states that it closes the session.
Repeated commands never resume or close a replacement runtime.

## Ownership and persistence

Keep the detailed process lifecycle vocabulary unchanged. Add the public
`sessionState` enum active/closed/archived in the session projection. Active and
Closed follow actual runtime/finalization; Archived is the user's stored state.
Do not add archive to attention or observed-status values.

A small store owned by MetadataStore persists organization records in the
workspace metadata directory under `session-state/<id>.json`. The version-1
record binds the session ID and metadata created_at identity, a closed/archived
choice, a monotonic revision and changed-at time. Missing records are legacy
unarchived sessions, derived as Active or Closed; no bulk rewrite is required.
Use existing bounded atomic-file replacement conventions and bounded strict
record parsing. Archive/Restore do not rewrite producer-owned session JSON, so
late external metadata snapshot writers cannot silently erase the user's choice.
A changed creation identity is not allowed to inherit another record's archive.

Read and mutation failures remain visible. Do not silently treat malformed or
unreadable organization state as Closed or delete its history. Affected rows
report state unavailable (null enum plus diagnostic), disable state-changing
commands, and are protected from automatic deletion; other rows remain usable.
No automatic corruption reset is part of this feature. Archive/Restore failures
leave the last durable state intact. The list is not optimistically moved
before the server acknowledges success.

SessionApplication owns state commands. It captures the workspace/store identity
and exact current runtime generation, checks the supplied row action token,
reserves that session's operation and serializes with resume/forget/retention.
It releases synchronous locks while awaiting runtime cleanup, then revalidates
all captured identities before any durable mutation. Never hold workspace or
sessions mutexes across awaits; follow current lock order and metadata lanes.
A workspace switch, competing resume, changed session identity, expired token or
unresolved cleanup returns a visible conflict without changing the replacement.
Ordinary producers cannot write this state through agent metadata PATCH.

The projection supplies an opaque `sessionActionToken` binding workspace store,
row/session identity, organization revision and lifecycle/runtime identity. It
is a stale-action precondition, not a permission credential. It remains stable
across unrelated output/summary refreshes. Each explicit state command carries
its intended action and token. Server checks are authoritative even if menu
availability is stale. Existing close/forget routes keep their compatibility
meaning but share the same eligibility/serialization checks; direct forget and
retention may not delete creating, running, stopping or finalizing runtimes.
The new command path may compose Close with Archive/Delete; it never treats an
accepted kill signal as completed Close. A bounded 15-second wait reports an
unresolved close without performing the second action; cleanup can continue.
A retry requires refreshed state and does not silently resume a pending action.

Manual Delete uses the existing evidence/history deletion owner, and only then
removes the organization record. A crash can leave an orphan state record, which
must not resurrect a deleted session. Missing metadata never becomes a row.
Fresh session creation checks for conflicting stale organization identity and
must not adopt it. Current failed-deletion recovery and recommendation evidence
cleanup remain binding; this feature makes no new atomic multi-file-delete claim.

## Retention

Archived sessions are excluded from automatic age/count deletion and from its
count budget. Closed sessions keep the configured existing retention behavior.
Restore makes the session eligible for that policy again, without falsifying
its last-activity timestamp; explain this near Restore and in retention help.
Archive/state commands and automatic deletion serialize and recheck final state
before mutation. Unknown state is protected. Existing default 0/0 retention
remains unchanged. No new per-session content-growth policy is added.

## Session list and context menu

Provide an accessible **State** filter with All unarchived (default), Active,
Closed and Archived. It filters the existing Sessions surface, not a new tab
or central destination; All unarchived preserves the current active-plus-history
view. Within a filter retain current date grouping/sorting. Changing the filter
never selects, closes, resumes or acknowledges a session. An explicitly selected
terminal/history may stay visible even when filtered out, with its state clear
in Details; do not silently select the next row.

Each row supports right-click, the context-menu keyboard key/Shift+F10 and a
visible More actions button. Opening the menu does not select/acknowledge the
row; actions bind to that exact row/token. Use accessible menu roles, roving
focus, arrows/Home/End, Enter/Space, Escape and outside dismissal, with focus
return to the invoker or a valid Sessions fallback. Do not nest interactive
menu controls inside an invalid listbox option structure. Keep menu state in
one focused renderer component, reuse existing tokens and avoid a new library.

Replace the current Kill/Delete row affordances with this coherent menu. Closing
is labeled Close in the UI while retaining existing internal kill/terminal-outcome
semantics. Pending actions disable conflicting row actions and expose progress;
failures stay visible. Successful Archive/Restore retains the selected history
and shows the resulting state; Delete clears only the deleted selection and
returns focus without auto-selecting another session. Workspace changes close
menus and fence pending presentation updates.

Archived sessions never appear as current attention/action obligations. Current
Actions already excludes dead sessions; keep it tied to runtime authority and
add explicit state checks where necessary. No recommendation history is erased
by archiving. Resume and Close transitions remain under existing orchestration
ownership/capacity rules; archive state grants no plan or result authority.

## Compatibility and recovery

The added organization store and optional/additive API fields do not rewrite old
metadata or tool sessions. New clients fail visibly if the state command is
unsupported; they do not turn Archive into Delete. An old client will still see
archived rows because it lacks filtering; it must not be advertised as archive
aware. Binary downgrade preserves archive files on disk but cannot promise the
old binary honors their retention protection: document that limit, and require
export/backup before a destructive downgrade with retention enabled. Ordinary
code rollback is safe only while retaining archive-aware retention guards.

## Verification and acceptance

- Backend: legacy read, durable archive/restart/restore, stale token and identity,
  malformed state, atomic write failure and archived resume rejection.
- Lifecycle: Active Close then Archive/Delete, finalizer delay, failed close,
  timeout, repeated operation, resume race, workspace replacement and no deletion
  while cleanup is unresolved. Existing runtime/native gates remain unchanged.
- Deletion/retention: only Closed/Archived deletion; archived age/count protection;
  restore re-eligibility; delete failure/partial cleanup without resurrection.
- Renderer: all filters, no selection on menu/filter, pointer/keyboard menu,
  confirmation before active close/delete, exact-row targeting, pending/error
  handling, selected-history behavior and stale workspace settlement.
- Real Electron flow: Active → Closed → Archived → Closed; Archive/Delete on
  Active through Close first; archived history reads and delete confirmation;
  200% zoom, narrow layout, focus and palette token use.
- Required checks: focused failing-then-passing regression tests, full
  scripts/verify-repo.sh, applicable current-diff /code-review, documentation and
  worktree close-out checks. No native tool archive synchronization is claimed.

## Alternatives considered

A boolean filter-only change cannot enforce backend delete/close rules or define
restore. Adding archived to process lifecycle would confuse retention and live
runtime ownership. A field in producer-replaced session JSON risks losing the
user choice before external producers migrate. A separate organization store
keeps one sidecar-owned fact and leaves detailed runtime lifecycle authoritative.

## Approval and implementation boundary

On owner approval, create the next available ADR recording this additive state
ownership and close-first protocol, update MVP/domain/runtime/retention contracts,
and execute the linked plan. No runtime implementation has started in this draft.
See [implementation plan](../plans/2026-10-18-session-archive.md).
