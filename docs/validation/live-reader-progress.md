---
type: Validation Record
title: Live progress reader verification
description: Separates owned-fixture verification of the isolated live reader from the sole gated candidate attempt.
tags: [orkworks, qualification, windows, diagnostics]
status: stable
workflow_status: terminal-unavailable-allowance-spent
---

# Live progress reader verification

Tracking [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
The owner accepted the [combined package](../superpowers/specs/2026-11-27-live-reader-integration-design.md)
after #2222. [ADR amendment](../adr/0123-synthetic-reader-diagnostics.md#amendment-2026-11-28-isolated-live-profile)
and [plan](../superpowers/plans/2026-11-28-live-reader-progress.md) precede code.

## Fork inventory and current evidence

New pair-progress-windows-build files fork the complete replacement reader and
candidate stage logic. Candidate inputs, tool selectors, helper closure, SDK
selector, macro/startup flags and actual-argv/map/archive predicates stay exact.
Only v2 request/output domains, fixed derived roots, bounded streaming retention,
one leased payload scratch array, expanded-read precharges, monotonic scheduling,
source/output-before-settlement, protected parent admission and receipt retention
change. The SDK wrapper changes only its own environment marker domain. Every
old replacement and synthetic source/workflow remains unchanged. The sole old
CI exception is one five-minute direct Node fixture step; LF rules name only
new own files. No dependency is added.

Local Node tests cover v1/v2 separation, canonical framing, malformed/late/extra
bytes, strict output parsing, unknown/known settlement, decreasing grants,
parent-admission reserve, source/output refusal, inherited tool/manifest/SDK
predicates, transport doubles and deadline crossing. Local Python tests execute
six fake-registry cases and explicitly skip the one Windows-only HKCU fixture.
One fixed read-only review round covered correctness and missing coverage. Its
findings led to a shared strict request decoder/manifest pipeline/output writer
exercised by the native fixture, explicit conservative row/output encoding
precharges, and an explicit unavailable-publication outcome on reserve expiry.
Full repository verification passed at the unchanged product base: 3,752 Rust
unit tests (25 ignored) plus integrations, 59 OpenCode reporter tests, desktop
type/native checks, 1,815 desktop passes (five skipped), desktop build and all
2,583 docs tests/build. Warnings concern unchanged Rust/bundler code. Later
qualification-only fixes rerun their scoped Node tests and hosted CI.

The owned Windows producer-to-parent fixture passed at code head d35d817a3e486ed30a8e721ef64ad564dd0d741a
in [job140880625282](https://github.com/Rambolarsen/orkworks/actions/runs/46113026445/job/140880625282):
Node 22.23.5; 25 tests; 21 progress records,1,157,447 charged fixture bytes,
2,867 stream bytes,one PS child,one final zero-hash Node timeout,one payload
allocation/max lease,one owned termination and scratch removal. This tests
owned inputs, not installed candidate capacity. Earlier native runs exposed a
PowerShell counter/fixture scope collision and truncated fixture-summary JSON;
both were corrected without weakening an assertion or extending a timeout.

Hosted review found a successful-publication deadline gap. Injected-clock
regressions reproduced expiry before/during writes and at the final result
write; publication now withdraws queued records before refusal. Further tests
protect final receipt credit from ordinary work and aggregate snapshot costs,
and require undrained pipes to detach at the final deadline. Local suite now
has 24 passes and one Windows skip. Final code head d35d817a3e486ed30a8e721ef64ad564dd0d741a received a
completed Codex no-findings review (cycle3/3). Its Windows SDK/reader fixture,
docs build, workspace-reporting, Windows process-ownership, required Desktop/Rust
and all auxiliary CodeQL/custom-inference checks passed. No further code
review round is inferred. A docs-only evidence update preserves those code blobs.
An auxiliary Windows Job failure at earlier head84c19fb0 matched open [#2211](https://github.com/Rambolarsen/orkworks/issues/2211);
no ownership code/assertion was changed, and the later pass does not classify that
earlier occurrence. Passing model tests do not establish physical native work,
capacity, historical timeout cause, runtime qualification or production pins.

The normal-CI live-reader harness reaches only owned synthetic files, archive
refusals and process doubles, at most one PowerShell fixture child and one final
zero-hash Node timeout child. Its PS tests report cumulative hash work, exclusive
allocation metrics and false qualification flags; parent applies the cumulative
64 MiB work and 1 MiB streams ceilings. Installed selectors/live main cannot be
called from these fixtures. The cited native evidence applies to that exact hosted source head; later code
changes need their own passing fixture before merge. Encoding work is deliberately overcharged: each primary row
reserves 512 plus six times the root/path/identity string lengths before JSON
conversion; archive member rows likewise use their names/digests. The final
serializer charges its whole allowed output ceiling before allocating JSON/UTF-8.
These conservative logical debits are not physical bytes or throughput. TAR
request precharges update the same budget immediately, so serialization cannot
spend pending decompression credit. No precharge is refunded on EOF or error.

## Sole candidate attempt — terminal unavailable (2026-11-28)

[Tooling PR #2241](https://github.com/Rambolarsen/orkworks/pull/2241) merged as
d870796b634a0f8259d0c418fb0455ea0079a8a1. Its final Markdown-only tip43144233
preserved every code/config blob reviewed at d35d817a; all final-tip required and
auxiliary checks passed. The [immutable freeze](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7235038592)
preceded fresh zero-run history and an unchanged-main check, then the sole
[run46114787230](https://github.com/Rambolarsen/orkworks/actions/runs/46114787230),
attempt1, was created2026-11-28T06:02:42Z at that exact SHA. History after completion
contains exactly that one run. Allowance is **1/1 spent**, with zero retries.

The [candidate job140886056904](https://github.com/Rambolarsen/orkworks/actions/runs/46114787230/job/140886056904)
ran06:02:46–06:05:24 UTC on win22/X64 image20261123.376.1. The controller records
06:02:59.489–06:05:20.578 UTC,141.089 seconds. Node22.23.5, PowerShell7.6.10 and
the fixed CPython3.12.10/X64 runtime were observed. These identities and byte
hashes describe the selected trusted base; they do not attest OS/vendor authenticity.

**Result: unavailable at stage=sources, reason=time-limit.** The complete before
manifest reader reached its120-second child deadline. Twelve selected PE pins,
one named SDK pass, the Cargo/rustc version queries and one locked fetch preceded
it. No complete input manifest, build, oracle, linked candidate, second SDK pass,
after reader or target execution was admitted. Final success,
candidate_evidence_complete, runtime_qualified and production_pin_accepted are
false. All unused later slots are closed; remaining time/hash credit is not a
renewed allowance. No spent workflow/policy/pin/history was changed.

### Retained progress and limits of attribution

The receipt retains ten validated arrival rows across two invocations. Parent
arrival times use the original monotonic job clock; they are not per-phase CPU
or physical I/O durations. Nonces are distinct and bound to v2 tuple sequences.

| Reader / checkpoint | Parent elapsed ms | Logical h | f / d / a |
| --- | ---: | ---: | --- |
| preflight start | 2,178 | 0 | 0 / 0 / 0 |
| preflight ready | 5,721 | 43,200 | 0 / 0 / 0 |
| preflight serialize-begin | 6,556 | 122,095,024 | 0 / 0 / 0 |
| preflight serialize-end | 6,573 | 189,203,888 | 0 / 0 / 0 |
| preflight ok | 6,574 | 189,203,888 | 0 / 0 / 0 |
| before start | 19,538 | 0 | 0 / 0 / 0 |
| before ready | 20,290 | 43,200 | 0 / 0 / 0 |
| before archives-begin | 20,295 | 43,200 | 0 / 0 / 0 |
| before archives-end | 37,084 | 2,176,513,082 | 0 / 0 / 257 |
| before root-begin / msvc | 37,100 | 2,176,513,082 | 0 / 0 / 257 |

The before reader emitted no root-end or terminal tuple. Its last validated
checkpoint starts the MSVC root, following an archives-end self-report of257
completely verified packages. This is narrower evidence than the old spent
reader's missing report. It does not establish completed MSVC file/directory
counts, a complete package/input inventory, work after that checkpoint, an
individual slow file, allocation/RSS pressure, throughput or the historical
attempt's cause. Zero f/d in the last row does not mean no files were subsequently
processed. No raw partial manifest was admitted or salvaged.

Only the completed preflight has admitted allocation metrics: one1 MiB payload
scratch allocation, max one lease and61,069,112 payload hash bytes including the
43,200-byte policy. Its PE payload sum is61,025,912; policy plus two PE passes is
122,095,024, then the conservative64 MiB output-encoding charge yields189,203,888.
The failed before reader's metrics are unavailable, not fabricated zeroes; it
supplies no measured allocation reduction for that full live closure.

The incomplete before reader spends its entire33,944,060,067-byte grant once,
regardless of the earlier2,176,513,082-byte checkpoint. Total job charge is
34,221,592,929 bytes within32 GiB; unknown hash work remains explicit. That figure
is conservative logical work, not measured physical bytes. After the unknown
settlement, the200,304-byte final source pass,753-byte SDK receipt and65,536-byte
result commitment precharge account for266,593 further bytes. Remaining credit
is138,145,439 bytes and is closed to further work. Job streams total10,154 bytes,
well within64 MiB. One owned termination drained; the receipt reports
owned-scratch-removed. Failure publication retained the prefix while dropping
bulk evidence, rather than using emergency credit to salvage it.

The SDK subrecord is incomplete: one named pass,13 API calls,1,358 metadata
bytes,1,000 hashed bytes,zero open handles and zero enumeration. Named
product10.1.26100.9457 and its full-width write markers matched within that pass;
no same-job before/after bracket exists. SDK success and positive_record_observed
remain false and inventory_complete remains false. The SDK ledger including its
753-byte receipt charges1,753 bytes, below64 KiB. A partial SDK observation does
not admit the candidate.

### Independent bounded readback

[Artifact15251440704](https://github.com/Rambolarsen/orkworks/actions/runs/46114787230/artifacts/15251440704),
pair-progress-windows-build, contains exactly two allowlisted files and9,740
uncompressed bytes. The3,912-byte ZIP SHA-256 is
d4c6674258152fa34e3b8cfce1298752da56abd898d5f4296fe25dcac42b84c4,
matching the Actions artifact digest. Expiry is2026-12-12T06:05:20Z (14 days).

| File | Bytes | Independently recomputed SHA-256 |
| --- | ---: | --- |
| result.json | 8,987 | 56fb24f4e7ec8c1b97583bc67783f1df7823543c2108990df3381feb21ed9b35 |
| sdk-record.json | 753 | 6939c967ad9fecccccc5b84c2452389890fc821389d0c6326f1d35a2d72af2bf |

A separate Python readback, without collector/parser imports, bounded the
compressed download, rejected foreign/duplicate/archive-link members, checked
complete UTF-8/duplicate-free JSON and every retained commitment, and verified
run/head/attempt/history, false authority flags, all twelve PE pin reports,
strict reconstructed v2 phase/nonce/sequence/counter/line limits, source byte
bindings, SDK counters and six recorded command derivations. The commands are
two own readers, one isolated SDK child, one Cargo -Vv, one rustc -vV and the
literal locked fetch. No build/oracle/inspection/target command appears.
Reconstructing retained tuples checks their representation; it is not a second
observation of the original wire or native filesystem. PE reports are not
independent rehashes of raw installed binaries, which were not downloaded.

All ten initial controller-source reports match the frozen Git bytes with their
recorded materialization: six new own/workflow files stay LF; four untouched
imported historical sources match LF-to-CRLF conversion. Combined Git blobs are
198,617 bytes and runner source bytes200,304, within the1 MiB per-pass cap.

| Source | Runner encoding / bytes | Matched runner SHA-256 |
| --- | --- | --- |
| scripts/qualification/pair-progress-windows-build.mjs | LF / 48127 | 76fa8d2415dde8d124987419bc93bd62c386884358e1fe43f63e72d635ba318c |
| scripts/qualification/pair-progress-windows-build-protocol.mjs | LF / 7759 | 1cc13ccf92db909a8cd6fed45103181801bc0a2572543170e1f37a67944eaa6b |
| scripts/qualification/pair-progress-windows-build.ps1 | LF / 26841 | f8160c28432db50052390300775fedf1dac9ac43191c4daa2e831ae44f262d0b |
| scripts/qualification/pair-progress-windows-build-sdk.py | LF / 4309 | 49e527db40b71ecb056137b27ffb8e7bf005d3c5009f2f5d1cd5577bed702a7e |
| scripts/qualification/pair-progress-windows-build.inputs.json | LF / 43200 | 1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6 |
| scripts/qualification/pair-fixed-windows-build.mjs | CRLF / 31274 | f3af7ca64074db7ce2bc3c30aff68af046c4147e9c4283045faf03eb288ae529 |
| scripts/qualification/pair-fixed-windows-build.inputs.json | CRLF / 8350 | f8eccaafd28e77966f6614c1ec4716c6cd3acb18d5bfc437fd6f3dd216c653db |
| scripts/qualification/pair-windows-metadata-evidence.mjs | CRLF / 13145 | 19aaf7a47a117951820325b33fbeb4206682e00464883990c9f81872d0e20de2 |
| scripts/qualification/pair-windows-sdk-positive-record.py | CRLF / 14786 | acca2438deef46c1453a46186853e560b1c7bba03424881b015a36439978f5a8 |
| .github/workflows/pair-progress-windows-build.yml | LF / 2513 | f5353d7d7753861af2a6504e2ff09645ad1eff57b08c2270529a0e2eced09ae2 |

This checks retained initial hashes against the frozen sources. It does not
independently reproduce a second native source/input seal or recover the
missing reader output. The sealed parent path checks its final sources before
failure publication; the retained reason is time-limit, not source-final-unavailable.
No retained program or installed input was executed or loaded as code for
readback. Raw local archive/receipt copies are removed after this terminal record
is published; the hosted artifact keeps its fixed expiry.

### Disposition and separate follow-up

Gate1 remains Investigate. Gate2/bootstrap, actual macro/link custody, startup
and ordinary compatibility, source/newness/cutoff/catalog/initializer, native
mechanisms, recovery/durability, complete profile and activation remain gated.
Every prior allowance and this allowance are spent; no retry, repaired input,
changed selector, larger ceiling, new availability probe or automatic follow-on
job is authorized. The qualification umbrella #1176 stays open.

The next useful separate task is a source-only audit of the frozen MSVC-root
walk and the remaining checkpoint gap. It should distinguish visible source
costs from unknown runtime causes before proposing any changed reader or new
finite experiment. This terminal record does not start or authorize that work.
