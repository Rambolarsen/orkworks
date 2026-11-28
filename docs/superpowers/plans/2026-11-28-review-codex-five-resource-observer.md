---
type: Qualification Plan
title: Qualify the fixed five-resource Codex physical observer
description: Bounded implementation and native macOS qualification plan for the dormant five-resource Codex observer and absence witness over the startup-sealed custody pool.
tags: [orkworks, review, codex, custody, observer, qualification]
status: stable
workflow_status: active
---

# Qualify the fixed five-resource Codex physical observer

Outcome: deliver and natively qualify the dormant fixed five-resource Codex
physical observer and `home/.orkworks/harnesses.json` absence witness over the
startup-sealed macOS custody pool, completing Step 4 of
[the custody-owner qualification plan](2026-11-22-review-codex-custody-owner-qualification.md)
and Stage 1 of [the capture qualification roadmap](2026-11-22-review-codex-capture-qualification.md).
Track [#2304](https://github.com/Rambolarsen/orkworks/issues/2304) under
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899),
[#1966](https://github.com/Rambolarsen/orkworks/issues/1966),
[#1972](https://github.com/Rambolarsen/orkworks/issues/1972), and closed
[#1981](https://github.com/Rambolarsen/orkworks/issues/1981).

Authoritative contracts:
- [Complete Codex capture inventory and custody](../specs/2026-11-22-review-codex-capture-design.md)
- [Keep Codex capture custody in a startup-sealed owner pool](../specs/2026-11-22-review-codex-custody-owner-design.md)
- [Ordinary-review catalog and integration leaves](../specs/2026-11-20-review-configuration-leaves-design.md)
- [ADR 0122 (amended 2026-11-28)](../../adr/0122-startup-sealed-codex-custody.md)
- `#1959` standard POSIX projector (`crates/orkworksd/src/harness/integrations/codex_fragment.rs`)

Pass condition: a dormant macOS arm64 observer over `LeasePool` passes all unit
and native synthetic-root tests proving exact five-resource custody, absence
custody, single-component `O_NOFOLLOW` traversal, hard-link/root-aliasing
refusal, exact `#1959` projection without carrier retention, pairwise and
running-build source-byte matching, complete descriptor/byte/time budget
enforcement, clean `Revalidate`/`Release` close-before-ACK, and zero parent
resource-file descriptor installation — while normal sidecar startup remains
unchanged and dormant.

## Non-goals and preserved gates

- No production bootstrap activation or normal-startup custody pool construction.
- No shared mutation epoch enrollment (`HarnessStore`, workspace selection,
  `OwnedAction`, reporter writers, hook-observation writers).
- No catalog/workspace/readiness/context/native evidence joins, producer
  publication, physical admission, or prepared `Start`.
- No user-file writes, lock-file creation, `status`/`inspect`/`reconcile` calls,
  or native Codex CLI probes.
- No edits to frozen `crates/orkworksd/build.rs`.
- No fallback into an ordinary launcher or renewal of the original 30-second
  preview expiry.

## Decisions and module boundaries

### 1. Slot descriptor ledger (`18` per slot = `4` controller + `14` resource)

Under the owner-approved 2026-11-28 restoration in
[ADR 0122](../../adr/0122-startup-sealed-codex-custody.md), `DescriptorLedger`
reserves `18` descriptors per slot (`4` controller + `14` resource, no
cross-slot borrowing) under the unchanged `256` global descriptor ceiling
(`4 × 18 = 72 <= 256`).

Complete five-resource descriptor fit proof (parent + worker combined):

| Phase | Parent FDs | Worker Retained FDs | Worker Transient FDs | Total Resource FDs (limit `14`) | Total Slot FDs (limit `18`) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `Idle` | 0 | 0 | 0 | 0 | 4 |
| `BindAndObserveFive` traversal (`.orkworks`/`hook-scripts`/`.codex`) | 3 (`ws`, `home`, `pkg`) | up to 7 (before `packaged/*`) | 1 (`.codex` or `hook-scripts`, closed before next subtree) | **11** | **15** |
| `BindAndObserveFive` completion (`packaged/*`) | 3 | 9 (`3` roots + `home/.orkworks` + `5` regular files) | 0 | **12** | **16** |
| `Bound` steady-state | 3 | 9 (`3` roots + `home/.orkworks` + `5` regular files) | 0 | **12** | **16** |
| `Revalidate` peak | 3 | 9 | 1 (`.codex` or `hook-scripts`, opened directly from retained `ws` or `home/.orkworks`) | **13** | **17** |
| `Release` (post-ACK) | 0 | 0 (`PROC_PIDLISTFDS == 5` baseline) | 0 | 0 | 4 |

Bind traversal order (`home` subtree -> `workspace` subtree -> `packaged`
subtree) opens the `1` transient directory only while worker-retained FDs are
`<= 7` (`<= 11` combined resource FDs), closes `hook-scripts` before opening
`.codex`, and closes `.codex` before opening `packaged/*` (`0` transient, `12`
combined resource FDs). Revalidate peaks at `13` (`<= 14`). On mid-RPC
quarantine/shutdown (`CleanupUnconfirmed`), the parent closes its `3`
`LeaseRoots` and retains the worker's peak resource charge (`10` for
`Revalidate`, `9` for `BindAndObserveFive`) plus `4` controller FDs until child
reap is confirmed.

### 2. Fixed five-resource inventory and absence witness

The worker owns the exact compiled lexicographically sorted five-item inventory
and never accepts caller path lists, arbitrary inventories, or expected-template
bytes:

| Tag | Leaf ID | Role | Root + relative traversal | Returned bytes |
| --- | --- | --- | --- | --- |
| `0` | `codex-hook-fragment` | `native_owned_fragment` | `workspace` -> `.codex` -> `hooks.json` | Exact canonical output of `project_standard_posix_codex_fragment` (`<= 16 KiB`) |
| `1` | `installed-metadata-helper` | `installed_asset` | `home` -> `.orkworks` -> `hook-scripts` -> `session-metadata.py` | Complete original file bytes (`<= 256 KiB`) |
| `2` | `installed-reporter` | `installed_asset` | `home` -> `.orkworks` -> `hook-scripts` -> `report-harness-event.sh` | Complete original file bytes (`<= 256 KiB`) |
| `3` | `source-metadata-helper` | `source_asset` | `packaged` -> `session-metadata.py` | Complete running-build source bytes (`<= 256 KiB`) |
| `4` | `source-reporter` | `source_asset` | `packaged` -> `report-harness-event.sh` | Complete running-build source bytes (`<= 256 KiB`) |

Separate absence witness:
- Check `fstatat(orkworks_fd, "harnesses.json", AT_SYMLINK_NOFOLLOW)` under the
  retained `home/.orkworks` directory descriptor.
- Only `ENOENT` counts as absence. Any existing regular file (even empty),
  directory, symlink (including dangling symlink), special file, permission
  error, or I/O error denies availability with a closed internal category.
- Never create directories or lock files. If `home/.orkworks` itself is missing
  (`ENOENT`), deny cleanly (`MissingAsset`) without creating it; test the
  ancestor-plus-missing-suffix helper independently for both missing `.orkworks`
  and missing `harnesses.json`.

### 3. Traversal, aliasing, byte equality, and carrier-receipt rules

- **Single-component `O_NOFOLLOW` only:** Every directory hop uses
  `openat(parent_fd, single_component, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)`
  + `fstat` confirming `S_IFDIR`. Every regular file uses
  `openat(parent_fd, leaf_name, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)`
  + `fstat` confirming `S_IFREG` and `st_nlink == 1`. No `canonicalize`-then-open
  and no multi-component `openat`/`fstatat`.
- **Root, directory, and file aliasing refusal:** Require pairwise distinct
  `(st_dev, st_ino)` across all six directories
  `{workspace, home, packaged, workspace/.codex, home/.orkworks, home/.orkworks/hook-scripts}`
  and pairwise distinct `(st_dev, st_ino)` across all five regular files.
- **Pre/post-read and parent/name validation:** Record `fstat(fd)` before
  reading from offset `0` via `pread` and verify after reading that `fstat(fd)`
  (`dev`, `ino`, `size`, `mtime`, `mtime_nsec`, `ctime`, `ctime_nsec`,
  `nlink == 1`) is unchanged, `bytes.len() == st_size`, and
  `fstatat(parent_dir_fd, leaf_name, AT_SYMLINK_NOFOLLOW)` still resolves to the
  same `(dev, ino)` regular file with `nlink == 1`. On `Revalidate`, explicitly
  verify the bound `(st_dev, st_ino)` directory identities of `home/.orkworks`
  (via `fstatat(home_fd, ".orkworks", AT_SYMLINK_NOFOLLOW)`, `0` extra FDs),
  `home/.orkworks/hook-scripts`, and `workspace/.codex`.
- **Transient carrier & private receipt:** `hooks.json` raw bytes are bounded to
  `256 KiB`, projected once via `project_standard_posix_codex_fragment`, and
  dropped immediately before sending the response. Physical missing carrier
  (`MissingCarrier`) and projector `Ok(None)` (`MissingOwnedFragment`) are
  distinct internal denial categories. Malformed, partial, customized,
  namespace-ambiguous, or oversized (`> 256 KiB` input or `> 16 KiB` output)
  carriers deny (`MalformedOrUnsupportedCarrier` / `FileLimitExceeded`) without
  propagating parser diagnostics or raw paths. The worker retains only
  `CarrierReceipt { file_identity, projected_len, projected_sha256 }` — never
  raw carrier bytes or a whole-carrier digest.
- **Pairwise & running-build source bytes:** Both worker and parent hold the
  immutable running-build bytes compiled from
  `crates/orkworksd/scripts/session-metadata.py` and
  `crates/orkworksd/scripts/report-harness-event.sh` (also hashed into
  `codex_capture_owner::build_identity()` alongside `codex_fragment.rs` and the
  new observer module). Worker verifies `source-* == running-build` and
  `installed-* == source-*` on `BindAndObserveFive`, and rereads all 5 opened
  files from offset `0` on `Revalidate`, checking metadata/parent-binding
  identity and fresh full byte/projection equality without caching raw script
  copies in the lease state.
- **Budgets:** One `3 s` shared wall deadline per operation; `256 KiB` per file
  (reading up to `256 KiB + 1` to detect concurrent growth); one shared `2 MiB`
  (`2_097_152` bytes) raw read budget per `BindAndObserveFive` or `Revalidate`
  counting every byte read plus `1` byte per EOF confirmation probe; `16 KiB`
  projected fragment cap; `512 KiB` combined five-resource leaf byte cap; and
  `1 MiB` channel frame cap. No implicit retry.

### 4. Wire protocol extension (`OWCO` v1)

Keep Step 2 synthetic frame kinds `10`–`11` intact so all existing Step 2/3
qualification tests remain unmodified. Extend `OWCO` v1 on the existing split
unidirectional channels (`AF_UNIX` request socket + OS `pipe()` response):

| Kind | Name | Direction | Ancillary FDs | Payload |
| --- | --- | --- | --- | --- |
| `20` | `BindAndObserveFive` | `parent -> worker` | Exact 3 `SCM_RIGHTS` `O_DIRECTORY` FDs (`[workspace, home, packaged]`) | `lease_id[16]`, `lease_seq: u64` (`>= 1`), `op_seq: u64` (`== 1`), `ttl_ms: u32` (`1..=30_000`) (36 bytes) |
| `21` | `FiveObserved` | `worker -> parent` | 0 (`pipe()`) | `lease_id[16]`, `lease_seq: u64`, `op_seq: u64`, `status: u8` (`0` Observed, `1` Denied), `denial: u8`, `root_identities[3]`, plus when `Observed`: `absence_ancestor(dev, ino)` and exact 5 ordered `(tag: u8, dev: u64, ino: u64, len: u32, bytes[len])` records (`payload <= MAX_PAYLOAD_BYTES` (`1 MiB`), sum of `len <= 512 KiB`) |

`Revalidate` (`12`) / `Revalidated` (`13`) and `Release` (`14`) / `ReleaseAck`
(`15`) are reused as-is, dispatched by the worker's `ActiveLeaseMode`
(`Synthetic` vs `FiveResource`). On `FiveObserved(Denied)`, the worker closes
all received roots and any opened descriptors and confirms
`PROC_PIDLISTFDS == 5` before sending the denial frame. Parent checks
`root_identities` against `LeaseRoots`, independently validates source/installed
and running-build byte equality, pairwise distinct file inodes, and caps; if
any parent-side check fails on `FiveObserved(Observed)` (where the worker has
already retained its 9 FDs), the parent drops `LeaseRoots` and quarantines the
slot (`ProtocolViolation`), never leaving it `Idle` or `Bound`. On
`FiveObserved(Denied)`, parent surfaces the typed closed `DenialCategory` via
`LeaseError::ObservationDenied(DenialCategory)` (zero raw paths or parser
diagnostics). On success, parent rechecks expiry after RPC before publishing
`Bound` (recording lease mode / `bound_resource_fds` on `BoundLease` so
`Revalidate` resets to `12`, not `6`) and returns an ephemeral
`FiveResourceObservation` (dropping raw bytes once consumed/hashed without
storing them in `LeaseHandle` or `BoundLease`).

### 5. Module layout

All code stays dormant under `crates/orkworksd/src/taskmaster/codex_capture_owner/`,
preserving standalone compilation for integration tests that include
`mod.rs` via `#[path = "../src/taskmaster/codex_capture_owner/mod.rs"]`
(zero direct `crate::harness::*` imports inside `codex_capture_owner`):
- `observer.rs` (new): fixed 5-resource inventory constants, compiled-in
  running-build script bytes, `FragmentProjector` function pointer
  (`fn(&[u8]) -> Result<Option<Vec<u8>>, FragmentProjectionError>`),
  `ReadBudget`, `FileIdentity`, `CarrierReceipt`, `DenialCategory`,
  `AbsenceWitness`, single-component `O_NOFOLLOW` traversal, 6-dir/5-file
  aliasing and hard-link checks, `pread` + pre/post-read verification, and
  `FiveResourceCustody` (owns the `9` worker FDs plus bounded receipts;
  implements `observe_and_bind` and `revalidate`).
- `framing.rs`: add `BindAndObserveFive` (`20`), `FiveObserved` (`21`),
  `DenialCategory`, and `ObservedResourceRecord` codec with strict length,
  order, tag, `16 KiB` / `256 KiB` / `512 KiB` / `MAX_PAYLOAD_BYTES` (`1 MiB`)
  cap enforcement.
- `worker.rs`: accept the injected `#1959` projector callback from `main.rs`
  (wrapping `crate::harness::integrations::codex_fragment::project_standard_posix_codex_fragment`),
  wire `BindAndObserveFive`, `ActiveLeaseMode::FiveResource` revalidation, and
  close-before-ACK / close-before-denial `PROC_PIDLISTFDS` verification.
- `lease.rs`: update `SLOT_DESCRIPTOR_RESERVATION = 18` (`14` resource FDs),
  store lease mode / `bound_resource_fds` on `BoundLease`, add
  `LeaseError::ObservationDenied(DenialCategory)`, add
  `bind_and_observe_five_resources` returning `(LeaseHandle, FiveResourceObservation)`,
  enforce `12` bind-peak / `13` revalidate-peak ledger accounting, quarantine
  on parent-side `FiveObserved(Observed)` validation failure, and preserve
  exact unconfirmed-cleanup (`9` on Bind / `10` on Revalidate) and
  `Revalidate`/`Release`/expiry semantics.
- `mod.rs`: include `observer.rs`, `../../harness/integrations/codex_fragment.rs`,
  `../../../scripts/session-metadata.py`, and
  `../../../scripts/report-harness-event.sh` in `build_identity()`.

## Delivery steps and verification matrix

1. **Framing, ledger, and pure observer unit tests (TDD, including binary-target projector integration tests via `cargo test --bin orkworksd`):**
   - `SLOT_DESCRIPTOR_RESERVATION = 18` (`4 + 14`), no cross-slot borrowing,
     unchanged `256` global ceiling (including boundary denial before opening
     `LeaseRoots` on `BindAndObserveFive`, and `+1` transient denial on
     `Revalidate` at `4 + 12 + 240 = 256` leaving steady-state `12` intact).
   - `BindAndObserveFive` / `FiveObserved` round-trip and negative codec tests
     (bad tag order, duplicate tag, `> 16 KiB` fragment, `> 256 KiB` file,
     `> 512 KiB` aggregate, `> MAX_PAYLOAD_BYTES` (`1 MiB`) payload, non-zero
     denial with `Observed`, `Denied` with `denial == None (0)`, unknown denial
     code, trailing bytes).
   - `ReadBudget` unit tests proving `2 MiB` exhaustion and `1`-byte EOF-probe
     charging; `AbsenceWitness` unit tests for missing `.orkworks` vs missing
     `harnesses.json`; `observer.rs` pre/post-read concurrent-mutation check and
     real `#1959` projector callback tests.
2. **Native macOS five-resource qualification suite (`crates/orkworksd/tests/codex_custody_five_resource.rs`):**
   - **Positive bind, revalidate, release, and reuse:** synthetic temporary
     `workspace`, `home`, `packaged` roots containing valid `#1959` carrier
     (including unrelated user hook entries that must not leak into the
     projected leaf) and exact running-build script bytes; confirm exact 5 leaf
     bytes/digests, worker `PROC_PIDLISTFDS == 14` (`5` baseline + `9` lease
     FDs) while bound, parent `PROC_PIDLISTFDS` never contains any of the 5
     resource file inodes or intermediate dir inodes, worker `proc_listchildpids == 0`,
     `Revalidate` returns `Fresh` without extending expiry and resets ledger to
     `12`, `Release` returns worker `PROC_PIDLISTFDS` to exact `5` baseline FDs
     before `ReleaseAck`, and the same worker PID reuses cleanly for a second
     lease.
   - **No-follow, hard-link, non-regular, and aliasing refusals:** symlink at
     `.codex`, `.orkworks`, `hook-scripts`, `hooks.json`, or any script; hard
     link (`st_nlink == 2`) on `hooks.json` or any script; FIFO (`mkfifo`) at a
     leaf path (must not hang and must deny `NonRegularRejected`); directory at a
     leaf path; unreadable (`0o000`) directory or file (must deny, never
     absence); root/directory aliasing across all 6 directories (`workspace == home`,
     `workspace == .orkworks`, `packaged == hook-scripts`, `packaged == home`,
     `packaged == workspace`, hard-linked `source-*` and `installed-*`).
   - **Absence witness controls:** `home/.orkworks/harnesses.json` present as
     empty file `{}` (`AbsenceTargetPresent`), directory (`AbsenceTargetPresent`),
     dangling symlink (`SymlinkRejected`), missing `.orkworks` (no directory
     created, `MissingAsset`); creating `harnesses.json` after bind causes
     `Revalidate -> Changed`, and deleting it again cannot resurrect the
     invalidated lease.
   - **Projection & carrier hygiene:** missing `hooks.json` (`MissingCarrier`)
     vs JSON with no OrkWorks entries (`MissingOwnedFragment`), malformed JSON,
     partial 5-event bundle, customized matcher, foreign namespace marker,
     `> 256 KiB` carrier (`FileLimitExceeded`); verify unrelated carrier secrets
     do not appear in `FiveObserved` bytes, `LeaseHandle`, or denial responses.
   - **Source-byte, mutation, and quarantine controls:** modified `installed-*`
     (`PairwiseByteMismatch`), modified `source-*` (`SourceByteMismatch`),
     in-place edit or atomic rename-replace of `hooks.json` (including unrelated
     hook entry edit -> conservative `Changed`), in-place edit or replace of any
     installed/source script (`Revalidate -> Changed`), directory swap/delete of
     `home/.orkworks`, `.codex`, or `hook-scripts` (`Revalidate -> Changed`),
     synthetic-peer test returning `FiveObserved(Observed)` with mismatched
     script bytes or aliased file inodes (parent drops `LeaseRoots` and
     quarantines/retires slot), and `SIGSTOP`-induced zero-wait quarantine
     confirming exact `CleanupUnconfirmed` charges (`9` resource FDs mid-Bind,
     `10` resource FDs mid-Revalidate) cleared on `reconcile_unconfirmed()`.
   - **Multi-run stability & full repo verification:** 10x stability run on
     `codex_custody_five_resource`, existing `codex_custody_*` suites unchanged,
     `bash scripts/verify-repo.sh`, independent current-code review (`/code-review`),
     `bash scripts/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.

## Risk assessment and review

| Dimension | Rating | Evidence and mitigation |
| --- | ---: | --- |
| Dependencies | 3 | Reuses qualified `LeasePool`, split channels, `codex_fragment.rs` (`#1959`) via callback injection, and compiled-in script assets; no `HarnessStore` or runtime callers touched. |
| Blast radius | 2 | Dormant under `codex_capture_owner`; normal sidecar startup and manual Codex integration paths remain untouched. |
| State changes | 1 | None; no user-file writes, lock files, persisted records, or schema migrations. |
| Reversibility | 1 | Local revert of dormant module and tests; no migration or user-file rollback needed. |
| Uncertainty | 3 | Exact macOS `openat`/`fstatat`/`pread` + `PROC_PIDLISTFDS` accounting and FIFO/symlink/mutation controls must be proven natively on arm64. |

Total: 10/25 (all five dimensions known). Disposition: **Ready** (independent
plan review completed on 2026-11-28; all four P2 findings — standalone
projector-callback compile boundary, complete 6-dir/5-file aliasing plus
`home/.orkworks` revalidation, parent-side `FiveObserved(Observed)` quarantine /
`BoundLease` mode reset / `DenialCategory` surfacing, and Bind peak table +
4 added verification cases — and the payload-cap/ADR cross-reference nit are
incorporated).
