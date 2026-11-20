---
type: Implementation Plan
title: Codex isolation probe and render components
description: Proposed bounded Rust extraction preserving manual startup while exposing deterministic isolation classification and argv rendering.
tags: [orkworks, review, codex, implementation]
status: draft
---

# Codex isolation probe and render components

**Workflow: proposed; owner design/plan approval pending. No implementation yet.**

Goal: separate the existing Codex isolation probe from deterministic command
rendering so later pure capture can reuse the actual source behavior without
launching a process. Pass only when existing manual new/exact-resume isolation
behavior and failure policies remain unchanged.

Spec: [concrete design](../specs/2026-11-20-review-codex-isolation-design.md).
Track [#1941](https://github.com/Rambolarsen/orkworks/issues/1941), under
#68/#66/#1899 and #738/#741. Use native inline execution with TDD and a fresh
final reviewer; this is one tightly coupled source change, not parallel writers.

## Scope and boundaries

One implementation task, one PR. Own
`crates/orkworksd/src/runtime/codex_launch.rs` and its adjacent tests in the same
module; update the evidence/design disposition when verified. Keep
`session_runtime.rs` and its call site unchanged unless an existing regression
requires a minimal test-only fixture correction supported by failure evidence.
No new runtime mode, bypass flag, owner/cache, public API, native profile row,
asset/projector, storage, reservation or prepared Start implementation.

Material interfaces, all private crate Rust:

- `IsolationAdvertisement::{Advertised, NotAdvertised}`: classification only.
- `isolation_advertisement(stdout: &[u8], stderr: &[u8]) -> IsolationAdvertisement`:
  preserve current stream separator, lossy UTF-8, ANSI and first-token semantics.
- `render_isolated_arguments(args: &[String], advertisement: IsolationAdvertisement)
  -> Vec<String>`: preserve all args or prepend one exact flag as specified.
- `async probe_isolation_advertisement(command: &CommandSpec)
  -> Result<IsolationAdvertisement, StartupError>`: existing probe resolution,
  environment, ownership, output/deadline/error behavior without command mutation.
- `async isolate_session(command: &mut CommandSpec) -> Result<(), StartupError>`:
  existing entry point delegates to those seams; only replaces args on success.

No signature takes an eligibility boolean or permits callers to assert a native
profile. Rendering returns an argv value; it is never a custody/admission proof.
First prepared eligibility, actual evidence publication and consumer bypass
remain unimplemented, even after this plan passes.

## Task: extract the real source behavior and verify compatibility

- [ ] Add failing pure classification/render tests in `codex_launch.rs`: split
  streams with no trailing LF, ANSI-styled exact option, lookalike/substrings,
  no option, a flag before/after `--`, empty args/entries, preserving duplicate
  existing flags and argument order. Pin current lossy UTF-8 behavior explicitly;
  do not silently change it while extracting. Confirm failures reflect missing
  seams rather than fixture failures.
- [ ] Extract the enum, pure classifier and renderer, then separate the current
  probe into `probe_isolation_advertisement`. Keep actual `isolate_session`
  calling it and applying the rendered args. Preserve the three-second shared
  budget, 64 KiB per stream with overflow denial, capability/environment filter,
  exact current errors/log messages, cwd, null stdin and native cleanup ownership.
- [ ] Extend actual-wrapper synthetic tests for complete unchanged command on
  unsuccessful/oversized/help timeout, and NotAdvertised preservation. Reuse
  existing environment-filter and owned probe fixtures; do not write user config
  or execute the installed Codex. Verify failed probe cannot reach rendering.
- [ ] Run focused `runtime::codex_launch::tests` and relevant existing
  `session_runtime.rs` two-session environment-isolation/cancellation regressions;
  use exact discovered test names. Existing manual exact-resume args must retain
  the flag-before-args behavior. Investigate any failure before changing fixtures.
- [ ] Run `bash scripts/verify-repo.sh` with repository Node/pnpm setup, then
  doc/worktree/whitespace close-out checks. No dependency or generated lockfile
  changes belong in this task. Update executed counts/limits in the existing
  design/PR evidence, with production gates still closed.
- [ ] Obtain a fresh whole-diff correctness review and required `/code-review low`
  of the current Rust diff before merge; escalate only if documented risk/size
  changes. Address findings, rerun affected checks and follow the normal PR
  lifecycle. Do not activate a prepared consumer as review cleanup.

## Review focus and acceptance

Every relevant condition above has a direct assertion or existing wrapper/runtime
regression. Review especially a flag after `--` masquerading as already isolated,
stream-boundary loss, ambient capabilities reaching help, failed output mutating
argv, and a future caller mistaking the enum for evidence. The first four belong
to focused/runtime assertions; the last is enforced by the absence of any new
consumer/eligibility path and the explicit source-only disposition.

The future owner/capture/admission contract is specified to constrain later work;
its foreign-owner/native/executable/epoch/physical tests are outside this plan.
A passing parser/extraction PR does not satisfy those gates or complete #68.
#1941 may close only after its design/approval criteria are actually met; do not
close it just for drafting this plan.

## Investigated risks and plan review

The source audit and follow-up investigation established that version-cache
entries have independent TTL/generation and truncation/stream policies; their
text cannot substitute for the isolation probe. The existing native executable
identity is metadata-based and is not complete executable custody. The native
compatibility table remains empty; extracting help parsing cannot populate it.
The current #1933 join already permits base/final argv to differ while requiring
final/configuration equality, so no speculative leaf schema migration is needed.

| Dimension | Rating | Reason |
| --- | ---: | --- |
| Dependencies | 2 | One existing Rust owner and wrapper; existing probe process/runtime regressions. |
| Blast radius | 3 | Shared manual new/resume isolation, bounded by unchanged wrapper and call site. |
| State changes | 1 | No new durable state or activated path; probe effects remain existing ones. |
| Reversibility | 1 | Local refactor revert preserves metadata and installed artifacts. |
| Uncertainty | 2 | Current semantics read in source; synthetic/runtime checks still need execution. |

Total9/25 for the bounded extraction, not the future capture/creator architecture.
Self-review: source obligations, signatures, compatibility, error recovery and
check coverage reviewed once against the design; no prewritten implementation
or second owner introduced. Disposition: **Ready for owner review; implementation
blocked on concrete design/plan approval.**
