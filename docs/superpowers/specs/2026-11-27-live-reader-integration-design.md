---
type: Evidence Acquisition Proposal
title: Progress reader for one Windows candidate batch
description: Proposes an isolated live-profile reader with retained progress and buffer reuse, plus one separately gated candidate job under unchanged outer limits.
tags: [orkworks, hierarchy, windows, qualification, diagnostics]
status: draft
workflow_status: accepted-implementation-in-progress
---

# Progress reader for one Windows candidate batch

Tracking [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base 510d7244adb7194e8a997d02418fc1c98b996ad4. This document follows the
[synthetic evidence](../../validation/reader-diagnostic-synthetic.md) and
[ADR 0123](../../adr/0123-synthetic-reader-diagnostics.md). The owner asked to
continue with a live-reader integration proposal, not implementation or dispatch.

**Author proposal for one combined owner decision:** accept the new isolated
reader/controller tooling, the same named input candidate, and one finite new
Windows candidate job described below. Publishing or merging this document
accepts none of those choices. All earlier allowances remain spent; no old
workflow, driver, pin, policy, result or history is reopened or edited.
Implementation and the eventual dispatch require acceptance of this full package.
One such acceptance covers both; passing the stated verification/freeze/history
gates then needs no second approval for the unchanged package. A changed package
still requires a concrete new owner decision.

## Outcome, alternatives and evidence limits

Obtain an attributable macro/link/SDK/input candidate result, or a terminal
unavailable result with the last validated reader phase retained. Use one reusable
1 MiB payload buffer in each reader while preserving every input/custody check.
The synthetic fixture measured 4,123 versus one payload allocation with equal
manifests; it did not identify the spent Windows timeout's phase or demonstrate
live capacity, memory savings, throughput or a successful candidate build.

Retaining the old unavailable result is a valid alternative. Another standalone
installed-input probe would spend work without the actual build association.
Editing the spent reader, widening the synthetic profile, dropping a root/pass,
caching checks by path, parallel reading or raising the 120-second manifest limit
would answer a different question. The proposed isolated fork preserves the
existing candidate recipe and gives the next finite attempt useful diagnostics.
No resulting sidecar, SQLite, SQL/database, model, startup, installer or runtime
activation is executed. ADRs 0118/0119 and all qualification gates remain binding.

## Concrete source and candidate selection

Use the [frozen replacement policy](https://github.com/Rambolarsen/orkworks/blob/510d7244adb7194e8a997d02418fc1c98b996ad4/scripts/qualification/pair-replacement-windows-build.inputs.json)
without changing any value: 43,200 bytes, SHA-256
1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6.
Copy those exact bytes to the new own policy filename after approval. Its twelve
PE paths/hashes, versions, roots, four source pins, 345 admitted package identities,
37 build-script entries and 14 proc-macro entries remain exact. The package count
is an upper bound on admitted cached packages, not a claim that all 345 are
fetched/executed for this target. Unknown packages/helpers still deny.

Static repository inspection rehashed all four source pins and found matches:
Cargo.toml, Cargo.lock, rust-toolchain.toml and build.rs. No installed file, SDK,
registry, package or native tool was inspected for this proposal. Recheck those
pins before future implementation/freeze; a changed pin is a new selection
question, never an automatic refresh.

The [previous package's exact selectors and effective commands](../plans/2026-11-27-pair-replacement-windows-build.md#candidate-and-fixed-inputs)
are reused as proposed inputs for this new batch, not as an unspent approval:

| Item | Selected input for the proposed new batch |
| --- | --- |
| Target/build | Release orkworksd, existing default features, x86_64-pc-windows-msvc, jobs2 |
| Rust | Cargo/rustc1.100.0 at the policy's fixed stable toolchain paths and hashes |
| MSVC | 14.44.35207 root; cl.exe file19.44.35230.0/product14.44.35230.0, SHA-256 e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226 |
| Other PE inputs | The same exact link/lib/dumpbin/c1/c1xx/c2/mspdbcore/rc/mt plus Cargo/rustc selectors; twelve total, no devenv query |
| SDK | Windows Kits/10, directory10.0.26100.0, named product10.1.26100.9457 at the exact held HKLM64 product-key selector |
| SQLite | Existing3.46.0 amalgamation and locked rusqlite/libsqlite3-sys/cc package bytes, unchanged build-owned -USQLITE_ENABLE_MEMORY_MANAGEMENT and THREADSAFE=1 recipe |

The selected SDK product child remains
{45e335f3-c807-43b8-9ed1-43666cfcf939} below the held Uninstall parent, with the
same exact DisplayName/DisplayVersion/KitsRoot10 values, strict REG_SZ decoding,
fixed buffers and write-marker comparison. Two same-job passes bracket the
actual build/oracle/link work. Keep max32 API calls,4 KiB metadata,64 KiB SDK
hash work, zero enumeration, no third pass and inventory_complete=false.
The SDK subrecord keeps schema3/evidence_kind sdk-positive-build-bracket because
its meaning is unchanged; a subrecord alone cannot admit either candidate kind.

## Isolated implementation boundary after acceptance

Create only new scripts/qualification/pair-progress-windows-build* files:
Node controller, dedicated progress codec, PowerShell reader, exact policy copy,
SDK wrapper and their synthetic tests. Add the new manual workflow
.github/workflows/pair-progress-windows-build.yml. Existing-file exceptions are
one five-minute direct Node test step in the normal sdk-controller-fixture job,
LF rules for explicitly named new own files only, and necessary documentation
indexes/evidence/ADR amendments. Leave every old qualification and synthetic
file's behavior and allowlists unchanged; no new dependency or interpreter install.

Fork the existing candidate stage logic and reader checks, with a narrow diff
inventory for each changed boundary. Pure parameterized guarded helpers may be
imported read-only and sealed, including bounded file reads/hashes and reviewed
argv/oracle/link parsing. Never invoke an imported main, spent dispatch guard or
spent runner. Policy-bound validators and live dispatch/history selection belong
to the new controller and new policy; no implicit fallback to an old policy.
The new own budget/runner uses one monotonic active clock and streaming reader
observation rather than the spent CommandRunner's buffered rejected promise.

Preserve the old PowerShell reader's complete archive/member, package/helper,
extra-file, PE/version, reparse/ID, bounded enumeration, sorting/encoding and
post-input checks. Port only the progress/buffer ownership mechanism proven by
the fixture; the toy fixture Reader is not a production reader. Its fixed
1 MiB-file/one-package/1,027-member/synthetic-root assumptions cannot be copied
into the live profile. Live files remain at most512 MiB, read in 1 MiB chunks.
No hash cache, ancestry cache or source/registry verification pass is removed.

Record the scoped extension of ADR 0123 before code after acceptance; do not
supersede ADRs 0118/0119 or change product resource/startup decisions. A concise
implementation plan and TDD cover the changed interfaces. Scope/risk changes
need review; unchanged accepted recipes do not need approval again.

## Request, roots and reader wire domain

New protocol: reader-live-progress-v2. Canonical request is exactly
[2,"pinned-windows-candidate-v1",mode,nonce,hash_grant,policy_sha256], at most4 KiB.
Mode is preflight, before or after. Nonce is fresh128-bit lowercase hex (32 chars),
never reused; policy_sha256 is the exact digest above. Integers are canonical
nonnegative safe decimal JSON integers. No arbitrary paths/root dictionaries,
reference strategy, synthetic profile, discovery or budget override is accepted.

The parent creates exclusively a new
RUNNER_TEMP/ork-pair-progress-windows-build-2026-11-27 directory, refuses reuse,
and derives working/evidence/requests/outputs paths. Request/output files use
requests/NONCE.json and outputs/NONCE.json, created exclusively; no overwrite.
The reader recomputes seven roots from the frozen policy plus the checked
workspace and owned Cargo home: msvc, registry, rust, sdk_bin, sdk_include,
sdk_lib, source. Registry is owned working/cargo-home/registry/src with the
adjacent cache. Source is the exact checked-out workspace, excluding only .git;
all generated outputs stay in owned target/scratch. Reject reparse ancestors,
UNC/ADS aliases, changed held/name identities and any root/policy mismatch.

The own Node orchestrator is the loaded process.execPath, never searched or
replaced. PowerShell is exactly C:/Program Files/PowerShell/7/pwsh.exe (require
version7.x); Python is exactly
C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe (require3.12.10/X64).
Record their actual paths, in-process versions and bounded byte hashes plus the
actual SystemRoot/image identity, as the previous package did. Runtime binaries
are the named trusted base, not vendor/OS-image attestation or a permission to
adopt alternate installs. No extra version-query child is added.

Launch only the fixed PowerShell path with -NoProfile -NonInteractive,
-WorkingDirectory OWNED_SCRATCH, -File NEW_READER and -Nonce NONCE. Its minimal
environment contains SystemRoot/WINDIR, fixed System32 PATH, owned TEMP/TMP and
new reader/workspace/scratch correlation markers. Normalize Windows paths as
proved in the fixture; markers/nonces are correlation, not capabilities or
protection against a malicious hosted OS. In-process native setup comes after
request framing/start; no interpreter search, extra version-query child or
ambient proxy/credential/reporting/build-flag inheritance reaches this reader.

A progress tuple is [2,nonce,seq,phase,root,h,f,d,a]. A cooperative error adds
one tenth element from bad-request, input-denied, hash-limit, input-limit,
identity-drift, format-denied, output-limit, reader-unavailable. Keep canonical
printable ASCII JSON plus one LF only, strict safe integers/round-trip bytes,
sequence1 onward, no BOM/CR/whitespace/escapes/extra fields, maximum192 bytes
including LF and at most21 records/4,096 stdout bytes per reader. Unknown fields,
nonce, version, root/order or bytes fail closed. No byte follows a terminal.
The v1 synthetic decoder and all legacy single-object readers reject this v2
domain; no existing parser is widened.

Successful before/after sequence is the same21 boundaries as the synthetic
contract: start, ready, archives-begin/end, root-begin/end for the seven labels
in the fixed order above, serialize-begin/end, ok. Preflight is exactly five:
start, ready, serialize-begin, serialize-end, ok; it hashes/checks the twelve
selected PE files between ready and serialize-begin. Its f/d/a are zero because
there is no primary manifest. ready means owned paths/policy/native setup are
admitted for the current mode, not that PE/source content has passed.
Preflight admits only control/workspace identities, exact policy binding and
native setup before reading the selected PE paths. The owned registry/src may
still be absent before the sole fetch; preflight never enumerates or creates it.
Before/after ready additionally admits all seven manifest roots and the owned
registry/cache parents after fetch. Missing roots then deny; no repair or
alternate source is created to rescue admission. Error replaces any remaining
suffix of a valid prefix; failed binding emits no invented accounting tuple.

Counter h is logical charged hash/read-for-hash work, including precharges,
not throughput. f/d count completed primary file/directory rows, a completely
verified primary packages; no progress record is inventory/admission proof.
Require h<=grant<=32 GiB, f+a<=200,000, d+(a?2:0)<=50,000, a<=345,
monotonic/ranged counters and zero f/d/a for preflight. At ok, independently
check f=m.files.length, a=m.crates.length, d=m.directories.length;
existing file_count=f+a, directory_count=d+(a?2:0) and complete byte totals
must still match. Error counters are diagnostic self-reports only; only closed,
verified h can settle known work. Missing/invalid accounting spends the grant. Parent data admission
uses the existing trusted-reader model: validate complete output structure,
policy bindings and row/member associations; do not copy the toy fixture's
parent-side rehash of every payload. All mandatory live payload passes and
held file/name identities stay in the worker, with the full after comparison.
The parent checks its own source seal and held output; synthetic tests supply
independent literal goldens. No new unbudgeted parent payload pass is implied.

The complete output is a strict object with exactly schema=1,
evidence_kind=progress-reader-output, mode, nonce, policy_sha256, data and metrics.
Reject duplicate/unknown keys and malformed/trailing JSON. The whole encoded
object, including envelope/metrics, is at most64 MiB. Precharge row/envelope
encoding and check the final encoded bytes before an exclusive write/flush/close.
Data retains the old preflight tools/version/hash report or the complete old
manifest fields/rows/crates/counters, according to mode; do not drop checks or
fields to fit this envelope. Parent validates the old data contract against the
new exact policy, then verifies h and the new envelope binding. Metrics has
exactly strategy=reuse, scratch_allocations, scratch_capacity_bytes,
maximum_leases and hash_bytes (actual payload bytes hashed, bounded by h).
Before/after equality compares complete normalized data content/IDs/helpers;
exclude only data.hash_bytes, the outer mode/nonce and metrics, whose debit/
allocation predicates are checked independently. h need not match across live
before/after passes: admission budgets, TAR precharges and unchanged-content
repeated work remain individually verified. This is not the synthetic A/B
requirement that identical fresh/reuse algorithms debit equally.

## Retention, settlement and single-buffer ownership

Feed bounded chunks to the codec as they arrive, before awaiting close. Retain
immutable [parent_elapsed_ms,tuple] snapshots for each invocation, not shared
mutable arrays or raw lines. Parent elapsed uses the one job monotonic clock;
accept no record after its active deadline, and never infer a completed phase
from silence. Keep at most191 unframed bytes; charge all observed chunks,
including malformed/overflow bytes, before any parsing/retention. If an observed
chunk crosses the stream cap, report that actual excess honestly only as a
failed subject; never clamp counters or retain a truncated success. Reader stderr
must be empty; count unexpected bytes but retain no raw error text.

Retain at most18 KiB of timelines/status inside the existing64 KiB result.json.
At most47 successful records produce47×192=9,024 wire bytes across all readers
and47×256+2,048=14,080 retained trace bytes; both are below the inherited report/
trace bounds. The 256-byte arrival-row bound includes a parent time up to
2,520,000 ms; actual acceptance stops by the40-minute active deadline. No extra
trace file or uploaded artifact slot is introduced. Prefixes survive timeout,
failed close, source/output admission failure and receipt-only publication.
They never authorize build, refund work or establish an inventory by themselves.

A terminal is eligible only after EOF/expected close, complete strict framing,
disposed data handles/leases and known accounting. Before refunding an ok grant,
verify own/imported sources/policy against the invocation's held source seal,
read/hash the complete held output, bind its nonce/mode/policy digest envelope,
check counters/rows/metrics and admit it. A cooperative nonzero error may settle
h only after the same source verification; it has no output admission. Unknown
source identity, timeout, bad terminal, overflow, rejected output or failed drain
charges the full reservation exactly once, even when a checkpoint said less.
Any unexpected failure stops all later stages/children; never retry a subject.

Retain132 MiB of remaining job hash credit outside each PowerShell grant; if
no positive grant fits, start no child. This is a subdivision of the same32 GiB,
not extra work credit. It covers two64 MiB output read/commitment passes, up to
two1 MiB own-source passes,64 KiB SDK receipt work and64 KiB final-result work,
leaving1,920 KiB margin. Precharge every pass. A source pass checks its combined
1 MiB cap before reading each next file; compare after every reader close and
finally after the last drained child, including failure paths. Extra source
checks remain charged within their stated pass/subledger bounds; no data/hash
work follows a reader terminal in the child.

The reserve is per outstanding invocation, never a once-per-job allocation or
new credit. Let L=32 GiB, H=all already charged work, and R=L-H before this
reader, with no other child/hash reservation outstanding. Its grant is
G=R-132 MiB; deny if G<=0 or a prior reservation is unsettled. Earlier source/output passes are
already in H and cannot be refunded. With one reader at a time, parent work
before settlement has a strict129 MiB subledger (one post-close1 MiB source
pass plus two64 MiB output passes). Protect the other3 MiB from ordinary reads
or extra source passes; refuse before consuming that emergency tail. Even if h=G, at least3 MiB remains for one final1 MiB
source pass,64 KiB SDK receipt and64 KiB result work:1,920 KiB margin. If that
leaves too little for another grant, stop rather than replenish the reserve.
Over a complete job there can be five own-source passes (initial, three
post-reader, final), all charged to32 GiB. They are not all future obligations
of a single outstanding grant. Explicitly test three sequential settlements
against one decreasing ledger and refusal of a next grant after maximal h.

For an unknown reader or rejected output, discard partial/bulk evidence before
reading it and publish only bounded result/SDK receipt observations; a failure
cannot consume the reserve to salvage a binary, manifest or log. Preserve the
validated progress prefix and full-grant charge in that receipt. Known-success
artifact snapshot/hashing must also precharge remaining work; insufficient
credit means unavailable/receipt-only, not an overdraw or promised maximum fit.

Each reader allocates one1 MiB payload byte array and leases it exclusively to
one hash loop at a time. New hash state for every file/member; consume only the
n bytes returned, never stale tails. No nested/parallel/cross-process lease or
escaped payload reference. Release in finally before disposing active streams;
settle any TAR meter charges and all handles before either terminal. Retained
rows/digests are immutable. Allocation metrics cover payload scratch only,
excluding protocol/policy/JSON/framework/native allocations and RSS. Each ok
reader reports exactly one scratch allocation/capacity1 MiB, maximum one lease,
actual payload hash bytes and its policy digest in a bounded metrics envelope;
failed subjects without admitted output have unavailable metrics, not fabricated
zeroes. The explicit content comparison above retains every custody field; diagnostic
envelope/debit fields never substitute for that equality.

Debit rules preserve the original repeated work: regular-file hash length;
PE hash plus a full-length metadata-pass charge; archive identity hash and
compressed read pass; every expanded TAR read request precharged before I/O;
extracted file hashes and manifest hashes separately. Member hashing consumes
the same metered TAR payload and does not claim a second unmetered read or a
refund. EOF/throw still retains precharges. Policy/source/output/receipt hashes
and repeated copies remain in the job pool. Extra conservative unused-read
precharges may differ from the spent reader's post-read meter; report that basis
rather than equating h to physical bytes or prior-run throughput.

## One proposed finite new job

New workflow pair-progress-windows-build.yml on main; new immutable concurrency
group pair-progress-build-allowance-2026-11-27, cancellation disabled, no matrix.
Parent verifies actual win22/X64 image identity and exact GITHUB_SHA equals the
reviewed merged freeze. Keep contents/actions read, checkout with no persisted
credentials, parent-only history token, fresh dispatch history0/1 before launch
and exactly the current first-attempt run in bounded authenticated history at
runtime. Do not dispatch if frozen main changed; do not silently refreeze it.
Any attempted dispatch/infrastructure/preflight/capacity failure consumes1/1;
zero reruns, replacement jobs, alternate selectors, repaired inputs or retries.

| Work | Proposed maximum; no old allowance is reused |
| --- | --- |
| Whole job | One windows-2022 X64 job,45 minutes |
| Own orchestrator | One Node process,40-minute monotonic active deadline plus at most2 minutes total owned drain/failure publication |
| Own PowerShell readers | Three maximum: preflight60s, before120s, after120s, clamped to remaining active time; no live reference/A-B pass |
| SDK readers | Two if reached, fixed CPython3.12.10 X64 path, -I -S -B,30s each; same32-call/4-KiB/64-KiB pools |
| Selected-tool versions | One Cargo -Vv and rustc -vV,30s each; no compiler/IDE version child |
| Fetch | One locked target-specific crates.io fetch, net.retry=0,4 minutes |
| Build | One locked/offline release orkworksd build, jobs2,35 minutes |
| Oracle/link tools | One derived cl.exe preprocess-only command,2 minutes; up to four lib commands (two LIST/two EXTRACT) and one dumpbin DEPENDENTS,30s each |
| Hash/read work |32 GiB cumulative across everything;132 MiB protected parent share during a reader grant |
| Manifest/input | Each pass200,000 files including archives,50,000 dirs including cache slots,512 MiB/file,12 GiB file bytes,64 MiB encoded manifest |
| Streams |64 MiB job stdout/stderr, oracle subset32 MiB; each own-reader report4 KiB; no pool reset |
| Evidence | Same exact15-file allowlist,256 MiB total uncompressed,14 days, new artifact pair-progress-windows-build |
| Termination | At most one fixed taskkill /PID CURRENT_OWNED_CHILD /T /F,10s inside shared drain reserve; retain unknown scratch, never target foreign/replacement PIDs |
| Resulting program | Zero target, SQLite, model, SQL/database, installer, release or activation executions |

Use the identical selected executable paths, controlled build environment,
macro/startup recipe, reviewed helper closure and literal Cargo/oracle/link
command derivations from the previous package. In particular:

```text
cargo fetch --locked --target x86_64-pc-windows-msvc --manifest-path crates/orkworksd/Cargo.toml --config net.retry=0
cargo build --locked --offline --release --target x86_64-pc-windows-msvc --bin orkworksd --jobs 2 --manifest-path crates/orkworksd/Cargo.toml -vv --message-format=json
```

Stage order stays source/config/PE preflight; first named SDK pass; Rust version
queries and sole fetch; complete package/helper verification and before manifest;
sole build; actual-argv-derived macro oracle and linked-artifact association;
second SDK pass and after manifest/source seal; bounded publication. No metadata
command, exploratory availability query, extra preprocessing or target execution.
Every failed stage closes unused later slots. Root labels preserve argv/source
associations in redacted diagnostics; no ambient dumps or rejected SDK values.

Use build schema3/evidence_kind progress-sidecar-candidate. Legacy build schema2
and synthetic schema1 consumers must reject it. Final success and
candidate_evidence_complete require every effective macro/link/SDK bracket,
complete input/source identity comparison and bounded publication check. Always
runtime_qualified=false, production_pin_accepted=false and target_executions=0.
All failure/cleanup/publication paths clear final admission. A complete progress
trace or SDK subrecord cannot set a candidate flag.

Active expiry latches failure before any later child or candidate admission.
Only bounded own termination/drain, final own-source checks and receipt-only
hash/publication may use the same job's two-minute reserve after that latch;
they remain charged to the same32 GiB ledger and original monotonic deadline.
No bulk snapshot, output-admission salvage, new subprocess stage or timer/hash
pool reset becomes eligible in this failure phase. If those obligations cannot
finish, retain explicit unavailable cleanup/publication rather than promise an artifact.

The15 allowed names remain result.json, inputs.json, cargo.jsonl, diagnostics.txt,
compiler.json, oracle.c, oracle.i, sqlite3.lib, libsqlite3_sys.rlib, orkworksd.exe,
candidate.map, dependencies.txt, archive-members.txt, link-custody.json,
sdk-record.json. No raw PE inputs, crate caches, registry rejection data or
additional progress artifact. Traces and scalar allocation metrics live only in
bounded result.json. Receipt-only fallback must keep the valid prefix rather
than the spent publisher's minimal fallback that discarded diagnostics.

## Verification, approval and delivery

Before any live dispatch, synthetic tests must demonstrate v1/v2/candidate-schema
separation; preflight with an absent not-yet-fetched registry and strict missing-root
refusal in before/after; nonce/profile/policy binding; all exact phase sequences/counters;
byte-at-a-time/coalesced framing; malformed/partial/late/overflow/after-terminal
refusal; held output/source admission before settlement; full unknown-grant
charging and132 MiB reserve arithmetic; reader deadline crossing during spawn,
clean/error/bad close and drain-before-cleanup; receipt-only prefix retention;
exclusive leases/empty/large-then-short inputs and independent digest/ID goldens.
Compare fresh/reuse only on owned test inputs, with live selectors unreachable
from normal CI. Cover all legacy archive/helper/PE/config/SDK/source/encoding
refusals and unchanged actual-argv/map/archive evidence predicates.

A new in-process fixture harness stays within the synthetic envelope:120s active
plus120s drain/publication,64 MiB hash work,1 MiB child plus injected streams,
64 pure vectors/512 KiB injected,8 KiB/vector, three PS children30s each, one
final zero-hash Node timeout child, one owned termination,2,048 files/256 dirs/
4 MiB inputs/1 MiB per file,8 MiB scratch,64 KiB result/18 KiB traces. Model the
large live limits with lower per-vector admission thresholds/transport doubles;
never reset this suite's pool or create more children to test a boundary.
Normal CI may run only these owned fixtures/fakes and isolated SDK tests, never
installed discovery, package fetch or the live candidate main. Keep fixture
summaries tests_passed distinct from failed subjects and all qualification flags false.

After one scoped acceptance: record the ADR extension and concise implementation
plan; implement new files/config exceptions with TDD; verify/review the current
diff and required CI; merge tooling; freeze exact sources/policy/commands on
#1176 and verify new workflow history0/1. Only then perform the sole newly
accepted dispatch, independently read back bounded evidence, record success or
unavailable, close every unused slot and remove only owned working copies.
No automatic retry follows a useful diagnostic or a second timeout. A changed
candidate, method, ceiling, helper, output or workflow selection needs a concrete
new decision before dependent work. Gates1–4 and all source/compatibility/
bootstrap/native/recovery/durability/activation obligations remain separate.

## Investigation and review boundary

Least confidence: whether the full live closure fits120s and32 GiB. Inspected
source confirms that the old parser receives no report on rejected close, that
its grant was settled before output admission and that publication fallback
could discard prefixes. The synthetic fix tests those transport/settlement
boundaries; it cannot predict installed-input volume or a runtime hot path.
This proposal therefore retains deadlines, strengthens failure evidence and
reserves parent work; it makes no capacity promise or historical attribution.

The project could mistake more diagnostics for custody or repeated qualification.
Distinct protocol/result domains, unchanged checks, all-or-unavailable predicates,
new history1/1 and explicit authority boundaries prevent that interpretation.
Source pin and policy hash checks above are repository-only static evidence;
there was no native or installed-input experiment in this task.

Review routing fixed before dispatch: root sole writer; maximum two fresh
read-only Luna reviewers, one round. Builder reviewer asks which material
choices remain missing for implementation. Independent correctness reviewer
checks scope, accounting, command/input bindings and authority against source.
Both cite evidence and mark native claims UNVERIFIED. Root applies findings;
no new harness, live call, implementation or second subagent round. Private
function names/test organization remain builder choices; wire meanings, source
checks, caps, selected package and allowance are owner decisions proposed here.

## Proposal review disposition

One fixed read-only round completed. The builder found an ambiguous approval
phrase; root clarified one combined acceptance of new tooling, named candidate
and one job, with verification/freeze/history gates before that dispatch and no
second approval for unchanged choices. Root also closed the output envelope,
metric meanings and before/after comparison independently of synthetic A/B work.

The correctness reviewer questioned whether the132 MiB reserve omitted earlier
source passes. Root checked the proposed sequential-grant rule against
WorkBudget.remaining/reserve in the existing controller: each earlier pass is
already debited before the next grant. The reserve protects one outstanding
reader's output admission and final failure tail, not all job work at once.
The explicit L/H/R/G calculation above resolves that interpretation without
raising any limit. Static arithmetic modeled three settlements and maximal-h
refusal; it did not execute a reader or establish installed capacity. Both
reviewers left native closure/capacity and historical timeout cause UNVERIFIED.
No implementation, live inspection, workflow creation or dispatch occurred.

## Owner acceptance — 2026-11-28

The owner replied “keep going” after merged #2222 and its concrete combined
acceptance handoff. This accepts the named new tooling, unchanged candidate and
exactly one new 45-minute job together, with the verification, merge, freeze and
fresh-history gates above. It does not reopen earlier spent allowances. The
[implementation plan](../plans/2026-11-28-live-reader-progress.md) tracks delivery;
no installed-input execution follows merely from this status amendment.
