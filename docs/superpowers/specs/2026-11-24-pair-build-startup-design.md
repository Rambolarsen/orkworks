---
type: Design Proposal
title: Pair build provenance and process startup allocator custody
description: Owner-accepted shared candidate build-flag and early bootstrap recipe for further qualification; actual artifact, startup behavior and execution remain unqualified.
tags: [orkworks, hierarchy, sqlite, windows, startup, qualification]
status: draft
workflow_status: accepted-qualification-pending
---

# Pair build provenance and process startup allocator custody

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Source base:
`5df223cbcbb23d58b4e4035021e750fe37a19ea0`. This writes a concrete recipe, not an
instantiated reproducible artifact or an already installed bootstrap. Merge
publishes the proposal; it does not accept the new global build choice.

Current decision: [scoped acceptance](#scoped-build-and-startup-acceptance--2026-11-24)
accepts the exact build/startup choices below for further qualification. Earlier
proposed/pending language records the reviewed design, not a remaining approval
requirement. No compiler flag or bootstrap is enabled by this record.

## Proposed decision and scope

**Author recommendation:** retain one bundled SQLite 3.46.0 instance, the accepted
public pair VFS/early allocator router, original worker/PairCore and
[ADR 0118 resource meanings](../../adr/0118-pair-resource-assurance.md). For a future
qualified Windows x64 MSVC candidate, **undefine SQLITE_ENABLE_MEMORY_MANAGEMENT**
in that shared SQLite build so its default page caches cannot recycle pages
between pair and unpaired connections through a shared PGroup. Install the
already accepted allocator router through one private single-threaded bootstrap,
before constructing Tokio or starting any SQLite consumer.

The flag is a new owner-pending amendment, not permission inferred from source
inspection. It changes all SQLite consumers in the affected binary. The early
router architecture is already accepted and is not reopened. No private engine
copy, custom page cache, source patch, dependency bump, new numeric limit, global
heap cap or extra restrictive counts from #1470 are selected.

The accepted limits remain hierarchy 1 GiB and shared auxiliary 2 GiB logical
file-length high-water and a fixed 8 MiB arena including bookkeeping/copies.
Native host costs remain outside those numeric caps. Lower budgets, source/
cutoff/newness/initializer/catalog/recovery/durability/activation gates remain.

## Source result that requires the amendment

The cached amalgamation SHA-256 was rechecked as
`c01235302fe80da901fb70c7622c39147e29d9f29b7f6eb746b23517f320c90d`.
References below are to those SQLite 3.46.0 bytes in the
[pinned libsqlite3-sys 0.30.1 source](https://docs.rs/crate/libsqlite3-sys/0.30.1/source/sqlite3/sqlite3.c).
The existing [controlled-route map](../../validation/pair-resource-enforcement-coverage.md)
remains valid; this investigation follows its unresolved page-cache/startup edge.

- Cached build.rs, lines122–140, defines SQLITE_ENABLE_MEMORY_MANAGEMENT for the
  bundled build. The [root declaration](../../../crates/orkworksd/Cargo.toml)
  selects rusqlite bundled/hooks; the lock pins rusqlite 0.32.1 and libsqlite3-sys
  0.30.1. This is not a completed Windows target feature graph.
- pcache1Init,55853–55899, forces `separateCache = 0` when that macro is defined.
  pcache1Create,55918–55943, then associates caches with the same global group.
- pcache1FetchStage2,56030–56102, can take an unpinned LRU page from another cache,
  keep it if allocation sizes match, and reassign its pCache. A new page allocation
  is attempted only if no reusable page remains. An existing adequately sized
  hash table permits this page transfer without a new allocation callback.
- Thus the allocator can correctly attribute every allocation event yet a pair
  still receive an unpaired-origin page outside its arena. The reverse transfer
  can retain pair-origin storage in an unpaired cache. Pointer classification
  during free/realloc and an early router alone do not observe this reuse step.

This is a reachable conditional source path, **not an observed capacity overrun**
or proof every workload takes it. Its conditions include a shared group, a
purgeable requesting cache, the recycling condition, an available unpinned page
and matching allocation size. A small successful workload cannot remove it.

| Alternative | Assessment |
| --- | --- |
| Keep the existing flags and candidate unavailable | Valid default; no new execution or silent weakening of the arena requirement |
| Undefine the memory-management feature for the shared candidate | Recommended smallest change: source selects per-cache groups when core mutexes remain enabled, with no external pool/custom cache. Global memory-reclamation behavior changes and must be reviewed/qualified |
| Add a public PCACHE2 wrapper/custom cache | Could enforce ownership at fetch/recycle boundaries but adds a new process-global integration and lifetime/ABI design; not selected here |
| Serialize all unpaired SQLite work, flush global caches, copy/relabel a reused page or use a private engine copy | Not selected: changes availability/ownership or repeats an unaccepted engine direction. A point-in-time flush cannot prevent subsequent recycling |

## Exact proposed build rule and provenance

The required semantic result is **macro absent**, not defined with value zero:
the pinned branch uses `#if defined(SQLITE_ENABLE_MEMORY_MANAGEMENT)`. The crate's
build.rs,281–291, accepts explicit -D/-U extras after its bundled definitions.
Propose the build-owned input
`LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT` for the candidate, with no
other unreviewed extras. This document does not set that environment variable.
[MSVC /U](https://learn.microsoft.com/en-us/cpp/build/reference/u-u-undefine-symbols?view=msvc-170)
can undefine a symbol previously supplied with /D; actual compiler argument
handling and the effective macro state must be checked in the later authorized
build, not assumed from an environment string.

Keep SQLITE_THREADSAFE=1 and the reviewed bundled source/features. The pinned
absence branch computes separateCache true when core mutexes are enabled
(55875–55880); no external PAGECACHE pool, custom PCACHE2 or runtime single-thread
mode is allowed into this candidate without its own provenance/accounting proof.
Those are eligibility restrictions, not instructions to overwrite someone else's
configuration. Reject unknown prior configuration instead of resetting it.
Closed pair commands still exclude shared-cache adoption, arbitrary URI options,
WAL/mmap, extension loading, external SQL and other unsupported engine features.
A separately proven file/connection owner remains necessary.

A build is reproducible/eligible only when the following non-secret facts are
fixed and retained against the actual linked sidecar artifact:

| Input/evidence | Required closure |
| --- | --- |
| Source and dependency identity | Repository/Cargo.lock digests, pinned SQLite/build.rs and crate-source identities, exact resolved Windows feature graph; no linked-library/sqlcipher/in_gecko/loadable-extension override |
| Toolchain | Exact Rust/Cargo, MSVC compiler/linker and Windows SDK versions/identities, target x86_64-pc-windows-msvc and build profile, recorded before the authorized invocation. A floating stable/windows-latest label is insufficient |
| Compiler configuration | Complete effective C/Rust/link arguments, relevant Cargo config and known CC/CFLAGS/target variants, CL/_CL_, RUSTFLAGS/wrappers/forced includes and crate default handling. Only reviewed build-owned values; no unrelated environment/credential dump |
| Effective semantics | Evidence that MEMORY_MANAGEMENT is absent, THREADSAFE=1, no unsupported allocator/cache/extra-init/shared-cache/alloca overrides, and default allocator/cache behavior matches the reviewed recipe. Compile-option strings are corroboration, not whole-artifact or configuration custody proof |
| Output and custody | Actual object/archive/sidecar identities and hashes, build source association and exact qualification evidence; private profile selection binds that artifact, not caller flags, a filename or package version |

The current [release workflow](../../../.github/workflows/release.yml) uses floating
windows-latest and rust-toolchain stable. It is not silently changed into this
qualified recipe. The previous compiler component's literal arguments, including
the memory-management define, remain evidence only for its recorded build; do
not reuse it as qualification of the amended artifact or renew its spent budget.
The earlier offline metadata failure remains unresolved; no metadata retry,
download, compilation or target execution was used for this document.

## Existing consumers and the startup seam

The runtime has ordinary SQLite consumers before any future pair is selected:

| Source owner | Existing use to preserve |
| --- | --- |
| [Codex store](../../../crates/orkworksd/src/codex_session_store.rs),248/286/310 | Read-only saved-session and label lookups. session_projection.rs,547, and session_application.rs/harness_reporting.rs invoke them |
| [OpenCode store](../../../crates/orkworksd/src/opencode_session_store.rs),11–22/65–103 | Read-only exact saved-conversation verification on spawn_blocking, called from session_application.rs,6709 |
| [Ordinary SessionStore](../../../crates/orkworksd/src/session_store.rs),111/129 | Existing private ordinary read-write adapter and its workers/tests; no production pair capability is inferred |
| Hierarchy mechanics/test entry points | Test-only SQLite constructors do not traverse process main; they cannot establish production startup ordering or be used to install a router into a live shared test process |

Cached rusqlite inner_connection.rs,71/88 and420–449, performs a threading/mutex
check before sqlite3_open_v2; do not assume the first visible open is the first
library initialization opportunity. SQLite itself can auto-initialize on API
calls. The public [configuration API](https://www.sqlite.org/c3ref/config.html)
is not threadsafe and rejects most options once initialized. Pinned
sqlite3_config,180219–180237, permits only LOG and PCACHE_HDRSZ anytime; GETMALLOC,
MALLOC and GETPCACHE2 are not late-install mechanisms. Their getter names also
do not imply no mutation: GETMALLOC fills the default allocator table if absent
(180296–180302), and GETPCACHE2 can fill the default cache table (180357–180365).

The future private bootstrap belongs in sidecar startup **after** the existing
Git-scan early-return branch but **before**
`run_sidecar_runtime(run_sidecar())` in
[main.rs](../../../crates/orkworksd/src/main.rs),225–240. That is before Tokio
worker threads, AppState construction, session listing, hooks or background tasks.
The inspected Git scan branch in git.rs,26–35, invokes Git inspection and has no
located SQLite call; it does not create a pair/bootstrap capability. Any future
pre-bootstrap SQLite consumer invalidates this ordering argument and must be
routed or cause pair unavailability, not hidden behind lazy initialization.

## Private bootstrap state and configuration custody

Propose one process-owned registry with stable storage established before its
callbacks can be installed. It holds the saved allocator method table, original
pAppData, phase, build association and eventual arena registry. This is private
volatile control state, not a serialized bearer/epoch/source registry. No generic
Connection, raw callback table or caller-supplied qualified boolean escapes it.

| Phase | Action and authority |
| --- | --- |
| Unexamined | Sole startup thread checks reviewed build association and source-owned configuration history; no pair allocation and no SQLite workers |
| UnavailableWithoutRouter | Preflight rejection makes no configuration calls. If a query was attempted, preserve its exact resulting state, including possible default-table fill; no router is installed, pair stays unavailable and ordinary behavior is not reset |
| SavedBeforeInstall | GETMALLOC may first fill SQLite's default allocator table, then copies current methods into stable owner storage. Do not call GETPCACHE2 in this recipe: it adds a possible default-fill mutation without proving backing-pool custody |
| DelegatingInstalled | MALLOC installs the router once. Pair context is absent, so all initialization/unpaired allocation delegates to the saved methods. Retain callback storage for process lifetime even if subsequent bootstrap steps fail |
| InitializedDelegating | Explicit sqlite3_initialize outside any pair context finishes global initialization through the saved allocator. Record the actual outcome. This only permits further pair qualification, not construction/source admission |
| FailedAfterInstall | Non-OK/unknown initialization, custody or later bootstrap outcome makes pair availability sticky-false. Keep installed storage and original allocator delegation; no restore/unregister/shutdown/reset/restart to force eligibility |

GETMALLOC's default fill is the same table selection the pinned library would
perform when initializing with no allocator configured. That source equivalence
assumes the reviewed default build and no earlier writer; it is not a measured
ordinary-compatibility result. A later failure cannot be reported as pristine
configuration, and the bootstrap must not undo the fill using private fields or
restore/reset calls. Preserve the observed state and continue ordinary behavior
without forcing pair eligibility. Check query-then-failure in qualification too.

The saved allocator's xInit/xShutdown are delegated when the engine invokes the
router lifecycle, not invoked a second time by the bootstrap itself. Save all
methods and pAppData, not just malloc/free. Router callbacks must not recursively
allocate through SQLite or unwind across C. Known non-arena pointers stay with their
saved owner; growth inside pair context denies instead of delegating. Unknown
provenance is not permission to pass an arbitrary pointer to the saved allocator.
No pointer migration into a pair allowance is allowed.

Absence of a public PAGECACHE getter is not filled with a guessed zero. Source-
owned startup must account for **all** configuration writers in the linked
program. No located repo calls to SQLITE_CONFIG_PAGECACHE/PCACHE2/MALLOC currently
supply such a bootstrap; defaults can be trusted only for a reviewed complete
build/startup path with no unknown earlier writer. GETPCACHE2 would return methods, possibly after filling defaults,
not backing-pool provenance; it is deliberately omitted from this recipe. Unknown or foreign configuration denies this
profile without changing unpaired cache/allocator/thread settings.

The registry remains stable through unpaired connections as well as pair
active/retiring/unknown lifetimes. Pair creation/open/prepare/finalize/destructors
enter only the actual original worker's context after global initialization.
Internal connection lookaside backing allocated there can use the arena; an
external/foreign lookaside or cache buffer denies unless its full provenance is
already admitted. No new zero-lookaside or page-count setting is introduced.
All xMalloc/realloc and controlled callback buffers still obey the accepted arena
and provenance rules; the new flag closes only the identified cross-cache reuse
path, not every possible allocation escape or source/native prerequisite.

## Compatibility, qualification and next deliverable

This shared flag has a real ordinary-consumer cost. The pinned
sqlite3_release_memory,30238–30248, becomes a no-op returning zero when the macro
is absent; the [public API](https://www.sqlite.org/c3ref/release_memory.html)
documents that behavior. Cache pressure/reclamation/performance can change for
Codex, OpenCode and ordinary SessionStore calls too. Scoped source searches found
no direct repo sqlite3_release_memory/release_memory/shared-cache enable call;
that does not prove all transitive/internal pressure behavior is unchanged.
Do not claim ordinary compatibility from unchanged SQL text or from this search.

Before any eligible profile, qualify the actual changed artifact and bootstrap:
repeated configuration/late init, prior custom allocator/cache, partial init,
failed original xInit, callbacks after failure, stable method/pAppData lifetime,
concurrent unpaired calls, pointer free/realloc provenance, per-cache ownership,
OOM and clean/unknown retirement; then actual native VFS and paired recovery/
durability. Exercise existing Codex/OpenCode reads and ordinary storage with the
changed library, preserving their read-only permissions and error/availability
contracts. Do not change foreign native databases to create evidence.

One bounded offline implementation plan may follow scoped acceptance and source
closure: private build-identity/configuration-state checking plus bootstrap/router
state tests using a fake API, with no real sqlite3_config/initialize/open calls or
main.rs wiring. The exact native build/toolchain inputs and execution allowance
remain a separate finite plan; a fake API test cannot qualify startup ordering,
ordinary compatibility, native API behavior or the 8 MiB arena implementation.
The existing [EOF accounting slice](../../validation/pair-resource-enforcement-coverage.md#bounded-private-implementation-slice-and-remaining-gates)
remains a separate coupled owner task, not an authorization hidden in this recipe.

## Owner decision and review boundary

Approve or reject the **one shared candidate build-flag change and the startup
custody/failure recipe above for further qualification**. This is the new scoped
choice; the accepted VFS/worker/PairCore/resource package is unchanged. If rejected,
keep the existing build and unavailable candidate; do not infer acceptance of a
custom PCACHE2 wrapper or another workaround. If accepted, record that exact
scope and synchronize the canonical design/ADR before dependent implementation.
No code, flag, bootstrap, SQLite configuration, migration, model/native/compiler
probe, target metadata retry, execution budget or runtime activation changes here.

One bounded fresh-context round used two read-only `gpt-6-luna` reviewers with
separate correctness and builder/completeness questions. The builder found no
plan-changing unanswered question. Correctness review verified the conditional
cache-reuse argument and identified that GETMALLOC/GETPCACHE2 can fill default
global method tables. Author disposition: verified against the pinned C source,
recorded default-fill effects and query-then-failure state, and omitted unnecessary
GETPCACHE2 from the proposed recipe. The author also made the already accepted
non-arena-growth/unknown-provenance rule explicit; it is not a new fallback.

No second independent review of the author clarification is claimed. Root is sole
writer; maximum three active agents, one review round. Review/merge does not
accept the new global feature change. Gate 1 remains Investigate; #1176 stays open.

## Scoped build and startup acceptance — 2026-11-24

After reviewed [#2043](https://github.com/Rambolarsen/orkworks/pull/2043), head
`ff07e3d64e84236da5d876f10aae865b72d8b0af`, merge
`67fa53ea45b5a1fafc4bdb2b84b9f34b631117d2`, the owner answered “keep going” to the
explicit question approving **this shared build-flag change and startup recipe
for further qualification**. Acceptance covers the candidate input
`LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`, its effective macro-absence
and ordinary memory-reclamation consequences, pre-Tokio bootstrap placement,
configuration custody and the reviewed getter-default-fill/retained-failure
phases. GETPCACHE2 remains omitted. This is not proof of the compiled artifact,
actual startup behavior, unpaired compatibility or complete allocation coverage.

[ADR 0119](../../adr/0119-pair-build-startup-custody.md) records the scoped build
selection and startup refinement. The VFS/router/worker/PairCore architecture,
ADR 0118 capacities and all lower/source/cutoff/newness/catalog/initializer/
recovery/durability/activation gates remain binding. No repeat acceptance of the
unchanged choices or this exact flag/recipe is needed. Custom PCACHE2/private
SQLite-copy alternatives and additional defaults remain unselected.

No flag, SQLite configuration, bootstrap code, metadata retry, compiler/native/
model/database/SQL probe, renewed budget, migration, source registration or
runtime activation is authorized or performed by this acceptance record. The
[qualification plan](../plans/2026-11-24-pair-resource-assurance-qualification.md)
remains Investigate before execution. A bounded private fake-API implementation
plan may be prepared; its execution still needs source closure and the applicable
slice authorization, not another acceptance of unchanged architecture.
