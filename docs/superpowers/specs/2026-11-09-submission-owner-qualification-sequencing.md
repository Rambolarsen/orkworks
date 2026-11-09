---
type: Qualification Sequencing Amendment
title: Source-only submission owner before physical qualification
description: Accepted bounded candidate implementation route that breaks the owner-existence prerequisite cycle without enabling production input or maintenance.
tags: [orkworks, sessions, terminal, maintenance, qualification]
status: stable
workflow_status: accepted-source-only-candidate
---

# Source-only submission owner before physical qualification

Tracking: [#1584](https://github.com/Rambolarsen/orkworks/issues/1584), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Consumes the accepted [input-outcome interface](2026-11-08-terminal-input-outcome-interface-design.md),
[bounded settlement](2026-11-08-maintenance-settlement-bounds-design.md),
[registration/settlement](2026-11-08-maintenance-registration-settlement-design.md)
and [maintenance admission](2026-11-08-backup-maintenance-admission-design.md).
The [qualification plan](../plans/2026-11-08-backup-maintenance-qualification.md)
and [source audit](../../validation/backup-maintenance-writer-audit.md) remain the
canonical gates/evidence. Backup/adoption/v1, no resend, unknown protection,
original ownership and no-automatic-stop policy are unchanged.

**Status:** accepted for the single source-only candidate. After #1587, the owner
answered “lets keep going” to the explicit request for a reviewed implementation
plan then this one unwired module with offline tests. [#1589](https://github.com/Rambolarsen/orkworks/issues/1589)
records that scoped approval on 2026-11-09. Source pin `69f46a3a750668c25d7e48cf1e812ce50a6098e3`.
The terminal/runtime/application files inspected in #1567 are unchanged from
`26e4e2a`; repeating that audit cannot produce missing operations or numeric fit.
No code, product tests, native/SQL/model probes or live operations ran here.

## Concrete sequencing problem

The current plan permits a dependent implementation plan only after actual owner
operations and physical/resource guarantees exist. The accepted input interface
also correctly says those operations do not exist yet. Under a literal all-code
interpretation, the first actual owner cannot be implemented to obtain the
required evidence. Another paper inventory, hypothetical enum or mocked drained
module cannot close that cycle.

Keep the full gates for production wiring, complete maintenance and physical fit.
Propose only one earlier **source-only candidate implementation**: the private
original-submission record and its logical reservation/phase/outcome ownership.
It has no ability to submit terminal input, touch stores or issue maintenance
receipts. Component checks can establish that candidate's actual bookkeeping
behavior; they cannot qualify the absent producer integration or native profile.

## Accepted choices for the single candidate

1. Permit one bounded source-only candidate owner implementation and its offline
   component verification before full producer/native/resource qualification,
   after this amendment is approved and a concrete implementation plan reviewed.
2. Keep that candidate unwired: no production construction/callers, terminal or
   SQL/file/model effects, scheduler integration, maintenance Q or readiness result.
   Its data/role handles carry no live input/maintenance authority.
3. Treat finite checked constructor parameters and synthetic fixture values as
   candidate verification inputs only. Production numeric values, real producer
   identity/admission, complete physical accounting and activation remain gated.

These unchanged sequencing choices are accepted, without changes to outcome
semantics. Approval permits planning/executing this single candidate slice under
the checks below. It would not authorize wiring, relax no-resend/unknown handling,
qualify a module by name or make a fake success profile acceptable.

## First candidate slice and ownership

Suggested concrete location: a private `runtime/input_submission.rs` module,
declared in `runtime/mod.rs`, with its own scoped tests. Final file placement and
exact types belong to the bounded implementation plan. Do not split/refactor
existing terminal/application modules or introduce a public service/trait/plugin
registry for this slice.

The module must implement the accepted record's real local ownership rules:

| Responsibility | Required candidate behavior |
| --- | --- |
| Original record | One bounded instance binds immutable opaque context/slot data and input byte length; duplicate/conflicting job registration rejects without replacing an unresolved original. Context data is not a qualified RootKey or reconstructed runtime permit. |
| Unique reporting roles | Preparation, writer, physical-return and bookkeeping responsibilities have owned exact phase handles. Observation is bounded and read-only. A role can record only its own original facts; cloning observation cannot clone execution/reporting authority. |
| Input facts | Preserve known no-input-effect, known full byte write and partial/unknown under the accepted evidence rules. Checked reported-prefix facts never imply tool execution. Flush failure cannot reverse known full write; missing evidence is unresolved. |
| Independent completion | Phase completion is distinct from input truth and result delivery. Dropped observer/notification cannot erase the record. Dropped reporting role or conflicting report retains unresolved/protection rather than success or a zero-job proof. |
| Deadline/accounting | One supplied absolute deadline governs new local admissions; no renewal, hidden retry or slot growth. Retain reserved logical charges through pending/unknown obligations. Known physical-local settlement can release execution charges only under the accepted distinction from retained result/protection charges. |
| Result retention | Bound result/error storage and observer/notification state; no per-write/unbounded report history or prompt log. Consuming a known result is not permission to clear an unrelated unresolved phase. |

The implementation must be an actual reusable ownership/accounting module,
exercised through its own private interface. Tests do not bypass it to mutate
counters or assert a fictional producer is drained. Synthetic reporting roles
prove only the candidate's local rules. They are not actual terminal writer
reports, native completion or production no-effect authority.

This first slice owns no native root/directory/PTY/store handle. It may carry
bounded opaque identity/reference data for the future real owner, but cannot
acquire/release a real lease or create producer authority. The complete native
resource vector remains the future qualified integrating owner's responsibility.
Do not hide missing ownership behind an uncharged external callback or detached
physical task; neither is part of this candidate.

## Finite parameters without invented production fit

The candidate constructor requires explicit finite, nonzero applicable limits for
its own records/phases/observers and bounded data/result/notification storage,
plus valid absolute deadline semantics. Missing limits or overflow fail before
admission. Reserve before module-owned variable allocation; bound every retained
identity/result/error field. Parameters cannot create new recipe slots at runtime.
Fixed internal state and bounded allocation capacities must be reviewable.

Checks must distinguish logical reservation counts/bytes and retained capacity
from measured allocator/native footprint. State no RSS, allocator peak, platform
handle or whole-application fit result from counter tests. Any input byte charge
must match the data capacity the candidate actually retains; a byte length for an
external payload is merely declared context, not measured ownership of that
payload. No complete reservation certificate is returned.

Synthetic limits are selected only to exercise exact boundary/overflow/exhaustion
behavior and are labelled fixtures. They cannot become default production config,
borrow the existing two input slots/32 file jobs, or claim to be approved budgets.
The candidate has no production-default constructor or settings entry. Production
instantiation needs a later owner-qualified profile and actual writer/root/phase
integration; this amendment does not supply them.

## Verification scope and stop conditions

After approval, the implementation plan must name exact interfaces and meaningful
failing checks for duplicate identity, phase misuse, stale generation/context,
observer loss, role loss, partial/full/flush distinctions, conflicting reports,
independent physical completion, deadline before/after admission, result retention,
reservation transfer, capacity exhaustion and arithmetic boundaries. Exercise
actual candidate handles/resources rather than a mirror implementation.

The scoped unit target may compile and test this module using synthetic data and
in-process concurrency. It must not launch a sidecar/terminal/native harness,
open SQLite, inspect live metadata, submit input, call a model or touch retained
user stores. No external process/profile experiment is authorized. Reuse normal
repository build/format/static/docs checks. Full repository verification remains
required by the root guide for a code PR; inspect its commands and any execution
capability gates before running it. If a required check needs separately gated
native/SQL/model execution, report the exact check and obtain its finite authorized
route; do not silently skip it or label the PR merge-ready.

Source-only integration verification must show no production construction/call of
the candidate and no effectful imports/operations in its module. A fresh current-
diff code review, repository checks and required CI still apply to later code.
If implementation requires changing actual writers, public API/schema, resource
policy or runtime startup, stop and split that dependent task instead of expanding
this exception. No current-diff review or execution gate is waived here.

## Evidence progression and handoff

| Stage | What can advance | What remains unavailable |
| --- | --- | --- |
| This written amendment | Decision on one early source-only candidate slice. | All actual owner implementation and physical qualification. |
| Approved candidate plan + verified code | Candidate local ownership/accounting behavior with exact source/test evidence. | Production writer reports, complete root/module registration, real payload/handle/resource profile, native/external quiescence, Q/readiness and restore. |
| Later separately bounded producer integration | Only real producer/phase/outcome evidence established by that integration and its qualified resource/native owners. | Any unimplemented/unsupported platform, complete-catalog or backup claim. |
| Full existing qualification | Existing plan gates may advance only with their actual required evidence. | No bypass based on candidate counters or synthetic fixtures. |

The complete maintenance qualification plan's steps 1–2 and its general step 3
gate remain unchanged for production/controller work. This is a narrow explicit
exception to their sequencing for the one candidate slice, not a statement that
those boxes are closed. Add a bounded candidate plan only after approval; do not
mark a ready native/backup/SQL profile or broadly authorize follow-on modules.

The useful next result would be tested source code for this actual private owner,
followed by honest evidence of its remaining producer/resource gaps. No further
source inventory of unchanged terminal paths is required to plan that slice.
The owner approval above supplies the single-candidate sequencing decision.
The [bounded plan](../plans/2026-11-09-input-submission-owner-candidate.md) routes
implementation and verification; production qualification remains separate.


## Review disposition

Fresh builder and independent authority reviews found no material findings in the
one-candidate scope, fixture/evidence separation, resource disclaimers or required
check boundaries. The sequencing conflict concerns treating the existing gate as
applying to all candidate code; it does not invalidate the production prerequisite
or claim production qualification is impossible. This review was reused for the unchanged acceptance; no production gate was waived.
