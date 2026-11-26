---
type: Evidence Acquisition Proposal
title: One SDK servicing registration observation
description: Proposed SDK-only registry observation after the spent toolchain inventory reached its registry-entry limit, without accepting a compiler pin or build.
tags: [orkworks, hierarchy, windows, sdk, qualification]
status: draft
workflow_status: proposed-awaiting-owner
---

# One SDK servicing registration observation

Outcome: retain a bounded current Windows SDK product-registration observation,
or one terminal unavailable result, before considering a replacement build.
Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: `efb0117f3876c51c5a3125963df6ba43759b334a`.
**Proposal only. Drafting, merging and ordinary PR checks grant no qualification
job, registry query, new build or retry allowance.**

## Evidence and smallest useful next step

The [completed inventory](../../validation/pair-resource-enforcement-coverage.md#read-only-inventory-terminal-result-2026-11-26)
retained all13 fixed PE identities on run45889209265, including the new compiler
version and stable candidate hash. Its two uninstall roots contained688 entries;
the accepted256-entry pre-enumeration guard correctly stopped before scanning
product records. This is missing servicing evidence, not an absent SDK package.
The rc/mt file versions and fixed SDK directories do not replace that evidence.
The [old inventory allowance](2026-11-26-pair-toolchain-inventory.md#terminal-execution-record-2026-11-26)
is fully spent, including unused slots; leave its tooling, limits and history unchanged.

Author recommendation: a new SDK-registration-only observation with a separately
reviewed1,024-entry combined ceiling, two bounded comparison passes and no PE reads.
The higher cardinality is an explicit new cost for owner review, not a repair or
silent increase of the old256 limit.1,024 is an author-selected cap above the
observed688, not measured assurance that the next hosted image will fit.

Alternatives: a fixed product-code lookup would read less, but no exact SDK
registration key was retained and inventing one would risk a false absence.
Reusing the public image manifest would repeat image-build context rather than
observe this runner's registration. A full toolchain rescan or another build
would repeat work beyond the missing SDK product question. No historical compiler
cause can be recovered by this proposed observation.

## Selected metadata surface

Use only the64-bit local-machine registry view and these exact roots:

- `SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`.
- `SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall`.
- `SOFTWARE\Microsoft\Windows Kits\Installed Roots`, value `KitsRoot10` only.

The first two reproduce the old selectors, without accidentally applying a
second32-bit redirect. Open read-only. Check both uninstall subkey counts before
materializing names: their combined total must be at most1,024. Enumerate only
immediate names, at most256 UTF-16 code units per name, without recursive child
or value enumeration. Read only `DisplayName` for each entry; a missing value is
a recorded nonmatch. A non-string, oversized, inaccessible or malformed value
makes completeness unavailable rather than being silently skipped. Require REG_SZ
for collected strings; use DoNotExpandEnvironmentNames and reject other value
kinds without expanding them.

Match the literal case-insensitive prefix `Windows Software Development Kit`
with either end-of-string or a following space. For at most8 matching entries,
read only `DisplayVersion` and `Publisher`; retain the root/subkey identity and
those three fields. Require exactly one matching record with DisplayVersion
`10.1.26100.9457` and Publisher `Microsoft Corporation`, both ordinal exact
comparisons without trimming, for the selected observation. The DisplayName prefix
uses ordinal case-insensitive comparison; its suffix is retained context, not a
second version selector. This identifies a product-registration claim only, not
a proven association between every package component and the fixed directory.
Retain other SDK matches as context, never select another servicing version.
Zero expected matches, multiple expected matches, absent/malformed fields or
more than8 SDK matches are unavailable, not fallback selection or proof of absence.
Do not read uninstall commands, installer locations, user hives or other properties.

Read KitsRoot10 and check directory existence/reparse status for only the three
fixed leaves `C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0`,
`Lib/10.0.26100.0` and `bin/10.0.26100.0` under that same SDK root, and each
ancestor back to C:/. This is ten distinct fixed directory probes per pass.
Check ancestors before descendants; a missing/reparse/unknown ancestor denies
without following it. Normalize only slash direction and one optional trailing
separator in KitsRoot10, then compare ordinal case-insensitively to the fixed
root. Reject relative/UNC/device paths, dot segments, expansion strings or other
forms; no general path resolution, recursion, contents or alternate roots.
This observes registration plus directory presence, not correct header/library bytes.

Bound returned registry strings to512 UTF-16 code units for DisplayName,
64 for DisplayVersion,128 for Publisher and260 for KitsRoot10. Use standard
managed read-only RegistryKey operations in the reviewed PowerShell controller;
these are post-read field limits, not a pre-allocation or native-memory guarantee.
Charge both passes against an8MiB cumulative metadata limit. Each attempted
entry contributes one UTF-8 JSON array `[rootIndex,subkeyName,displayName]`, with
null for a missing name value; each SDK match additionally contributes
`[rootIndex,subkeyName,displayVersion,publisher]`. Count the bytes of compact
JSON.stringify-compatible encoding plus one LF per array, without Unicode
normalization; include nonmatches, failed-field nulls and bounded status arrays
for root/directory probes. Reserve the maximum allowed next array before reading
its fields, then charge its actual encoding; an insufficient remainder denies
before that read. Refuse overflow; never truncate success. This is logical
metadata accounting, not a memory-allocation guarantee.
No Win32_Product/WMI/installer API, reg.exe, developer shell, Add-Type/native helper
or additional child tool is selected.

## Observation consistency and evidence limits

Allow at most one initial pass and, only after its complete eligible result,
one comparison pass. A failure stops collection; these are not retries. Each
pass includes both
root counts, a bounded complete name set, selected values, KitsRoot10 and directory
observations. Compare sorted full root/name/DisplayName observations, all SDK
fields and directory observations; duplicate names, partial enumeration or any
change is unavailable. Compare names rather than index order. No rescan until
stable or per-value retry is allowed. Missing DisplayName is distinct from empty.
These two reads are a drift check, not an atomic registry snapshot or protection
against a change-and-revert between reads. State that limit in the receipt.

Retain only SDK-matching records, fixed root/directory observations, per-pass
counts, consumed metadata bytes and a SHA-256 commitment of the complete sorted
scan used for comparison: sort arrays by root index then ordinal subkey name,
serialize with the byte encoding above and hash the LF-delimited concatenation.
Include missing-value status distinctly; store only a null commitment for an
incomplete pass. Nonmatching application names/keys/fields stay in
bounded owned memory and are discarded; do not put them in logs or artifacts.
Keep actual ImageOS/ImageVersion, OS/architecture, exact checkout and controller
hashes and timestamps. Metadata is neither installer authenticity nor complete
component custody. Do not execute or upload an SDK or compiler binary.

A successful result concerns this runner only. It must not be joined to the
older13-file inventory as if both came from the same host or image lifetime.
It accepts no replacement compiler pin and proves no build, linked inputs,
macro setting or runtime behavior. Any later candidate-build proposal must close
its own exact co-availability/input-seal obligations using its approved identities.

## Proposed new finite allowance

| Unit | Proposed limit and terminal rule |
| --- | --- |
| Dispatch | One manual windows-2022 x64 job,10-minute whole-job bound, serialized allowance-specific first-run guard; any submitted or ambiguous dispatch/preflight/infrastructure failure consumes1/1. |
| Controller | One reviewed PowerShell controller process,60-second wall bound and256KiB combined captured stdout/stderr; drain/terminate before scratch removal. Only standard workflow/Node admission and upload infrastructure besides that controller. |
| Registry | Two complete passes, each at most1,024 combined uninstall entries, hence at most2,048 entry observations; at most8 SDK matches per pass. Only the named fields/roots; all successful, missing and failed probes count toward the relevant allowance. |
| Metadata |8MiB cumulative canonical observed names/fields across both passes; field/name bounds above. These are read/evidence limits, not a hard process-memory or native allocation cap. |
| Files | At most ten fixed SDK directory status probes per pass; no SDK/tool file reads, hashing, copying or DLL loading. Reading/hash-pinning the reviewed controller source is infrastructure, not permission to inspect binaries. |
| Evidence | One allowlisted artifact containing result.json, sdk-registration.json and diagnostics.txt, at most1MiB uncompressed,14-day retention. Only fixed error categories/numeric codes; no raw exceptions, unrelated registry data, environment or credentials. |
| Acquisition and execution | Zero tool/dependency downloads, installation, Cargo/rustc/SDK/compiler CLI, fetch/metadata/build, SQL/SQLite/native harness/model or resulting-program execution. Registry reads use existing OS/managed APIs only. |
| Retry | Zero reruns, second dispatches, alternate image/SDK/version, limit increase, repair or fallback query. All unused slots close with the sole dispatch. |

The controller environment is constructed, not inherited: only SystemRoot,
WINDIR, owned TEMP/TMP, PATH set to SystemRoot/System32, validated ImageOS/
ImageVersion/OS/architecture/run identity and literal selected budget variables.
Use an explicit PowerShell path supplied to the Node admission process, not a
PATH search; no user profile is loaded. History credentials stay in Node
admission and never reach the controller; no ambient environment dump. Freeze reviewed merged source/controller
SHA and counters on #1176 before dispatch. Use new owned scratch, close all registry
handles, drain the controller, retain the bounded result and delete only owned
working scratch. Pre-controller runner/checkout/storage failures may leave only an
Actions terminal outcome; do not invent an artifact or read counts in that case.

## Terminal receipt contract

The implementation may choose field names, but its reviewed closed versioned
JSON schema must preserve these meanings: selected source/controller hashes,
run/attempt/runner identity, admitted limits, per-pass attempted entry/field/
directory counters and charged bytes, complete/incomplete flag and scan digest,
SDK matches, root/directory statuses, comparison result, controller exit/signal/
timeout/output-limit disposition, artifact sizes/hashes and owned-scratch cleanup.
Result is `observed` only for the unique exact expected product and complete
matching passes; otherwise `unavailable` with a closed fixed category. Required
categories distinguish entry/field/metadata limit, missing/ambiguous SDK, drift,
controller timeout/output/failure and infrastructure. Numeric OS error codes are
optional; raw exception strings never substitute for these fields.

Charge an entry attempt before its DisplayName read and a field/directory attempt
before the corresponding probe; duplicate/failed probes consume rather than
refund. Enumerated names themselves remain subject to the pass cardinality guard
and byte reservation. Do not issue another probe to recover missing counters.
A partial controller receipt records only known counts and a null incomplete-pass
digest. If timeout/kill prevents a receipt, the Node wrapper retains the controller
outcome and marks internal counts unknown (null), with admitted maxima separate;
it must not invent zero reads or claim completed cleanup before process closure.
Evidence failure/overflow yields a bounded failure receipt or Actions-only outcome,
never truncated successful JSON. No partial pass can classify SDK absence.

## Delivery and verification handoff

- [ ] Obtain owner acceptance of this exact SDK-only surface and new limits.
- [ ] Then implement separate manual workflow/controller and allowance-specific
  admission checks; do not modify the spent inventory workflow. Use synthetic
  registries/data for ordinary CI, never inspect installed SDKs in PR tests.
- [ ] Cover688 entries completing under the new cap,1,024 boundary versus1,025
  refusal before name enumeration, both-root accounting, missing/malformed/oversize
  data, SDK prefix/version/duplicate-match rules, metadata/output/time/artifact
  limits, drift and unordered-name comparison, redaction and handle/process cleanup.
  Prove tests fail without their selected guards, then pass. Review exact selectors
  and all result/denial paths before dispatch; no product/native test claim.
- [ ] Freeze reviewed merged tooling and unused counters, use the sole dispatch,
  independently read back its terminal receipt/Actions outcome and retain the
  result in the existing enforcement coverage concept through a result PR.

Success requires bounded complete matching evidence and stable comparison, not
merely controller exit0. Missing, changing or excessive inputs yield one useful
terminal unavailable record with no new allowance. #1176 remains open; Gate1
remains Investigate. Bootstrap/source/ordinary compatibility, native resource/
allocator/VFS/context, initializer/recovery/durability and activation stay gated.

## One-time plan review

Disposition: **Ready for scoped allowance review; execution remains unapproved.**
This proposal separates the old safe refusal from the new larger enumeration cost,
excludes binary/toolchain work and names its non-atomic/post-read limits. Logical
registration is not source/native eligibility or a replacement build decision.
The implementation and result checks above cover the selected observations and
terminal failures; unavailable infrastructure has an explicit Actions-only outcome.

Dependencies3 (hosted runner and public installation metadata); blast radius1
(read-only observation); state changes1 (owned evidence/scratch); reversibility1
(scratch disposal, no installed changes); uncertainty3 (future registry contents
and non-atomic drift observations). Total9 is a judgment aid, not approval.
One bounded round used two fresh read-only Luna reviewers, root as sole writer.
Correctness found no material contradiction. Builder questions concerned exact
matching/path normalization, byte commitments, receipt/partial counters and the
collection environment; the author resolved those choices above. No new owner
acceptance or second review round is claimed. All limits remain proposals.

## API basis

Microsoft's [uninstall property mapping](https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key)
identifies DisplayName/DisplayVersion/Publisher as product name/version/manufacturer
metadata; it does not authenticate this installed SDK. Registry enumeration is
[unordered and may change](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regenumkeyexw).
The controller must compare complete bounded sets and disclose non-atomicity,
not infer a consistent snapshot from equal counts. The API facts inform this
proposal; its ceilings and matching policy are author choices for owner review.
