---
type: Diagnostic Contract Proposal
title: Synthetic reader progress and buffer reuse contract
description: Proposed bounded phase framing, timeout retention and single-buffer comparison using only owned synthetic inputs, without renewing a qualification allowance.
tags: [orkworks, windows, qualification, diagnostics, resources]
status: draft
workflow_status: proposed-owner-acceptance-required
---

# Synthetic reader progress and buffer reuse contract

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: 18687b6a00c94f4492b683a8056c8114f2fc537e. The owner asked to
continue by defining this synthetic-only contract after the
[source diagnosis](../../validation/pair-reader-timeout-diagnosis.md).
**This document proposes new choices; publication is not acceptance or code
implementation authority.** One scoped owner acceptance would authorize only
the synthetic implementation and normal fixture CI described here. It would
not authorize a replacement live collector, installed-input inspection, package
fetch, compiler execution, qualification dispatch or an increase in any spent
allowance. The exact Windows timeout cause remains unproven.

## Outcome and boundaries

Obtain two things on owned fixture data: a retained, bounded last-known phase
when a child times out, and an apples-to-apples comparison of fresh versus
reused payload buffers. Preserve all file/archive/identity/membership/encoding
checks. A successful fixture proves this contract on those inputs; it is not a
candidate, SDK bracket, runtime/profile, production pin or Windows capacity
qualification. Gate 1 remains Investigate; all source/compatibility/bootstrap/
native/profile/recovery/durability/activation gates and spent allowances remain.

Author-selected proposal: fixed phase boundaries instead of a per-file heartbeat;
a compact canonical tuple instead of arbitrary log objects; one reusable buffer
instead of parallel readers, path/hash caching or omitted input roots. The fixed
sequence exposes startup, archive, root and serialization boundaries with finite
records. It deliberately cannot identify the current file, crate or exact progress
inside a stalled phase. Those limits are preferable to unbounded logging or an
unreviewed output channel.

Code additions are confined to new files under
scripts/qualification/pair-reader-diagnostic*: a Node parser/fixture driver,
PowerShell fixture worker and their tests. The sole existing code/configuration
file exception is
one bounded test step in the normal sdk-controller-fixture job in
.github/workflows/pr-ci.yml. That step runs
node scripts/qualification/pair-reader-diagnostic.test.mjs directly, with a
five-minute step timeout; tests stay in process so the test runner adds no
implicit worker child. Other existing steps stay unchanged. No new job, workflow,
manual dispatch or qualification entry point. No spent driver/helper/policy,
existing attributes, Python SDK reader or source pins are modified. Pure guarded
budget/file helpers may be imported read-only; never call a spent main function.
Required documentation is also in scope: the accepted scoped ADR, its index
entries and a synthetic validation record. No product or old qualification
contract is changed. No new dependency, interpreter discovery or installation
is included.

## Fixture ownership and execution envelope

The parent creates a fresh exclusive directory named ork-reader-diagnostic-
followed by 32 random lowercase hex characters below the runtime temporary
directory. Existing paths are refused, not reused. It derives seven roots under
its owned fixtures directory: msvc, registry, rust, sdk_bin, sdk_include, sdk_lib,
source. The registry root is its own registry/src tree with an adjacent owned
registry/cache tree. These are labels for synthetic directories, never the
installed paths bearing those names. The request cannot substitute roots or
select a live profile. No environment/CLI/config switch enables discovery.

Each request is a canonical five-element tuple [1, "synthetic-v1", mode, nonce,
hash_grant], at most 4 KiB. Mode is reference, reuse or guards; nonce is a fresh
128-bit value rendered as 32 lowercase hex characters; hash_grant is no larger
than the parent's remaining pool. No path or root dictionary is carried in it. The parent refuses
nonce reuse. Request/output filenames are derived from that nonce in owned
requests/outputs directories. The worker recomputes these paths and fixture
roots; it rejects unknown fields, profiles, paths, ADS/UNC aliases and every
reparse/symlink ancestor. The parent keeps its ownership/identity witness until
child drain and cleanup. The marker/nonce is correlation, not a bearer capability
or protection against a malicious hosted OS.

Only the own Node controller and fixed
C:/Program Files/PowerShell/7/pwsh.exe may spawn. PowerShell uses -NoProfile
-NonInteractive. No Cargo, rustc, cl, lib, dumpbin, registry API, SDK/PE inspection,
Python, shell discovery or resulting program is invoked. The child's minimal
environment contains SystemRoot/WINDIR and owned TEMP/TMP, fixed System32 PATH
and the synthetic marker; no inherited credentials, proxies, compiler flags,
package-manager configuration or reporting capability. Runtime version is
recorded in process, without a separate version-query child. Non-Windows runs
exercise the pure parser and the owned Node timeout fixture; native fixture
results are explicitly skipped rather than invented.

| Scope | Proposed synthetic ceiling; no inherited execution allowance |
| --- | --- |
| One synthetic driver execution | 120 seconds active plus at most 120 seconds for owned termination/drain/failure publication; CI step timeout 5 minutes, no automatic retry |
| Native children | At most three fixed-purpose PowerShell invocations: reference, reuse, guards; at most 30 seconds each, clamped to remaining active time |
| Owned Node failure child | At most one, at most one second, run last; it emits one start record and stalls to exercise owned timeout/drain. Other transport failures use in-process fakes, not more children |
| Termination | At most one fixed C:/Windows/System32/taskkill.exe /PID current-owned-child /T /F, max 10 seconds inside drain reserve; no foreign or replacement PID; unresolved drain retains scratch and fails |
| All child stdout/stderr plus injected parser bytes | 1 MiB cumulative across this driver, never reset per child/vector; received/injected chunks are charged before parsing/retention, with the two sources labelled separately |
| Pure parser/transport vectors | At most 64 in-process vectors, each at most 8 KiB injected bytes; at most 512 KiB combined inside the same 1 MiB stream pool; no hidden repeats, fuzz loop or child per vector |
| One reader stdout | 4,096 bytes total including LF bytes; max 21 records, max 192 bytes per record including LF |
| Hash/read-for-hash work | 64 MiB cumulative including parent/child repeated passes, own source seals, fixture/golden creation hashing, output reads and result commitments; missing terminal work report spends the whole grant |
| Fixture input union | At most 2,048 input files including cache archives, 256 directories, 4 MiB file bytes in one suite, 1 MiB per file; one fixture package; max 1 MiB compressed archive and 4 MiB expanded TAR stream |
| Encoded manifest | At most 1 MiB per native reference/reuse output, with precharged rows and final encoded-size refusal |
| Whole owned scratch | At most 8 MiB materialized files including controls, fixture input and both manifest outputs; precharge writes, no spill or capacity rescue |
| Own/imported source seals | At most 1 MiB combined per source pass, at start and after the last child; both passes and repeated reads are in the 64 MiB hash pool |
| Retained diagnostic result | At most 64 KiB, including at most 18 KiB for three native timelines and the one transport prefix; no full manifests, raw invalid output, raw input paths or raw exception text |

These numeric work/input ceilings are smaller than the earlier live ceilings
where comparable, but the protocol and synthetic commands are new proposed
scope. No old job allowance is inherited.

Native hash-worker grants leave 4 MiB of the remaining 64 MiB pool reserved for
parent output
verification, the final own-source seal and failure publication. This covers up
to 2 MiB for one manifest read/commitment, 1 MiB for the final source pass and
64 KiB for a result commitment, with 960 KiB still reserved; it is not extra
work credit. If no positive grant fits, start no native hash worker. The final
owned Node transport child is a fixed literal script with no file/hash work,
no manifest output and h=0; it receives no hash grant. It tests real timeout/
prefix retention, not unknown nonzero hashing. Its decoder expects exactly the
nonce-bound start tuple with h=f=d=a=0; any other record is an unexpected
fixture failure. Its prefix/disposition is retained separately from native
source-reader completion. The nonzero unknown-grant case
uses an in-process transport fixture and a small reserved debit within the same
64 MiB pool, labelled as fixture accounting rather than physical hashing.
No fixture, subcase, mode, timeout or failed child resets a pool. Per-vector negative-test
limits are additional lower limits over the same cumulative ledger, not fresh
work credits. Every unexpected failure stops later children. Expected negative
subjects may be tested independently within the same driver/clock/pools; a failed
subject cannot resume or run any later stage of its own. Native guard refusals
are handled inside guards; the sole expected real timeout runs after all native
children, so no child follows its termination. In-process fake transport cases
start no OS process and never reset global time/work limits. Unknown hash work
and unresolved cleanup are failed subject outcomes, not permission
to consume the reserve for more testing. The process/runtime is trusted as in
the earlier controller; these are logical work/input bounds, not a total RSS,
host-memory or hard I/O-latency guarantee.

The parent also writes a canonical synthetic fixture catalog, at most 256 KiB,
at a derived fixed control path in owned scratch. It contains only version 1,
the single fixture package identity, the archive SHA-256, sorted relative-member
SHA-256 pairs and null build-script/proc-macro dispositions. The parent computes
these goldens from its literal fixture data with charged hashing and keeps the
catalog fingerprint in invocation state. The worker reads this fixed file with
bounded held-file/identity checks, never an external policy path. Unknown,
duplicate or noncanonical catalog fields/members refuse. Parent verification
binds the output catalog hash to that expected fingerprint. The catalog is also
sealed before the first and after the last child, within the same 1 MiB combined
source/control pass cap and 64 MiB work pool. It counts in the 8 MiB scratch cap;
it supplies no real package, tool or SDK authority.

## Wire format and phase sequence

New protocol name: reader-diagnostic-v1. It is not accepted by a legacy single-
object parser and is never fed into a spent collector. UTF-8 with no BOM, but
all wire bytes must be printable ASCII except the single LF ending each record.
No CR, whitespace outside JSON strings, blank line, control character, raw path,
package name, timestamp or arbitrary diagnostic text is permitted.

A progress record is exactly this nine-element array, with no object keys:

```json
[1,"0123456789abcdef0123456789abcdef",1,"start","",0,0,0,0]
```

Positions are version, nonce, sequence, phase, root label, h, f, d, a.
Version is exactly 1. Sequence starts at 1 and increases by one; nonce must equal
this child's request. Integers use canonical nonnegative decimal JSON tokens,
are safe integers and obey the request's stricter caps. Globally h is at most
32 GiB, f+a at most 200,000, d+2 (when a is nonzero) at most 50,000 and a at most
345; synthetic-v1 further restricts h to its grant, f+a to 2,048, d+2 to 256 and
a to 1. Fractions, exponents, -0, strings as counters, escaped spellings,
noncanonical spaces and unknown/extra fields are refused. Parsing followed by
canonical compact serialization plus LF must exactly equal the received bytes.

Counter h is charged hash/read-for-hash work, including precharges; it is not
physical throughput. Counter f is the number of completed rows in this
invocation's primary manifest,
d its completed directory rows and a its completely verified primary fixture
archives/packages. The guards profile has no primary manifest, so f=d=a=0; its
independent test vectors still charge h and every outer input/work limit.
Archive identities are counted by a rather than duplicated
in f; the two cache-directory slots are accounted for separately. Counters are
cumulative within this child, monotonic and never exceed the declared limits.
At an admitted ok result, f/d/a must agree with the actual complete manifest
counts (or zero for guards) and h with the worker debit report. At a cooperative
error there may be no complete manifest: f/d/a are range-checked monotonic
diagnostic self-reports only, never independently verified inventory or source
admission. Only the closed, valid h report is eligible for known work accounting.
A checkpoint cannot prove a complete inventory or the absence of later work.

For reference/reuse the exact successful sequence is:

1. start, empty root: bounded request/profile/nonce framing is valid. This is
   not file-identity admission; it precedes own native-type setup.
2. ready, empty root: native declarations/TAR runtime loaded and owned fixture
   roots/identity restrictions validated.
3. archives-begin then archives-end, empty root: archive/member/helper/extra-file
   checks run unchanged; a increases only after the whole package verifies.
4. root-begin then root-end for each root in the exact order msvc, registry,
   rust, sdk_bin, sdk_include, sdk_lib, source. The root field must match the
   current expected label; all other phases have an empty root.
5. serialize-begin then serialize-end, empty root: sorting, complete encoding,
   size check, owned output write/flush/close and data-handle disposal have
   completed before serialize-end is emitted.
6. ok, empty root: the worker has no remaining data/hash work and exits zero.

That is 2 + 2 + 14 + 2 + 1 = 21 records. The guards invocation has only start,
ready, ok, with f=d=a=0 and h reporting its cumulative owned-vector work.
For a cooperative error, an error terminal may replace any remaining suffix of
a valid prefix; it has ten elements, adding exactly one fixed reason at the
end. Reasons are bad-request, input-denied, hash-limit, input-limit,
identity-drift, format-denied, output-limit, fixture-assertion or
reader-unavailable. It exits nonzero. If request binding is not valid, emit no
invented nonce/counters; nonzero close or silence remains unavailable.
No records, whitespace or suffix bytes may follow either terminal.
Emit either terminal only after every file/TAR/hash cleanup finally block has
settled h and released its data handles and buffer leases. No further data reads
or hash/debit work may occur after either terminal. If those conditions cannot
be established, omit terminal accounting and leave the full grant charged.

The 192-byte line cap is conservative for these closed fields. It yields
21 × 192 = 4,032 bytes, below the existing 4,096-byte report limit. Across three native
children that is at most 12,096 report bytes; the sole Node transport prefix
adds at most 192, totalling 12,288 within the shared 1 MiB synthetic stream
pool, alongside injected vectors, and below the older 64 MiB live ceiling. Each incoming byte, including malformed
or surplus bytes, counts; a partial line never permits a larger allocation.

## Parent retention, failure and accounting

The driver feeds byte chunks into the bounded parser **as data arrives**, before
awaiting the child-close promise. It retains each complete validated record
as [parent_elapsed_ms, child_tuple] in the invocation's fixed trace. Snapshot
each validated tuple as an immutable value before advancing the decoder; never
retain a mutable line buffer or reuse a retained tuple array. Native
workers each have at most 21 retained records; the Node transport test has only
its one expected start prefix. Pure in-process vectors retain no per-vector
timeline in the final result, only bounded aggregate pass/failure dispositions. Parent elapsed time is an integer from 0 through 240,000 ms on the one driver
monotonic clock, not a child-supplied timestamp or a disk-work time. Check the
active deadline before accepting a record; late data cannot extend it. Before the first record the parent state is
awaiting-start; this covers interpreter launch/request handling. start-to-ready
separates own native setup. No missing phase completion is inferred from silence.

Charge before accepting bytes or appending a row. The parser holds at most 191
unframed bytes; a 192-byte row includes its LF. Input chunk boundaries are
irrelevant. One retained arrival/tuple row is conservatively capped at 256
encoded bytes. Thus 3 × 21 × 256 + 1 × 256 = 16,384 bytes; adding at most
2,048 bytes of trace wrappers/status/counter metadata gives exactly 18 KiB
(18,432). Reject a trace exceeding that cap instead of truncating it. The whole
result still refuses above 64 KiB; these are sublimits, not extra allowances.

On malformed/replayed/out-of-order data, excess bytes/records, stream overflow,
timeout, child error or bad close, latch that subject's failure and permit no
continuation of it. An unexpected outcome also stops the driver and all later
children. An expected negative fixture is a test of refusal, not reader success;
its work remains charged and its bounded disposition records a failed subject. The
last valid prefix survives the rejected close promise; raw invalid output and
incomplete tails are not retained. Record only the tail-byte count, bounded
reason and explicit incomplete/invalid status. Stop accepting protocol records
once failure is latched; close/destroy pipes and terminate/drain only the current
owned child. Count received overflow bytes honestly even if they exceed a cap;
overflow can only produce a failed subject outcome, never an admitted or
truncated reader result.

A parsed ok or error terminal does **not** release a reservation by itself.
A work report is usable only after EOF/close with its expected exit status,
valid complete framing and h within grant. ok also requires held-file output
admission and all source/identity checks; the guards result requires its bounded
assertion summary. For error, no partial output or f/d/a comparison can replace a
complete manifest: h is trusted only as a debit report from the sealed own worker
after cooperative close. Partial counters never refund h or prove source success.
Timeout or invalid/missing terminal accounting conservatively
spends the whole grant, even when earlier checkpoints report less. A checkpoint
cannot refund that difference. No duplicate completion or second settlement.
If output verification fails after a report, the fixture fails and cannot run
another child. Do not delete working inputs while any child is undrained.
Native worker stderr is expected empty; unexpected bytes count toward streams, latch failure and are
not retained as raw diagnostics.

## Buffer ownership and paired evidence

The reference path preserves per-file/per-member scratch allocation. The reuse
path is otherwise identical: one 1 MiB byte array allocated once per reader
invocation and leased exclusively to one hash/read loop at a time. Archive
identity hashing, member streaming, extracted-file verification and manifest
hashing borrow it sequentially. No parallel reads, nested lease, cross-process
pool, escaped reference, hash-state reuse or caching by filename. Release the
lease in finally on every success/error path; a nested lease is a fixture
failure. Do not dispose a stream while a lease still refers to its active data.

Hash exactly the n bytes returned by each bounded read, not the whole buffer.
Reset each IncrementalHash per file/member and consume bytes before the next
read overwrites them. Retained records contain immutable paths/IDs/digests and
counts, never slices referencing the mutable scratch buffer. Tail bytes from a
larger prior read must not affect a later short/empty file. Count the worker's payload-scratch allocations/cumulative capacity, maximum
simultaneous payload leases and bytes hashed. Protocol/catalog/JSON buffers are
subject to their own bounds and are not payload-scratch allocations. These
metrics also exclude framework/runtime allocations and cannot be reported as
process memory or a production speedup.

The parent materializes one immutable positive fixture tree, used sequentially
by both reference and reuse on the same identities. One owned package archive
contains 1,024 one-byte members, an empty member, a normalized Cargo.toml and
one 1 MiB file with deterministic nonzero content;
remaining roots contain small deterministic files. All size/count/encoded caps
above must be checked during generation. Golden membership/content hashes come
from the parent's literal fixture data with charged hashing. Compare complete
sorted rows, file IDs, archive origin/member/helper dispositions and byte/hash
charges before accepting the pair. Compare manifest content independently of
nonce/strategy/timing diagnostics. Keep reference and reuse traces distinct.
No source mutation occurs between them. Guard/refusal vectors use separate
owned tiny trees after that comparison and cannot change either positive input.

Reference/reuse outputs keep the existing complete manifest rows/counters and
add a separate bounded metrics object: strategy, scratch_allocations,
scratch_capacity_bytes, maximum_leases and hash_bytes, plus catalog_sha256.
These are scalars, not arrays or live references. The equality comparison
excludes this metrics object and nonce/timing envelopes; it includes every
custody row and byte/hash debit. The guards profile writes only a summary of at
most 4 KiB containing its at-most-32 expected vector dispositions and cumulative
h; it never publishes a manifest. The parent checks that reference allocates
per work unit, reuse allocates exactly one 1 MiB scratch array, maximum leases
never exceeds one and both strategies produce the same independent goldens.
Per-vector lowered admission limits exercise oversize/growth cases using tiny
owned files; even negative fixtures must remain inside the outer materialized
file/directory/byte caps.

An improved allocation count with identical outputs/checks is useful evidence;
no elapsed-time threshold is a correctness requirement. Phase arrival timings
are host-specific diagnostics, not a promised 120-second Windows live capacity
or a claim that buffer allocation caused the spent failure. No source root,
verification pass, archive/member check, reparse/ID guard, size precharge,
encoded-size refusal or final drift comparison may be removed to obtain a win.

## Acceptance evidence after scoped owner approval

The future implementation must demonstrate:

- Exact framing/nonce/sequence/root/terminal acceptance; byte-at-a-time and
  coalesced chunks; duplicate/foreign nonce, unknown phase, wrong shape/version,
  non-ASCII/BOM/CR, escaped/fractional/unsafe numbers, partial EOF, data after
  terminal and every byte/record/trace/result cap fail closed.
- Timeout before start, after a complete checkpoint, mid-record and after an ok
  record but before close. Latest valid prefixes remain diagnostic only; all
  unreported grants stay spent, no later child starts and drain precedes cleanup.
- Native fresh/reuse equality on small, empty, large-then-short and repeated
  reads; one reusable buffer/max one lease; independent digest goldens and
  before/after source/file identity checks. Reference allocation arithmetic is
  measured for the fixture, not extrapolated to the unknown runner input count.
- Retained archive/member authenticity, traversal/duplicate/type/size/extra-file
  refusals; wrong archive hash before decompression; reparse/identity/size/growth
  and encoded/work/count caps. At most 32 independent small negative native
  vectors share the same cumulative pool; pure parser vectors need no children.
- Clean/nonzero close, spawn failure, stream overflow, termination/drain failure
  and cleanup/publication failure have explicit subject outcomes. Every failed
  subject has reader_complete=false and cannot admit or continue. tests_passed
  describes the fixture assertions: it may be true for an expected refusal only
  when the expected reason, accounting, retained prefix and drain invariants all
  match. Unexpected outcomes or real cleanup/publication failures make it false.
  Fake transports supply process/error branches; the single owned Node timeout
  child runs last and uses at most the sole owned termination fallback. No live
  tool or foreign PID is targeted.
- Distinct schema 1/evidence_kind synthetic-reader-diagnostic. Final result uses
  tests_passed, not a candidate success field; candidate_evidence_complete,
  runtime_qualified and production_pin_accepted are always false, target_executions
  is always zero. Keep measured work, full charged unknown grants and unknown
  work flags distinct. Do not import this result into candidate admission.

Store only the bounded synthetic result in owned temporary scratch or the
normal CI log; no new uploaded artifact, retention schedule or expansion of the
spent 15-file allowlist. Remove scratch after final validation only if drained;
unresolved cleanup remains failed with its owned path available for handoff.
Normal CI may run these bounded owned fixtures after implementation approval;
it must not turn them into installed SDK/toolchain discovery or a candidate run.

## Delivery and review boundary

After one owner acceptance of this whole synthetic contract, record the scoped
diagnostic-protocol decision in an ADR before code, without changing or
superseding the resource/build-startup ADRs. Then resolve a concise implementation plan against these acceptance criteria, implement the new files
and the sole CI-step exception with TDD, verify appropriate repository/native
fixtures and current-diff
review, then publish a PR. Approval does not select a live integration strategy
or revive the old job. A later installed-input/candidate package needs separate
concrete reviewed tooling/source/budget/command/output choices and a fresh finite
owner allowance. This contract contains no such allowance.

Builder choices left open are private function names, test organization and
serialization implementation consistent with these exact bytes/limits. Changes
to framing, counter meanings, identity/retention policy, scope or ceilings are
material design changes, not builder discretion.

Proposal review topology, fixed before dispatch: root is sole writer; maximum
two fresh read-only Luna reviewers, one round. One builder/completeness reviewer
asks what is still missing to plan the scoped implementation. One independent
correctness/authority reviewer checks arithmetic, parser/failure semantics,
source constraints and the synthetic-only boundary. Both cite this document or
source for material findings and mark uncertainty UNVERIFIED. No reviewer edits,
new harness, live query or implementation. Root applies findings before handoff.


## Proposal review disposition

One bounded fresh-context round completed before handoff. The Luna builder
review reported nothing important left. The independent Luna consistency review
found two material ambiguities: terminal counter/manifest equality could not
apply to an early error, and the new-file scope omitted the existing CI-step
exception. Root checked both against the written/source boundary and clarified
error counters as diagnostic self-reports with h-only cooperative accounting,
plus the single named workflow edit. Neither clarification grants source
admission, changes a ceiling or reopens an allowance. Root also made the fixture
catalog, output metrics and in-process test command explicit as author choices.
Root further distinguished fixture-suite test success from failed subject
outcomes, limited real failure injection to one final Node child (so one owned
termination suffices), and bounded in-process vectors within the same pools.
No material questions are deferred to an implementer or treated as owner-approved.


Static author checks of the closed tuple fields found at most 103 encoded bytes
including LF for conservative maximum-field progress/error examples; the
192-byte line cap leaves margin. The 4,032-byte report, 18,432-byte retained-trace
and 4 MiB parent-reserve calculations were also checked directly. These are
proposal arithmetic checks, not executable-parser, native-worker, performance or
qualification evidence. Implementation/native fixtures remain pending acceptance.
