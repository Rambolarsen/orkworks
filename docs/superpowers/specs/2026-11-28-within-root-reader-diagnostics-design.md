---
type: Diagnostic Contract Proposal
title: Bounded checkpoints inside the MSVC reader root
description: Proposes an isolated sparse reader wire domain and owned-fixture verification without renewing any installed-input allowance.
tags: [orkworks, hierarchy, qualification, windows, diagnostics]
status: draft
workflow_status: proposed-awaiting-owner-decision
---

# Bounded checkpoints inside the MSVC reader root

Tracking [#2263](https://github.com/Rambolarsen/orkworks/issues/2263), a scoped
prerequisite of [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
This follows the [source audit](../../validation/msvc-reader-source-audit.md)
merged in #2258 and the
[spent live-reader package](2026-11-27-live-reader-integration-design.md#accepted-package-terminal-state-2026-11-28).
Inspection base 22c86d0124fd549cfe15d48c30ba744eaef9fd34.
All choices below are the author's proposal, not owner acceptance. Publishing
or merging this document authorizes no code or execution. All earlier allowances
remain spent; the terminal unavailable result stays unchanged.

**Requested future owner decision:** accept the isolated diagnostic tooling,
new representations and finite owned-fixture verification below. This decision
would cover only implementation and those owned synthetic fixtures. A real
installed-input/candidate attempt needs a separate concrete finite proposal and
owner decision after tooling verification. No dormant live workflow or new
candidate attempt is included in the requested tooling acceptance.

## Purpose, alternatives and limits

The last validated before-reader row was root-begin/msvc. It precedes the root's
initial path checks; its zero file/directory counters do not say whether any
later work completed. The source contains repeated ancestry/identity calls,
payload hashing and row encoding, but source counts do not identify runtime
costs. The proposed checkpoints distinguish early completed boundaries and a
few later completed-row milestones, while retaining the same input/custody work.
They are diagnostic self-reports, never partial input admission.

Keeping the terminal unavailable outcome is valid. A checkpoint file polled by
the parent would introduce extra I/O, race/custody rules and repeated hash work.
Unbounded heartbeats or per-file logging would consume an uncontrolled report
budget. Removing checks, caching by path, parallel reads, changing selectors or
raising the 120-second manifest timeout would answer a different question.
The proposed fixed sparse sequence uses the existing stdout transport and
explicitly changes only its new domain's inner representation/cardinality.

This is not continuous profiling. A long gap after an initial checkpoint or
between milestones still cannot identify the active file, operation, CPU time,
I/O latency, RSS, throughput or the older attempt's cause. Directory-heavy walks
and stalls after the first file remain coarsely observed. No raw path, filename,
file identity, payload, rejected value or child wall-clock timestamp is logged.
A future unavailable result can remain inconclusive; it is not permission to
add samples or repeat an experiment.

## Isolated tooling and compatibility

After acceptance, add new scripts with the stem
scripts/qualification/pair-root-diagnostic-windows-build: controller .mjs,
-protocol.mjs, reader .ps1, exact policy .inputs.json, SDK wrapper -sdk.py,
owned tests .test.mjs and -fixtures.ps1. Read-only pure guarded helpers may be
imported and sealed as in the previous package; never invoke a spent main,
dispatch guard or runner. Keep old qualification, synthetic and v2 files and
allowlists byte-identical. No dependency or interpreter installation is added.
Existing-file exceptions are explicit LF rules for the new files, one five-minute
direct Node test step in the normal sdk-controller-fixture CI job, and necessary
ADR/documentation/index updates. No new live/manual qualification workflow.

Use new request domain reader-root-diagnostic-v3 and profile
pinned-windows-root-diagnostic-v1. The exact six-element request is
[3,profile,mode,nonce,hash_grant,policy_sha256], at most 4 KiB, with modes
preflight/before/after, fresh lowercase 32-hex nonces and the same strict
canonical safe-integer, exact-key/byte and non-reuse checks. No arbitrary paths,
root dictionaries, sampling knobs, alternate strategy or limit overrides.

Copy exactly the frozen 43,200-byte policy, SHA-256
1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6,
from [d870796b](https://github.com/Rambolarsen/orkworks/blob/d870796b634a0f8259d0c418fb0455ea0079a8a1/scripts/qualification/pair-progress-windows-build.inputs.json).
No pin refresh or installed availability check is implied. Before implementing,
compare the four repository source pins from Git bytes (all four matched at
this proposal inspection base); mismatch blocks the
unchanged candidate route and must be reported, not repaired automatically.
Complete ancestry/held-name identity, metadata, archive/member/package/helper,
PE/preflight, source and before/after equality checks remain mandatory. Retain
one exclusive 1 MiB payload buffer and every repeated verification/debit pass.
The MSVC manifest still calls ordinary file sealing without PE/text capture.

Use the existing fixed PowerShell/Python paths, minimal environment and fixed
argv forms from the accepted v2 contract, replacing only own filenames and
correlation markers with the new stem. New owned scratch is
RUNNER_TEMP/ork-pair-root-diagnostic-windows-build-2026-11-28, exclusive and
refuse-on-reuse; request/output names stay requests/NONCE.json and
outputs/NONCE.json. Native setup, root derivation and mode-specific admission
retain their old ordering. Source seals include every new own/imported source
and exact policy; no source pass may exceed the inherited combined 1 MiB cap.

The tooling has no live dispatch path: command-line live entry refuses before
creating scratch, probing installed inputs or spawning a child. Only the owned
fixture entry can call guarded components with literal fixture roots/fakes.
A later finite-attempt proposal must name the exact workflow/guard/history and
freeze changes that would enable live entry. This prevents tooling acceptance
from being interpreted as an unspent candidate allowance.

## Exact wire and phase state machine

A v3 record is exactly ten elements:
[3,nonce,seq,kind,root,h,f,d,a,detail]. JSON uses canonical printable ASCII plus
one LF, no BOM/CR/whitespace/escapes/extra keys or fields. All numeric elements
are canonical nonnegative safe integers. seq begins at 1 and increases by one.
Root codes 0–6 mean msvc, registry, rust, sdk_bin, sdk_include, sdk_lib, source;
7 means no root. Kind codes are fixed:

| Kind | Meaning | Root/detail |
| --- | --- | --- |
| 0 / 1 | start / ready | 7 / 0 |
| 2 / 3 | archives-begin / archives-end | 7 / 0 |
| 4 / 5 | root-begin / root-end | Current root 0–6 / 0 |
| 6 / 7 | serialize-begin / serialize-end | 7 / 0 |
| 8 | ok | 7 / 0 |
| 9 | cooperative error | 7 / reason code 1–8 |
| 10 | MSVC sample | 0 / sample code 1–11 |

Error codes 1–8 map in order to bad-request, input-denied, hash-limit,
input-limit, identity-drift, format-denied, output-limit, reader-unavailable.
Binding failures before the request is admitted emit no invented tuple.

The structural before/after sequence retains the existing 21 meanings: start,
ready, archives-begin/end, seven ordered root-begin/end pairs, serialize-begin/end,
ok. Preflight remains the same five structural boundaries and admits no samples;
its f/d/a remain zero. Code translation changes no existing boundary's meaning.
Only kind 10 may interleave, and only inside the msvc root-begin/end pair.
A cooperative error replaces any remaining suffix and is terminal. At most
32 total records include that error; it never adds a 33rd row. No record/byte
follows either terminal. Unknown version/profile/code/order/field denies.
Legacy v1, v2 and single-object readers reject v3; the new decoder rejects them.
No parser falls back or coerces a numeric code to an old phase string.

The MSVC sample sites are fixed, each emitted at most once per invocation:

| Detail | Emit only after this point completes | What the next silence still includes |
| --- | --- | --- |
| 1 | Initial Assert-PlainPath and root stack push | First directory identity and later work |
| 2 | First popped directory's Get-DirectoryIdentity returns | Directory row encoding/retention and enumerator advance |
| 3 | First root-directory enumerator yields its first entry, before entry classification | Path/attribute checks, traversal and selecting the first ordinary file |
| 4 | First ordinary MSVC file has its first held handle information checked against expected size/type, before hash-state/lease work | Payload hash, subsequent identities and row work |
| 5 | That file's payload digest is complete and hash/lease released | After/current identity and drift checks, close and row work |
| 6 | That Read-SealedFile call returns after all checks and stream disposal | File-row precharge/JSON/retention |
| 7 | That file's complete row is retained and LiveFiles incremented | All remaining walk work |
| 8 / 9 / 10 / 11 | Completed MSVC file-row count reaches exactly 16 / 256 / 4,096 / 65,536 | Every operation until the next milestone or root-end |

Samples never begin an operation or claim it completed early. Detail 2 does not
increment d: it is identity completion before row admission. Detail 5 does not
increment f: sealing/row checks remain. All h/f/d/a values use the exact existing
logical/completed-row snapshot definitions; a counts verified packages. Because
msvc is the first manifest root, f at its row milestones is the MSVC completed
row count. h includes prior policy/archive/encoding work and is not payload
throughput. Keep all monotonic/range constraints from v2 unchanged.

After detail 7, samples 8–11 emit exactly at their counts with the corresponding
f value; do not round, clamp, backfill or repeat. After detail 11, the checkpoint
budget is exhausted and the walk continues unchanged to root-end with no more
samples. There is no saturation-success flag, fabricated terminal or quota
renewal. Empty directories/files use the same sites: no yielded entry means no
detail 3; no ordinary file means no details 4–11. Successful output validation
must independently require every sample whose condition was reached, including
1/2 for the nonempty directory walk, 3 when the root had an entry and 4–7 when
any ordinary file was sealed. Sample omissions on failed prefixes cannot be
inferred from silence; an error may stop before any next site.

Samples form a contiguous prefix 1..N; no code may be skipped and followed by
a higher code. On successful output, derive required N from the complete MSVC
manifest: 2 for a root with no entries, 3 for entries but no ordinary files,
7 for 1–15 files, 8 for 16–255, 9 for 256–4,095, 10 for 4,096–65,535 and 11
for 65,536 or more. A root always contributes its own directory row; more MSVC
directory rows or any MSVC file row proves the root had an entry. During a
failed prefix, do not infer a site was reached from silence or require an
unreached suffix. Samples 1/2 require f=d=0; 3 requires f=0,d=1; 4–6 require
f=0,d>=1; 7 requires f=1; 8–11 require f=16/256/4,096/65,536 respectively.
At root-end or a cooperative error, refuse known settlement if reported
completed counters prove a required sample was skipped. All error prefixes
remain diagnostic and source verification still precedes any known settlement.

## Report, retention, admission and settlement bounds

The new inner limits are explicit owner choices: **32 records × 128 bytes
including LF = 4,096 bytes** per reader. The old v2 21-record/192-byte limits
remain unchanged. V3 tightens the line ceiling and increases only its own record
cap; there is no outer stream or report increase. The maximum numeric tuple
fits 128 bytes, including nonce and LF; fixtures must carry independent literal
maximum-width goldens rather than trusting encoder/decoder agreement.

At most 127 unframed bytes are retained. Stream-charge every observed chunk
before parse, even malformed/late/overflow bytes, and retain no raw stderr or
rejected line. Require empty stderr. Retained immutable rows are
[parent_elapsed_ms,tuple], using the one parent monotonic clock, not a child
clock. Each row must fit 144 UTF-8 bytes. Five preflight plus two 32-record
manifest invocations give 69 rows: **69 × 144 + 2,048 = 11,984 bytes**, below
18 KiB for the complete trace/status aggregate. Cap all non-row trace/status
metadata at 2,048 bytes in aggregate across invocations, retaining the previous
closed status fields and finite reason vocabulary. Independently enforce the actual
18 KiB aggregate and 64 KiB result limits; do not infer they fit from the row
arithmetic if metadata changes. No new file or artifact slot.

Feed/retain before awaiting close, including a failed read. Decoder failure
latches and cannot be cleared by a late valid tuple. Deadline crossing during
spawn/data/close denies; no new active work after failure. No timer reset at a
root/sample. Preflight keeps 60 seconds; each manifest keeps 120 seconds for
startup/root admission/archives/all walks combined, clamped to the same active
job deadline. Keep the single owned kill, shared drain deadline and unknown
termination/scratch outcome rules. A live attempt remains absent, not newly
granted by these carried-forward limits.

Use the same 32 GiB cumulative work pool and reserve arithmetic G=R−132 MiB
for one outstanding reader, with 129 MiB parent admission subledger and protected
3 MiB final tail. Do not refund prior source passes or reset pools. Progress JSON
conversion is bounded control work as in v2, not a new payload hash or a claim
that h measures CPU/encoding cost. Streams count actual wire bytes; source,
policy, output and result hashing/precharges stay in the shared work ledger.

The complete output is a new strict envelope with schema=2,
evidence_kind=root-diagnostic-reader-output, mode, nonce, policy_sha256, data
and metrics only. Retain every data/metric predicate and complete comparison
from the accepted v2 contract; a new envelope is not a smaller manifest. The
whole object stays within 64 MiB, precharged before full serialization. A future
candidate result uses schema=4/evidence_kind=root-diagnostic-sidecar-candidate;
old candidate/synthetic consumers reject it. The SDK subrecord remains its
unchanged schema 3 meaning and cannot admit the new candidate alone.

A known success requires strict EOF/expected close, disposed streams/lease,
all expected structural/conditional samples, post-close source seal verification,
held complete output read/hash/bind/admission, correct completed counters and
sample conditions against that data, then exactly-once settlement. A cooperative
nonzero error needs the same source verification before known h settlement;
there is no partial-output admission. Timeout, malformed/missing terminal,
source mismatch, rejected output or undrained process spends the whole grant.
Keep validated prefixes in bounded receipt-only failure publication; do not
salvage bulk artifacts, invent completed roots or make candidate flags true.
Runtime qualification, production pin acceptance and target executions remain
false/false/zero even for successful owned-fixture results.

## Proposed owned-fixture verification after acceptance

Extend [ADR 0123](../../adr/0123-synthetic-reader-diagnostics.md) before code and
write a concise implementation plan for this accepted scope. Do not amend the
source audit's runtime conclusions or supersede ADRs 0118/0119. TDD and a bounded
current-diff review cover new code; builder choices are private function/test
organization, not wire/domain/ceiling/authority changes.

Use the previous fixed fixture envelope only: 120 seconds active plus at most
120 seconds shared drain/publication, 64 MiB cumulative hash work, 1 MiB actual
child plus injected streams, at most 64 pure vectors/512 KiB injected/8 KiB per
vector, three PowerShell children of at most 30 seconds each, one final zero-hash
Node timeout child and one owned termination. At most 2,048 files/256 dirs,
4 MiB inputs/1 MiB per file, 8 MiB scratch, 64 KiB result/18 KiB traces. Model
large live limits with literal fake transport/state/output vectors; no per-case
pool reset, extra child, package fetch or installed selector. Normal CI can run
only these owned fixtures/fakes and its existing isolated SDK tests.

Required discriminating/refusal coverage:

- Independently specified v1/v2/v3 and output/candidate domain rejection; maximum
  tuple/arrival/result sizes and all 32 records within the unchanged report cap.
- Exact structural/sparse sample order, duplicate/wrong-root/preflight/unknown
  codes, conditionally absent samples, milestone counts and no samples after
  saturation; complete-output checks catch missing required sites.
- Literal owned file/directory trees distinguish path admission, directory
  identity, enumeration yield, first held file, payload completion, final seal
  and row admission. A controlled failing/blocked subject at each site retains
  only the prior validated prefix; it does not become a fixture failure if that
  refusal is the expected result. No real toolchain is used.
- Empty tree/file, directory-only root, nested first file, short reads, single
  lease and unchanged digest/identity/manifests; every legacy source/helper/
  archive/PE/encoding refusal remains covered, without widening old fixtures.
- Byte-at-a-time/coalesced/partial/CR/BOM/bad JSON, report/line/record/trace limit,
  late/error/after-terminal data and strict failed-close/deadline latching.
- Held source/output admission before known settlement; rejected output and
  missing terminal spend the whole grant once; three sequential grants use one
  decreasing pool and cannot consume the emergency tail. Receipt-only failure
  keeps the prefix and closes all later stages without a new child.

A future native/candidate proposal must select exact reviewed sources/policy,
commands and new workflow/history guard, state one finite allowance and every
outer ceiling, freeze after merge, then independently read back bounded results.
It must resolve source-pin drift rather than silently update inputs. Nothing in
this proposal promises installed availability, completion within 120 seconds or
an optimization. #1176 and all compatibility/bootstrap/native/recovery/durability/
activation gates remain open.

## Bounded proposal review

Root is the sole writer. One fixed read-only round, maximum two lower-cost Luna
reviewers: builder checks material missing decisions from this document;
correctness checks source/domain/order/bound arithmetic and authority. Both
read the merged draft, cite file/line evidence and mark native claims UNVERIFIED.
No writer delegation, live call, coding harness or second review round. Root
resolves source-backed/minor points; unresolved owner choices remain explicit
for the eventual concrete acceptance. Proposal publication is not acceptance.

## Proposal review disposition

One fixed read-only Luna builder/correctness round completed. The builder found
nothing important left; the correctness reviewer found no material contradiction
in source order, bounds, compatibility, settlement or authority. Root made the
already ordered samples explicit as a contiguous prefix, derived successful
sample cardinality from the complete manifest and stated counter predicates
and the 2,048-byte non-row metadata subcap. These close validation wording,
without changing sample sites or outer limits. No second agent round.

Independent static arithmetic encoded a maximum-field-width tuple at 79 bytes
including LF and its parent-arrival row at 88 bytes, below the proposed 128/144
ceilings; 32 × 128 = 4,096 and 69 × 144 + 2,048 = 11,984 < 18,432. This is
size arithmetic only; it imports/executes no reader or candidate controller.
Native behavior, installed capacity, runtime cost and historical cause remain
UNVERIFIED. No tooling, workflow or installed-input experiment ran in this task.
