---
type: Implementation Plan
title: SQL observation origin qualification plan
description: Approved strict record and captured final-scan owner qualification before SQL assessment input integration.
tags: [orkworks, sqlite, observations, provenance, plan]
status: draft
workflow_status: implementation-active
---

# SQL observation origin qualification plan

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [accepted original-owner contract](../specs/2026-11-08-sql-observation-origin-capture.md).
Base: `d4565340052e21d3de97128091908c513ee0d5b4`.
Pass condition: a captured final-scan observation survives segment rewrite and
SQL owner close/reopen with its exact original scope/value; stale/deleted/reused/
foreign or unproven data fails without reintroduction, backfill or byte loss.
Owner accepted after #1554 on 2026-11-08 with “go ahead.” Root implements
inline. The first branch delivers the strict codec/data reader only; private
producer, shared admission and deletion composition follow as a separate task.

## Investigated uncertainty

The creation retained by CapturedEnding disappears inside CapturedWriter and
StoredObservation. The legacy opener also truncates tails and rewrites bounded
segments before returning, so adding an optional Deserialize field alone cannot
establish preserved qualified input. Current idempotency keys hash only session
ID/key and payload, omitting original creation/epoch. A duplicate could otherwise
silently acknowledge unknown/foreign history as qualified.

Recommendation inputs are mutable and include inherited/target/completion/
rollup dependencies; readiness projects observations before hashing. Those are
real additional owners, not safe source-free inputs. This plan does not upgrade
them, remove them from assessment snapshots or pretend to complete SQL admission.

Alternatives investigated: current-row backfill is barred by accepted provenance;
per-observation sidecars add file growth and ambiguous two-file commit states;
reusing the tolerant legacy opener can remove the very bytes a strict reader
needs. Choose one tagged line and a separate checked SQL admission/strict reader.

Risk assessment: dependencies 4 (SQL, final-scan, file/deletion owners), blast
radius 3 (private file mode and legacy refusal), state changes 3 (new tagged data,
no existing rewrite), reversibility 3 (source-only opt-in, legacy refuses SQL),
uncertainty 4 (owner paths traced, actual integration/physical evidence pending).
Total 17/25. Unknown physical rollback remains a rollout gate, not a guarantee
from these source-level ratings.

## Deliverables after one owner acceptance

- [x] Record the owner's concrete decision and a dated ADR 0103 clarification
  before dependent code. Reuse format4 acceptance; do not ask about it again.
- [x] Deliver the strict durable observation boundary in `workflow_observations/`.
  Keep public observation values unchanged. Add private origin/tagged observation
  and tombstone codecs, canonical digest and finite bounds from the spec. Read
  copied complete segments without effects; explicit unqualified data is not a
  qualified input. Cover golden vectors, unknown/duplicate/null/missing fields,
  scope/value/filename mismatch, exact 2 KiB/64 KiB and existing 2 MiB accounting,
  corrupt/interrupted/oversized input and byte-preserving refusal. Pin independent
  strict-inventory boundaries (4,097 entries, 4,096 segments, 64 MiB aggregate reads,
  65,536 nonblank lines) including empty/tombstone-only/legacy segments, a bounded
  10,000-observation retained selection and the original 2-second scan deadline.
  Pin the 768 KiB selected-snapshot limit, single 64 KiB line buffer and full 1 MiB
  file-budget reservation without raising existing 32-job/32 MiB totals; exact
  and one-over snapshot cases refuse rather than shrink the selected set.
  At each boundary accept otherwise valid complete inventory; one extra entry,
  file, byte or line refuses with zero effects/no partial snapshot. Unknown or
  nonregular entries refuse; late scan settlement retains resources without
  delivering or launching a replacement scan. No production
  origin constructor or producer routing in this first independently reviewable
  deliverable. Existing legacy storage tests retain their assertions.
- [x] Deliver the canonical shared owner and original final-scan claim in
  `workflow_observations`, `session_store` typed reads and
  `session_application/sql_writes/lifecycle/history`. Add no public SQL interface.
  Distinguish newly captured before-inference origins from parsed data; original
  worker opening identity, exact creation, workspace/root/lease/runtime fences
  survive all waits. Cover header/epoch replacement with equal ID/creation,
  workspace/store/root/lease/runtime replacement, missing/tombstoned/reused rows,
  live-row/deletion-job Corrupt refusal and unchanged closed/archived eligibility.
  Original and pre-file checks use one bounded SQL read transaction. Cover legacy clones/independent
  opens, owner loss/reopen, Busy, unavailable/corrupt storage, read/byte/queue
  deadlines and cancellation before admission. No I/O under application guards.
- [x] Deliver private pure retry/eviction rules for complete record data before
  composing file writes. Qualified retries match original scope/key/payload and
  return the original receipt; legacy/foreign collisions refuse. Eviction copies
  the exact origin/digest of its own value and preserves legacy representation.
  Cover inclusive retry expiry, unchanged inputs, ambiguous duplicate receipts,
  existing confidence/payload semantics, and exact segment count/byte bounds.
  This is a data-only prerequisite: no original-origin constructor, counter/file
  effects, retained write grant or batch routing. Selected read snapshots omit
  tombstones; the future writer must supply complete bounded inventory.
- [ ] Compose origin persistence with the existing grounded at-most-five
  final-scan candidates and the SQL Workflow deletion stage. One opaque original
  grant covers actual file work and finite settlement, never a fresh post-wait
  lookup. Preserve sequence/rate/grounding/source checks and idempotency behavior.
  Cover qualified duplicate/tombstone retry, legacy collision refusal without
  upgrade, failed append/sequence gap, trim/rewrite origin preservation,
  source eviction with exact copied tombstone, admitted batch before delete,
  delete before delayed batch, partial delete, source recreation, and captured Workflow deletion identity replacement (epoch,
  creation/job identity changed or job disappeared/lost its tombstoned row after capture) refusing
  before any removal, counter or retry-state effect; same-job progress advancing
  remains eligible according to existing stage bits. Cover cancellation
  after file admission and unchanged non-target evidence/progress. Keep legacy
  methods unavailable on the admitted SQL owner; no generic bypass.
- [ ] Compose copied close/reopen fixtures through the strict reader and existing
  assessment source codec. A new qualified observation contributes its exact
  original pair/value to the accepted envelope; any mixed legacy observation,
  recommendation or readiness input refuses whole assessment admission. Do not
  add a production OriginalInputs constructor, scheduler, model call, pending
  publication, source-aware recommendation cleanup or startup/activation route.
- [ ] Update canonical domain/architecture/validation claims with actual evidence;
  run scoped regressions and `bash scripts/verify-repo.sh`, current-code manual
  review, required/routed CI and the normal PR/owned cleanup lifecycle. Source
  origin capture alone is not publication/deletion or installed-profile proof.

The two codec and owner/producer deliverables can be rejected independently;
execute sequentially and keep each within its own meaningful tests/review. Do
not split setup-only commits into tasks or introduce generic storage frameworks.

## Review disposition and next gate

Initial self-review: Ready for independent source/authority and builder checks,
then one owner decision. Every mandatory contract section maps to a deliverable
above. The review asks whether original identity survives the actual owner paths
and whether error/cleanup/retry states have an implementable refusal outcome.
Physical behavior, complete startup and production origin coverage remain
Unknown. Pending acceptance is a gate on implementation, not on this docs PR.


Review routing: root is the sole writer. At most two fresh read-only
`gpt-6-luna` reviewers at high effort, one round with bounded clarification:
one verifies source/authority claims against the cited owners, the other asks
which missing decisions would change implementation or qualification. Both
consume this single draft and cited source, not each other's conclusions. Root
reconciles their material findings before the one owner decision. No runtime
or physical evidence is inferred from a Ready document review.


## Independent review corrections

Source/authority review required explicit live-row/deletion-job inconsistency
refusal, in addition to original scope and creation. The contract now reuses the
accepted full read-transaction conditions at capture and pre-file validation.
Builder review required aggregate inventory bounds (including empty and
retry-only segments) and a replaced captured Workflow deletion-job test. Numeric
scan limits, no-partial-snapshot outcomes and exact-job comparison before effects
are now stated in the contract and owning deliverables. These are proposed
qualification choices; no existing acceptance or runtime evidence is extended.
Bounded correction follow-up found both source/authority and builder reviews
Ready for the owner decision. Source review also clarified that ordinary job
progress is not identity replacement. Root inspection of file_budget.rs found
its existing 1 MiB-per-job/32 MiB aggregate encoded allowance: the proposed scan
now reserves a full existing slot, caps selected encoded results at 768 KiB and
uses one 64 KiB line buffer without expanding that budget. This additional
resource-accounting clarification is included in the bounded builder follow-up.


Final bounded review disposition: both independent reviewers are Ready for the
owner decision after the source-row, aggregate budget and exact-job corrections.
Their final checks verified the existing encoded file-job limits and ordinary
same-job progress semantics against source. No material issue remains in the
reviewed proposal. Actual runtime/physical/profile behavior is unverified;
this result authorizes neither implementation nor production activation.


## Strict data prerequisite implementation

The first independently reviewable slice implements the immutable origin/tagged
record codec and strict descriptor-relative data reader. It changes no legacy
opener or producer path and adds no production origin constructor. Expected
scope comparison is data validation, not current SQL row validation or input
admission. The reader returns no I/O grant; the original shared owner remains
the next prerequisite under this already accepted contract.

All 18 focused tests pass. An independent canonical value digest, exact origin/
line/segment/aggregate/selected-result boundaries, copied reopen and no-repair
failures, original deadline and retained full-slot lifetime are covered. The
selected snapshot counts the compact JSON array of complete retained records,
including tags/internal hashes. Large byte/count fixtures extend only a test
request deadline to isolate counters; no throughput/physical-memory result is
claimed. The ordinary 2-second capture/deadline path has its own queued-expiry
regression. Other platforms return Unsupported instead of a path fallback.

Fresh manual medium code reviews cover codec/data authority and native reader/
lifetime. Review found an unterminated whitespace-tail acceptance bug; its
behavioral regression was red, EOF now requires no partial bytes, and the fix
was re-reviewed. A further deterministic growth regression exposed a second
metadata sample adopting growth after the segment-size check. The reader now
retains the original validated metadata sample; that regression is red to green.
Both independent review scopes are APPROVE, including the bounded growth-fix
follow-up. Final Rust formatting and normal build pass.

The full repository verifier has been run but has not passed locally: existing
native CLI-probe fixtures returned cleanup-related failures on macOS. The
18 focused tests pass; remaining repository checks and current-head CI evidence
are recorded in the PR. Source-currentness, original claims, shared admission,
legacy refusal, producer/deletion races and activation remain unqualified.


## Shared owner and original-capture qualification

The next branch qualifies canonical shared admission and the sealed original
final-scan claim. It retains the worker's checked startup epoch and native root
identity rather than adopting a later header/path. SQL source checks use one
bounded read transaction; source data alone grants no file access.

The legacy refusal preflight is separate from strict SQL inventory admission.
It streams at most 65,537 entries, 65,536 inspected lines and 64 MiB, with a
64 KiB line limit and an original 2-second deadline. This preserves the existing
10,001-segment legacy count fixture; it does not increase the strict SQL reader's
4,097-entry/4,096-segment limits. Workflow-directory links now refuse before
repair because a non-following preflight cannot safely inspect through them.
The original-root link behavior and non-target bytes remain covered separately.

Read settlement releases its session lane and file-operation exclusivity when
actual I/O ends. Returned immutable data retains its existing file-job capacity
and original owner/lease/worker. Source capture itself reserves a full existing
file-job slot, and late root checks retain that slot and lease until settlement.
Producer persistence, captured Workflow deletion and assessment composition are
not implemented by this prerequisite.


Current-code medium review is APPROVE across both independent read-only scopes,
including one bounded correction follow-up. Review found deadline gaps before
initial workspace capture, during candidate-held application checks and while
preflighting ignored directory entries. All three behavioral regressions were
red. Application checks now use nonblocking try-locks under the original
pending/cancellation/deadline state; expiry releases the candidate and lane even
while application locks remain held. Preflight checks its original deadline at
entry, around enumeration/native operations and before every successful return.
The corrected runs pass 15 deadline tests and 205 observation-related tests.

The first full Rust run found six old fixtures attempting legacy observation
opening on SQL artifact roots. Replacement fixtures now use independent
unqualified stores; the admitted-caller-loss fixture copies its ordinary legacy
bytes to preserve the cached-count assertions. The recommendation-only reopen
fixture uses an explicit unqualified observation slot and makes no Workflow or
whole-workspace adoption claim. Every original assertion remains. The affected
original-owner (59), Workflow-cleanup (7) and recommendation-reopen (1) runs pass.
The fresh complete verifier and exact-head CI evidence are recorded in the PR;
producer persistence and deletion/assessment composition remain pending.


The corrected complete local repository verifier passes: 3,308 Rust unit tests
(18 existing ignored), all Rust integration targets, reporter tests, desktop
native ABI/type checks/tests/build and documentation checks/build. The current
20 reviewed Rust file hashes match the verified source. Required CI and the PR
lifecycle remain recorded with the concrete PR, not inferred from this local run.
