---
type: Evidence Acquisition Proposal
title: One registry-only Windows SDK servicing inventory
description: Proposed finite registry-only observation of the missing SDK servicing identity after the consumed component inventory exceeded its entry budget.
tags: [orkworks, hierarchy, windows, sdk, qualification]
status: draft
workflow_status: accepted-tooling-in-progress
---

# One registry-only Windows SDK servicing inventory

Outcome: retain a bounded SDK product/version observation, or one terminal
unavailable result. Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: efb0117f. The [completed component inventory](../../validation/pair-resource-enforcement-coverage.md#read-only-inventory-terminal-result-2026-11-26)
identified cl.exe19.44.35230.0 and13 stable file identities, but its256-entry
pre-enumeration cap rejected688 aggregate uninstall entries. It scanned zero
entries. Its dispatch and all earlier allowances remain spent.

**This is a new proposal. Its merge authorizes no tooling implementation or runner
job.** The owner requested this narrower evidence proposal after #2088; no new
registry read, runner query, acquisition or inspected-tool execution occurred.
The [resource qualification plan](2026-11-24-pair-resource-assurance-qualification.md)
and its source/runtime/compatibility gates remain unchanged.

## Selection and alternatives

Author recommendation: isolate the missing SDK registry observation from the
previous13-file inventory. Increase only this new request's entry capacity to
1,024 per pass; remove every target PE, directory and default-toolset read.
The larger entry cap is a new requested allowance, never a retry or amendment
of the spent256-entry batch. The retained688 count motivates the bound but does
not guarantee that a later image fits. Oversize/missing/drifting inputs remain
useful terminal unavailable results; there is no further capacity escalation.

A direct known product subkey would reduce work further, but the retained
receipt contains no such key. The inspected [pinned Microsoft package tree](https://github.com/microsoft/winget-pkgs/tree/abf6b6c222be86689a6ceedae5c301e401ea4a45/manifests/m/Microsoft/WindowsSDK/10/0/26100)
lists releases through10.0.26100.7705 and does not contain the exact9457 release.
This is a statement about that inspected tree, not all possible vendor records.
Do not invent a product GUID or extrapolate one from another release. Installer
acquisition/extraction, package-manager queries and repair would add unnecessary
inputs/execution. The already observed rc/mt versions and public image manifest
do not replace an installed SDK product record.

## Exact registry surface and controller

Use local HKLM, explicitly the64-bit registry view, read-only handles, only:

- SOFTWARE/Microsoft/Windows Kits/Installed Roots: KitsRoot10.
- SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall: immediate child names,
  each child's DisplayName, and DisplayVersion only for matching SDK names.
- SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall: same fields.

No recursion, other value names, HKCU/remote registry, values containing commands,
installer invocation or filesystem follow-through. KitsRoot10 must equal
C:/Program Files (x86)/Windows Kits/10/ after separator/trailing-slash normalization;
it is metadata, never an authority to read a directory.

Use a new own Node guard plus an isolated standard-library Python collector.
Author-selected fixed interpreter:
C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe, invoked once with
-I -S -B and the reviewed own script. Missing/wrong runtime is terminal; no PATH
search, setup-python, download, alternate interpreter or package installation.
The [frozen image manifest](https://raw.githubusercontent.com/actions/runner-images/win22/20261123.376/images/windows/Windows2022-Readme.md)
lists Python3.12.10; it does not prove a future runner's path or availability.
The collector reports its own runtime version and rejects anything other than
64-bit CPython3.12.10 before registry reads. Tooling verification must validate
the fixed runtime selector without live SDK inventory.

Python ctypes may load only the standard runtime and the system registry API
library at C:/Windows/System32/advapi32.dll for this collection. It must not load
an inspected compiler/SDK DLL or compile an interop shim. Use fixed buffers for
[RegEnumKeyExW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regenumkeyexw)
and [RegQueryValueExW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regqueryvalueexw),
plus read-only open/query-info/close calls. Do not use bulk key enumeration or
an allocating convenience value reader before checking the bound. Registry API
calls are metadata reads, not execution of installed SDK tools.

## Finite allowance proposed

| Unit | Limit and terminal rule |
| --- | --- |
| Dispatch | One manual windows-2022 X64 job; three-minute whole-job bound; one unique serialized first-run history guard. Any dispatch/preflight/infrastructure failure consumes1/1. |
| Collection | One own collector process,60-second deadline; two planned read passes, not failure retries. Minimal child environment, no credentials/user configuration. |
| Entries | At most1,024 immediate child-name reads across both uninstall roots per pass,2,048 total. Check aggregate root counts before each pass; stop before enumeration when over capacity. No probe beyond the approved count. |
| Values | DisplayName at most256 printable ASCII characters; SDK DisplayVersion at most128. Read version only when the name contains Windows Software Development Kit, case-insensitively; at most32 matching records per pass. KitsRoot10 at most256 characters. At most2,114 value reads in total. |
| Buffers | Key-name buffer256 UTF-16 code units including termination; DisplayName/KitsRoot10 buffers514 bytes and DisplayVersion buffer258 bytes, accepting only correctly terminated REG_SZ with no embedded NUL, malformed UTF-16 or environment expansion. Oversize/type/error denies, without resizing/retrying the read. Missing unrelated DisplayName is recorded as absent and skipped. |
| Work | At most10,000 registry API calls,4MiB total raw returned metadata, and8MiB hashed including both controller-source passes (combined source size at most1MiB) and receipt hashes. Count missing/error calls too; debit before each operation. No numerical claim about total runtime/OS allocation. |
| Retention | Export only matching public SDK key/name/version records, counts, bounded statuses, canonical observation digests, source/runtime/image/run identities and counters. Never export unrelated product names, environment dumps, uninstall commands or registry trees. |
| Evidence | One artifact: result.json, sdk-inventory.json, diagnostics.txt; at most256KiB uncompressed combined,14-day retention. Output overflow yields only a bounded failure receipt. |
| Other operations | Zero PE/file-version/hash inventory of installed tools, SDK directory/default-text reads, Cargo metadata/fetch/build, compiler/linker/archiver, sidecar/SQLite/native/model/target execution or dependency/toolchain/SDK downloads. Checkout and evidence upload remain workflow infrastructure. |
| Retry | Zero reruns, second dispatches, alternate roots/images/components, repair, fallback queries or live quota increases. All unused slots close with the sole dispatch. |

For each pass retain root counts/write markers before and after enumeration,
reject duplicate names, missing/disappearing children or changed counts/markers,
and hash a canonical sorted observation including names and bounded read outcomes.
Require equal pass digests and matching SDK product records with exact
DisplayVersion10.1.26100.9457. Different matching SDK versions may be retained as
context but cannot substitute. Missing required metadata, ambiguous conflicting
records, changed pass digests or any exhausted limit makes success false.
These two observations are not an atomic registry snapshot or authenticity proof;
concurrent changes can still limit their interpretation.

Reuse the first-run guard pattern read-only; never alter consumed workflows or
history. Freeze the exact merged reviewed tooling/source SHA on #1176 before
dispatch, require manual attempt1/exact main checkout/sole history, record actual
image labels, and recheck own source hashes. Stage in fresh owned scratch, drain
collector references/close registry handles, retain evidence and remove owned
working scratch. Admission/runner/checkout/storage failure can yield only an
Actions terminal outcome; absent JSON permits no invented inspection counts.

## Deliverables and checks

- [x] Owner accepts this exact new registry-only allowance and tooling work.
- [x] Add a separate workflow/controller and offline fake-registry tests. Reuse
  existing guard tests without changing the consumed batch. Prove wrong-head/
  rerun denial,1024/1025 pre-enumeration boundary, fixed-buffer overflow and
  malformed/type/missing values,32/33 matching records, changed membership/value
  digests, operation/byte/time/output caps and closure/cleanup. Windows tests
  use isolated synthetic data or fakes, never installed SDK/uninstall inventory.
  Review selectors and fixed runtime path before dispatch; normal CI is separate.
- [ ] Freeze merged heads/counters, dispatch once, independently read the terminal
  receipt and retain its hashes/observations in the existing coverage concept
  via PR. No inspected binary or raw registry tree is retained.

Any successful result identifies SDK metadata on that run's image only. It does
not retroactively establish co-availability with run45889209265's compiler,
explain the old compiler-byte change, accept a replacement build pin or establish
full helper closure. A later build still requires an exact input seal and its own
finite allowance; neither this proposal nor a successful job grants one.
#1176 stays open; Gate1 remains Investigate and later runtime/bootstrap/source/
compatibility/recovery/durability/production gates remain unchanged.

## One-time plan review

Disposition: **Ready for scoped allowance review; execution not authorized.**
Scope/simplicity: registry only; no repeat of the thirteen PE reads. Direct-key
selection is unsupported by the retained/pinned public evidence. Explicit fixed
buffers avoid allocating an entire growing key/value collection before denial.
Coverage: each limit and denial has an offline test obligation; both successful
and unavailable outcomes have terminal-record deliverables. Two-pass comparison
is disclosed as observation stability rather than atomic custody or admission.

Dependencies3 (hosted image, fixed runtime and installed registry metadata);
blast radius1 (disposable read-only job); state changes1 (owned scratch/receipt);
reversibility1 (scratch removal, no installation); uncertainty3 (future runtime
availability and SDK record shape unknown). Total9 is a review aid, not approval.
The concrete next decision is acceptance of tooling plus this one job, not a
replacement build or an increase to any earlier spent budget.

## Scoped allowance accepted (2026-11-27)

The owner answered “keep going” to the explicit #2092 tooling-plus-one-job
question. Proposal merge20bac1523d09e4aa89d59a3f703475a36756dccd. This accepts only
the exact registry-only surface, limits, tooling and terminal-result work above.
The original proposal/no-execution statements describe the pre-acceptance stage.
Dispatch0/1; reviewed merged tooling and a frozen source head are still required.
No earlier batch is renewed and no replacement pin, build or runtime profile is
accepted. The collector uses decimal strings for full-width registry write
markers so the JSON/Node boundary cannot round them.

## Tooling review disposition (2026-11-27)

One bounded independent safety/correctness and coverage round reviewed tooling
headfff78059fa22914326ab1696e660a5bf6797058a. The owning agent applied one tested
fix pass: bounded source descriptor reads with pre-hash byte charging; whitelist
projection on unavailable observations; parent-handle-relative child opens;
name-length and value-length/type output charging; failed-close outstanding-handle
accounting; and a sole failure receipt on stream/evidence overflow. These fixes
preserve the accepted surface/limits rather than increase an allowance.

Regression tests observed the missing collector/guard and the reviewed privacy,
source, parent-binding and closure failures before the relevant fixes. A fake
cross-root fixture reaches2,114 value reads across two passes with32 matches;
the33rd match denies before its version is read. Registry timestamps use exact
decimal strings. A Windows orchestration fixture uses mocked public history and
an owned constant-output Python stub, never the real registry collector.

The exact fixed interpreter, isolated flags, native read/overflow/type/missing/
relative-open/failed-close handling and failure orchestration/working-scratch
cleanup passed on windows-2022 in normal CI run45954970061, job140359313136,
headdacfb9d9. The initial Windows orchestration fixture failed at963807d5 because
its Node ESM import used a drive path instead of a file URL; this test-only path
fix then passed. Windows-latest fixtures passed too. All these registry reads
use a disposable HKCU fixture; no installed SDK/uninstall data is queried.
Full current-head CI and hosted review remain separate pre-merge requirements.
Dispatch remains0/1. No second local review round or deferred minor findings.
