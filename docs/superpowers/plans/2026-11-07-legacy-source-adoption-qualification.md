---
type: Qualification Plan
title: Retained legacy source-adoption qualification
description: Close inventory, approval, publication and restart contracts before implementing the accepted additive recovery policy.
tags: [orkworks, sessions, migration, recovery, qualification]
status: draft
workflow_status: qualification-pending
---

# Retained legacy source-adoption qualification

## Outcome and authority

Produce reviewed contracts and evidence sufficient to write bounded adoption
implementation tasks under [#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Pass condition: every accepted policy requirement has an exact owner, input,
rejection/publication/recovery rule and meaningful verification oracle, with no
unknown ownership or rollback used as permission to write live data.

The owner approved the [additive policy](../specs/2026-11-07-legacy-source-adoption-amendment.md)
after #1495 for ADR and plan updates. [ADR 0103](../../adr/0103-sqlite-session-storage.md)
records acceptance for qualification. This plan does not authorize runtime
implementation, live repair, probe execution or dependency bypass. Reuse the
approved policy; obtain a new decision only if contract work would materially
change it or requires a new security/migration design.

Dependencies remain [producers #1242](https://github.com/Rambolarsen/orkworks/issues/1242),
[private store #1243](https://github.com/Rambolarsen/orkworks/issues/1243),
[historical semantics #1248](https://github.com/Rambolarsen/orkworks/issues/1248) and
[complete backup/restore #1250](https://github.com/Rambolarsen/orkworks/issues/1250).
All four were open at this plan's preparation. Their partial source foundations
do not establish qualified maintenance/adoption. Existing native/hierarchy and
rollout gates stay in force; this ordinary-history work cannot consume another
task's finite compile/native budget.

## Investigated uncertainties and boundaries

The [legacy-copy review](../../validation/legacy-migration-recovery-review.md)
traces absent completion/deletion provenance and the actual workspace-open
fences. Current `migration.rs` only copies/skips and logs mkdir/copy failures;
the ordinary adapter remains source-only at cutover. These are not existing
inventory/approval/publication contracts to reuse without qualification.

The biggest coupled risk is treating pre-activation external-file publication as
atomic with SQL or using content equality as ownership. The accepted policy
already rejects both. Tasks below must bind actual identities/progress and test
partial states before implementation can be planned. Creating a second startup
repair engine, changing source priority or inferring deletion intent are rejected
alternatives, not additional tasks.

This is contract qualification, not exploration of live history. Use pinned source
inspection and declared synthetic/copied fixtures. Static investigation supplies
contracts and test oracles; it does not prove native identity or durability.
Record missing platform/build/owner evidence as Unknown. No new execution budget
is available here; propose a finite evidence request separately when a concrete
remaining uncertainty cannot be resolved read-only.

## 1. Inventory, report and owner-confirmation contract

Tracking: [#1500](https://github.com/Rambolarsen/orkworks/issues/1500).
The [v1 inventory/report/confirmation contract](../specs/2026-11-07-legacy-adoption-inventory-approval.md)
records the owner-accepted logical representation and component oracles; qualification
of its referenced physical/backup/application capabilities remains required.

**Owner and artifact:** #1244 owns a written contract under
`docs/superpowers/specs/` for the import inventory/approval seam; #1250 supplies
backup identity/completeness inputs. Update that directory index. No production
module or transport endpoint is added in this task.

**Consumes:** accepted policy, current root/creation/protection owners, and the
backup contract's actual available guarantees. Unimplemented backup inputs must
be named as required unavailable capabilities, never filled with dummy success.

**Deliver:** one versioned canonical inventory/report/decision contract. Specify
field meanings, byte encoding/order, IDs/digests, unknown/error representation,
supported path/identity forms and complete streamed accounting. Define the
captured root/source/global/backup/protection bindings, every candidate class,
owner adopt/exclude decisions, page completion and confirmation invalidation.
Specify application-owner confirmation versus agent/report capabilities and
its replay/stale/cancellation outcomes before choosing an implementation route.
Approval needs the entire reviewed decision batch; a page token is no authority.

**Checks:** independent golden examples and rejection oracles cover source-only
adopt/exclude with possible-deletion warning, same-ID creation conflict, differing
global bytes, missing payload association, corrupt global/protection work,
unsupported-but-readable exclusion, unreadable input blocking, root/backup/
source/target changes and unreviewed late candidates. Verify exactly 100 report
entries and the inclusive 256 KiB complete-envelope limit, multi-page completion,
one oversized entry, metadata's existing 1 MiB import limit and no silent
truncation/omission. Source and expected bytes must be independently derived.

**Pass:** fresh contract review can determine every candidate's result and whether
confirmation is valid from the recorded inputs alone. Resolve actual authority/
codec ambiguities before publication/recovery work depends on them. A material
new approval/security contract requires its applicable owner review before code.

## 2. External publication, backup and restart contract

Tracking: [#1510](https://github.com/Rambolarsen/orkworks/issues/1510).
The [conditional publication/recovery contract](../specs/2026-11-07-legacy-adoption-publication-recovery.md)
records logical phase/effect and interruption oracles. After #1512, the owner
accepted its three scope choices and unchanged conditional representation for
further qualification; #1514 records acceptance. It establishes no qualified
native/backup/control/resource producer. Reuse that approval within scope;
step 3 remains blocked until actual evidence and dependencies are qualified.

**Owner and artifact:** #1244 owns the companion publication/recovery contract;
#1250 owns complete-backup validity/restoration and existing external-file owners
retain their files. Keep one authoritative contract per seam; link the step-1
contract instead of reproducing it.

**Consumes:** reviewed step-1 manifest/authority; pinned metadata/payload ownership,
root/identity/exclusive-create/flush behavior; actual backup and maintenance
interfaces. Source inspection may proceed while dependency implementations are
pending, but the result remains conditional until their inputs are verified.

**Deliver:** exact progress/ownership states and allowed transitions from approved
staging through each external payload, SQL publication and the unchanged format-2
marker. State the durable evidence required before/after each effect, separately
account for this attempt's additions and baseline data, and define recovery or
stop/backup-restore for every partial/unknown state. No pathname/content match
may grant deletion/overwrite authority. Cover owner confirmation loss/cancellation,
identity reuse and external changes without releasing maintenance into normal open.

Record complete on-disk inventory/progress/payload accounting and any relevant
existing limits; derive enforcement before claiming resource fit. Identify which
platform operations are source-proven, which are conditional and which require
bounded native evidence. Do not invent OS identity, exclusive publication or
flush guarantees or silently borrow linked-pair resource/probe allowances.

**Checks:** a state/effect matrix covers interruption before and after every
backup/progress/payload/SQL/marker boundary; one payload published while another
fails; foreign replacement at the same pathname; missing/invalid marker; source
change, lost quiescence or backup validity; repeated recovery; and post-cutover
SQL writes. Oracles require current global bytes unchanged, no unapproved
resurrection, complete candidate accounting, normal open blocked by unresolved
evidence and only the existing activation point changing authority.

**Pass:** fresh review identifies an evidence-backed recovery outcome for every
state. Unqualified physical identity/durability or incomplete backup restoration
blocks dependent implementation; a logical matrix is not native qualification.

## 3. Integrate evidence and write bounded implementation tasks

**Owner:** #1244 amends [task 5 of the storage delivery plan](2026-11-01-sqlite-session-storage.md)
only after steps 1–2 and their dependencies supply the required contracts/evidence.
Keep the evidence in its owning contract/validation record; do not create a second
status ledger or duplicate already reviewed artifacts.

**Deliver/check:** map each accepted amendment requirement and its regression
matrix to an implementation owner and test, including historical-fact/organization
preservation, source-only/deleted ambiguity, missing payloads, exclusive creation,
manifest/owner confirmation, complete reports and all interrupted states. Verify
no runtime resume, image/native-store import, startup replay, current-data overwrite
or retention/evidence-authority change enters the task scope. Run the applicable
source/TDD/full-repo/current-diff review gates when code is eventually authorized;
platform/live rollout remains separately controlled by #1247.

**Pass:** a reviewed implementation plan names actual interfaces and bounded
verification, carries no assumed recovery/native/resource result, and leaves no
missing prerequisite hidden behind a mock. Unknown evidence blocks implementation;
acceptance of this policy is not evidence that those steps have been achieved.

## Plan review and risk disposition

Assessment concerns the eventual adoption implementation, not the harmlessness
of committing this plan. Ratings follow the local plan policy; they grant no
permission and no total is computed when a dimension is Unknown.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 5 | Four open delivery dependencies plus application-owner/backup/external-file contracts; preserve gates and resolve in steps 1–2. |
| Blast radius | 4 | Shared workspace import, historical records and external payloads; restrict to first activation and existing maintenance owner. |
| State changes | 5 | Future adoption changes authoritative history and external files; explicit exact decisions, additive-only writes and unchanged marker. |
| Reversibility | Unknown | Complete backup restoration/native publication recovery are not yet qualified; step 2 must close this before code. |
| Uncertainty | 5 | Canonical approval/manifest, full accounting and physical identity/durability lack qualified adoption evidence. |

Total: incomplete. **Disposition: Ready for step 1's written contract drafting;
Investigate for dependent runtime implementation.** A fresh read-only plan and
acceptance-fidelity review on 2026-11-07 found no material scope, coverage,
authority or recovery gaps for that next gate. Unchanged policy approval and
source evidence were reused. No qualification contract or runtime/native result
was claimed complete by the review.

## Subsequent representation acceptance and owner audit — 2026-11-07

After #1504, the owner accepted v1's exact logical representation and confirmation
ordering for further qualification. Step 1's written representation/review is
delivered; do not reopen that unchanged approval. The
[evidence-owner audit](../../validation/legacy-adoption-evidence-owners.md)
pins actual available primitives and unqualified receipt/transport capabilities.
Step 2 may use those findings for conditional written contract work, but cannot
claim production readiness, physical fit or backup restoration until its actual
required inputs and platform evidence are qualified. Dependent implementation
remains Investigate; no new execution or probe budget follows.
