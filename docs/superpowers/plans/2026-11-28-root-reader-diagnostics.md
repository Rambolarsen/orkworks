---
type: Implementation Plan
title: Isolated root reader diagnostics on owned fixtures
description: Delivers accepted v3 sparse checkpoints with disabled live entry and bounded synthetic verification.
tags: [orkworks, qualification, diagnostics, implementation]
status: draft
workflow_status: ready
---

# Isolated root reader diagnostics on owned fixtures

Outcome: implement [#2274](https://github.com/Rambolarsen/orkworks/issues/2274)
under the [accepted contract](../specs/2026-11-28-within-root-reader-diagnostics-design.md).
Success means an isolated strict v3 reader/controller/codec verified on owned
fixtures, with no reachable live CLI or new qualification workflow. Root owns
all edits and executes inline using TDD. [ADR 0123](../../adr/0123-synthetic-reader-diagnostics.md)
records the accepted extension before code. Inspection base 6864b71c; all four
frozen repository source pins match. No installed-input allowance exists.

## Boundaries and decisions

New files only under scripts/qualification/pair-root-diagnostic-windows-build:
.mjs controller, -protocol.mjs, .ps1 reader, .inputs.json exact policy, -sdk.py
wrapper, .test.mjs tests and -fixtures.ps1 owned native fixtures. Existing-file
exceptions: explicit new-file LF entries, one five-minute pr-ci SDK fixture
step, this acceptance/ADR/plan and canonical evidence/index updates. Legacy
qualification/synthetic files are unchanged and sealed read-only imports never
call their mains/dispatch paths. No new dependency, interpreter or live workflow.

The codec owns numeric phase/sample validation and bounded retention; controller
owns complete output/source admission before exactly-once settlement and
receipt-only encoding. Reuse the guarded old WorkBudget/CommandRunner and pure
manifest/tool validators; expose only guarded components, with live CLI denied.
Copy the complete PowerShell custody functions into the new namespace and add
samples only at accepted completion sites. Fixture failure injection stays in
test code; it cannot become a production behavior knob.

All sizes/limits, request/output domains, sample conditions, phase meanings,
accounting and future approval boundaries are exactly those in the contract.
Model large milestones with literal fakes; do not create more than 2,048 files.
Keep fixture vector/stream/work/process accounting cumulative, not per-case.

## Delivery and checks

### Task 1: Closed v3 wire and accounting

- [ ] Write literal successful/full/empty/nested and refused prefix vectors,
  legacy/domain/framing/order/counter/late/size controls and reserve/settlement
  cases before implementing the codec. Watch assertions fail for missing v3
  behavior, then implement the minimal new codec.
- [ ] Preserve 21 structural meanings, contiguous samples and counter-proven
  omissions; validate final sample cardinality from complete MSVC rows. Bound
  32 records/128-byte LF lines, 127-byte tail, 144-byte immutable arrival rows,
  2,048-byte aggregate non-row metadata, 18 KiB traces and 64 KiB result.
- [ ] Verify dedicated Node tests (at most 64 pure vectors/512 KiB injected,
  8 KiB/vector), strict source/output/terminal settlement and decreasing grants.

### Task 2: Reader, admission and disabled live entry

- [ ] Add failing tests for v3 complete output/sample admission and rejection
  before creating scratch, installed probes or children. Then implement new
  controller/library and SDK wrapper with all live CLI entries denied.
- [ ] Copy exact policy and full reader custody checks, changing own namespace,
  envelope/request domain, numeric emitter and accepted sample sites only.
  Seal every transitive own/imported source/policy within 1 MiB per pass.
- [ ] Test held source/output admission with real owned files and transport
  fakes, full-grant failure and bounded receipt-only prefixes. Preserve complete
  manifest/PE/helper/archive validation and all flags false/false/zero.

### Task 3: Bounded owned Windows evidence and integration

- [ ] Write native fixture assertions against the new reader before it exists;
  retain the existing literal hash/lease/archive/helper/path refusal coverage,
  then exercise empty, nested, first-file/milestone and controlled stopped
  samples. Native work uses at most three 30-second PowerShell workers plus
  one final zero-hash Node timeout child and one termination.
- [ ] Enforce the contract's one cumulative fixture pool: 120 seconds active
  plus 120 seconds shared drain/publication, 64 MiB hash, 1 MiB streams,
  4 MiB inputs/8 MiB scratch, 18 KiB traces/64 KiB result. Native cases skip
  explicitly off Windows. Add only the approved CI step and LF entries.
- [ ] Run dedicated tests, then bash scripts/verify-repo.sh with pinned Node
  and installed desktop/docs dependencies. Review the current diff and wait
  for applicable CI including owned Windows evidence before merge. Record
  actual evidence/limits; no native capacity or historical-cause claim.
- [ ] Tie off a matching active Taskmaster recommendation if present; keep
  #1176/gates open, update #2274, run doc/worktree close-out checks and clean
  only the owned merged branch/worktree through finish-pr.sh.

## Review focus and pre-execution disposition

Check interleaved structural/sample order; false completion inferred from
silence; source/output failure refunding a grant; late/overflow data after a
latched failure; and accidental live entry through a script or imported main.
Each has a dedicated Task 1/2 refusal case, with native sample-site cases in
Task 3. No independent subsystem or migration is included.

Review: Ready. Dependencies 1 (guarded local imports), blast radius 1 (new
profile, CLI disabled), state changes 2 (additive wire domain only),
reversibility 1 (remove new files/config step), uncertainty 3 (native behavior
must be observed in owned Windows CI); total 8. No runtime state or rollback
migration. One fixed final read-only review round will check correctness and
coverage; no writer delegation or additional coding harness.
