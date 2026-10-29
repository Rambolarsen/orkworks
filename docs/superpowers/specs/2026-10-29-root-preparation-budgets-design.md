---
type: Design Proposal
title: Initial root preparation budgets and bounded retained custody
description: Proposed finite preparation, run inventory, proof history and whole logical store limits for the initial non-sharing root profile.
tags: [orkworks, hierarchy, preparation, budgets, retention, custody]
status: draft
---

# Initial root preparation budgets and bounded retained custody

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `77a7f03b` after PR #1145.
**Proposed owner decisions only. No production capacity, executable plan,
fixture/checker change, database, migration, native probe or runtime implementation
is accepted or delivered by this document.**

## Outcome and short plan

The [accepted initial effects](2026-10-28-root-preparation-effects-design.md#owner-acceptance-for-documentation-fixtures--2026-10-29)
leave numerical preparation and run-inventory limits open. The
[logical effect witnesses](../../validation/root-preparation-effects.md)
measure complete commands but deliberately use fixture-only caps. Propose finite
limits and an exhaustion policy here, obtain scoped owner review, then produce
separate boundary witnesses if accepted. Keep the old catalogs and their exact
measurements unchanged. Native feasibility and authority qualification follow
under their own approved plans; neither documentation acceptance nor a passing
logical witness establishes them.

This is the initial no-reference-input, no-materialized-output profile only.
The full mutable lifecycle, child assignments, reports, candidates, results,
evaluations, dispositions, plan consumers and shared-learning custody are not
added. Their source-specific aggregate limits and closed custody/effect codecs
remain prerequisites before those producers can write. Existing reporting
credits and summary defaults keep their own values and acceptance status.

## Named proposed decisions

All byte limits are inclusive; MiB is 1,048,576 bytes. These are author-proposed
starting defaults for owner review, not observed workload requirements or native
measurements. Checked integer sums apply; overflow denies, never wraps.

| Budget | Proposed ceiling | Charge and scope |
| --- | --- | --- |
| preparation-retained-bytes, run | 8 MiB (8,388,608 bytes) | Existing exact preparation projection, summed over that run |
| preparation-retained-bytes, workspace | 128 MiB (134,217,728 bytes) | Same occurrences over every retained run, including old run epochs and the pending draft |
| preparation-run-count, workspace | 32 | Distinct run IDs with pending draft or retained admitted source; closed runs count |
| Retained proof bytes, workspace store | 64 MiB (67,108,864 bytes) | Complete canonical keys and values of every commit_proof row, with no commit-link projection |
| Retained proof count, workspace store | 4,096 | Every commit_proof row, including initialization and abandoned-draft command proofs |
| Complete retained logical bytes, workspace store | 512 MiB (536,870,912 bytes) | Canonical key plus full canonical value for every logical row in the selected store, including header and proofs |
| Complete retained logical row count, workspace store | 8,192 | All logical rows, including zero ledgers and unavailable/never-initialized markers |

Keep the existing one outstanding workspace draft, at most 16 artifact references,
1 MiB raw artifact-occurrence aggregate, 4 KiB run/parent values, 2 MiB encoded
artifact-value bound, and 1,081,344-byte draft/source envelopes. No nested source
limit is raised. The inherited proposed 16 MiB complete-command and 4,096 changed-row
limits remain independent. These totals do not accept the proposed main/journal/
scratch ceilings or provide an enforcement mechanism for any of them.

The preparation ceilings use the accepted two resources and owner mapping
unchanged. reserved remains zero. No new quota row or resource tag is introduced.
The other four kinds of totals (proof bytes/count, complete bytes/count) are
store-owner admission/reload invariants derived from actual retained rows, not
persisted quota_usage rows. Do not put a proof's own byte charge into its manifest
through a self-accounting quota update. A future cached accounting/index scheme
needs its own exact codec, reconciliation and command-effects review; callers
cannot supply trusted totals.

## Why these numbers, and what they do not promise

One maximal initial source envelope, the raw 1 MiB capability-artifact aggregate
encoded as separate Base64 occurrences, bounded keys/wrappers, and the two 4 KiB
membership rows leave substantial room under 8 MiB. This is a conservative sizing
rationale, not an executed maximum-length proof. The 128 MiB workspace cap permits
16 fully charged 8 MiB runs; 32 smaller runs may fit the count cap instead. There
is no reservation or guarantee that a later admission fits just because a draft
fit. Admission must replace the draft charge with the exact complete source and
membership charge, and may be denied without losing the pending draft.

The 64 MiB/4,096 proof bounds deliberately make the no-pruning profile finite.
Repeated successful prepare/cancel cycles can fill history even with no retained
run. That cost is explicit; neither an unbounded audit chain nor automatic
proof garbage collection is selected. The 512 MiB/8,192 whole-store bounds keep
omitted envelopes, ledger rows, summaries and metadata from becoming free custody.
They are not an allocation guarantee: independent maxima need not coexist, and
512 MiB of canonical logical content does not prove fit in a 1 GiB database.
Physical indexes/pages/free pages/journal/auxiliary files and allocator memory
need separate bounded native evidence.

Rejected alternatives: copying the fixture's 16/32 MiB and eight-run caps as
production approval; allowing unlimited closed-run history; TTL or oldest-first
eviction; retaining only hashes of reviewed bytes; digest-based cross-run reuse;
and a new persisted aggregate counter whose own proof charge is circular.
A safe proof-pruning protocol could improve longevity later, but its affected
set, protected links and failure/recovery cases are a separate design decision.

## Complete custody inventory and exact accounting

Preparation usage is exactly the accepted key-plus-committed-body projection of
root_drafts, root_preparations, run_states, parent_attachments and
preparation_artifacts. Omit only the already named commit-link paths; every
excluded link must still resolve. Artifact Base64 and all duplicated inline
bootstrap/configuration/instruction bytes are charged where retained. An equal
contentDigest under two reference IDs produces two separately owned occurrences
and two full charges. No physical deduplication credit is inferred.

A pending draft owns exactly its artifact-reference set. Admission atomically
transfers that same run's ownership to the immutable retained source, keeps the
artifact rows and their original links, removes the draft, and adds initial run,
reserved parent and subject membership. A retained run has exactly one original
root preparation source. No source replacement or second retained source revision
is supported by this profile. A changed bootstrap requires a fresh burned run
identity and new review under the accepted producer; it consumes another retained
run slot if admitted. A cancelled unadmitted draft releases its preparation
occurrences/count only; its storage proofs remain charged.

Proof bytes/count include full canonical stored proof values and keys, including
their manifests and predecessor references. Full-store bytes/count include those
same proofs once, plus every other physical logical row occurrence once. Overlap
between a domain cap and the whole-store cap is intentional, not a second copy
of physical data. Never subtract the preparation projection from full-store usage
or omit lastCommit, receipt.commit, checkpoint.applied, quota rows or header bytes
there. Whole-store accounting is a distinct retained-state check, not the
complete-command formula: replaced/deleted old bytes can leave the retained-state
sum while still providing no negative credit against command bytes.

Enumerate the selected store's complete registered logical row inventory, not
just current heads, the selected run, reachable artifacts or rows shown in the UI.
Unknown tables/row variants, orphan rows, missing required ledger/proof/source or
ambiguous ownership fence the store. An unregistered producer cannot hide in an
unclassified-byte allowance. This defines a bounded logical custody perimeter;
it does not supply external-file, cross-store or cross-workspace ownership proof.
Ordinary session metadata and the separate burned-reservation namespace are not
moved into this store. Their lifetime, finite storage and authority still require
their own contracts and can independently block executable qualification.

## Admission, reload and exact retry

1. Resolve current operation-specific authority, exact source/reservation and
   required old state under the accepted producer/ordering contract. Preparation
   remains excluded from the #744 claim-first exception.
2. Validate the bounded retained inventory and reconcile the existing preparation
   ledgers with exact ownership. Any missing, over-limit or ambiguous state is
   unavailable; never treat missing rows as zero or fabricate a new epoch.
3. Derive the exact complete effect under the existing command recipe, including
   the new proof, final header, every quota update and both summary branches.
4. Apply the effect to the captured logical state and compare all proposed final
   preparation, count, proof and whole-store totals. Validate complete-command
   bytes/changed rows independently, including all deletion effects.
5. Revalidate the captured authoritative state and operation-specific cutoff
   under the eventual qualified native ordering; commit the complete old-to-new
   transition atomically or preserve the old state. No split admission or later
   artifact write is permitted.

Initialization must fit with its nine workspace zero ledgers and first proof.
Draft preparation needs an unused retained-run slot and exact final byte capacity.
Admission keeps the run count unchanged; cancellation decrements it only for an
unadmitted exact draft under the accepted checks. Neither command resets unrelated
ledgers, removes old run epochs or transfers an occurrence to a selected run.
The summary-fitting and never-initialized branches both charge their complete
actual rows to whole-store totals; a never-initialized marker is never a free
source of future summary capacity or a lazy-initialization grant.

An identical authorized retry of a retained admission is a read and adds zero
rows, bytes, proofs or run slots. Run current authorization/context checks before
retry lookup as already required. At an exactly full valid budget it can return
the original receipt. This does not permit a read to ignore corruption, incomplete
inventory, missing proof, unavailable authority or an over-limit store. No source
means no receipt reconstructed from a digest or burned sequence.

Reload derives these same totals from full exact rows before complete hierarchy
availability. Use an overflow sentinel at 8,193 logical rows and 4,097 proofs;
stop bounded reads on byte overflow, never accept a truncated sum. These are
logical scan bounds only. Native enumeration/index coverage, visited-row counting,
query preparation, scratch, interruption and reload/recovery costs remain
unqualified; fitting these counts does not prove the inherited visited-row budget.

## Retention, exhaustion and recovery

No TTL, LRU, background deletion, source truncation or automatic rollover creates
capacity. Pending-draft discard uses current separately authorized workspace UI
scope and may discard stale review bytes without reviving old eligibility. It
still needs a complete valid atomic cancel command and one new proof. If that
proof or any independent bound cannot fit, cancellation is denied and retained
bytes remain. Revocation of review authority is independent and stays effective;
a failed deletion never restores authority. No protected cleanup reserve or
promise of successful disk cleanup is introduced.

In this profile, retain every admitted source, artifact occurrence and initial
membership, and every storage proof until a separately accepted purge/pruning
protocol exists. Closure, ordinary session forgetting, archive, catalog retirement,
changed selection, timeout and a new run epoch do not release these charges.
The profile must not claim that a full store can recover capacity by deleting a
closed run: closed-subject/incoming-dependency/revision/shared-custody purge gates
remain unimplemented and independently required.

A full proof or whole-store budget denies new durable mutations, including a
cancel needing a new proof. Existing valid read/history access remains subject
to its original authorization/availability checks; ordinary stop/PTY controls do
not wait for hierarchy SQL. No automatic new store/epoch, old-store unlink,
quota reset or successful-write fallback is allowed. Any future explicit
replacement/export/recovery process needs a separate reviewed contract; this
document does not promise a recovery path beyond preserving evidence and
reporting the capacity limitation. Corrupt, ambiguous or over-budget stores stay
unavailable and retain their original bytes for separately authorized recovery.

The existing storage contract permits safe removal of unreferenced historical
proofs without recursive chain replay; this profile deliberately does not use
that permission. Validate each retained row/link and proof under the inherited
contract. Do not turn the finite witness's complete predecessor chain into a
generic production requirement, and do not reconstruct forgotten acceptance from
hashes. No new authenticity or same-user rollback protection is claimed.

## Boundary witnesses after acceptance

| Case | Required oracle |
| --- | --- |
| Run/workspace preparation total equals its cap; next occurrence adds one byte | Exact fit allowed if all other checks fit; one-byte overflow denies complete command |
| 31 retained sources plus one pending draft; attempt a new draft after admission | Admission keeps count 32; another distinct retained run is denied, including a new run epoch |
| Workspace has fewer than 32 runs but 128 MiB preparation usage | Count availability cannot override byte denial |
| Draft fits but source plus initial membership exceeds remaining byte budget | Admission denied; exact old draft/artifacts remain; no partial source/parent/summary |
| Two equal artifact contents under different IDs or runs | Both occurrences charged fully; no shared owner or digest discount |
| Proof count 4,095 then one successful command; next cancel | First may reach 4,096; next mutation denied even if it would delete all preparation bytes |
| Complete new proof exceeds remaining proof-byte space | Deny atomically; do not omit manifest entries or prune old proofs |
| Whole-store total fits projection caps but full wrappers/links exceed 512 MiB | Deny; excluded commitment links are included in complete retention |
| Zero ledger or never-initialized marker pushes row count from 8,192 to 8,193 | Deny; zero usage/absent SummaryRow payload does not make a logical row free |
| Exact authorized receipt retry at an exactly full valid store | Return original receipt without new mutation; revoked context, corruption or over-budget state still fails |
| Missing ledger, orphan artifact, unregistered row or changed inventory during capture | Unavailable/revalidate; no partial sum, inferred zero or accepted stale total |
| Repeated prepare/cancel with zero admitted sources | Proof history grows to finite exhaustion; burned identities remain burned |

These are proposed future logical oracles, not executed tests. Use independently
generated boundary bytes and a separate consumer after scoped acceptance, with
new catalogs so earlier goldens remain exact. Maximum-length source/key/wrapper
and proof-manifest examples must justify fit or explicit denial; small synthetic
measurements cannot qualify maximum profiles. Native fault/space/query/authority
and recovery evidence remains a separate deliverable.

## Owner review and remaining gates

Review the 8/128 MiB preparation limits, 32 retained runs, 64 MiB/4,096 proof
limits, 512 MiB/8,192 complete logical limits, derived accounting without new
ledger rows, and explicit no-pruning/exhaustion behavior as named choices.
In particular, approve or revise the usability cost that repeated cancellations
can eventually deny even further discard, and that retained closed runs consume
capacity without a supplied purge path. Merging a proposal only stores it;
it does not record acceptance of these choices.

This bounds initial-profile custody only. Full #744 assignment-record/payload
aggregate defaults, artifact/report/plan/shared-learning producer custody,
external reservation history, complete purge/pruning/recovery protocols and later
run/parent lifecycle remain separate no-go gates. #740 stays deferred/no-go until
November. Native excluded-action ordering, physical/query/resource/durability
qualification, broader component review and separately approved executable plans
remain prerequisites. Keep the component issues open.
