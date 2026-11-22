---
type: Design
title: Complete Codex capture inventory and custody
description: Proposed first-profile resource inventory, retained custody and shared configuration owner for ordinary-review Codex capture.
tags: [orkworks, review, codex, custody, source-contracts]
status: draft
---

# Complete Codex capture inventory and custody

**Proposed for owner approval; documentation and source inspection only.**
Track [#1966](https://github.com/Rambolarsen/orkworks/issues/1966), under
#68/#66/#1899 and #738/#741. Inspected source base:
`12d4c93c` (main after #1959). No actual capture, installed hooks/native probe,
mutation enrollment, production role or prepared Start is enabled here.

Reuse the accepted [configuration leaves](2026-11-20-review-configuration-leaves-design.md),
[isolation evidence](2026-11-20-review-codex-isolation-design.md),
[owned-fragment projector](2026-11-22-review-codex-fragment-design.md) and
[source/admission contract](2026-11-19-review-source-qualification-design.md).
This proposal supplies the missing actual-inventory/owner design; their existing
byte codecs, 30-second preview lifetime and separate admission gates do not change.
The [qualification plan](../plans/2026-11-22-review-codex-capture-qualification.md)
is staged evidence work, not permission to activate the whole producer.

## Outcome and selected first profile

A future private producer may publish an integration leaf only from a complete,
code-selected inventory and retained live owners. A client cannot select expected
resources, supply currentness facts or turn Installed into authority. Failure is
unavailable/changed, with no repair, mutation, retry or ordinary-launch fallback.

Propose one first profile: stock canonical Codex, explicit qualified model,
direct macOS arm64, source-owned standard POSIX commands, packaged sidecar assets,
active workspace selection, exact installed owned hook bundle and matching
historical hook observation. The existing stock catalog requires the user harness
document to be physically absent. Present even-empty documents remain unavailable.
Wrappers, custom definitions/paths, developer source-directory fallback, Windows,
Linux, legacy/no-option versions, resume and native app-server launches are out.
These are my proposed availability choices, not prior owner decisions. Manual
launch/install/repair/remove behavior retains its existing broader support.

Packaged-only is deliberately narrower than current `scripts_source_dir`, which
chooses executable-adjacent scripts if `is_dir`, otherwise manifest scripts.
Capture never runs that ambient selection and calls its result custody. It needs
a retained running-build/package owner selecting that exact packaged root and
source asset bytes. No package-signature or executable-custody proof is inferred
from the checkout, a version string or a content digest.

## Complete leaf inventory

The compiled profile owns the following exact lexicographically sorted records.
Every item has presence **required** and state **regular**. Required means the
native fragment must be complete as well as its physical carrier being regular.
No caller-supplied inventory may weaken, omit, duplicate or add an item.

| ID | Role | Owner-selected path | Bytes in leaf |
| --- | --- | --- | --- |
| codex-hook-fragment | native_owned_fragment | workspace/.codex/hooks.json | Exact canonical output of the #1959 standard POSIX projector |
| installed-metadata-helper | installed_asset | home/.orkworks/hook-scripts/session-metadata.py | Complete original file bytes |
| installed-reporter | installed_asset | home/.orkworks/hook-scripts/report-harness-event.sh | Complete original file bytes |
| source-metadata-helper | source_asset | packaged-scripts/session-metadata.py | Complete running-build source bytes |
| source-reporter | source_asset | packaged-scripts/report-harness-event.sh | Complete running-build source bytes |

Source and installed bytes must match pairwise and the source bytes must match
the selected immutable running-build asset owner. Required assets need the
profile's readable/regular readiness checks; reporter execution is through the
source-owned shell command, not permission to execute an arbitrary installed file.
Executable mode alone is neither necessary proof nor source identity.

The hook leaf path names its physical carrier, while bytes represent only the
projected owned fragment. Retain a separate private carrier receipt to recheck
that relation; never pretend projected bytes are the entire file. A physical
missing carrier and projector `None` both deny this required item, with distinct
internal categories. Malformed, partial, customized, namespace-ambiguous and
oversized carriers deny. Unrelated native configuration remains ambient; it is
never copied to a leaf, preview, error or persistent record.

Five is the complete **leaf resource set for this narrow profile**, not the whole
operating-system/native read set. Catalog source/absence, typed active selection,
trust observation, home/root/executable owners, isolation/native/model evidence
and contextual helper policy are mandatory separate joins below. Omitting them
cannot be excused by calling the five-item list complete. Future changes to the
handler's additional assets or owned command template require profile review and
new fixtures before availability, rather than silently ignoring a sixth input.

## Typed inputs outside resource bytes

- **Catalog:** exact live HarnessStore/catalog/build owners, complete embedded
  source bytes, proved absence of user document, successful stock resolution and
  the selected base command, all under the accepted catalog leaf contract.
- **Workspace:** exact workspace lease/metadata owner, generation and canonical
  root; typed sorted active IDs/revision, with Codex active and no unknown/retired
  IDs. No whole workspace-memory export and no read-error-as-default path.
- **Roots:** retained canonical workspace, effective home, stable script root and
  packaged source root. The future prepared environment must bind the same home
  consumed by literal `$HOME` commands. Neither inherited HOME nor CODEX_HOME may
  silently retarget it after capture; the first profile rejects an override it
  cannot bind. Scalar UTF-8 canonical path limits remain 4 KiB.
- **Readiness:** opaque reference to the exact hook-observation owner plus typed
  valid expected fingerprint and its live revision, bound to this shared epoch.
  Missing/invalid/unreadable observation denies; no arbitrary observation JSON
  enters a resource. Matching historical fingerprint is only the existing
  readiness prerequisite, not current-session execution, user acceptance,
  executable custody or native-role qualification.
- **Native/isolation:** exact executable/custody, version/model/profile evidence,
  workspace and base/final command bindings required by #1946. The completed
  Advertised parser enum is insufficient. Discovery occurs only through a
  separately authorized qualification operation before capture, never on preview,
  refresh, status or Start. This proposal creates no such operation.
- **SessionStart context:** the actual reporter invokes adjacent metadata helper
  `instructions` or `worktree-instructions` according to the API/storage marker.
  `promptHook:disabled` does not disable this path. Before producer availability,
  qualify an exact code-owned context policy and the late allocation/marker inputs
  against the accepted one-turn instruction boundary. Do not suppress the helper,
  choose a fallback or claim an empty context. This remains a blocking evidence
  obligation, not a new permission or an inferred completed policy.

Readiness/context references remain private live joins, not new leaf fields or
renderer booleans. Their changes advance the same owner epoch. No public schema,
extra digest domain, raw-trust cache or secret value is introduced.

## Nonmutating observation and retained custody

Do not call status/inspect, `with_locked_snapshot`, install, reconcile, asset
resolution fallback or a native command. Store leases can create lock files;
inspection may revoke other authority. A new private read-only observer is needed.

The observer takes only the compiled profile and owner references. It traverses
from retained roots by directory-relative, no-follow operations, rejecting symlink
components, nonregular files, unreadable paths, hard-linked regular assets/carrier
and changed root/file bindings. Lexical containment or canonicalize-then-open is
insufficient. Reject source/installed root aliasing and unexpected home/workspace
root substitution. Exact opened-file identities and parent/name bindings are
retained with the result; path strings and inode/stat equality are not authority.
Any observed replacement or mismatch invalidates the affected preview permanently.

For allowed absence of the stock user harness document, retain the existing
ancestor directory custody plus missing suffix and freshly recheck it. Only
NotFound under validated parent custody is absence. Permission, malformed path,
busy/timeout, symlink and I/O failure are unavailable. No directories are created.
A later appearing document denies; same final absence cannot restore an already
invalidated preview. This absence witness is separate from the five required items.

Read complete bounded bytes from the opened object, not a second path open. Check
identity/size/change indicators around reads and revalidate parent/name bindings.
Carrier bytes are transient, projected once, then dropped; its retained receipt
contains custody/projection relation, no carrier copy or whole-carrier digest.
On status/new validation, reread bounded carrier and compare the projection while
checking the receipt. Other files require complete fresh byte equality with their
retained source-owned expectations. An unrelated carrier edit may conservatively
invalidate on an observed identity/change indicator even if projection is equal.
No promise of atomic exclusion against uncooperative external writers is made.

Proposed physical observer mechanism: one private same-sidecar worker process per
capture/validation, receiving retained root descriptors and a fixed compiled
operation selector over a private bounded channel. On macOS it returns opened
resource/directory descriptors via descriptor passing plus bounded typed result
bytes; the parent owns them before the worker is reaped. No caller pathname list,
public command mode, shell command, external harness or arbitrary file reader.
The worker must prove no-follow traversal and same-object descriptor transfer;
a summary of metadata alone cannot qualify custody. This new mechanism is an
explicit qualification prerequisite, not a claim existing process helpers supply it.

Proposed per-operation caps: one worker, three-second wall deadline, 256 KiB per
file including carrier, 64 KiB embedded catalog, at most 16 retained directory/file
descriptors including absence/root custody, and 2 MiB total raw file reads across
a single capture or validation. Count retries/EOF checks against that total;
there is no implicit retry. Bound channel result at 1 MiB before decoding and
reject descriptor count/type mismatches. The five-file maximum raw input is
1,280 KiB; catalog and bounded rechecks must also fit the 2 MiB total. Accepted
leaf caps independently remain 128/512/640 KiB and 16 KiB projected output: deny
aggregate overflow instead of assuming per-file fit. Whole-tree native scanning,
raw helper output and additional native probes are not part of this observer.

At most four observer operations may exist process-wide; saturated requests fail
fast. Cancellation/timeout closes the publication path and terminates/reaps the
worker; retain its permit, descriptor/byte reservations and owner identity until
cleanup is confirmed. Unconfirmed cleanup cannot publish and blocks replacement
work in that slot. No queued I/O or OS wait under terminal-input/application
locks. These proposed physical bounds require actual macOS fault/stall/cleanup
proof; logical fixtures cannot attest them. Existing preview entry/aggregate caps
also apply. Propose a process-wide cap of 256 retained descriptors across all
observer/draining operations and preview custody bindings. Reserve 16 before
starting each observer, transfer the actual charge on publication, and release
unused reservation only when ownership/cleanup is confirmed. Saturation denies
without evicting another preview; this may reduce available previews below the
logical cache count. Drop bindings on invalidation, expiry, workspace replacement
or shutdown, without extending preview lifetime.

## One shared checked owner

Propose a process-local configuration capture owner spanning global catalog plus
workspace integration dependencies. It has a UUIDv4, a checked positive u64 epoch,
a short in-memory admission fence and retained owner references. Every relevant
controlled writer enrolls **before its first possible effect**, increments the
epoch, marks the owner unavailable while in flight, releases the short fence,
then does its existing I/O. Concurrent capture never waits for that writer.

Successful completion does not renew old captures. Failed or uncertain publication
also leaves old epochs invalid. Uncertainty keeps availability denied until an
explicit qualified recovery validates all owners; successful readback alone is
not recovery proof. Overflow permanently denies until process/owner replacement;
replacement invalidates all old references rather than recycling identity.
Conservative global invalidation is acceptable, including unrelated catalog edits.

| Controlled input | Writer/effect that must enroll | Current source evidence / gap |
| --- | --- | --- |
| Catalog inputs and Arc publication | mutate, migration/recovery, startup load/replace | HarnessStore writes before catalog publication; its content revision and file lease do not implement this owner |
| Active selection/workspace | metadata active-ID writes, restore/cutover, workspace replacement | Workspace metadata owner/revision exists; shared capture enrollment is missing |
| Hook carrier/assets | install, repair, remove, unreferenced cleanup, source/stable root replacement | Codex OwnedAction retains workspace lease and integration claim, not shared catalog epoch |
| Trust/readiness | every observation write and clear, including failed publication | Hook Store has busy flags and clear epochs; ordinary writes do not advance a capture-wide checked epoch |
| Build/home/executable/model/context | owner replacement, qualification invalidation, effective environment policy change | Independent exact owners/evidence must enroll; path/version equality cannot substitute |

This table is a coverage obligation, not an assertion every writer is enumerated
or integrated already. Before implementing enrollment, audit every source caller
including legacy/session-report, SQL producer, startup/recovery and cross-workspace
paths; unresolved bypass keeps the producer unavailable. External changes are
noticed through fresh observations; unobserved external ABA remains outside the
accepted history guarantee. Observed A→B→A always invalidates the old preview.

No writer acquires the new fence while holding a legacy store/workspace/observation
lock. Acquire a nonblocking owner mutation token first, release the fence, then
enter the writer's existing lane. Capture/final admission only compare immutable
references, revisions and prevalidated receipts under the fence; they do not
acquire those lanes. Multiple relevant owners must use one documented enrollment
order; until that concrete caller audit proves no inversion, enrollment is gated.

## Capture, status and future admission

1. Capture snapshots exact owners/epoch and typed values under the short fence;
   any busy/uncertain owner denies. Slow bounded observation and hashing happens
   outside application guards. Join the independently pinned five-item inventory,
   exact projector/source pairs, catalog resolution, typed readiness/context and
   prequalified isolation/native evidence. A missing gate denies the whole result.
2. Final publication compares those same owners/epoch and completed receipts under
   the fence. Controlled writers cannot enter between that check and publication.
   No disk recheck occurs inside the fence. External evidence is a fresh bounded
   observation with the stated residual race, not filesystem immutability.
3. Retain only bounded configuration/digests and live opaque custody/evidence
   bindings in the accepted preview owner. Raw leaves and transient input/decoded
   carrier are dropped after hashing; no second cache, export/log/store or tokens.
4. Status and Start repeat fresh bounded validation and compare the original
   owner/epoch without discovery or repair. Changed/unavailable invalidates the
   preview; nothing extends its original 30-second expiry or silently refreshes it.
5. Future admission serializes its final controlled-source check and first
   in-memory creating-session mutation through the same enrolled owner fence and
   accepted source/artifact/admission fences. Durable reservation precedes this
   per #1919. Disk flushes and deferred spawn stay outside hot locks; if the
   actual creator cannot prove this ordering, it remains unavailable. Consume
   frozen exact command and route proof; no ordinary-creator fallback or new probe.

These ordering requirements do not qualify the physical admission registry,
prepared creator, late reporting/mailbox allocation or external executable reuse.
Those remain separate gates even after an observer implementation passes.

## Alternatives, compatibility and review outcome

Reject whole-carrier retention, generic caller inventory, reuse of Installed,
file-watch-only freshness, a new independent trust epoch, or native probing from
preview. Each omits custody, leaks unrelated inputs or changes approved inspection
semantics. Retained descriptors plus bounded reread are chosen over path/stat-only
receipts; descriptor passing and macOS stall proof are the cost to qualify first.

No code, metadata migration, dependency, installed asset, user file or active
session changes in this task. Rollback removes the proposal links; future observer
or owner implementation remains dormant until all gates pass. Existing/manual
paths keep their semantics; failure to qualify a proposed observer is availability
loss for this optional future profile, not reason to narrow existing launch support.

Investigated uncertainties: source asset fallback, helper's additional context,
trust write/clear asymmetry, and write-before-catalog-publication all change the
boundary above. None is counted as already solved by fixture codecs. Proposed
hard-link refusal, packaged-only availability and physical caps require owner
approval. Current disposition: **Design for review; production Investigate.**

## Fresh review — 2026-11-22

A fresh builder review found nothing important left to ask before planning the
staged scope. An independent contract review found no required or optional issue
against accepted parent contracts and inspected source. Both reviewed the added
256-descriptor aggregate reservation/transfer rule. They explicitly declined to
claim physical observer, contextual/native evidence, enrollment or prepared
admission qualification from this documentation. This is review evidence for
owner consideration, not owner approval or production capability evidence.
