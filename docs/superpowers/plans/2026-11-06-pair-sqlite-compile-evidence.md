---
type: Qualification Plan
title: One Windows SQLite compile-only evidence attempt
description: Owner-approved bounded compiler evidence acquisition for the pair scratch analysis; one compile-only attempt and no complete envelope or backend implementation implied.
tags: [orkworks, sqlite, windows, qualification, resources]
status: draft
workflow_status: attempt-complete-envelope-unavailable
---

# One Windows SQLite compile-only evidence attempt

Tracking: [#1479](https://github.com/Rambolarsen/orkworks/issues/1479),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: ac7d7c5655cdb06be8d16f1e86f233e2666b03e2.

## Outcome and authority

Obtain reviewable compiler output for one explicit Windows x64 SQLite candidate
build, replacing guessed C stack frames with emitted-code evidence. The result
is a component input to the [first envelope gate](../specs/2026-11-06-pair-native-enforcement-design.md#compatibility-failure-and-qualification),
not a complete scratch bound, native-growth proof or fit result.

The VFS/router architecture already has owner acceptance under
[ADR 0112](../../adr/0112-pair-native-enforcement.md). Do not request it again.
Its qualification gate explicitly requires separate bounded authorization for
native execution; the [source analysis](../../validation/pair-native-envelope-derivation.md#missing-inputs-and-the-next-read-only-work)
also reserves compile probes. This plan requests one scoped approval covering
qualification tooling and the one attempt below. Merging this document supplies
neither approval nor a renewed earlier budget.

## Exact attempt boundary

| Item | Bound |
| --- | --- |
| Runner | One manually dispatched GitHub Actions job on windows-2022, x64 MSVC environment; no matrix |
| Wall time | 15 minutes for the entire job, including setup/download/compile/artifact collection |
| Compilation | One cl.exe driver invocation for one unchanged sqlite3.c translation unit; compiler child processes are allowed within that invocation/job |
| Target execution | Zero linked executables, SQLite calls, database opens, test workloads, native I/O experiments or model calls |
| Retry | Zero automatic/manual reruns, alternate compiler/runner/configuration, fallback or second dispatch under this approval |
| Source | libsqlite3-sys 0.30.1 crate from static.crates.io/crates/libsqlite3-sys/libsqlite3-sys-0.30.1.crate |
| Archive SHA-256 | 2e99fb7a497b1e3339bc746195567ed8d3e24945ecd636e3619d20b9de9e9149, matching crates/orkworksd/Cargo.lock |
| sqlite3.c SHA-256 | c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d |
| Native budget | The earlier dispatch 1/1 and qualifying-process 1/1 stay consumed; this is a separate proposed compile-only allowance |
| Data | Public pinned source and compiler artifacts only, in a disposable runner scratch directory; no workspace/user databases, session histories, auth settings or Copilot context |

Use the runner's installed x64 MSVC toolchain; record the actual image version,
compiler executable hash/version, Windows SDK version and resulting configuration.
This proposes a candidate compiler build, not an already pinned production build.
If the compiler/source/configuration cannot be obtained or verified, stop before
compilation. Record failure without substituting another toolchain.

Clear CL and _CL_ injection variables before invoking the compiler. Use a literal
argument list, no caller-supplied flags or response file. In the verified source
directory, the command is:

    cl.exe /nologo /c /TC /O2 /MD /W3 /FAsu /Fasqlite3.asm /Fosqlite3.obj [defines below] sqlite3.c

Each name below is one /D argument; bare names mean defined as 1. These are the
bundled crate's base defines from build.rs, not a new production feature profile:

    SQLITE_CORE SQLITE_DEFAULT_FOREIGN_KEYS=1 SQLITE_ENABLE_API_ARMOR
    SQLITE_ENABLE_COLUMN_METADATA SQLITE_ENABLE_DBSTAT_VTAB SQLITE_ENABLE_FTS3
    SQLITE_ENABLE_FTS3_PARENTHESIS SQLITE_ENABLE_FTS5 SQLITE_ENABLE_JSON1
    SQLITE_ENABLE_LOAD_EXTENSION=1 SQLITE_ENABLE_MEMORY_MANAGEMENT
    SQLITE_ENABLE_RTREE SQLITE_ENABLE_STAT4 SQLITE_SOUNDEX SQLITE_THREADSAFE=1
    SQLITE_USE_URI HAVE_USLEEP=1 HAVE_ISNAN _POSIX_THREAD_SAFE_FUNCTIONS

Do not add OMIT_RANDOMNESS, change the default VFS, install an allocator, or
silently disable features to improve the result. Compile-time feature presence
does not authorize runtime use under the accepted closed program.
Microsoft documents [compile without linking](https://learn.microsoft.com/en-us/cpp/build/reference/c-compile-without-linking?view=msvc-170)
and [assembler/source listing output](https://learn.microsoft.com/en-us/cpp/build/reference/fa-fa-listing-file?view=msvc-170).
No linker invocation is part of this attempt.

## Delivery and checks

1. **After scoped approval**, add one manual-only workflow at
   .github/workflows/pair-sqlite-compile-evidence.yml and its narrow PowerShell
   driver at scripts/qualification/pair-sqlite-compile-evidence.ps1. Root agent owns
   both. No product/backend/Cargo configuration changes. Restrict workflow token
   permissions to contents:read; use current repository-pinned checkout/artifact
   actions, no pull_request/push/scheduled trigger or cache.
2. Validate the driver/workflow without compiling SQLite. Check literal source
   identities/arguments, fail-before-compile ordering, one driver invocation,
   timeout, no link/run paths and evidence allowlist. Review the exact tooling diff
   through its normal PR before merge. Ordinary tooling CI does not dispatch it.
   No repeated architecture review or execution-mode choice is needed.
3. Freeze and record the merged workflow head; dispatch once with that exact head.
   Stop if it differs from the reviewed tooling or this plan. Record dispatch ID
   immediately; an infrastructure failure still consumes the one dispatch.
   Cancellation/timeout/partial logs are a terminal result, not retry credit.
4. Retain only source/archive/toolchain identity, literal compiler arguments,
   sanitized compiler diagnostics, assembler listing and object hashes/byte sizes.
   Do not upload the object, full environment, credentials or unrelated runner
   files. Artifact retention is one day; collect the allowlisted evidence within
   it. Scratch files expire with the runner.
5. Read emitted bodies/prologues and call edges for sqlite3_randomness,
   winRandomness, xorMemory and chacha_block. Identify conditional branches,
   inlining, register/spill storage, outgoing-call storage, tail calls and any
   dynamic/undetermined stack adjustment. Missing/optimized-away symbols are
   reported explicitly. Do not sum source local sizes or isolated frames into S.
6. Write one bounded result in the existing envelope-input concept: source/build
   identities, exact emitted facts, unknown terms and disposition. Link the run
   and exact artifacts/hashes. Close this attempt after that result, including
   failed/unavailable outcomes. No automatic next probe or new inventory PR.

## Acceptance and hard endpoint

A useful successful result contains the verified identities/configuration and
readable emitted-code evidence for the four helper paths, or explains precisely
which helper was folded away. Compilation alone is not a passing scratch gate.
An absent listing, warning that alters intended options, mismatched hash,
unexpected extra invocation or incomplete identity record makes this attempt
unavailable; retain the diagnostic and stop.

The callback [input closure](../../validation/pair-native-envelope-input-closure.md)
already records why scalar layouts and the 44-byte seed request are insufficient.
Even a complete helper listing leaves native imported callees, Rust/backend
frames, recursion/whole-program reachability, other SQLite paths, allocator
layout/lifetimes, F/P/R/L and the complete growth envelope unresolved. Inlining
and optimization mean source locals are not actual frame sizes. No observation
of a small workload, reservation default or missing symbol becomes a zero term.

The result must say whether it supplied new usable component evidence. If it
did not, stop this compiler-evidence approach rather than retrying by default.
The overall candidate stays unavailable until every required first-gate term has
a justified bound. Backend implementation planning, VFS/router installation,
profile eligibility, source admission, SQL execution and runtime activation
remain gated. #740 stays closed research; #1176 stays open.

## Investigated risks and plan review

The decisive uncertainty is whether listing output will expose useful helper
frames after optimization. The documented /FAsu output can expose generated code;
it supplies no automatic whole-call-graph or native-library scratch proof.
The explicit candidate configuration prevents mislabeling its output as the
current/future production image. No filesystem-growth experiment is bundled.

| Dimension | Rating / evidence |
| --- | --- |
| Dependencies | 3 — installed runner toolchain and pinned public crate; mismatch stops the attempt |
| Blast radius | 1 — disposable CI compilation, no product/runtime registration |
| State changes | 1 — repository qualification tooling and public build artifacts only |
| Reversibility | 1 — remove tooling/artifacts; no user database changes |
| Uncertainty | 4 — output usefulness for optimized helper frames remains unverified |
| Total | 10; judgment only, never execution authority |

Inline review disposition: **Ready for scoped execution approval**, covering
tooling plus this single compile-only attempt. Scope, exact arguments, source
identity, process/job budget, failure endpoint and remaining gate terms are
explicit. No implementation code is prewritten and no approval of unchanged
architecture is requested. Any material scope/argument/budget change requires
updating this plan and resolving that change before dependent execution.

## Scoped execution approval — 2026-11-07

After reviewed [#1480](https://github.com/Rambolarsen/orkworks/pull/1480), merge
5474a192c13760ee8052bb201c7fc2b1f9aad4fd, the owner answered “keep going” to
the explicit one-job/one-compiler-invocation/15-minute/no-target-execution/no-retry
request. This accepts the tooling and sole compile-only attempt above; no repeat
approval is needed within that scope. [#1485](https://github.com/Rambolarsen/orkworks/issues/1485)
tracks execution and its terminal result. No dispatch has occurred in this record.
All broader architecture, growth/scratch, production and source gates remain.

## Terminal result — 2026-11-07

The approved sole [run42352195449](https://github.com/Rambolarsen/orkworks/actions/runs/42352195449)
completed successfully on merged tooling24fbfc12ffc6571db98aa484b880aac13381ccc3:
one 195-second Windows-2022 job, one compiler driver invocation, exit0 and zero
target executions. The [existing envelope concept](../../validation/pair-native-envelope-input-closure.md#compile-only-component-result--2026-11-07)
records verified artifact identities, all four emitted helper bodies, own RSP
adjustments, outgoing call boundaries and the remaining unknowns. This supplied
useful component evidence and completes the attempt; dispatch1/1 and compiler1/1
are consumed. There is no retry, fallback, new execution budget, complete S/growth
manifest, backend implementation plan or activation implied. Historical approval-
pending/in-progress statements above describe their earlier phases; this dated
result is the current disposition. #1176 stays open; #740 stays closed research.
