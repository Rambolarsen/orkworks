---
type: Qualification Audit
title: Replacement reader timeout source diagnosis
description: Source-traced work and missing progress evidence behind the spent replacement batch, with an owned synthetic timeout experiment and no runtime-cause claim.
tags: [orkworks, windows, qualification, diagnostics, resources]
status: draft
workflow_status: source-only-diagnosis-runtime-cause-unproven
---

# Replacement reader timeout source diagnosis

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: 51101e1299d5470888bef4a1f7510c5890c3946f. The
[terminal record](pair-resource-enforcement-coverage.md#replacement-build-terminal-result-2026-11-27)
remains authoritative for run 46020662577: the first source-verification reader
timed out after 120 seconds, before any build or oracle. Its allowance and all
prior allowances remain spent. This audit reads the sealed source and uses two
owned local Node children; it does not rerun the collector, fetch candidate dependencies,
inspect an installed SDK/toolchain or dispatch a Windows job.

**Confirmed diagnosis:** the reader combines several substantial operations in
one deadline and has no retained intermediate progress. Its source contains
repeated per-file allocation and metadata work. These are candidates for a
future controlled experiment, not proof of the production timeout's hot path.
The exact phase, completed file count, actual bytes read and Windows performance
cause remain unknown. Neither a larger timeout nor a faster implementation has
been justified or authorized.

## Frozen source and control flow

The inspected files are byte-identical to the new own sources frozen for the
spent batch at a37ec6196355b795d1d7fd25a0c9a7c06ad281a4:

| Source | SHA-256 |
| --- | --- |
| scripts/qualification/pair-replacement-windows-build.ps1 | 909e686436e6f512153634241cf28e67de941d50859d94950b5a779613a54c6e |
| scripts/qualification/pair-replacement-windows-build.mjs | db58846b43d8d3c12f1826a73a0597780a833455cd887916921019e45d19aa11 |

The Node controller gives the Manifest child 120 seconds, clamped to the shared
active deadline. It awaits a closed child before parsing a single JSON summary
and reading reader.json. A timeout rejects that await; it does not return the
already buffered stdout to the reader caller. If no validated terminal work
report was processed, the caller spends the entire delegated hash grant and
sets hash_work_unknown. The 32 GiB ledger therefore cannot be used to estimate
throughput. These are separate facts from the child's last completed work.
Sources: [child deadline/close handling](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.mjs#L354),
[reader invocation/accounting](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.mjs#L886).

Within that child, work is strictly ordered:

1. Start PowerShell and compile the own in-process native/helper declarations;
   then read the request/policy and validate exact roots.
2. Authenticate every selected crate archive against its lock hash, stream its
   members, compare extracted files, check reviewed entrypoints and scan for
   unlisted files.
3. Enumerate/hash the seven input roots; create every file/directory row and
   serialize rows to charge their encoded size.
4. Sort the manifest, serialize the combined result, enforce its byte cap,
   write reader.json, then print the terminal summary.

No phase/root checkpoint is emitted or written along that path. The catch emits
an error summary only if control reaches it; forcibly terminating the process
cannot be assumed to run that catch. inputs.json is created by a later Node
stage, so an initial-reader timeout supplies no partial inputs.json. Adding a
line of logging to the old stdout protocol alone would also break its single
JSON parser, and buffered text would still be lost to the caller on timeout.
Sources: [ordered reader body](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L235),
[post-reader admission and inputs publication](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.mjs#L1110).

Consequently the retained evidence cannot distinguish an archive walk, a slow
input root, row conversion, sorting or final serialization. The source's sorted
root order is msvc, registry, rust, sdk_bin, sdk_include, sdk_lib, source, but it
is unknown whether execution reached even the first root. Archive verification
runs before all of them.

## Work that byte ceilings do not measure

| Source-backed work | Cost implication; runtime cause remains unproven |
| --- | --- |
| Every reader starts a fresh PowerShell process and compiles its own C# declarations before the main try/catch. Archive mode then loads System.Formats.Tar. | Startup/type compilation is inside the same child timeout and also lacks a progress record. Its latency is unknown; a future phase contract must distinguish parent process launch from the first valid reader checkpoint. [Startup](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L5), [archive load](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L148). |
| A fresh 1 MiB byte array is allocated by each Read-SealedFile call, even for a zero-length file. | Many small files can request far more cumulative array capacity than their total payload size. This is not a peak-memory or garbage-collection measurement. [Read-SealedFile](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L66). |
| Each regular archive member receives another fresh 1 MiB buffer; extracted members are then hashed individually, and registry files are hashed again by the manifest. | For M successfully processed registry members, these three member-level sites request 3 × M MiB per completed Manifest invocation, excluding archive buffers, markers and other roots. For a hypothetical M=10,000, that is 31,457,280,000 bytes (29.296875 GiB) of cumulative array capacity. It is not a count of this runner's files, resident memory or hashed bytes. [Member stream and extracted verification](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L188), [manifest hashing](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L133). |
| Each Read-SealedFile walks ancestry twice, opens two FileStreams and invokes Get-HandleInfo three times; each invocation makes two explicit native queries. | A path with d entries in its ancestry loop has 2d explicit GetAttributes calls in these two walks, plus six explicit native handle/path queries and other metadata work. This counts source-level calls, not physical disk reads or uncached OS operations. [Ancestry and identity](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L38). |
| Each crate lookup scans all 345 policy package keys through a PowerShell pipeline. | For A extracted crate directories the expression evaluates 345 × A keys, up to 119,025 at A=345. The actual A is unknown. A future indexed lookup would still need to refuse missing or ambiguous package identities. [Lookup](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L157). |
| The archive verifier also scans each extracted tree; the manifest subsequently traverses registry again. | Package authenticity, extra-file refusal and snapshot membership have distinct obligations. Their repeated traversal is visible in source; removing a pass or reusing a hash needs a custody argument, not just a speed claim. [Extra-file scan](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L218). |
| Each manifest row and archive-member row is serialized for size charging, followed by final sorting and whole-result serialization. | File-byte/hash quotas do not by themselves express this per-row conversion work. The encoded-size refusal is intentional and must survive any future optimization. [Rows](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L111), [final serialization](https://github.com/Rambolarsen/orkworks/blob/a37ec6196355b795d1d7fd25a0c9a7c06ad281a4/scripts/qualification/pair-replacement-windows-build.ps1#L258). |

One selected root is the whole Rust toolchain, not just rustc/cargo binaries.
However, no completed per-root inventory exists from this attempt. Do not
attribute the timeout to Rust documentation, an antivirus, storage throughput,
a particular crate, the maximum allowed file count or JSON conversion alone.
Those would be runtime hypotheses without distinguishing evidence.

## Owned synthetic boundary experiment

On macOS with Node v22.23.5, two owned Node children each emitted the same
32-byte JSON text declaring success=false and hash_bytes=7. One exited; the
other stayed alive until a 500 ms timeout. The experiment imported only guarded
exports from the sealed Node source, never its main entry point. Each case used
a 1,024-byte synthetic hash ledger, 512-byte stream cap and three-second parent
budget; the normal child's command cap was one second. The seven declared hash
bytes are fixture data, not real file hashing or a claim about the spent job.

| Synthetic case | Bytes seen by stream ledger | Accounting report available to caller | Final synthetic hash charge | Drained |
| --- | --- | --- | --- | --- |
| report-then-exit | 32 | Yes; clean close returned stdout | 7 | true |
| report-then-stall | 32 | No; await rejected time-limit and carried no stdout field | 1,024, entire grant | true |

This confirms the timeout/report visibility boundary and conservative-reservation
behavior. It does not benchmark PowerShell, run the native reader or explain
which internal phase exceeded 120 seconds. A clean close exposing a report is
not source admission: the fixture deliberately declares success=false.

Reproduction from the inspected repository root, using Node 22:

```sh
rtk proxy node --input-type=module <<'JS'
import assert from 'node:assert/strict';
import {WorkBudget, CommandRunner} from './scripts/qualification/pair-replacement-windows-build.mjs';
for (const stall of [false, true]) {
  const b = new WorkBudget({hashLimit: 1024, streamLimit: 512, seconds: 3});
  const runner = new CommandRunner(b), finish = b.reserve(1024);
  let accounted = false, visible = false, reason = null;
  try {
    const code = 'process.stdout.write(JSON.stringify({success:false,hash_bytes:7}));'
      + (stall ? 'setInterval(()=>{},1000);' : '');
    const x = await runner.run(process.execPath, ['-e', code], {
      cwd: process.cwd(), env: {}, timeout: stall ? 500 : 1000,
    });
    const report = JSON.parse(x.stdout);
    assert.equal(report.hash_bytes, 7);
    visible = true; finish(report.hash_bytes); accounted = true;
  } catch (error) {
    reason = error.message;
    assert.equal(reason, 'time-limit'); assert.equal(error.stdout, undefined);
  } finally { if (!accounted) finish(1024); }
  assert.equal(runner.drained, true); assert.equal(b.reserved, 0);
  assert.equal(b.streamed, 32); assert.equal(visible, !stall);
  assert.equal(b.hashed, stall ? 1024 : 7);
  console.log({stall, visible, reason, charged: b.hashed, drained: runner.drained});
}
JS
```

This local synthetic experiment is not a workflow dispatch, Windows availability
probe, package fetch or execution of a resulting program. No raw artifact was
reacquired; the audit uses the already published terminal record.

## Proposed next scope, without implementation authority

The useful next deliverable is a **synthetic-only diagnostic reader contract**
for a new, unspent implementation. Resolve bounded progress framing, parent
retention during timeout, nonce/invocation binding, phase/root labels, monotonic
counter meanings and strict parser/error/overflow behavior before code. Fit the
record count/bytes into the existing per-reader report, shared stream, result
and artifact limits; name the arithmetic rather than silently raising them.
A progress checkpoint is diagnostic only: it cannot complete a manifest, admit
a build, prove a final seal or refund an unreported grant after termination.

The first source-backed optimization candidate is reusing bounded scratch
buffers within one reader invocation. Synthetic small-file/allocation and
identity/drift/encoding/cap tests must establish its behavior while retaining
all archive/member, reparse, file-ID, extra-file and post-seal checks. Compare
work by phase with identical synthetic inputs; treat wall-clock samples as
host-specific observations. Do not cache ancestry checks by path string, reuse
an unchecked source hash, exclude roots or remove a verification pass to meet
the deadline.

This is a recommendation for a separate concrete proposal, not approval to
edit the spent files, create a replacement live collector, broaden normal CI
into installed-input discovery, increase any quota or dispatch again. No new
numeric budget, execution allowance, artifact type or production pin is selected
here. Gate 1 remains Investigate and all source/compatibility/bootstrap/native/
profile/recovery/durability/activation gates remain. #1176 stays open.


## Proposed synthetic contract

The [synthetic progress/buffer contract](../superpowers/specs/2026-11-27-reader-diagnostic-contract-design.md)
now makes the recommended next scope concrete. It proposes fixed bounded
phase/root tuples, parent retention during a rejected child close and one
reusable payload buffer against the same owned fixture identities. The proposal
requires owner acceptance before implementation; publication grants no live
collector, installed-input probe, candidate execution or renewed allowance.
