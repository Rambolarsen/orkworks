---
type: Evidence Acquisition Proposal
title: One fixed Windows sidecar candidate build
description: Proposed finite locked sidecar build with named Rust, MSVC and SDK inputs, macro evidence and linked-artifact custody; no resulting binary execution or runtime qualification.
tags: [orkworks, hierarchy, sqlite, windows, build, qualification]
status: draft
workflow_status: completed-unavailable
---

# One fixed Windows sidecar candidate build

Outcome: obtain one attributable linked sidecar candidate using the accepted
SQLite macro-absence recipe, or retain one terminal unavailable result. This
builds the actual crate/dependency chain rather than another standalone SQLite
object. Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: `d201afa0afec56a6f8c13c856fd19223f3710c6f`.

**New allowance proposed; no build is authorized by this document or its merge.**
[ADR 0119](../../adr/0119-pair-build-startup-custody.md) already accepts the shared
macro/startup recipe for further qualification. No repeat architecture decision
is requested. The [successful metadata observation](../../validation/pair-resource-enforcement-coverage.md#one-windows-metadata-attempt-completed--2026-11-26)
closes only its frozen graph request. Metadata, earlier compiler and native batches
remain spent. This proposal grants no bootstrap implementation or production use.

## Candidate selection and availability

Author recommendation: one release-profile orkworksd candidate with the existing
default feature request, x86_64-pc-windows-msvc, unchanged pinned crate sources and
build-owned `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`. Do not enable
vendored-git or another feature to rescue a failed build. A component-only repeat
cannot establish which SQLite archive was linked into the sidecar; a full native
runtime test would exceed the remaining evidence question and is not selected.

| Input | Fixed candidate selection and evidence |
| --- | --- |
| Cargo | 1.100.0, binary SHA-256 `f381bae27c59588eacf69208e115b2c001a2998afed83a2e9084ea93620737b1` |
| rustc | 1.100.0, x86_64-pc-windows-msvc, binary SHA-256 `d4f8a0283de2dba0dc458cd13c333337d2b76c3649eb39e91c4acce618e661a1` |
| C compiler | x64-host/x64-target cl.exe file version 19.44.35229.0, SHA-256 `fe251ef50a1545b1b0835ee17b1e785459712b38d79e45b5c1d3d28970a36619` from the retained earlier compiler component |
| MSVC directory | Select only VC/Tools/MSVC/14.44.35207; no latest/default toolset discovery as substitution |
| Linker/archiver/helpers | Explicit x64 link.exe/lib.exe and compiler helper files from that same selected toolset; record versions, hashes and complete used-input manifest before code generation |
| Windows SDK | Kit directory 10.0.26100.0, servicing package 10.1.26100.9457; explicit x64 include/library/tool paths, with an input manifest before compilation |
| Platform | One windows-2022 x64 hosted job; record actual ImageOS/ImageVersion. The runner label is not an immutable toolchain pin |
| Source | Frozen reviewed source/tooling SHA plus unchanged four input hashes below; SQLite3.46.0, rusqlite0.32.1, libsqlite3-sys0.30.1 and cc1.2.64 |

Cargo/rustc identities were observed in run45763908423. The C compiler identity
comes from the [earlier component result](../../validation/pair-native-envelope-input-closure.md#build-and-evidence-identity),
not an MSVC inspection in the metadata run. The official
[20261116.370 image manifest](https://raw.githubusercontent.com/actions/runner-images/win22/20261116.370/images/windows/Windows2022-Readme.md)
corroborates Rust/Cargo1.100.0, the SDK directory and servicing package. It does
not publish the individual linker/header/library hashes or prove a later runner
contains them. Those byte identities remain unmeasured. This is a named initial
candidate whose complete input identity must be sealed before compilation; it is
not an independently reproduced build with every toolchain byte previously pinned.

Availability of all selected components together is unverified. Missing compiler,
SDK servicing identity or helper provenance stops before building, consumes the
one dispatch, and supplies no installer/download/alternate-toolset permission.
Do not change the published release workflow, its floating stable/windows-latest
selection, dependency source or global workstation configuration for this attempt.

## Frozen source and configuration

Freeze the merged reviewed tooling SHA and exact source SHA on #1176 before the
single dispatch. The following source input hashes retain the metadata baseline;
any drift requires review before dispatch, never an automatic lock refresh:

| Input | SHA-256 |
| --- | --- |
| crates/orkworksd/Cargo.toml | `0134c524d6e92d0f01f3cf3953f687c1ee0b570e54da879659061238aa615cc8` |
| crates/orkworksd/Cargo.lock | `40845c5bdf0a4ed869e93af1de572220711f0d2fabf31f546488b1cdbab28ded` |
| rust-toolchain.toml | `d4d89c66d4e571b2f86ef1c5484191acca0465a101b6c46083eea9537e69fc25` |
| crates/orkworksd/build.rs | `1111ea2be9dfa3ddeb9402608723b50b01bd304a81d3de63d7d1427fe7915edf` |

Keep the existing LF byte-preservation attributes and check input hashes before
and after the attempt. Use owned empty Cargo/target/scratch directories, no warm
build cache, explicit binary paths and a minimal non-secret child environment.
Reject unknown ancestor/repository/CARGO_HOME configuration, wrappers, forced
includes, CL/_CL_, CFLAGS/CXXFLAGS variants, source replacements and extra Rust/
SQLite flags. Do not overwrite live unpaired configuration to force availability.

Only the reviewed candidate child receives LIBSQLITE3_FLAGS above. Preserve
THREADSAFE=1 and the accepted default allocator/cache recipe; no external pool,
custom PCACHE2, shared-cache adoption, new lookaside counts, alloca or other
unreviewed build override. CC_ENABLE_DEBUG_OUTPUT=1 records pinned cc command
construction. An explicit target linker and reviewed map/library diagnostics may
be supplied solely for custody evidence; their exact Rust/link argv must be
frozen in the tooling PR. No arbitrary caller-provided flags or commands.

The cached cc1.2.64 source,2045–2068, appends builder flags in insertion order,
then definitions and environment flags. Pinned libsqlite3-sys build.rs,122–140
and281–291, supplies both its default define and later undefinition through
builder flags. That supports the intended order only under the closed inputs;
actual compiler argv and preprocessing must still confirm it. An environment
string or a compile-option string alone is not effective-macro proof.

## Proposed finite allowance and exact commands

| Unit | Proposed limit and stop condition |
| --- | --- |
| Dispatch | One manual windows-2022 job, no matrix; serialized, sole-first-run history guard; any dispatch/infrastructure/preflight failure consumes1/1 |
| Time | 45-minute whole-job timeout; at most35minutes for the sole Cargo build, with remaining time for acquisition, preprocessing and collection |
| Acquisition | One locked target fetch, public lock-verified crates.io sources only, net.retry=0; no toolchain/SDK installation or Git/custom registry source |
| Build | One Cargo build command, jobs2; its dependency compiler/linker/build-script/proc-macro/helper descendants are explicitly included within this job allowance, not a one-cl-invocation claim |
| Macro oracle | At most one additional direct cl.exe preprocessing-only invocation, derived from the captured SQLite compile inputs; no object/link/run step in that oracle |
| Informational work | Read-only file/version/hash/SDK inventory and installed-tool version/target queries, under the same time bound; no exploratory build or alternate configuration |
| Resulting program execution | Zero sidecar, SQLite harness, SQL/database, profile, model, installer or native qualification executions; build helpers above do not qualify runtime behavior |
| Retry | Zero Cargo/dispatch reruns, keep-going, fallback feature/compiler/SDK, online build fallback or extra preprocessing attempt |
| Evidence | One allowlisted artifact, at most256MiB uncompressed, 14-day retention; overflow is failed evidence, never a silently truncated successful manifest |

```text
cargo fetch --locked --target x86_64-pc-windows-msvc --manifest-path crates/orkworksd/Cargo.toml --config net.retry=0
cargo build --locked --offline --release --target x86_64-pc-windows-msvc --bin orkworksd --jobs 2 --manifest-path crates/orkworksd/Cargo.toml -vv --message-format=json
```

Cargo means the pinned installed executable. The official
[build command contract](https://doc.rust-lang.org/cargo/commands/cargo-build.html)
supports explicit binary/profile/target selection. This authorizes compilation
and dependency build helpers only if accepted; it does not execute the resulting
sidecar. Cargo --offline controls Cargo acquisition, not arbitrary network use
inside build scripts. The reviewed tooling must close the locked build-script/
helper input and source paths before dispatch; extra downloads, external package
manager probes or unreviewed dynamic library selection are not allowed evidence.
Unknown provenance stops or leaves the candidate unavailable, without a retry.

## Effective macro and linked-artifact oracles

Capture the actual SQLite compiler executable, complete argv/response-file inputs,
working directory and effective non-secret environment. Verify acquired source
hashes against the existing pins. The sole preprocessing oracle uses those same
preprocessor-relevant inputs and compiler, replacing only code-generation/output
options. Its evidence-only translation unit checks MEMORY_MANAGEMENT is undefined
(not zero) and THREADSAFE equals1 before and after including the unchanged pinned
amalgamation. It must neither patch nor replace the SQLite source used in the
actual sidecar build. Retain the oracle bytes/hash, exact derivation and exit.
Unknown token/response-file handling or any macro failure is unqualified; do not
repair and invoke the oracle again within this batch.

Retain the actual SQLite object/archive identity and source association, Cargo
build-script outputs and final sidecar identity, plus explicit linker/map/library
selection evidence showing which SQLite archive fed that artifact. Establish
single-engine custody and review excluded linkage/feature paths; a successful
link or substring in an executable is not that proof. Record selected SDK/tool
inputs before building and recheck after collection. Do not claim byte-for-byte
reproducibility merely because the binary was hashed once.

Allowlist only result/input manifests, sanitized command/build diagnostics,
macro-oracle source/preprocessed evidence, SQLite archive, final unsigned sidecar
and link/map records. Exclude credentials, ambient environment dumps, Cargo cache,
user databases, unrelated objects, whole SDK trees and giant assembly listings.
No signing, packaging, release upload/publishing, install or runtime registration.
Retain bounded failure diagnostics and observed identities on drift. Remove owned
scratch after references/processes drain; failure/timeout gives no retry credit.

## Tooling, review and handoff

- [x] Obtain this exact new scoped allowance; old batches remain spent.
- [x] Prepare a separate manual qualification workflow and owned script with
  offline behavioral tests. Freeze effective argv/manifest/oracle derivation and
  all invoked helper selections in its PR. Reuse the proven first-run, input-pin,
  minimal-environment, bounded-evidence and failure-retention patterns; do not
  reset or mutate the consumed metadata workflow's history.
- [x] Before dispatch, close source/helper/configuration inputs through the
  reviewed tooling, verify the selected component selectors and record counters
  and frozen heads on #1176. Material recipe changes return to the owner; absent
  evidence is not permission to choose a different build. Tooling tests and normal
  repository CI are separate from dispatch; they must not launch this experiment.
- [x] Dispatch once, retain the terminal outcome and identities in the existing
  coverage concept, and close the batch whether successful, unsupported or timed
  out. Independently read back artifact hashes/inputs and review actual linkage.

Success supplies an attributable candidate artifact and macro/linkage evidence,
not a qualified native profile. Startup/configuration custody, arena/VFS/worker
implementation and context ownership, ordinary compatibility, source/cutoff/
newness/catalog/initializer/recovery/durability/activation remain independent.
The [parent gates](2026-11-24-pair-resource-assurance-qualification.md) and
[bootstrap-state implementation plan](2026-11-24-pair-bootstrap-state-tests.md)
remain gated; no test constructor or qualification witness is minted here.

## One-time plan review and decision boundary

Disposition: **Ready for scoped candidate/allowance review; Investigate before
execution until the reviewed tooling closes the stated input prerequisites.**
The plan distinguishes old observed hashes, named unmeasured SDK/linker inputs,
build-helper execution and prohibited resulting-program execution. It makes the
larger Cargo build allowance explicit rather than borrowing one old compiler
invocation. There is no promised fit/success, alternate installation or new
resource/architecture policy. Missing component availability is a visible risk.

Dependencies4 (fixed tools/SDK, registry and source/build-helper closure); blast
radius2 (disposable build job, no runtime data); state changes2 (owned build/evidence
files); reversibility1 (owned scratch disposal, no installation); uncertainty4
(unmeasured helper/SDK identities and actual build/linkage). Total13 is a judgment
aid, not approval or qualification. This initial candidate is not a fully prior-
byte-pinned toolchain or independent reproducibility result.

Approve or reject the reviewed tooling plus **one45-minute job containing one
locked fetch, one offline Cargo build at jobs2 and one preprocessing oracle**, with
zero resulting-sidecar execution and zero retries. No compiler, metadata retry,
source acquisition, tool query, flag/code change or experiment ran to write this
proposal; only repository/cached source and official documentation were read.

## Scoped allowance accepted — 2026-11-26

The owner answered “keep going” to the explicit #2066 question approving the
reviewed tooling and this one45-minute Windows build job. Acceptance covers only
the recipe/limits above, including build-helper descendants and the one macro
oracle. [#2066](https://github.com/Rambolarsen/orkworks/pull/2066) merged as
`cd12001b37a661232195f68def753b51a0d21c61`. Prior batches remain spent, and no
resulting-sidecar execution or runtime/bootstrap/source authority is granted.
Tooling must still close and review the stated inputs before dispatch; this
acceptance does not turn unknown provenance into evidence. At acceptance, dispatch stood0/1.

## Terminal execution record — 2026-11-26

Tooling [#2069](https://github.com/Rambolarsen/orkworks/pull/2069) merged as
`b54c55ecf0b38681eabf5f16ecadf3d366ea796c` after current-head review and all
applicable CI. The exact head and four unchanged source hashes were frozen on
#1176 before [the sole run45876634487](https://github.com/Rambolarsen/orkworks/actions/runs/45876634487).
It closed unavailable in toolchain preflight: the selected cl.exe existed but
its bytes did not match the approved pin. Cargo/rustc and source pins matched.
No fetch, Cargo build, preprocessing oracle or resulting-program execution ran.
SDK inventory, compiler-version query and effective macro/linkage evidence were
not reached. The dispatch consumes1/1, including all unexercised child slots;
no retry, replacement pin or new runner query is authorized.

The [canonical terminal record](../../validation/pair-resource-enforcement-coverage.md#fixed-windows-build-batch-closed-unavailable-2026-11-26)
retains observed hashes, the bounded artifact identity/expiry, independent
readback and cleanup. The finite task is complete with an unavailable outcome;
Gate1 remains Investigate, #1176 stays open, and runtime/bootstrap/source/
compatibility gates remain closed. Any follow-up evidence acquisition needs its own concrete
selection and finite allowance.
