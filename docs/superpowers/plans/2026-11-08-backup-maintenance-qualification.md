---
type: Qualification Plan
title: Backup maintenance owner qualification
description: Bounded actual-owner contract and evidence closure before implementing accepted maintenance admission and settlement.
tags: [orkworks, sessions, backup, maintenance, qualification]
status: stable
workflow_status: qualification-required
---

# Backup maintenance owner qualification

Tracking: [#1541](https://github.com/Rambolarsen/orkworks/issues/1541), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Goal: turn the accepted maintenance design into **actual owner contracts and
qualified evidence**, then a bounded implementation task only when its inputs
are available. Pass means no unowned writer, fabricated drained module, unknown
physical completion or hidden recovery/resource assumption remains.

Spec: [accepted maintenance admission](../specs/2026-11-08-backup-maintenance-admission-design.md),
[accepted backup/restore](../specs/2026-11-07-session-backup-restore-contract.md),
[bundle/evidence v1](../specs/2026-11-08-backup-bundle-evidence-v1.md).
Reuse their owner approvals, the [pinned writer audit](../../validation/backup-maintenance-writer-audit.md)
and its source evidence. Do not rediscover unchanged policy, rewrite accepted
bytes or treat the audit as a registered closed Modules list.

This plan is qualification work, not permission to implement the controller,
activate SQL, stop another session or run live backup/restore. No native/SQL/model
execution or new numeric allowance is supplied. Source-only contract inspection
is the ready first gate; conditional later steps cannot execute through Unknown.

## Ownership and boundaries

The private application maintenance owner owns admission state, original-root job
obligations and explicit release. Existing module owners retain their data,
operation/lane/native/lease/resource handles and physical-effect semantics.
#1242 owns producer qualification, #1243 the store, #1248 historical semantics,
#1250 backup/restore machinery and #1244 cutover/startup integration. Consume
those owners' actual contracts and evidence; do not duplicate their implementations.

Canonical affected source: SessionApplication and its sql_writes modules;
session_store worker; metadata/session_reader/workspace_memory; session runtime,
Peon and terminal persistence; Taskmaster evaluator/store/operation; observations,
retention/GC, image storage, branch associations and authenticated ingress/helper
owners named in the audit. This list routes investigation, not a new facade or
file-splitting refactor. Actual implementation files/interfaces are selected in
the later bounded plan after these contracts close.

## 1. Close actual per-writer interfaces and coverage

**Owner:** #1250 application maintenance contract, with each existing data/module
owner. Written evidence belongs beside the canonical maintenance contract/audit,
not a second independent status ledger.

- [x] For every audit family, pin the current source and exact existing admission,
  captured root/incarnation, retained handles, completion path and readiness path.
  Name missing operations as required unavailable interfaces; do not substitute
  a proposed signature, counter or test fixture for a known runtime capability.
- [ ] Specify the closed module/scope registration and its actual producing owner:
  empty namespaces, mutating reads, external ingress/relays, switched-away roots,
  global producers that can affect captured roots and unsupported direct writers.
  A caller-selected subset or sample of visible sessions cannot pass coverage.
- [ ] Close each original-job recipe: exact finite child/continuation slots,
  at-most-once admission, cancellation versus effect ordering, retained resources,
  physical acknowledgement and partial/unknown recovery/protection obligation.
  Include file removal followed by SQL progress and detached publication.
- [ ] Map each interface to Closing/Held/Releasing/Blocked, startup unverified,
  Q retirement and explicit release rules. Reuse existing lock order and identify
  how all waits occur outside projection/workspace/session/module/registry guards.

**Check/pass:** a fresh reviewer can trace every writer and continuation from
admission through physical settlement/readiness, or identify a precise missing
owner that blocks the next dependent step. A source-backed inventory with named
missing interfaces is a valid investigation result, **not** closed producer
qualification. Actual contract closure needs real owner operations; no mock
“drained” result may advance the implementation gate.

## 2. Obtain physical, format and readiness evidence

**Consumes:** step 1's closed actual interfaces and #1242/#1243/#1248/#1250
capabilities. Unknown on an input blocks its dependent work; source inspection
may still identify the missing evidence without executing it.

- [ ] Obtain actual root/lease/native identity, external quiescence and complete
  namespace/exclusion evidence for source/global/restore roots and disjoint
  output parents. Do not infer installed-helper state, OS descendant termination
  or supported platforms from source. Unclean-stop owner resolution remains.
- [ ] Obtain actual outer-job settlement acknowledgement before SQL close;
  closed-image/auxiliary/inert integrity evidence; original store reopen and
  module-readiness under the retained lease. Preserve legacy no-worker and
  preservation-only damaged-state handling without claiming recoverability.
- [ ] Close complete resource/time enforcement for job/continuation/root/module
  registration, outstanding/detached obligations, capture/preservation/staging,
  full inventories/control files, SQL auxiliary and callback/recovery scratch.
  Keep existing owners' allowances separate; choose no numbers by analogy.
- [ ] Obtain actual restore-fence discovery/settlement and startup routing from
  their existing owners. A process-local admission registry cannot replace the
  durable restore protocol or recover lost adoption approval.

**Check/pass:** each required guarantee has a real closed owner/profile and
corresponding source/execution evidence; every native execution request states
its platform, exact commands/fixtures, finite allowance, stop conditions and
expected result **before authorization**. This plan grants no such request.
No known receipt, fit or startup/restore support is claimed while evidence is
missing. Record unavailable results instead of adding success shims or silently
borrowing another session's budget.

## 3. Gate the bounded implementation handoff

Only after steps 1–2 and existing dependencies supply the required guarantees:

- [ ] Map the accepted admission/state/race oracles to exact module owners,
  interfaces, delivery boundaries and regression checks. Preserve original-root
  ownership across switches, finite settlement after cancellation, atomic Q
  retirement and explicit readiness before ordinary admission opens.
- [ ] Write/review a bounded implementation plan with actual affected files,
  meaningful failing tests and concrete completion results, using existing
  dependencies and owners. No unrelated refactor, public controller API,
  automatic session stop, new storage schema or probe budget enters the slice.
- [ ] For later authorized code, use TDD, scoped/full repository verification,
  current-diff review and required CI. Native/platform qualification and rollout
  stay separately controlled; no product readiness follows from source-only tests.

**Check/pass:** each conditional failure/ordering oracle in the accepted design
has an owning interface and verification route. Caller loss, late callback,
workspace switch, pending SQL progress, unknown owner, stale root and reopen/
fence failure cannot disappear behind a zero counter or matching digest.
Implementation remains blocked where a required actual producer/evidence input
is missing, even if a smaller pure model could pass synthetic tests.

## Review focus and risk disposition

Reuse the audit and resolved design review as the blind-spot investigation:
blocking callbacks after async abort, physical cleanup before SQL progress,
mutating reads, lost volatile state, first-mutation Q retirement and release
readiness are explicit acceptance checks above, not deferred edge cases.

Risk assesses eventual controller/capture work, not the reversible documentation
commit. Missing rollback or execution evidence remains Unknown.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 5 | Multiple real writer owners and open #1242/#1243/#1248/#1250 plus #1244 startup; steps 1–2 close actual contracts/evidence. |
| Blast radius | 4 | Shared workspace admission across lifecycle/metadata/background consumers; keep per-root ownership and reject unqualified coverage. |
| State changes | 4 | Future closure, physical maintenance effects and reopening affect authority/availability; retain exact existing owners and marker/fence rules. |
| Reversibility | Unknown | Actual restore/fence/readiness and physical settlement are not qualified; no cancellation rollback or automatic reopen claim. |
| Uncertainty | 5 | Source has local primitives but no complete known module/native/resource profile; qualification precedes dependent code. |

Total: incomplete. **Disposition: Ready for step 1's bounded written owner-contract
investigation; Investigate for physical/runtime implementation.** Existing approval
covers the unchanged maintenance design for further qualification, not a new
execution budget or proof that the steps are complete.

Fresh plan review and changed-scope acceptance/authority review found no material
findings. The Ready disposition is limited to step 1's written investigation;
no unknown owner/native/resource/rollback input was promoted to qualified, and
unchanged accepted design reviews were reused.


## Written investigation result — 2026-11-08

[#1544](https://github.com/Rambolarsen/orkworks/issues/1544) records the
[source-pinned owner-interface investigation](../../validation/backup-maintenance-writer-audit.md#owner-interface-investigation--2026-11-08)
at `1ab168d35a2cfd8689d82f984b52a0fad278f765`. The first inventory task above
is complete as a written investigation: existing handles, original-job recipes,
state mapping and precise unavailable operations are recorded beside the audit.
The historical audit pin is preserved. The new owned recommendation reopen path
is local evidence; it does not establish production adoption or complete release.

Remaining step 1 boxes stay open: documenting missing registration, finite-job
settlement and readiness integration does not implement or close those actual
owner contracts. Steps 2–3 stay blocked on those capabilities and qualified
physical/native/resource/restore evidence. **Disposition: written investigation
delivered; Investigate for actual contract closure and implementation.** No
runtime operations or product tests were executed and no execution budget was
created. The accepted maintenance design and its approval remain unchanged.


## Registration and settlement acceptance — 2026-11-08

[#1547](https://github.com/Rambolarsen/orkworks/issues/1547) supplies the
[accepted integration design](../specs/2026-11-08-maintenance-registration-settlement-design.md)
for fixed actual-producer coverage, root registrations, independently retained job
obligations, finite continuation slots and coordinated closure/readiness. After
#1549, the owner answered “go ahead” to the explicit four-choice approval request;
[#1552](https://github.com/Rambolarsen/orkworks/issues/1552) records acceptance for
further qualification. Existing maintenance choices stay accepted.
No remaining checkbox is completed by specifying an unavailable operation; real
owner integration and step 2 evidence still gate the bounded implementation handoff.

The next bounded prerequisite is to close actual owner integration inputs and
finite resource contracts against this accepted design, starting with exact
constructor/ingress coverage and retained SQL publication/cleanup completion
owners. This plan supplies no new execution allowance. Missing actual operations
remain unavailable; acceptance does not make a hypothetical catalog executable.


## Constructor and completion coverage result — 2026-11-08

[#1556](https://github.com/Rambolarsen/orkworks/issues/1556) adds
[concrete entry/owner coverage](../../validation/backup-maintenance-writer-audit.md#constructor-ingress-and-completion-coverage--2026-11-08)
at source `f5013bb189bfc6bb3155ed7b2cacc447e3069813`. Production open has effects
before workspace installation; direct metadata HTTP, integration-derived
revocation and background/global producers require coverage beyond an application
facade. Private SQL handles remain source-only. Cleanup binding can outlive its
async timeout; recommendation BUSY settlement has no finite maintenance wait
bound. Both require actual retained completion and resource/deadline contracts.

No remaining checkbox is completed: these exact insertion/transfer inputs do not
implement the actual complete registration or finite-job settlement operations.
Disposition remains Investigate for dependent implementation; unchanged accepted
choices are reused. No runtime execution or new numerical allowance was supplied.


## Binding and delivery bounds acceptance — 2026-11-08

[#1558](https://github.com/Rambolarsen/orkworks/issues/1558) provides the
[accepted finite-job refinement](../specs/2026-11-08-maintenance-settlement-bounds-design.md)
for complete retained reservation/result ownership, one absolute deadline and a
fixed store-attempt/notification/recheck graph without input resend. After #1560,
the owner answered “go ahead” to the explicit three-choice approval request;
[#1561](https://github.com/Rambolarsen/orkworks/issues/1561) records acceptance for
further qualification. Actual numeric profiles, delivery-outcome evidence,
compatible domain owner operations and physical/resource enforcement remain
unavailable; no qualification checkbox or implementation gate is completed.

Reuse the unchanged accepted policy; do not request repeat approval because actual
inputs remain missing. The next qualification inputs are the real delivery owner's
known/partial/unknown outcome evidence and actual reservation/deadline values with
enforcement and retained-result ownership. No value or executed fit follows from
acceptance; no remaining checkbox is completed.


## Delivery-outcome and limit evidence — 2026-11-08

[#1566](https://github.com/Rambolarsen/orkworks/issues/1566) records
[actual acknowledgement and enforced-limit evidence](../../validation/backup-maintenance-writer-audit.md#delivery-outcome-and-enforced-limit-evidence--2026-11-08)
at `26e4e2a15146344bf2d4a4495b7d35c75e1248f1`. Terminal input errors conflate
pre-write rejection with partial/unknown effects; all-byte success survives a
flush error and is distinct from blocking writer return and later bookkeeping.
Current numeric input/channel/startup/file/SQL controls have narrow owners, not
complete application-wide reservation/deadline semantics. No new values were chosen.

Actual truthful outcome transport, retained result/root ownership, full finite
resource enforcement and compatible deadline/notification operations remain
unavailable. No remaining qualification checkbox is completed, no physical fit
is claimed, and no runtime execution or implementation allowance follows.


## Terminal/application outcome interface acceptance — 2026-11-09

[#1569](https://github.com/Rambolarsen/orkworks/issues/1569) provides the
[accepted private outcome interface](../specs/2026-11-08-terminal-input-outcome-interface-design.md)
for an original submission record retaining direct writer facts, distinct input
truth/physical/application settlement, and rich recommendation consumption without
resend. After #1571, the owner answered “go ahead” to the explicit three-choice
approval request; [#1576](https://github.com/Rambolarsen/orkworks/issues/1576) records
acceptance for further qualification. Accepted no-resend,
retained ownership and finite deadline/settlement policy stand. No actual outcome
transport, numeric reservation/enforcement or physical readiness is qualified by
the accepted conditional record; all remaining qualification boxes and execution
gates stand.

Reuse the accepted interface without repeat approval of unchanged choices. Actual
submission/phase/result owner operations, complete numeric reservation/deadline
enforcement and physical/native evidence still need closure before a bounded
implementation handoff; an accepted paper record is not runtime capability.


## Accepted source-only owner sequencing exception — 2026-11-09

[#1584](https://github.com/Rambolarsen/orkworks/issues/1584) records a concrete
[accepted sequencing amendment](../specs/2026-11-09-submission-owner-qualification-sequencing.md).
Under a literal all-code reading, the current general handoff requires actual
owner operations before their first candidate implementation can be planned. After #1587, the owner approved exactly one unwired source-only submission-record/
accounting candidate and offline component checks; [#1589](https://github.com/Rambolarsen/orkworks/issues/1589)
records acceptance and the [bounded implementation plan](2026-11-09-input-submission-owner-candidate.md).

The approved exception covers that candidate only. It closes no remaining checkbox
and changes no production/controller, native/SQL/model, resource-profile, Q or
restore gate. Candidate evidence would establish only its actual local behavior,
never production producer/physical qualification or complete resource fit.


## Source-only candidate result — 2026-11-09

[#1589](https://github.com/Rambolarsen/orkworks/issues/1589) implements the approved
[single private owner candidate](2026-11-09-input-submission-owner-candidate.md).
Actual candidate handles enforce a fixed record pool, finite explicit limits,
four unique reporters, bounded observers and checked serial reuse. Input facts
remain distinct from phase completion; loss/conflict/partial/unknown is retained
as protection and cannot be force-retired. Thirteen offline candidate tests and
full repository verification passed; fresh current-diff medium review found no
material findings.

This is actual local bookkeeping evidence, not producer/native/whole-application
resource evidence. No production constructor or input/SQL/file/model effect was
added. Remaining full-catalog, real writer/physical completion, resource-profile,
native/external and restore gates stay open; no existing checkbox is promoted
to complete on the strength of candidate counters or synthetic reporting roles.


## First direct writer result proposal — 2026-11-09

After the actual candidate in #1593, [#1604](https://github.com/Rambolarsen/orkworks/issues/1604)
proposes the [existing writer/driver result slice](../specs/2026-11-09-terminal-writer-outcomes-design.md).
The actual loop would return checked byte/flush facts beside its writer through
the existing task join, while preserving the old acknowledgement projection.
This is proposed producer evidence, not another standalone accounting owner.

Owner approval of that concrete bounded source change is pending. The previous
one-module exception does not authorize it. Even after that preparatory slice,
production submission-owner construction, gap-free original root/resource/report
ownership, full phase integration, numeric/deadline/native/physical/restore
qualification remain open. No checkbox above closes on the strength of the
proposal or offline writer fixtures; a flush panic can still lose local facts
until the actual retained reporting path exists.


## Direct writer implementation scope accepted — 2026-11-09

After #1607 the owner answered “keep going” to the exact implementation question.
[#1609](https://github.com/Rambolarsen/orkworks/issues/1609) records acceptance
and actual implementation together under the [bounded plan](2026-11-09-terminal-write-outcomes.md).
The existing writer produces fixed checked byte/flush facts and returns them
beside its writer through the same task join, preserving coarse acknowledgements.
The production driver discards those facts after return; no application result
reader/ledger is activated. Eleven offline operation/actual-task tests, full repository verification
(3,371 Rust, 1,720 desktop and 1,204 docs tests plus builds/integrations) and
fresh current-code `/code-review low` passed; the review found no issues.

Only InputFact/Flush values are shared with the accounting module, whose Owner
remains unwired. Panic after full write/flush and driver loss can still lose
local evidence. Original root/resource/report transfer, retained publication,
all physical phase ownership, complete numeric/deadline/native/resource fit,
maintenance and restore remain open; no broad qualification box closes.


## Retained writer transfer proposal — 2026-11-09

[#1614](https://github.com/Rambolarsen/orkworks/issues/1614) proposes the
[checkpoint and reporter transfer](../specs/2026-11-09-retained-writer-transfer-design.md)
after #1611. One fixed byte checkpoint preserves known prefixes/full bytes before
flush, independently of immutable terminal facts. Unique writer/return reporters
would flow through the actual worker and join in a private optional path tested
with actual candidate handles; production constructors remain unreported.

The new bounded bridge/owner refinement requires one explicit decision before
implementation. Neither previous approval supplies that decision. It would
advance local retention/transfer evidence only, with no production Owner factory,
real root/resource/profile/admission/phase integration, force resolution, native/
physical qualification, Q/readiness or restore result. Native-return-to-checkpoint
panic and driver/resource loss remain honestly unknown; no broad box above closes.


## Checkpoint/reporting bridge scope accepted — 2026-11-09

After #1616 the owner answered “keep going” to the exact implementation question.
[#1620](https://github.com/Rambolarsen/orkworks/issues/1620) records approval,
the [bounded plan](2026-11-09-writer-reporting-bridge.md) and actual optional bridge
in one task. Fixed checkpoints preserve known bytes independently of immutable
terminal facts; unique reporter validation retains all inputs on refusal, and
writer publication/actual shared join keep completion distinct from ack/Drop.
All 41 related owner/writer cases (17 new) and full repository verification
passed: 3,414 Rust, 1,720 desktop and 1,204 docs tests plus builds/integrations.
The earlier unchanged SQL observer setup failure passed isolated and in the full
parallel run; its exact cause remains unproven. Fresh current-code medium review
approved the exact five-file manifest with no findings.

All production starts remain unreported/None, with no Owner factory or original
root/resource/profile/application integration. The bridge never completes
preparation/bookkeeping, grants input authority or qualifies native/physical
teardown, Q/readiness, backup or restore. Required current-head CI/PR evidence
still gates merge; no broad qualification box closes here.


## Original input scope proposal — 2026-11-09

[#1627](https://github.com/Rambolarsen/orkworks/issues/1627) proposes the
[concrete original-scope attachment](../specs/2026-11-09-original-input-scope-design.md)
after #1625. Existing recommendation delivery captures only a weak workspace
lease; keeping AppState alive cannot retain a workspace removed from it. The
proposed immutable Entry attachment owns the actual original strong lease and
captured runtime/channel intent, sharing lifetime through existing Core/reporters
rather than a second registry. Application coordination must eventually retain
the Owner independently of callers and current workspace selection.

Fresh builder and independent source/authority reviews found no material issues.
The new bounded handoff would implement actual typed capture/finite attachment
and offline lease/worker lifetime tests, after explicit approval. It introduces
no production capture/Owner factory, command routing, phase or graph consumer,
complete native RootKey/registration, resource/deadline profile, physical/
maintenance or restore qualification. All broad boxes above remain unchanged;
this proposal is not implementation approval. No separate acceptance-only task
is needed after the owner approves the concrete scope.


## Original input scope accepted — 2026-11-09

After #1629 the owner answered “go” to the explicit typed-capture/finite-record-
attachment/offline-test question. [#1630](https://github.com/Rambolarsen/orkworks/issues/1630)
records approval, the [bounded plan](2026-11-09-input-scope-retention.md) and actual
private capture/Entry retention together. Reuse the existing reporter/worker/join
bridge and accepted input/registration/bounds semantics. The original scope
retains the actual lease and runtime/channel intent; finite reservations include
rejected captures, with unused ownership returned intact.

Production callers/construction, preparation/bookkeeping/graph consumption, full
native RootKey/module registration and complete physical/resource/deadline fit
remain gated. A retained lease/sender is not PTY ownership, receiver liveness or
whole-job completion; last Core destruction releases objects without certifying
teardown. No broad box above closes or maintenance/readiness/restore is activated.
Verification and current-code review evidence belong in the bounded plan/PR.


## Complete input coordinator/recipe proposal — 2026-11-09

[#1638](https://github.com/Rambolarsen/orkworks/issues/1638) proposes the
[application coordinator and closed original input recipe](../specs/2026-11-09-input-coordinator-recipe-design.md)
after #1636. It reuses actual Entry/Core/scope/writer ownership and addresses
application-lifetime root retention, full reservation before effects, exact
continuation slots and rich graph/event settlement under one original deadline.
Current two runtime slots and 64-entry channel are source controls only, not a
complete memory/native/resource profile. Accumulated parser/hint state, reset
retry loops and downstream Peon/graph owners remain actual missing inputs.

The proposed next bounded private implementation would enforce the fixed recipe
with explicit fixture parameters and existing actual capture/writer/join checks.
Unsupported producer/multi-root/native/profile operations stay closed; local
state fixtures cannot certify their completion or qualify production. This is a
new proposed scope requiring explicit owner approval, not inferred permission
from the completed preparatory slices. Production construction/activation and
full module/root/resource/deadline/physical/readiness/restore qualification remain
gated. No broad checkbox above closes and no new execution allowance is supplied.


## Private input coordinator scope accepted — 2026-11-09

After #1641 the owner answered “keep going” to the explicit design/private-code
question. [#1644](https://github.com/Rambolarsen/orkworks/issues/1644) records
acceptance, the [bounded plan](2026-11-09-private-input-coordinator.md) and code
together. Routes, declared aggregate/per-job charges and fixed recipe state share
the actual Core/Entry; there is no second input-result ledger. Root Closing and
job admission are atomic; unique job/slot handles cannot repeat/renew a slot.
Original scope pool ownership and route serials are exact. Lost/protected work
retains charges; even removed route references remain bounded until Drop.

Production registration/Open, actual missing preparation/bookkeeping/reset/label/
graph adapters and physical/root/profile qualification stay unavailable. Test-only
transition inputs prove local rules; they cannot complete real producer phases or
certify whole-job/readiness. Actual writer/join checks preserve pending application
phases and late original facts. No production constructor/caller, new native/SQL/
model/resource experiment, shipping profile, maintenance/restore activation or
broad qualification box follows. Validation/review evidence is in the bounded plan
and PR; the next qualification must obtain actual missing owner operations and
profile inputs rather than repeat this local recipe.
