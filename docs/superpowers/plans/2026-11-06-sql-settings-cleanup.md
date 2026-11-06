---
type: Implementation Plan
title: Captured SQL workspace settings cleanup
description: Clear an exact deleted session selection through the original workspace-memory owner before recording progress.
tags: [orkworks, sqlite, sessions, storage]
status: stable
workflow_status: implementation-complete
---

# Captured SQL workspace settings cleanup

Continue [#1243](https://github.com/Rambolarsen/orkworks/issues/1243) after #1461,
within the accepted [storage design](../specs/2026-11-01-sqlite-session-storage-design.md#deletion-and-retention).

## Outcome and decisions

Add only `CapturedDeletion::clear_settings`, progress bit `0x08`, complete=false.
Keep the exact tombstone/job and existing close-first/runtime/lease/reader fences.
No recommendation cleanup, final row removal, scheduler, production selector or
live migration. Workspace memory retains MetadataStore ownership and JSON schema.

- Capture its shared memory owner without I/O under application guards. Bind a
  no-follow original workspace directory FD to the retained lease on the existing
  bounded blocking capture; independent binding errors remain file outcomes.
- Acquire the existing nonblocking edit permit only in admitted blocking work,
  outside application guards, and retain it through definitive SQL progress and
  detached-owner checks. Legacy writers and same-workspace reopen share the fence.
- Read latest `workspace.json` relative to that directory, no-follow/nonblocking,
  regular single-link file only; bound input to 1 MiB. Strict duplicate/trailing/
  malformed/known-field validation errors preserve the file. The private cleanup
  rejects oversized files for explicit repair; it never truncates legacy content.
- Clear only matching lastActiveSessionId and lastActiveAt; preserve all other
  top-level values, including unknown numeric/string/object values losslessly.
  Enable serde_json's existing raw_value feature and reuse strict validation;
  no new dependency or data format. A nonmatching or absent file is unchanged.
- For a match, write a private exclusive random temporary file relative to the
  original directory, synchronize it, verify the original file identity, rename
  relative to that same directory and synchronize the directory before progress.
  Clean only this attempt's temporary file on failure; never remove a preexisting
  obstruction. Parent path replacement cannot redirect work. Before rename,
  replacement of the exact target fails closed rather than overwriting it.
- Retry after a prior rename must sync the latest already-cleared file and parent
  again. Absence must sync the original parent under the same permit. File errors
  skip progress; SQL errors return partial outcomes without claiming rollback.
  Native file facade is source-qualified by Unix fixtures; other platforms fail
  Unsupported before effects. No new publication lock protocol is added.

## Delivery and checks

- [x] TDD real matching selection clear, preserved unrelated/unknown values,
  unchanged nonmatching/absent files, malformed/oversized/symlink/obstruction
  failure, original parent/lease/target identity and every sync boundary/retry.
- [x] Integrate the existing retained runner and record only bit0x08. Prove exact
  creation/closed ownership, existing progress bits, reopen durability, busy
  memory owner, permit retention through SQL failure/detachment, pre-admission
  cancellation/expiry and independent acknowledged input on one Tokio worker.
- [x] Run scoped suites, full repository verifier and current-diff medium review.
- [ ] Record PR CI and owned cleanup on the PR and parent issue.

## Plan review

Ready under unchanged accepted storage/metadata authority. Inspection confirmed
#1461 covers live producers, retention and same-workspace normalization; a new
lock is unnecessary. Legacy write_atomic lacks fsync and original-directory
anchoring, so cleanup uses a private narrow facade rather than claiming legacy
replacement is durable. Raw JSON values avoid converting unknown large integers
through f64 during replacement; strict first-pass validation rejects ambiguity.
Dependencies3(existing memory/file/SQL owners), blast radius2(private stage),
state changes2(controlled fixture edits only), reversibility1(production JSON
unchanged), uncertainty2(sync/race/failure fixtures), total10. Physical power-loss,
Windows activation and live latency remain separate qualification gates.

Final local evidence: 15 focused tests passed. A staged-entry substitution
regression failed before the added inode check and passed after it; both
independent medium reviews then cleared the source. Full repository verification
passed: 3,079 Rust unit tests (18 existing ignored), seven integration, 31 reporter,
one Electron ABI proof, desktop type-check/build and 1,694 tests (three existing
skipped), docs 1,202 checks/build and diff/doc/worktree checks. Source-only unused
API warnings are intentional pending production integration. Public-doc reminders
need no installer claim change; the serde_json raw_value feature adds no new
package or agent-policy rule, so the dependency reminder needs no AGENTS.md edit.
