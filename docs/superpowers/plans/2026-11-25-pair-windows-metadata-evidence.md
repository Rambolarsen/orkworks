---
type: Evidence Acquisition Proposal
title: One Windows pair dependency metadata attempt
description: Proposed finite dependency acquisition and offline Windows metadata run before selecting an exact compiler build; no compilation or pair qualification.
tags: [orkworks, sqlite, hierarchy, windows, qualification, dependencies]
status: draft
workflow_status: proposed-owner-allowance-required
---

# One Windows pair dependency metadata attempt

Outcome: replace the recorded uncached-target metadata failure with either one
complete, attributable Windows dependency graph or a retained terminal failure.
This is the first stage of build-evidence preparation, **not a candidate build**.
Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: `b9193b71cae6eee5490b8b960f314f260d5d1ddd`.

The [configuration-writer audit](../../validation/pair-resource-enforcement-coverage.md#configuration-writer-source-audit--2026-11-24)
cannot establish actual target features, installed toolchain or linked artifact.
[ADR 0119](../../adr/0119-pair-build-startup-custody.md) already accepts the shared
candidate flag/startup recipe for further qualification; do not request that
choice again. This proposal requests a **new, separate metadata-only allowance**.
Prior native/compiler batches remain spent. No allowance is granted by writing,
reviewing or merging this document.

## Selected scope and alternatives

Author recommendation: acquire dependencies and metadata first, then use the
observed identities to propose a fixed compiler/linker/SDK build separately.
Combining discovery and compilation now would leave the actual toolchain unknown
when requesting the build allowance. Repeating source searches cannot produce
the missing resolved graph. Neither alternative is selected here.

The runner image label below selects an inventory environment, not a reproducible
production toolchain. No SDK/compiler installation, cl/link invocation, object,
archive or sidecar build is proposed. File/version inventory of installed MSVC
and SDK components may inform the later build proposal but cannot qualify them.

## Proposed finite allowance

| Item | Limit and terminal rule |
| --- | --- |
| Dispatch | One manually dispatched workflow run, one windows-2022 x64 job, no matrix; dispatch is consumed even on infrastructure/preflight failure |
| Time | One 15-minute job timeout including checkout, acquisition, metadata and artifact upload; stop on expiry |
| Cargo operations | At most one locked target fetch, then at most one locked/offline metadata command; stop after either failure, no fallback command |
| Informational tool calls | At most one explicit cargo -Vv and one rustc -vV; Cargo's necessary version/target-configuration queries are included within the two bounded operations, not code-generation authority |
| Network | Public locked crates.io registry dependencies/index only, Cargo net.retry=0; no Git dependencies, custom registries, credential provider, new sources or toolchain downloads |
| Retry | Zero reruns, alternate runners/targets/toolchains, lock updates, online metadata fallback or second dispatch |
| Evidence | One allowlisted artifact, at most 32 MiB uncompressed, 14-day Actions retention; oversize means failure, never silently truncate a successful graph |
| Data | Disposable runner checkout/cache/scratch only; no user/workspace databases, native session histories, production credentials or recommendations |
| Execution excluded | No build/check/test/run/bench/doc, build-script or proc-macro execution, Rust/C code generation, target binary, SQL/SQLite API, model or native qualification probe |

These are proposed acquisition limits, not changes to product resource budgets.
No total network-byte/native-host-memory ceiling is claimed. The job timeout
bounds this attempt; partial download or upload failure supplies no retry credit.

## Frozen inputs and preflight

After scoped approval, implement only the evidence tooling below through its own
reviewed PR. Before dispatch, record the merged tooling SHA and exact source SHA
on #1176. Freeze Cargo.toml, Cargo.lock, rust-toolchain.toml and relevant build
entrypoint hashes there; changed inputs require a new disposition before dispatch.
No floating source ref or user-provided arbitrary command is a workflow input.

Use read-only repository permissions and checkout without persisted credentials.
Use only an already installed matching Cargo/rustc toolchain, addressed by its
actual binary paths rather than a rustup proxy that could download stable. Record
binary hashes, version outputs and runner ImageOS/ImageVersion first. If tools
are missing, inconsistent or require acquisition, stop without replacement.

Run from the frozen checkout with an owned empty child-process CARGO_HOME and
owned target/scratch directories. Account for all ancestor/repository Cargo
configuration and allow only the reviewed crates.io configuration; unknown
configuration denies. Explicitly select the recorded rustc and remove inherited
Rust/compiler wrappers, flags, registry/source replacements and credential-provider
inputs from the Cargo child environment. Retain an allowlisted non-secret input
manifest, never the ambient environment or credentials. Do not repurpose HOME.

Preflight rejects Git/custom/path dependencies outside the frozen checkout,
non-public source URLs or any lock/source identity mismatch. Validate the literal
command allowlist and timeouts before acquisition. Keep source and lock bytes
unchanged and compare their hashes afterward, including failure paths.

## Exact acquisition and interpretation

From the frozen repository root, the only dependency operations are:

```text
cargo fetch --locked --target x86_64-pc-windows-msvc --manifest-path crates/orkworksd/Cargo.toml --config net.retry=0
cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc --manifest-path crates/orkworksd/Cargo.toml
```

Here cargo means the recorded installed binary, not an executable selected from
an unexamined PATH. Preserve the existing default feature request; do not use
--all-features, --no-default-features, --no-deps or change dependency versions.
No LIBSQLITE3_FLAGS change is needed for metadata, and no effective macro state
is inferred from the output.

[Cargo fetch](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html) acquires
locked dependencies for the target; the subsequent operation is offline.
[Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)
provides a resolved graph. Its filter applies to resolve, while the packages
array can still contain other platforms. Analyze resolve nodes, feature lists,
dependency kinds and target predicates; do not equate every packages entry with
a Windows linked consumer. Preserve host/build/dev distinctions and treat this
as evidence for this exact metadata request, not a release link/unit graph.

Retain complete metadata JSON plus SHA-256, exact commands, exits/times, sanitized
failure diagnostics, frozen input identities, tool/image identities and a derived
SQLite-family edge/feature summary. Check the pinned rusqlite/libsqlite3-sys
versions and acquired SQLite/build.rs hashes against the existing source record;
unexpected features or drift remain Investigate, not automatically approved.
No-match or a null/missing resolve is not success. Partial/oversize output is a
failure receipt, not a complete graph. Retain no crate cache or credential files.

## Delivery, verification and handoff gates

- [ ] Obtain the scoped allowance for the limits above. This does not accept a
  compiler build, implement the bootstrap or unlock the parent execution gates.
- [ ] Prepare `.github/workflows/pair-windows-metadata-evidence.yml` and
  `scripts/qualification/pair-windows-metadata-evidence.ps1`, with offline Node
  contract tests beside the script. Review the exact tooling diff before merge.
  Tests must cover literal commands, no rerun/build/online fallback, identity/
  configuration denial, failure receipts, output cap and evidence allowlist.
  Tooling CI runs those offline tests only; it never dispatches the experiment.
- [ ] Freeze the reviewed merged SHA, record allowance and unconsumed counters,
  then dispatch exactly once. Record run ID immediately. Review sanitized
  complete evidence, or retain terminal unavailable status and consumed counters.
  Remove only owned scratch; runner disposal does not claim source reclamation.
- [ ] Append outcome and identities to the existing coverage concept. Only then
  propose an exact separately authorized compiler-build batch using the observed
  inputs, fixed MSVC/SDK identities and accepted macro-absence recipe. Do not
  silently install a different version because the observed one is unsuitable.

A complete graph closes only the missing metadata observation for that request.
Actual effective flags, linked artifact, startup/router/VFS/context custody,
ordinary compatibility, source authority, recovery/durability and production
activation remain separate. The [parent qualification plan](2026-11-24-pair-resource-assurance-qualification.md)
retains Gate 1 Investigate and the
[bootstrap-state plan](2026-11-24-pair-bootstrap-state-tests.md) remains unexecuted.

## One-time plan review and owner decision

Disposition: **Ready for scoped allowance review; not executable yet.** The
proposal separates installed-tool discovery and metadata from a later fixed build,
keeps public dependency acquisition explicit, and defines terminal failures and
retained evidence. No new architecture or resource-policy choice is requested.

| Risk dimension | Rating and evidence |
| --- | --- |
| Dependencies | 3: hosted runner, installed tools and public registry; missing inputs stop |
| Blast radius | 1: isolated evidence job, no runtime or production data |
| State changes | 2: disposable dependency cache and bounded retained evidence |
| Reversibility | 1: owned scratch disposal; no database or installed-tool change |
| Uncertainty | 3: actual installed tools and complete target resolution are not yet observed |

Total 10 is a judgment aid, not authority. Approving this exact proposal would
authorize the reviewed tooling and one bounded metadata attempt after its PR;
it would not authorize compilation, target execution, retries or implementation.
No acquisition, metadata retry, tool query or experiment ran to write this plan.
