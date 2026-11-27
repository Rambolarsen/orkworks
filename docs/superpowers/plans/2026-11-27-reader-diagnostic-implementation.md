---
type: Implementation Plan
title: Synthetic reader diagnostic implementation
description: Implements the owner-accepted bounded synthetic diagnostic contract with test-first protocol and native fixture checks.
tags: [orkworks, qualification, diagnostics, implementation]
status: draft
workflow_status: implementation-complete
---

# Synthetic reader diagnostic implementation

Outcome: retain the last validated phase across an owned timeout and demonstrate
fresh/reuse equality with independent synthetic goldens. Authority is the owner's
“keep going” after #2193; [contract](../specs/2026-11-27-reader-diagnostic-contract-design.md),
[ADR 0123](../../adr/0123-synthetic-reader-diagnostics.md), tracking
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). No qualification run.

## Scope and decisions

Root is the sole implementation writer in the owned reader-diagnostic worktree.
New pair-reader-diagnostic Node modules own strict decoding, shared budget,
held-file witnesses, fixture generation, process drain and result publication.
The new PowerShell worker owns native identity/archive verification and scratch
leasing; its test companion runs guard cases in the one guards invocation.
Only one existing workflow step changes, as specified in the accepted contract.
Use Node standard libraries and in-process Add-Type; no package dependencies.

## Investigation and review

Least confidence: Windows handle identities and TAR streaming. Inspected the
spent replacement reader's Read-SealedFile, Get-HandleInfo and Assert-CrateArchives
without invoking it. Preserve before/after/open-name identity comparisons,
ancestor reparse refusal, archive digest before inflation, membership/helper and
extra-file checks. New code has its own names and bounds; no spent main import.
The project could confuse an expected refusal with source success: keep
reader_complete distinct from tests_passed and all qualification flags false.
The fixed PowerShell path is unavailable on macOS; native execution must be
explicitly skipped locally and verified in the one normal Windows fixture CI step.

Plan review: Ready. Scope is additive/reversible, no runtime callers, no migration.
Dependencies 1 (standard runtimes); blast radius 2 (one CI step); state changes 1
(owned temporary files); reversibility 1 (remove additions/step); uncertainty 3
(native behavior needs Windows CI). Total 8. Unexpected native failure stops later
children, preserves full unknown grants and requires drain before cleanup. Failed
CI cannot be retried by silently starting an extra qualification job. Parent and
worker enforce the same finite suite envelope; no lowered-case pool resets.

## Deliverables and checks

- [x] Strict parser and ledger in pair-reader-diagnostic-protocol.mjs; pure tests
  first in pair-reader-diagnostic.test.mjs. Test canonical framing/chunking,
  phase order, every cap, immutable prefixes, EOF/close settlement, expected
  transport failures and shared model debits. At most 64 vectors/512 KiB injected
  in the shared 1 MiB stream pool; run directly, no test-runner subprocess.
- [x] Owned fixture/controller in pair-reader-diagnostic.mjs; native worker and
  guard tests in pair-reader-diagnostic.ps1 / pair-reader-diagnostic.test.ps1.
  Test-first assertions compare complete sorted manifests and IDs, archive
  members, helper dispositions, independent digests, allocation counts and h.
  Three native children maximum, same positive identities, guards separate;
  one final zero-hash Node timeout child. All source/control, file, scratch,
  output, stream, hash and clock limits remain those in the contract.
- [x] Add the sole bounded fixture-CI step and synthetic validation record/index.
  Run direct suite locally (native explicit skip), Windows CI native suite,
  appropriate full repository verification, docs build and close-out checks.
  Inspect the verified diff, obtain fresh-context branch review under the local
  review routing policy, then current-head hosted review and required CI. PR
  lifecycle ends at merge or an explicit bounded handoff; no live dispatch.

## Execution ledger

Preparation complete before code: scoped acceptance, ADR and plan review above.
Record task outcomes and any material implementation rulings here.

Review routing fixed before dispatch: maximum two read-only Luna verifiers, one
round, root sole writer. Correctness verifier checks protocol/native/process
semantics against the contract; completeness verifier checks missing acceptance
coverage and authority/budgets. No reviewer runs children or writes files. Root
vets cited findings and records disposition; no second subagent round.

Task 1: complete — 46 bounded pure vectors were written first; observed missing
ledger assertion, implemented parser/accounting, then observed passing suite.
Task 2: in progress — end-to-end assertion observed missing controller, then
controller implemented. Local direct suite now exercises one real owned Node
timeout, retains one immutable prefix, drains and removes scratch; native skipped.
Ruling: canonicalize the runtime temp directory before exclusive creation on
macOS, whose /var alias is a symlink; below it every input still rejects symlinks.
Native test assertions precede Windows execution; no local native claim.

Review disposition: both read-only reviews completed. Root verified/fixed missing
volume/directory identities, premature reservation settlement, incomplete guard
summary matching and allocation arithmetic. Added failed-output regression,
exact 32 dispositions, 4,123 reference allocations, modeled boundary checks,
real event-observer fake transport cases and finalization failure checks.
Sixty-four vectors pass locally; native claims remain pending Windows CI. No second
subagent review round was started. Two local setup failures were diagnosed:
temporary-root alias was canonicalized; Cargo was restored to verification PATH.

Implementation rulings: the non-hashed bounded request read is excluded from h,
so reference/reuse mode spelling cannot create a false payload-work difference.
Catalog/TOML/data work remains charged. Guard summary records the in-process
PowerShell version to satisfy runtime provenance without another child. The
separate guard-directory witness permits only its expected timestamp change
from creating/removing the own reparse vector; its identity and full positive
input witnesses remain mandatory. doc-check's README nudge needs no edit: this
additive test-only ADR changes no product architecture or installer claim.

Source settlement ruling: verify own source/control seals after each native
close while its reservation is still held, including cooperative errors; every
repeated read remains in the existing pool. Always perform the final source
seal after the last drained child on failure paths too. An unverified source
report keeps the entire grant unknown. Its regression failed with 9 versus
1,024 charged bytes before the correction, then the 64-vector suite passed.
Documentation metadata, index validation, 2,583 documentation tests and site
build passed. Existing chunk warnings concern unchanged bundling. Full
verify-repo is still running before publication; native results await normal CI.

Windows CI at 8c255d64 failed before any start record (empty child streams),
charged the full unknown grant, stopped later children and removed drained
scratch. Source inspection found a one-argument Span overload call to ReadExactly; it
was changed to the explicit byte-array/offset/count overload, without claiming
that it was the observed failure cause. Fixed
numeric pre-start stage exit statuses now distinguish request-open/read/decode/
binding and native setup without raw exception text. Native failure status is
reported as failed, never mislabeled as a non-Windows skip. Verification of this
correction remains pending the new-head normal CI run. API overload reference:
[Stream.ReadExactly](https://learn.microsoft.com/en-us/dotnet/api/system.io.stream.readexactly?view=net-10.0).
Substantial review cycle 1/3 will request one fresh Codex review for the fix head;
Copilot remains unavailable under the already observed quota.

Current-head Codex review also found that the embedded registry/src slash could
produce a dot root row on Windows. Root() now normalizes with GetFullPath before
comparison; existing independent directory assertions cover the empty root row.
CodeQL identified a single-occurrence newline replacement in a malformed-input
fixture. Replace all LF bytes to express the intended full CRLF malformed record;
the existing encoding-refusal vector remains the behavior check. Both inline
items will receive explicit dispositions on the fix commit.

Final pure vector (64/64) reproduced a deadline crossing between spawn and
observer setup: throwing before event ownership could skip drain. The observer
now installs handlers and latches active-timeout, then drains the owned child.
The regression failed before this correction. No new process/vector allowance
was added; the existing last pure slot is consumed.

At f843728f, current-head hosted review completed with no findings, but Windows
still failed before start with fixed exit 65: the owned-directory binding,
before request-read code. Pass PowerShell's documented WorkingDirectory using
only the already-owned cwd; normalize TEMP/TMP path comparisons with the same
Windows ordinal-insensitive semantics as native final-path checks. Each failed
binding now has its own fixed status. No path/root/profile request field or
live input switch is introduced. Cycle 2/3 will request one new current-head
Codex review after native CI provides a meaningful result; no extra subagents.

Implementation verification complete: native fixture passed at code head
690ee1999b43ee8fb86f9f59965945f4276103d8, followed by a completed no-findings
Codex review at 17:25:49Z. Full local verify-repo exited zero, including all
Rust/Desktop/docs checks. See the canonical synthetic validation record for
exact measurements and the independent nonrequired #2209 Windows reporter
failure. This final evidence update changes documentation only; source code
matches the reviewed/native-tested head. PR #2203 still follows its bounded
required-CI/review/merge lifecycle; the implementation checkboxes describe the
deliverables and completed local/native checks, not an assertion that an open
PR has already merged. No qualification allowance was consumed or granted.
