# Repository Identity Runtime Implementation Plan

> **For agentic workers:** This plan is gated. Do not implement runtime code until the owner approves this exact plan and the capability/platform gates below are satisfied.

**Goal:** Establish a locally registered Git repository identity and bind the exact source worktree into approved orchestrated admission, allocation, spawn, and resume decisions.

**Architecture:** Add a focused sidecar identity module that discovers Git common-directory and worktree relationships, captures exact canonical paths and native directory identities, and persists bounded registrations under a retained advisory lock. Its internal API supplies immutable repository and source-worktree bindings to the separately gated #610 admission work; this issue does not wire allocation, launch, or resume. Unsupported platforms/filesystems fail closed, and identity metadata never grants workspace ownership or process authority.

**Tech Stack:** Rust 2021, `git2`, `serde`/`serde_json`, SHA-256 (`sha2`), OS randomness (`getrandom`), `fs2`, existing `windows-sys` filesystem APIs, `tempfile` fixtures.

**Spec:** `docs/superpowers/specs/2026-10-04-configuration-learning-design.md` (accepted identity contract, with runtime/platform gates); `docs/superpowers/specs/2026-10-04-agent-role-configuration-design.md`; `docs/superpowers/specs/2026-10-04-orchestrator-preparation-design.md`; `docs/superpowers/specs/2026-10-04-taskmaster-orchestration-scope-design.md`; `specs/taskmaster.md`; ADR 0080.

**Tracking:** [#810](https://github.com/Rambolarsen/orkworks/issues/810), prerequisite to [#610](https://github.com/Rambolarsen/orkworks/issues/610) under initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738).

## Global Constraints

- Preserve ADR 0060: one workspace lease and sidecar per instance; the registry is not workspace history, peer-instance discovery, or ownership state.
- Preserve ADR 0077: exact-plan approval, parent-only delegation, ordinary child sessions, manual integration, and no automatic cleanup.
- Repository-family identity and selected workspace/source-worktree identity are separate required bindings; a match grants no permission to adopt an existing worktree or read arbitrary files.
- Existing ordinary sessions retain current behavior; unbound legacy orchestration records cannot launch by path/name inference.
- No credential, remote URL, transcript, process/peer state, or Git-config contents enter identity records.
- No new dependency without a separately reviewed ADR/spec change; unsupported identity capability fails closed without a path-only fallback.
- Do not claim native confinement, filesystem atomicity across same-user races, or identity proof against direct registry rollback.
- This issue adds no HTTP route and does not activate orchestration. Runtime integration remains gated by reviewed #740 evidence, accepted #741/#742 consumer contracts, #610 prerequisites, and its own exact implementation plan.

---

## Complexity review before drafting

| Dimension | Rating | Evidence and response |
| --- | --- | --- |
| Dependencies | 4 | The identity module supplies fields consumed by #741/#742 and later #610 integration; keep launch integration out of this issue and require consumer review before use. |
| Blast radius | 4 | The registry is installation-local persistent state, though no admission or child-launch path is wired by this issue; make all reads fail closed and keep identity data separate from workspace ownership. |
| State changes | 4 | The local registry adds durable identity and retirement records; preserve bounded storage, epoch reset, and old-binding invalidation semantics. |
| Reversibility | Unknown | Runtime registry migration/reset has not been exercised; implement a new-epoch reset and rehearse crash-before/after-publish and restore behavior before activation. |
| Uncertainty | 5 | Native directory identities and durable publication are not verified across supported OS/filesystem combinations; make evidence collection the first blocking task and keep unsupported combinations unavailable. |

**Total: Incomplete (reversibility unknown). Plan quality: Investigate.** Resolve native identity and durable-publication evidence before dependent code; no automatic platform fallback is allowed.

## Investigated blind spots

- **Least confidence:** handle-backed canonical path and object identity stability on macOS, Linux, Windows, and non-local filesystems is not established by the Git-layout fixtures. Task 1 must produce OS/filesystem evidence or an explicit unsupported result for each claimed combination.
- **Project blind spot:** a repository ID could be mistaken for workspace ownership or source-directory proof. Carry a separate full `sourceWorktreeBinding`, retain the existing `workspaceId` and lease, and revalidate all three at admission boundaries.
- **ADR 0060 boundary:** its path-only workspace history and no-peer-registry decision remain intact. Store only repository registration/binding data in the separate identity registry; do not enumerate instances or add attention, health, process, port, or active-workspace state.

## Files and responsibilities

- Create `crates/orkworksd/src/repository_identity/mod.rs` for the crate-internal operations and typed failure results. Enrollment remains unreachable from a route or generic session creation until a separately approved #610 consumer exposes it only through Electron-authorized run preparation.
- Create `crates/orkworksd/src/repository_identity/records.rs` for strict versioned records, canonical bytes, validation, and binding digests.
- Create `crates/orkworksd/src/repository_identity/discovery.rs` for `git2` discovery and verification of root/private/common-directory relationships.
- Create `crates/orkworksd/src/repository_identity/platform.rs` for handle-backed canonical paths and versioned native directory-object identity; expose unsupported explicitly.
- Create `crates/orkworksd/src/repository_identity/store.rs` for bounded registration, retirement, locking, atomic publication, reset, and recovery.
- Modify `crates/orkworksd/src/main.rs` only to declare the private identity module; do not construct it in application state or add a route in this issue.
- Do not modify the orchestration record/store, `session_application.rs`, or `plan_handoff.rs` in this issue. A later #610 issue must consume these bindings at approval/allocation/spawn/resume and keep plan handoff's advisory path resolution separate.
- Add focused module and cross-platform tests beside the owning Rust modules; keep test-only fake object identities unable to qualify production platforms.
- Update the identity ADR/spec, #741/#742 consumer specs, #610 plan, and this plan only when implementation behavior is verified.

## Task 1: Qualify native identity and durable-publication primitives

**Files:**
- Create: `crates/orkworksd/src/repository_identity/platform.rs` after the evidence gate passes.
- Test: `crates/orkworksd/src/repository_identity/platform_tests.rs` or the platform module's test submodule.
- Update: `docs/validation/repository-identity-platforms.md` with exact OS/filesystem evidence and reproducible commands.

- [ ] Probe canonical handle-backed path and directory-object identity on each claimed OS using opened handles; record mechanism/version, identity fields, alias behavior, and unsupported filesystems.
- [ ] Probe durable atomic publication and retained-lock behavior for new and replaced registry files, including failure before publication and after publication.
- [ ] Verify `git2` behavior for primary, linked, separate-Git-dir, bare-backed worktree, submodule, clone, and nested repository layouts without inferring common directories by suffix removal.
- [ ] Mark any unverified OS/filesystem combination unsupported; do not use path-only identity or enable orchestration there.
- [ ] Stop dependent implementation when the native identity/durability evidence is missing or contradictory.

## Task 2: Add strict identity records and Git discovery

**Files:**
- Create: `crates/orkworksd/src/repository_identity/{mod.rs,records.rs,discovery.rs}`.
- Modify: `crates/orkworksd/src/main.rs` to declare the module after the service boundary is reviewed.
- Test: module-local deterministic records and disposable Git fixture tests.

- [ ] Define schema-v1 `RepositoryRegistration`, `RepositoryBinding`, `PathIdentity`, `DirectoryObjectIdentity`, and `ResolvedWorktree` matching the accepted contract; reject duplicate JSON keys, unknown fields, invalid base64, invalid IDs, oversized fields, and invalid digests.
- [ ] Implement canonical JSON and the `orkworks.repository-binding.v1\n` / `orkworks.worktree-binding.v1\n` digests with shared byte fixtures and checked byte counts.
- [ ] Discover only from the explicit existing cwd; ignore inherited Git routing variables, reject bare/non-Git input, and obtain worktree root, private Git directory, and common directory from verified Git APIs.
- [ ] Verify `.git` directory/file, `commondir`, linked-worktree metadata, and separate Git directory relationships; never treat `.git` file presence as linked-worktree proof.
- [ ] Capture canonical exact native path units and directory-object IDs from the platform module; return typed unsupported/drift/invalid failures instead of a lossy string fallback.
- [ ] Add fixtures showing primary and linked worktrees share repository binding but differ in worktree binding; clones and submodules remain separate; aliases resolve consistently.

## Task 3: Implement bounded local registry and recovery

**Files:**
- Create: `crates/orkworksd/src/repository_identity/store.rs`.
- Reuse: `fs2` retained advisory locks and `harness::integration::atomic_replace` only after verifying its platform behavior matches this registry's new/existing-file requirements.
- Test: storage fixture tests for concurrency, bounds, crash stages, malformed state, and reset.

- [ ] Store the v1 epoch and at most 1,024 active/retired registrations under the accepted installation-local path, enforcing the 8 MiB registry, 8 KiB record, 4,096-byte native path, and 128-byte object-field limits before publication.
- [ ] Retain one lock file inode; lock only short registry reads/writes and never across discovery, Git mutation, process launch, workspace lease operations, or model calls.
- [ ] Implement `resolveExisting(cwd)`, `enroll(cwd)`, `retire(exactBinding)`, and `revalidate(expectedBinding, cwd)` with deterministic duplicate, drift, unregistered, ambiguous, unsupported, and retired dispositions.
- [ ] Under the lock, reject duplicate active locators/object identities and serialize concurrent enrollment; relocation requires explicit retirement and cannot reactivate retired IDs when moved back.
- [ ] Make publication atomic and durable according to Task 1 evidence. I/O failure is not absence; malformed or corrupt state blocks resolution until explicit reset.
- [ ] Implement explicit reset/restore as a fresh random epoch that invalidates all old bindings; preserve the documented limits around direct byte-for-byte rollback and never silently restore prior active state.
- [ ] Verify registry capacity exhaustion blocks new enrollment without evicting/rebinding existing records; verify no file, worktree, or branch is created/deleted by registry operations.

## Task 4: Verify the substrate and keep runtime gated

**Files:**
- Update: `docs/agents/architecture.md`, `docs/adr/0080-repository-identity-across-worktrees.md`, and the repository-identity validation artifact.
- Test: Rust crate tests and platform CI fixtures.

- [ ] Run Rust formatter, focused identity tests, and the full Rust crate test suite on supported CI platforms; prove unsupported platforms fail closed.
- [ ] Run `git diff --check`, `bash scripts/doc-check.sh`, and the documentation build; update generated/currency references without claiming a supported production adapter absent evidence.
- [ ] Record exact platform evidence, tested commit, fixture coverage, and any unsupported matrix entries in the validation artifact and implementation issue.
- [ ] Keep orchestration activation closed. Create a separate #610 consumer issue/plan after #740, #741, and #742 are reviewed; it must bind and revalidate identities at approval/allocation/materialization/spawn/resume.
- [ ] Obtain the required code review and current-head CI before any PR is merged; this implementation plan does not authorize deployment or feature availability.
