---
type: Evidence Acquisition Proposal
title: One read-only Windows component inventory
description: Proposed finite file and SDK inventory to identify the changed compiler candidate without acquisition, compilation or execution of inspected tools.
tags: [orkworks, hierarchy, windows, toolchain, provenance, qualification]
status: draft
workflow_status: accepted-tooling-in-progress
---

# One read-only Windows component inventory

Outcome: retain the missing current compiler/file-version and component
co-availability metadata, or one terminal unavailable result. Tracking:
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Inspection base:
`11faa78457d84ee4cb56c103d01f0de0f75cc7e4`.

**New allowance proposed. This document and its merge authorize no runner job.**
The [retained provenance review](../../validation/pair-resource-enforcement-coverage.md#compiler-provenance-review-2026-11-26)
found an older default-selected compiler observation and a newer fixed-path hash
mismatch. It cannot prove the historical reason for the byte change. The
[fixed-build batch](2026-11-26-pair-fixed-windows-build.md#terminal-execution-record--2026-11-26)
and all earlier compiler/native/metadata batches remain spent.

## Selection and alternatives

Author recommendation: inventory the same fixed installed component paths on one
windows-2022 x64 runner before selecting a replacement build. Merely substituting
the observed hash would leave its version and SDK/helper co-availability unknown.
A second build would spend compiler/acquisition work before answering that smaller
question. A reconstructed old image/toolchain would introduce installation and
provenance decisions outside this request.

The observed new cl hash is an inspection candidate, not an accepted build pin:
`e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226`.
Record it against the old `fe251ef50a1545b1b0835ee17b1e785459712b38d79e45b5c1d3d28970a36619`
observation without claiming binary equivalence or a cause of the change. A third
hash is retained as drift/unavailable; no search for another compiler is allowed.

## Exact metadata surface

Use these fixed roots, without vswhere, developer-shell/default-toolset selection,
PATH search, installer repair, directory recursion or alternate-version fallback:

- Rust: USERPROFILE/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin.
- MSVC: C:/Program Files/Microsoft Visual Studio/2022/Enterprise/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64.
- SDK: C:/Program Files (x86)/Windows Kits/10/bin/10.0.26100.0/x64.
- Visual Studio: C:/Program Files/Microsoft Visual Studio/2022/Enterprise.

Read at most these13 PE files: Rust cargo.exe and rustc.exe; MSVC cl.exe, link.exe,
lib.exe, dumpbin.exe, c1.dll, c1xx.dll, c2.dll and mspdbcore.dll; SDK rc.exe and
mt.exe; Visual Studio Common7/IDE/devenv.exe. For each, record the selected path,
regular-file/reparse status, size, SHA-256, PE machine and bounded FileVersion/ProductVersion
resources. Read PE metadata as data; do not launch an executable, load an inspected
DLL or invoke a compiler version/help command. Missing/reparse/oversize files and
unavailable version resources are explicit observations. Missing required
compiler metadata or any of the13 selected files makes co-availability incomplete.
Rust release labels remain the previously observed hash pins when their PE
version resources are absent; do not infer a new Rust release from an empty field.

Also inspect only:

- The installed Windows Kits root registry value and existence of the fixed SDK
  Include/Lib/bin directories for10.0.26100.0, without recursive enumeration.
- Matching public Windows SDK product name/version fields from the two existing
  machine uninstall registry roots, maximum256 entries, bounded strings. Require
  servicing identity10.1.26100.9457 for the selected SDK; no Win32_Product query.
- The fixed Visual Studio VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt
  as bounded text, for context only. It never changes the selected14.44.35207 path.
- Actual runner OS/architecture and ImageOS/ImageVersion, reviewed checkout SHA
  and inventory-controller script identity/hash. No ambient environment dump or user configuration scan.

Cargo/rustc byte pins stay f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1
and d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1.
Record every available selected observation before classifying drift or missing
components, so one mismatch does not discard the diagnostic inventory. Do not
collect whole binaries, SDK trees, Cargo caches, certificates or unrelated files.
PE machine describes host-file architecture, not emitted target behavior.
File resources and image labels are metadata, not an authenticity/signature or
complete used-helper attestation; a later build would still need its full input seal.

## Proposed finite allowance

| Unit | Limit and terminal rule |
| --- | --- |
| Dispatch | One manual windows-2022 x64 job, no matrix,10-minute whole-job limit; one serialized allowance-specific first-run history guard. Any dispatch/preflight/infrastructure failure consumes1/1. |
| Reads | At most13 named PE files, each at most256MiB and at most1GiB total hashed; fixed text at most4KiB and registry maximum256 entries. No alternate path or retry. |
| Acquisition | Zero dependency/toolchain/SDK downloads, Cargo fetch/metadata, package-manager probes or installation. Checkout and the sole evidence upload are workflow infrastructure. |
| Execution | Only the reviewed inventory controller and standard workflow infrastructure. Zero execution/loading of inspected tools/DLLs; zero compiler/linker/archiver, Cargo/rustc CLI, sidecar, SQLite/SQL, native harness, model or qualification execution. |
| Evidence | One allowlisted artifact: result.json, inventory.json, diagnostics.txt; at most4MiB uncompressed,14-day retention. Overflow retains only a bounded failure receipt, never truncated successful evidence. |
| Retry | Zero reruns, second dispatches, alternate images/components, live repairs or fallback queries; all unused slots close with the sole dispatch. |

The controller may read public Actions history for its first-run guard. Its
minimal environment must exclude credentials from collected metadata/diagnostics;
the history credential stays confined to that read. Freeze the merged reviewed
tooling/source SHA on #1176 before dispatch. Recheck selected file identities
after collection; drift denies co-availability. Stage evidence in new owned
scratch, drain controller references, retain the bounded receipt and remove owned
working scratch. Do not edit consumed workflows/history or the fixed-build pin.

## Deliverables, checks and authority boundary

- [x] Owner accepts this exact new inventory-only allowance.
- [x] Prepare a separate manual workflow/controller with offline behavioral tests
  for non-dispatch/rerun denial, exact file allowlist, no inspected-tool launch,
  missing/drifting inputs, metadata retention, byte/entry/time/evidence limits
  and cleanup. Review the exact controller selectors before dispatch; reuse
  existing guard patterns without changing the spent workflow.
- [ ] Freeze reviewed merged heads/counters, dispatch once, independently verify
  the terminal JSON/file hashes and record the result in the existing coverage
  concept via PR. No inspected binary is uploaded or executed.

A complete inventory answers current metadata/co-availability only. It does not
prove why the old hash changed, approve a replacement compiler/version, qualify
macro/link/runtime behavior or grant a later build. Review the actual inventory
before proposing any new exact build selection and separate finite allowance.
No bootstrap/allocator/VFS/context implementation, compatibility, source/cutoff/
newness/catalog/initializer/recovery/durability or production activation follows.

## One-time plan review

Disposition: **Ready for scoped allowance review; no execution authorized.**
The proposal addresses the missing current metadata without repeating a failed
build or claiming that a hash-only replacement supplies provenance. Unknown
historical identity remains unknown; absent components produce a useful terminal
record rather than installation or another selection. The path/count/byte/entry/
time/artifact limits and no-execution checks are explicit acceptance criteria.

Dependencies3 (hosted image and installed component metadata); blast radius1
(read-only disposable inventory); state changes1 (owned receipt/scratch);
reversibility1 (scratch disposal, no installation); uncertainty3 (current
co-availability unknown, historical cause cannot be recovered by this job).
Total9 is a review aid, not approval. No architecture/resource-policy change is
proposed. This proposal was written using repository/retained job records and
official image manifests; no new runner or inspected-tool operation ran.

## Scoped allowance accepted (2026-11-26)

The owner answered “keep going” to the explicit #2076 question approving tooling
plus this one10-minute inventory job. Proposal merge:
d67135d00cb764fb23ac9126ecaff5152e4837f0. This accepts only the listed metadata
surface, limits and terminal-record work. It grants no inspected-tool execution,
replacement build pin or later build. Prior batches remain spent. Dispatch0/1;
reviewed tooling and exact merged-head freeze are still required before dispatch.

## Tooling review disposition (2026-11-26)

One bounded independent read-only round reviewed correctness/allowance safety and
acceptance/evidence coverage, with the owning agent as sole writer. No inspected-
tool execution path or additional read allowance was identified. Review findings
clarify delivery and verification: pre-controller admission or unavailable runner/
checkout/storage can yield only an Actions terminal outcome, without a JSON
artifact. Retain that outcome as unavailable and do not invent inspection counts.
Ordinary failures after owned writable evidence setup retain the bounded receipt.
This is not retry credit or a stronger artifact-delivery guarantee.

Registry pre-enumeration and additional PE section/directory/length branches have
source review rather than exhaustive behavioral fixtures. Tests cover output
policy denials, valid embedded fixed-info, invalid RVA, byte/text limits and actual
controller timeout/output closure. These limits prevent treating tooling tests
as authenticity, full helper closure or profile qualification. Windows synthetic
reader tests passed on the completed code; normal repository CI remains a
separate pre-merge requirement. Dispatch is still0/1.
