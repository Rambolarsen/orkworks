---
type: Evidence Acquisition Proposal
title: One replacement Windows sidecar candidate build
description: Proposed finite candidate using observed compiler bytes, a same-job named SDK record and sealed inputs, with effective macro and linked-artifact evidence only.
tags: [orkworks, hierarchy, windows, sqlite, qualification]
status: draft
workflow_status: accepted-tooling-in-progress
---

# One replacement Windows sidecar candidate build

Outcome: obtain one attributable release-profile orkworksd candidate with the
accepted SQLite macro semantics and object-to-sidecar association, or retain one
terminal unavailable result. Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base dfddedaefba871d69b585e01bd944b32ba3fbd3b. The
[source reconciliation](../../validation/pair-resource-enforcement-coverage.md#toolchain-and-sdk-source-reconciliation-2026-11-27)
establishes why separate metadata/component/SDK observations cannot replace this
same-job build evidence. Every earlier allowance remains spent.

**One combined owner decision is requested:** accept the named input candidate,
new isolated tooling and the exact finite job below. Publication/merge of this
proposal authorizes neither implementation nor dispatch. Reuse the accepted
[ADR0119 recipe](../../adr/0119-pair-build-startup-custody.md); no repeat startup or
resource architecture approval is requested. This batch changes no production
build/profile, release workflow, source authority, bootstrap or runtime route.

## Candidate and fixed inputs

Author recommendation: use the observed newer compiler for one new batch. The
spent compiler pin and its workflow remain unchanged. Keeping that old candidate
unavailable is valid; another standalone component probe would still lack the
actual compile/link witness. A runtime experiment would exceed this question.

| Selection | Exact candidate |
| --- | --- |
| Build | Existing default features; release orkworksd binary; x86_64-pc-windows-msvc; jobs2 |
| Cargo1.100.0 | C:/Users/runneradmin/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe; SHA-256 f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1 |
| rustc1.100.0 | Same toolchain/bin/rustc.exe; SHA-256 d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1 |
| Compiler | C:/Program Files/Microsoft Visual Studio/2022/Enterprise/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/cl.exe; file19.44.35230.0/product14.44.35230.0; SHA-256 e87e78b3ef6580b75aebda843f8a22e874612d9f766344046551399f898eb226 |
| SDK | C:/Program Files (x86)/Windows Kits/10; directory10.0.26100.0; servicing10.1.26100.9457; explicit x64 include/library/tool selectors |
| SQLite | Unchanged3.46.0 amalgamation c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d; rusqlite0.32.1/libsqlite3-sys0.30.1/cc1.2.64 |
| Platform | One windows-2022 X64 hosted job; retain actual ImageOS/ImageVersion, never infer immutable contents from the label |

Require the12 relevant PE byte identities from the
[frozen component record](https://github.com/Rambolarsen/orkworks/blob/f398e8e39f976f02184bbd7fb24624deebf47731/docs/validation/pair-resource-enforcement-coverage.md#current-file-observations):
cargo.exe, rustc.exe, cl.exe, link.exe, lib.exe, dumpbin.exe, c1.dll, c1xx.dll,
c2.dll, mspdbcore.dll, rc.exe and mt.exe at that record's exact selectors.
Devenv.exe is not a selected helper and is not queried. The remaining installed
files under the named roots are first-observed sealed inputs for this batch;
they are not independently authenticated vendor pins. Any required pin/version,
path, source or configuration mismatch is terminal before compilation, without
an alternate directory, installer, download, feature or automatic pin update.

All four [frozen source pins](2026-11-26-pair-fixed-windows-build.md#frozen-source-and-configuration)
were directly rehashed at this inspection and still match: manifest, lockfile,
rust-toolchain.toml and build.rs. Preserve them exactly in the new policy. Freeze
the reviewed merged source/tooling head on #1176 before dispatch; all actual
repository sources must additionally participate in the manifest below. No lock
refresh, dependency change or Cargo metadata invocation is included.

## Same-job SDK observation with an exact selector

The [positive SDK record](../../validation/pair-resource-enforcement-coverage.md#positive-sdk-record-terminal-result-2026-11-27)
now supplies a source-backed immediate key component. Use only local HKLM64:

- SOFTWARE/Microsoft/Windows Kits/Installed Roots: KitsRoot10.
- SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall: open only
  child{45e335f3-c807-43b8-9ed1-43666cfcf939}, relative to that held parent;
  query only DisplayName and DisplayVersion in the held child.

Require exact admitted DisplayName `Windows Software Development Kit - Windows 10.0.26100.9457`
and DisplayVersion `10.1.26100.9457`. KitsRoot10 must normalize only
separators/trailing slash to C:/Program Files (x86)/Windows Kits/10/.
Retain strict printable-ASCII REG_SZ decoding, limits256/128/256 characters,
fixed514/258/514-byte payload buffers plus8 metadata bytes each, missing-value/
ERROR_MORE_DATA/oversize/type/encoding/termination failures terminal. No resize,
repair, expansion, alternate key/view, wildcard or uninstall enumeration. A
missing named record is unavailable, never an SDK-absence claim.

Use two own Python invocations, one pass before compilation and one after build,
oracle and link inspection. Each opens the three named handles, reads before/after
write markers on Installed Roots and the product key, makes exactly three value
queries and closes every handle. Preserve full-width write markers as decimal
strings; compare fields structurally and hash deterministic canonical bytes.
Require stable within-pass markers and equal
cross-pass fields/markers/raw-value commitments. The first complete pass permits
this batch's build stages; final bracket success requires the second pass and
unchanged controlled file inputs. No third pass or retry. This deliberately
brackets the build instead of repeating the earlier immediate two-pass method.

Across both invocations: max32 registry API calls including close reservations,
4 KiB charged metadata, six value queries, zero enum calls. Expected success is
26 calls and2,716 charged bytes: two×(3 opens +4 info +3 query +3 close), and
two×(4×12 +522 +266 +522). Share remaining budgets through the parent; do not
reset the quota per child. Each invocation has a30-second deadline. Private
commitments cover successful raw type/length/bytes and exact selectors. Never
retain rejected raw values or inferred outputs after API errors.

SDK hashing across both passes and receipt is capped64 KiB: raw values at
most2,572 bytes, two framed canonical pass inputs at most8 KiB each, final SDK
receipt at most32 KiB; total at most51,724 bytes. These charges also count toward
the job hash ledger. Use a distinct SDK schema3/evidence_kind sdk-positive-build-bracket;
retain inventory_complete=false throughout. The old inventory/positive-record
success predicates must reject it. No unclassified unrelated names are read.
This record establishes the named product only, not a complete inventory,
authenticity or whole operating-system/loader provenance.

## Closed controlled inputs and child environment

Before selected-tool commands, verify source/configuration and the12 PE pins
with bounded reads. Then allow the sole public lock-verified fetch. Before any
build/helper code runs, verify every acquired crate archive/file checksum and
the reviewed37 build-script plus14 proc-macro entrypoint identities from the
[existing input audit](../../validation/pair-resource-enforcement-coverage.md#fixed-build-input-audit--2026-11-26).
Reuse its unchanged trusted-package source reviews and selected Windows helper
paths; unknown scripts, macros, packages, command formats or non-OS helpers deny.
No pkg-config/vcpkg/Git/CMake/Perl/NASM discovery, submodule repair or unreviewed
procedural-macro helper is permitted. Cargo offline is not a build-code sandbox.

Pre-build manifest roots: the fresh checkout excluding only .git; the fixed Rust
toolchain root; MSVC14.44.35207 root; selected SDK Include/Lib/bin10.0.26100.0
roots; and owned Cargo registry/src. Reject reparse/symlink/non-file inputs,
unknown paths and changed identity/size. Use held-file bounded reads; detect
named-path replacement and before/after membership/content drift. Source and
registry sources are immutable inputs; generated outputs belong only to the
owned target/scratch directory. Do not exclude source files to rescue a cap.

Recompute these same roots after all inspected-tool commands; compare complete
membership, byte identities and per-root digests before final admission. Track
at most200,000 files and50,000 directories per manifest pass, maximum512 MiB per
file,12 GiB total file bytes per pass, and64 MiB for retained inputs.json.
Stream with bounded enumeration/read buffers; precharge work before reading,
hashing or retaining a row. Unknown/growing input, overflow or capacity excess
is terminal. These are author-selected ceilings, not measured capacity or a
promise that every valid installation fits.

A single32 GiB cumulative hash/read-for-hash ledger covers pins, both input
seals, package archive/file verification, own sources, SDK commitments, receipts
and output hashes; charge repeated passes/copies separately. Whole-job time and
artifact limits still apply. Seal all own/imported controller sources twice,
at most1 MiB combined per source pass. Imported guards may be reused read-only;
never invoke a spent live entry point or edit a consumed policy/workflow/history.

The trusted execution base is the hosted Windows OS and explicitly named own
Node/PowerShell/Python controllers; record interpreter paths/versions/hashes and
SystemRoot/image identity. This is not an OS image attestation, dynamic loader
trace or sandbox for malicious build code. Controlled compiler/library/helper
inputs must remain inside the sealed roots; do not turn an unknown application
helper into an excluded OS cost. Later native/profile custody remains gated.

Use the existing reviewed minimal child environment: fixed Rust/MSVC/SDK/System32
PATH, explicit CC/CXX/AR/linker/INCLUDE/LIB and build-owned
LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT, CC_ENABLE_DEBUG_OUTPUT=1,
LIBGIT2_NO_PKG_CONFIG=1, VCPKGRS_DISABLE=1. Preserve THREADSAFE=1/core mutexes and
all accepted allocator/cache/override exclusions. Omit inherited credentials,
proxies, Cargo/Python configuration, CL/_CL_, CFLAGS/CXXFLAGS, RUSTFLAGS/wrappers,
package-manager roots and source overrides. Parent-only Actions history token
never reaches a child. Unknown ancestor/repository/CARGO_HOME configuration
refuses; do not overwrite live configuration. New owned cache/target/scratch,
no warm build cache or global/workstation/release changes.

## Exact requested job and commands

One manual workflow .github/workflows/pair-replacement-windows-build.yml on main,
expected reviewed merged SHA, first attempt only, contents/actions read,
checkout without persisted credentials. Unique concurrency group
pair-replacement-build-allowance-2026-11-27, cancellation disabled, fresh history
must contain only the current dispatch. Any attempted dispatch, infrastructure,
preflight or capacity failure consumes1/1. Zero retries/replacement jobs,
keep-going, alternative targets/toolsets, repaired inputs or quota increases.

| Work | Maximum allowance |
| --- | --- |
| Whole job | 45 minutes, one windows-2022 X64 job, no matrix |
| Own controller | One Node orchestrator; active work deadline40 minutes, at most2 further minutes to terminate/drain owned processes and publish failure; hard job timeout remains binding |
| Own data readers | At most3 fixed-purpose PowerShell invocations: preflight PE data60s, pre-build manifest120s, final manifest120s; C:/Program Files/PowerShell/7/pwsh.exe, -NoProfile -NonInteractive |
| Own SDK readers | Exactly the two planned passes if reached, max30s each; C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe, isolated CPython3.12.10 X64 with -I -S -B; no interpreter search/install |
| Version queries | At most one selected Cargo -Vv and rustc -vV, each30s; PE versions read as data, no compiler/IDE version child |
| Acquisition | One locked x86_64-pc-windows-msvc public crates.io fetch, net.retry=0, timeout4 minutes; no custom registry/Git/toolchain/SDK installation |
| Build | One locked/offline release orkworksd build, jobs2, timeout35 minutes; reviewed dependency compiler/linker/build-script/proc-macro descendants included within the same time bound |
| Macro oracle | At most one additional direct cl.exe preprocessing-only invocation from the captured actual SQLite inputs; timeout2 minutes |
| Post-link information | At most4 lib.exe commands (two LIST, two EXTRACT), one dumpbin.exe DEPENDENTS, each30s; no recompilation or resulting-program execution |
| Streams | 64 MiB aggregate child stdout/stderr across job; oracle output at most32 MiB within that pool; overflow stops and drains, never a truncated success |
| Evidence | One artifact pair-replacement-windows-build, max256 MiB uncompressed,14 days, exact allowlist below |
| Resulting execution | Zero sidecar, SQLite harness, SQL/database, native/model/profile, installer, signing, packaging, release or runtime registration |

The phase timeouts are maxima under one shared deadline, not additive time
credits. Clamp a child to remaining active time; once expired, start no later
stage. Termination may invoke fixed C:/Windows/System32/taskkill.exe once for
the current retained owned child PID with /T /F, max10 seconds inside the shared
cleanup reserve; no foreign target or replacement PID. Await closure before
cleanup. Unresolved termination retains scratch with cleanup unavailable, never
a forced deletion/reset. Clean up only owned scratch. Do not remove scratch while descendants may still reference it.
Pre-scratch/checkout/runner/storage failure may leave only an Actions unavailable
record; do not invent counters or promise an artifact on infrastructure failure.

Use the pinned executables and the same literal commands as the
[earlier candidate request](2026-11-26-pair-fixed-windows-build.md#proposed-finite-allowance-and-exact-commands):

```text
cargo fetch --locked --target x86_64-pc-windows-msvc --manifest-path crates/orkworksd/Cargo.toml --config net.retry=0
cargo build --locked --offline --release --target x86_64-pc-windows-msvc --bin orkworksd --jobs 2 --manifest-path crates/orkworksd/Cargo.toml -vv --message-format=json
```

Order: guard/source/configuration/PE preflight; first named SDK pass; pinned Rust
version queries and sole fetch; package/helper verification and initial manifest;
sole build; derived oracle and post-link information; second SDK pass and final
manifest; bounded publication. Failed stages start no later work except owned
termination/drain/cleanup/failure publication. The fetch never executes fetched
build code; verified package/helper closure must precede the build.
No metadata command, exploratory build, extra preprocessing or output execution.
Jobs2 limits Cargo scheduling; it is not a claim of only two native threads or a
single compiler invocation. Child tools may execute only as reviewed above.

## Oracles, evidence and denial semantics

Capture actual SQLite compiler, argv, working directory and approved effective
environment. Reject unknown/ambiguous command or response-file/forced-include
handling; no guessed parser repair. Derive the sole evidence-only oracle from
those same preprocessor-relevant inputs, replacing only output/codegen options.
Check MEMORY_MANAGEMENT absent, not defined-zero, and THREADSAFE=1 before and
after including the pinned source. Retain source/argv/derivation/exit hashes.
An environment string, compile-option string or earlier compiler oracle is
insufficient. Source and object used in the actual sidecar remain unchanged.

Require the unique Cargo libsqlite3_sys artifact, captured native SQLite object,
native archive and Rust archive member association; extracted bytes from both
archives must equal that object. Require final sidecar map rows naming that
selected Rust archive/member for SQLite symbols and no external SQLite import.
Retain actual final sidecar/object/archive/map/import identities and compare
controlled inputs after the last inspected command. These are candidate custody
observations, not reproducibility, startup, dynamic loader or runtime proof.

Allow only result.json, inputs.json, cargo.jsonl, diagnostics.txt, compiler.json,
oracle.c, oracle.i, sqlite3.lib, libsqlite3_sys.rlib, orkworksd.exe, candidate.map,
dependencies.txt, archive-members.txt, link-custody.json and sdk-record.json.
Include exact source/selection/counter/digest/cleanup records in those files.
No raw SDK rejection values, unrelated registry names, PE input uploads, caches,
user databases, ambient environment dumps or arbitrary extra files. Bounded
redacted diagnostics must preserve command/source associations via root labels.

New build schema2/evidence_kind replacement-sidecar-candidate distinguishes this
batch from earlier receipts. Final success/candidate_evidence_complete become
true only after macro/linkage/SDK bracket/input rechecks and bounded publication
all pass. On any later failure, clear final admission/success flags; retain
explicitly partial stage observations without promoting them to a complete
candidate. Always runtime_qualified=false and production_pin_accepted=false.
Approval accepts these compiler bytes only as input for this finite batch; a
qualified production compiler/profile pin is still a separate decision.

## Delivery and review after owner approval

One owner adds new scripts/qualification/pair-replacement-windows-build.mjs,
.ps1, .inputs.json and -sdk.py, own synthetic tests and the new manual workflow.
Add LF attributes only for the new own files, preserving all existing rules.
No spent implementation is patched or rearmed. Reuse accepted source audits
where inputs are unchanged; review changed candidate/SDK/limit/orchestration
boundaries. Normal CI may use synthetic files, fakes and isolated HKCU fixtures
only, never installed SDK discovery, package acquisition or the actual build.

Required TDD/fixture coverage: every selected pin/version and unknown config
refusal before build; exact-key-only SDK namespace, required ASCII fields,
missing/error/oversize/stamp/value drift, API/read/hash shared budgets and failed
close retention; first/second-pass failure and no third call; precharged file/
manifest/aggregate hash caps, reparse/replacement/growth, source and input drift;
unknown package/build-script/proc-macro/helper; exactly one fetch/build/oracle,
process deadline/overflow drain-before-cleanup; actual-argv oracle denials and
both archive-member/map/import associations; schema separation, false final
flags and receipt-only overflow failure. Test fixed runtime/Win32 ABI and owned
success/failure orchestration on windows-2022 without live SDK reads.

Before tooling merge: required repository verification, current-diff review and
all required CI. Then freeze reviewed sources/policy/commands on #1176, verify
new history0/1, dispatch once, independently read back bounded evidence and
record success or unavailable in canonical coverage. Close all unused slots,
remove only owned working copies and clean the merged worktree. No follow-on
job or bootstrap code is implied by a useful or unsuccessful result.

## Plan review and authorization

Disposition: **Ready for one combined owner decision; implementation/dispatch
remain blocked until that decision.** Root reviewed scope, coverage and resource
arithmetic against the accepted contracts. Two read-only Luna reviews found no
important builder decisions missing and no factual/authority/resource conflicts.
Root clarified the existing owned-termination fallback with one10-second taskkill
limit; it adds no successful-path command or retry. No deferred findings or
unresolved material choices are delegated to the runner. Proposed risk: dependencies3 (known build helpers and hosted
runtime), blast radius1 (isolated unsigned candidate), state changes2 (owned
acquisition/build artifacts), reversibility2 (scratch removable, dispatch remains
spent), uncertainty4 (unmeasured full inputs and actual macro/link output). Total
12/25. The finite attempt may end unavailable under any named cap.

Review topology: one bounded round, maximum two read-only Luna reviewers, one
checking builder completeness and one checking factual/authority/resource
consistency. Root is the sole writer and applies findings before handoff. This
proposal does not execute those future delivery steps. One owner decision must
cover input candidate, exact-key SDK method, tooling and this exact job together;
material changes after approval require a new decision before dependent work.
Gate1 remains Investigate, Gate2/bootstrap implementation and Gates3–4 remain
gated. All source/compatibility/recovery/durability/activation obligations persist.

## Accepted execution ledger (2026-11-27)

Owner replied “keep going” to the explicit combined request after #2159, accepting
this named candidate/SDK method/tooling and one exact finite job. Original
proposal language above is preserved as history. The new allowance is unused;
all earlier allowances remain spent. No repeat policy approval is needed within
this scope. Verification, current-diff review, required CI, tooling merge and
source freeze precede the sole dispatch.

Root owns one writer worktree replacement-windows-build at9d0d869e. Execute SDK
reader/shared-ledger tests, bounded reader/manifest and controller tests, workflow
fixtures, full verification, then one bounded fresh-context review round with
maximum two read-only Luna reviewers (correctness/authority and coverage/bounds).
Root alone applies findings. No implementer delegation or second coding harness.
Current status: implementation and fixture verification in progress; dispatch0/1.

SDK reader: six Windows fixtures passed. Policy primitives: seven local Node
tests passed after their initial missing-controller failures. Windows bounded
reader fixture failed on the deliberately missing implementation, then passed
after implementation in draft CI. Locked archive verification likewise reached
a missing-function failure before passing the Windows fixture. Its initial fixture
stream-name collision was corrected before observing that meaningful failure.
The controller entry point and manual workflow are now wired; ten local Node
checks pass, including full twelve-PE admission and manifest drift refusal.
Final native CI, full verification and fresh review remain pending.

Ruling: do not reuse the old helper's .cargo-checksum.json assumption; the
inspected local Cargo registry cache lacks that vendor-side file. Derive member
checksums from the actual lock-hashed crate archive with the own standard-runtime
reader, comparing every extracted file and reviewed entrypoint. This preserves
the accepted archive/file verification, roots, subprocess counts and hash/time
bounds. If the archive format/runtime is unsupported, the attempt is unavailable
rather than trusting unchecked sources or acquiring another tool.
