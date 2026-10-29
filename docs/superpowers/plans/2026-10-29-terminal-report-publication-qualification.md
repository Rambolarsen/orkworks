---
type: Qualification Plan
title: Terminal report producer and authority qualification
description: Investigate the real terminal producer and user-correction authority cutoff before proposing a bounded executable publication qualification.
tags: [orkworks, reports, preparation, authority, qualification]
status: draft
workflow_status: gated-on-source-and-ordering-decisions
---

# Terminal report producer and authority qualification

Outcome: replace the conditional terminal packet's captured producer/authority
inputs with attributable source and ordering evidence. The pass condition is an
owner-reviewed executable qualification handoff with explicit source creation,
consumer completeness, authority cutoff and native fault oracles. **This document
is ready for the next investigation/specification gate, not runtime execution.**

Tracking: [#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `77a7f03b3ad3fc95b7ac433b5fe8c3beb20fec86`.
Approved input: [terminal correction profile](../specs/2026-10-29-report-publication-profile-design.md#scoped-owner-acceptance--2026-10-29).
Existing evidence: [conditional row/commit packet](../../validation/report-publication-components.md).
No new cutoff, protocol/schema, numerical limit, implementation, migration or probe
is approved by this plan or its merge. #740 and separately approved executable
plans remain gates. Work here is repository inspection and written preparation.

## Investigated uncertainties

**Least certain assumption:** which event revokes a terminal user correction?
The [#742 source rule](../specs/2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections)
allows explicit user correction in an active, completed or cancelled current-epoch
run. Run completion/cancellation therefore is not, by itself, revocation of this
UI action. Child runtime/report authority remains closed. Treating ordinary stop
or terminal status as a user-correction denial would change the accepted product
contract. A correction must still pass current UI/workspace scope, epoch, lease,
source/health and exact predecessor checks.

**Likely project blind spot:** a terminal-looking row or ordinary ended session can
be mistaken for settlement. [CoordinatorStore](../../../crates/orkworksd/src/taskmaster/coordinator_store.rs)
explicitly owns durable plan data, not runtime authority.
[complete_session_ending](../../../crates/orkworksd/src/session_application.rs)
records an ordinary runtime's ending metadata/replay snapshot; it does not create
TerminalReportRunV1 or prove that orchestration grants, pending candidates and
retained dependency ownership have settled. The
[architecture](../../agents/architecture.md#proposed-ordinary-child-orchestration)
still describes orchestration as proposed. A targeted source search for the
accepted report/run/profile symbol names found no production caller in sidecar
or desktop source at this base. This is a bounded inspection, not exhaustive
proof that no differently named implementation or external writer exists.

**Outcome-changing ordering gap:** the
[authority race](../specs/2026-10-24-hierarchy-storage-contract-design.md#authority-and-commit-ordering-explicit-no-go)
remains for #742. The [accepted #744 claim-first exception](../specs/2026-10-24-hierarchy-authority-ordering-design.md#amendment-scope-and-exclusions)
explicitly excludes research-report writes. A precheck, interrupt, hidden response,
compensating deletion or replacement worker does not prove revoke-before-commit
persistence denial. No native mechanism or semantic amendment for this terminal
UI action has been accepted. Preserve that gap rather than call a row assay proof.

## Scope, ownership and inspected entry points

One coordinator owns this qualification handoff through its decision gates.
Prepare one source-traced packet with the #742 producer, #744 dependency/material
owner and #746 publication/read owner; no parallel production writer or separate
coding harness is required. Update the canonical owning contracts when a reviewed
choice changes them; keep detail in this plan until that choice exists.

| Responsibility | Inspection root / required handoff |
| --- | --- |
| Electron-authorized request | [taskmasterRequest](../../../apps/desktop/electron/taskmasterSettings.ts) and its main-process callers are precedents for fixed-path transport and selected-scope revalidation. They do not implement correctResearchReport or prove authority through SQL commit |
| Sidecar UI authorization | [readiness handlers](../../../crates/orkworksd/src/http/readiness_handlers.rs) demonstrate the existing Electron authorization helper. A generic authenticated request is not an exact report intent or commit fence |
| Run terminal producer | #742 must identify actual finish/cancel/settlement owners, row creation and every direct writer. CoordinatorStore and ordinary session ending cannot supply this proof by analogy |
| Workspace ownership | [WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs) owns a retained open advisory-lock file. A path/PID marker is not lease ownership, and existence of this type is not hierarchy worker disposition evidence |
| Durable publication and current reads | Reuse the accepted terminal profile and format-3 source/row/manifest domains, exact retained source dependencies, summary lineage and snapshot coverage. No native hierarchy storage adapter is qualified here |

Preserve the desktop electron/src boundary and ordinary session metadata contract.
This plan identifies inspection roots, not new runtime APIs or chosen module names.
An eventual implementation plan must name the actual owners after tracing all
callers and direct writes, including source-owner failure/recovery paths.

## Gate 1 — exact UI authority and ordering decision

- [ ] Trace one complete terminal user-correction intent from Electron's selected
  workspace through sidecar admission, queue, final source validation, durable
  commit, reference/snapshot publication and response commitment. Inventory the
  actual UI authority object and all scope/epoch/lease/health replacement writers;
  if a stage does not exist, record it as a missing producer seam, not a stubbed
  proof. Retain source revision and file/function citations.
- [ ] Write the per-event authority table below against that trace. Separate
  permission to persist from permission to deliver response bytes and from child
  report/launch authority. Exact accepted retry still requires current UI scope
  and rate admission before receipt lookup; a receipt restores neither authority
  nor currentness.
- [ ] At the physical-commit gap, either supply an enforceable native ordering
  mechanism with evidence obligations, or propose an explicit #742-specific
  semantic amendment for owner review. Compare the existing alternatives without
  silently selecting #744 claim-first, delaying ordinary stop, moving acceptance
  to a volatile CAS, or promising rollback from a timeout. If neither path is
  concrete, disposition Investigate and stop dependent execution.

| Event | Required decision/check |
| --- | --- |
| Run finish or cancellation | Preserve retained user-correction eligibility in the current epoch; close child/grant authority and keep terminal state terminal. This is not automatically a user-authority revocation |
| Parent/child ordinary runtime end | Prove immediate runtime authority closure independently. Do not deny an otherwise authorized explicit terminal UI correction merely because its child ended |
| UI selection change / workspace detach | Name the exact selected-workspace/UI intent cutoff and whether an already executing write may persist; do not infer it from a transport precheck or response suppression |
| Namespace rotation | Serialize against the exact old-epoch operation; old-epoch history stays read-only after activation. Name how unresolved commit disposition prevents false completion of rotation |
| Lease loss / workspace deletion / replacement sidecar | No writes through uncertain ownership, stale pending intent adoption or credential resurrection. Prove worker disposition and retained lease lifetime or remain no-go |
| Source health failure / dependency insertion or purge | Revalidate complete source-owned membership and currentness under the dependency boundary; do not conflate a valid UI token with source eligibility |

Acceptance: a single concrete cutoff/mechanism and revocation-event table is
reconciled with #742/#744/#746 and any affected ADR. **New or materially changed
authority/security/architecture behavior needs explicit owner approval before
its fixtures/adapters or executable plan depend on it.** Current authorizations
for conditional rows do not cover that change. This is the first stopping gate.

## Gate 2 — real terminal prestate and consumer proof

Begin only when Gate 1's applicable decisions are settled; discovery may proceed
read-only in parallel where independent of the selected cutoff.

- [ ] Identify the exact source command(s) that produce the accepted terminal
  run_states variant. Specify complete before/after rows and proof links for
  finish and cancellation, frozen plan/configuration scope, checked run/input
  counters, pending reporting disposition and child/grant revocation. Preserve
  nonempty blocker/history variants by rejection if unsupported; never erase them
  to enter the narrow profile. No synthetic subject-preparation proof may import
  terminal state or claim a real finish/cancel transition.
- [ ] Trace initialization/ownership of report head/receipt history and the shared
  report/preparation ledgers. Prove contiguous exact request/descriptor bindings,
  current epoch and retained source bytes. A digest or catalog entry is not prior
  acceptance. Define legacy/unknown-row behavior and recovery without automatic
  adoption, counter reset, reconstruction from latest head or silent migration.
- [ ] Inventory retained consumers from their actual owners, including the absence
  of unsupported preparation/proposal/plan/launch/learning inputs. Prove the full
  graph and index reconstruction under concurrent insertion/removal/purge, not
  just the selected R2 two-hop graph or a caller-provided list. Count all query
  visits and effects under existing limits; overflow fails closed, never truncates.
- [ ] Reconcile source health, eligible evaluation selection and owner-run tracking
  with #744/#746. Preserve original source/evaluation bytes, historical retention
  edges, subject CAS versus material revision, unavailable lineage protection and
  optional-only snapshot invalidation.

Acceptance: the terminal producer, creation/accounting and consumer/index mappings
are closed and reviewed in their canonical contracts. Any new codec/default has
scoped acceptance. Until then the existing captured flags and seed proof remain
UNQUALIFIED; this gate cannot be passed by adding more conditional success cases.
This is the second stopping gate before adapter qualification is authorized.

## Gate 3 — bounded executable qualification handoff

Only after the preceding definitions/approvals, draft a separate executable plan.
Do not run it as part of this document task. Name adapter/library versions,
OS/filesystem/mounts, private scratch stores, instrumentation, fault locations,
finite sample/repetition/call/time budgets and cleanup ownership. Tie each claimed
platform to native evidence; a mock, API branch, or one OS does not qualify another.
Retain exact commands, source revision, observed traces/disk states and limitations.
No production store, unsupported model/native probe, migration or launch is implied.

The eventual acceptance matrix must include:

| Case / evidence | Pass condition |
| --- | --- |
| UI request denied or rate-exhausted before retry | No publication/reinvalidation; saved receipt does not bypass authority or rate. Rate charge behavior is real and bounded, not a supplied boolean |
| Exact retry after a later correction | Original immutable receipt before mutable CAS; no new rows, material increment or snapshot invalidation |
| Applicable revocation immediately before/after every irreversible boundary | Outcome matches the explicitly accepted Gate 1 cutoff, including whether any post-revocation persistence is permitted. Record durable disposition separately from acknowledgement |
| Ordinary stop with deliberately stalled storage | Ordinary runtime revocation/stop does not wait for SQL/fsync or a hierarchy core lock. Record observed latency and overshoot; do not assert an unapproved hard deadline |
| Finish/cancel versus pending reporting and explicit user correction | Terminal eligibility/settlement matches the reviewed producer contract; no child bearer revival, launch or implicit reapproval |
| Edge insertion, cleanup/purge and source corruption races | Every protected consumer is included or conservatively fenced; complete membership and retention remain intact at acknowledgement/recovery |
| Required C1 versus optional O | Same source/evaluation bytes give the accepted 3→4 versus 3→3 material behavior; both relevant snapshots invalidate before a new current detail read |
| Row/lineage/quota/command/index work exhaustion | Preserve old complete state or required durable unavailable protection; reject the source if protection cannot commit. No history eviction, wrap or partial affected set |
| Crash before commit, after commit before publication, before ack, during recovery | Complete old/new state or fenced reconciliation; exact retry remains idempotent, no restored volatile credentials and no duplicate lineage |
| Permission, space, flush, lease and worker-retirement failure | No weakened durability, replacement-worker guessing, discarded journal or writes through lease loss; report unsupported outcomes |
| Sidecar/workspace replacement and stale detail capture | New lifetime scope plus recovered durable lineage; stale-reference/unavailable as appropriate, no implicit reselection or viewed marking |

Preserve the existing 16 MiB/4,096 changed-row and 100,000 visited-row boundaries,
shared source/summary/lineage limits and reviewed queue/deadline rules. Native
resource/time evidence must measure their enforcement; the conditional JS row
counter and component byte widths do not establish physical query work or latency.
Distinguish deterministic logical fault tests, process-kill evidence and actual
power-loss guarantees. No unsupported failure model is called passed.

Acceptance: concrete source/adapter work and its native test plan have explicit
review/approval, relevant #740 capability eligibility is current, and the runtime
plan's prerequisites are satisfied. Only then implement through TDD, run the
surface checks and repository verifier, and complete the applicable current-diff
/code-review gate. Required CI and review gates do not turn missing native evidence
into support. This plan does not grant that future execution approval.

## Compatibility, recovery and review

Reuse the accepted profile unchanged. Broader active states/consumers are excluded;
adding them is a separate reviewed design. Unknown/corrupt state is unavailable,
not a normalization or import request. A source change rejected before publication
preserves prior state. If commit outcome is unknown, fence and reconcile from
retained proof; do not retry as a new mutation or remove its storage evidence.
Rollback of a future native/schema implementation is **Unknown** until that
implementation's recovery and downgrade plan is reviewed. This docs-only plan can
be reverted through an ordinary PR without changing source state.

| Risk dimension | Rating / evidence and treatment |
| --- | --- |
| Dependencies | 5 — #742 authority/terminal producer, #744 source/retention and #746 storage/read owners have unresolved seams; Gates 1–2 settle them before execution |
| Blast radius | 5 — wrong ordering affects retained evidence, workspace ownership, current reads and stop responsiveness; scope stays terminal/evaluation-only |
| State changes | 1 for this documentation task; Unknown for the later adapter/schema work until its design exists |
| Reversibility | Unknown for future persistence; no rollback claim from a source-code revert |
| Uncertainty | 5 — no real terminal producer or qualified #742 commit fence was demonstrated by this inspection |

No total score: the future state-change and reversibility dimensions are incomplete.
The ratings are risk treatment aids, not approval thresholds.

Local reviewing-plans disposition: **Ready for Gate 1 written investigation and
ordering decision; Investigate for executable/native qualification.** Scope and
owner seams are explicit; each gate has deliverables and an observable acceptance
condition. The review corrected the false assumption that run cancellation itself
revokes an explicit user correction, and separated ordinary session ending from
orchestration terminal settlement. No unchanged accepted fixture or architecture
was reopened. There are no runtime changes or executed native qualification tests
in this task. Record subsequent decisions/evidence here or in their canonical
contract, without claiming these unchecked gates complete on merge.
