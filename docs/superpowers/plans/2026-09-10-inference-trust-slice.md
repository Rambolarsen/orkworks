# Inference Trust Foundation Implementation Plan

> **For agentic workers:** Use executing-plans inline. Root owns all writes. This continues the approved adapter architecture, not a new design approval cycle.

**Goal:** Implement executable identity and durable explicit grants without enabling custom execution.

**Architecture:** Resolve a validated capability without executing it, hash its defaulted configuration with harness origin and canonical path, and store grants outside harness JSON. Reuse Taskmaster's process and filesystem persistence locks. Consumers must supply freshly resolved identity; this slice deliberately exposes no HTTP/IPC approval path.

**Tech Stack:** Rust, serde, SHA-256, tempfile, existing fs2 persistence lock.

**Spec:** [Approved adapter design](../specs/2026-09-10-custom-inference-adapters-design.md), ADR 0055; issue #503.

## Constraints and routing

- Missing trust data means no grants; malformed, duplicate-key, unsupported, unreadable, and oversized data fail closed without overwriting it.
- Each successful approve/revoke increments a checked generation; stale generation or changed identity rejects approval. Atomic replacement preserves prior data on failure.
- Identity contains harness ID, origin, defaulted capability, canonical executable path, and format version. No executable contents or credentials are read.
- Ignore empty/relative PATH entries. Canonicalize symlinks. Reject non-executable files. Windows native executables use `.exe` resolution; shell scripts require an explicit native wrapper rather than implicit shell invocation.
- Root sole writer; at most two read-only reviewers, one correctness question and one missing-security-coverage question, one review round. Review runs alongside verification/bookkeeping, not implementation workers. No commit/push/PR.

## Task 1: Executable identity

Files: `harness/inference.rs` (immutable command accessor), new `taskmaster/inference_trust.rs`, new `taskmaster/inference_trust/identity.rs`, registration in `taskmaster/mod.rs`.

Interface: `AdapterIdentity::resolve(harness_id: &str, origin: DefinitionOrigin, capability: &InferenceCapability, path: Option<&OsStr>) -> Result<AdapterIdentity, String>`. Identity fields remain private; getters expose digest, harness ID, and absolute target.

- [x] Add tests for equivalent defaulted definitions and differing arguments, origin, ID, path; Unix fake executables verify symlink resolution, PATH precedence, and no execution.
- [x] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml inference_trust -- --test-threads=4` and observe missing identity behavior.
- [x] Implement resolution and canonical serialization: `Sha256::digest(serde_json::to_vec(&record)?)`; no PATH lookup at future spawn beyond the returned absolute target.
- [x] Rerun tests, adding rejected missing/non-executable targets and relative-only PATH.

## Task 2: Separate grant persistence

Files: `taskmaster/inference_trust.rs` and tests; expose existing lock helpers only to Taskmaster siblings in `taskmaster/runtime.rs`.

Interface: `InferenceTrustStore::new(root: PathBuf)`, `inspect(&AdapterIdentity) -> Result<TrustStatus, String>`, `approve(&AdapterIdentity, &TrustRevision)`, `revoke(&str, u64)`. `TrustRevision` contains generation and adapter digest; status includes revision and approval. Approve compares the current resolved identity against the expected revision before persisting. Revoke works even when a removed executable can no longer resolve.

- [x] Add real temporary-directory tests: `assert!(!store.inspect(&identity)?.approved)`; approve at generation zero; reopen and verify approval; revoke then reapprove increments to three; stale revisions fail without changing disk.
- [x] Observe failures before implementing strict version-1 document parsing, bounded reads, complete validation, and checked generation increments.
- [x] Under both existing Taskmaster locks, reread durable data, validate the expected generation, mutate, and atomically persist a private temporary file. No cached grant state and no generic settings mutation path.
- [x] Add malformed/unknown/duplicate/oversized/overflow and independent-store concurrent-approval tests. Verify unchanged disk after rejection.
- [x] Run Rust tests/build/fmt and repository verification. Review implementation with separate correctness and coverage questions, reconcile actionable findings, and record exact results.

## Not delivered here

Privileged approval endpoints/UI; custom runner; custom provider projection; cache/snapshot and atomic result acceptance integration. The foundation alone does not make a custom adapter runnable or prevent races in consumers that have not yet been wired in.

## Review reconciliation

- Coverage review: reproduced acceptance of malformed absolute grant paths, then rejected parent/dot components and control characters without checking current executable existence. Added real PATH precedence and cross-process file-lock tests; the latter launches only the local test binary and proves approval waits for the parent's Taskmaster lock.
- Correctness review: added Windows no-follow open flags. Reused the existing cross-platform `harness::integration::atomic_replace` helper, closing the temporary file before publication; this preserves the repository's Windows existing-target replacement behavior and Unix parent-directory sync.
- Two read-only reviewers, one round, root sole writer. Additional manual external review was offered; none was run. Windows execution remains unverified locally (macOS host); no Windows pass is claimed.
- Close-out investigation: future callers must resolve current registry identity under the registry mutation boundary and must integrate trust generation into cache/snapshot/final acceptance. These are already tracked in #503 and the approved design, so no duplicate issue was created. The current module does not enable inference or access login data.

## Verification — 2026-09-10

Final `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited 0 after review fixes: 1,103 Rust unit tests passed, two ignored, plus three integration/script tests passed; 687 desktop tests passed. Rust format/build, desktop type-check/build, docs build, diff check, documentation currency, and worktree currency passed. Existing warnings remain; the new foundation also has dead-code warnings until activation consumers are connected.

Focused trust suite: 12 passed; the subprocess helper is ignored in normal enumeration but explicitly executed by the cross-process-lock test. No actual custom adapter, provider call, credential access, or user trust grant was performed. No commit, push, or PR was created.
