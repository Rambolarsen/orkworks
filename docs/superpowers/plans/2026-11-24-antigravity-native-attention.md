---
type: Implementation Plan
title: Antigravity native terminal attention implementation
description: Qualification-first delivery of native Antigravity attention with Codex compatibility and ownership fences.
tags: [orkworks, antigravity, attention, plan]
status: draft
workflow_status: limited-activation
---

# Antigravity native terminal attention implementation

Outcome: a qualified Antigravity native question/permission wait projects Needs
You and resolves from native state, without Peon prompt inference or permission
participation. Implements [the approved design](../specs/2026-11-24-antigravity-native-attention-design.md)
and [MVP contract](../../../specs/orkworks-mvp.md#antigravity-native-terminal-attention-limited-1516-activation),
tracked in [#1996](https://github.com/Rambolarsen/orkworks/issues/1996).

One owner implements inline in the owned worktree. No worker fan-out or new
coding harness is needed for source work. The native probe requires an explicit
owner exception or a human start under the repository rule. Existing approval
covers this design; no repeated design ceremony is needed.

## Delivery order and checks

1. **Retain qualification first.** Use the prepared three-minute human-operated
   capture, or the same exact bounded probe under a separate owner exception.
   Establish app, fields, root convention, blocked question/permission and
   answer/denial/cancellation transitions. Include an already-allowed command
   negative case when available; otherwise retain its criterion as open. Keep
   raw PTY, titles, messages, prompts, answers and native IDs out of artifacts.
   No dependent runtime activation before the design gate is satisfied.
2. **Streaming profile and qualification selection.** Refactor
   `runtime/terminal_status.rs` into a shared native-terminal status
   module with explicit Codex/qualified-Antigravity source profiles. Keep the
   Codex accepted language and 256-byte bound unchanged. Pin the actual
   Antigravity language from retained frames; skip known optional text without
   decoding, bound before parsing, reject duplicates/foreign/child frames, and
   prove all chunk splits plus malformed/oversized recovery. Add failing tests
   before production changes; synthetic traces never count as native evidence.
3. **Launch and attention integration.** In the existing resolved-launch and
   `runtime/session_runtime.rs` paths, admit only built-in, exact qualified
   platform/version launches and source-owned emission/brief settings. Reuse
   the current bounded executable probe facilities, not a new process runner.
   Update `SessionApplication`, the runtime terminal-source state, hook gating
   and Peon attention policy together. Tests must prove the real stored/live
   tuple, explicit user/agent preservation, delayed hooks, sticky activation,
   clear/unknown ownership, fresh generations, replay exclusion, persistence
   failure and unchanged unqualified/overridden launch behavior.
4. **Synchronize and verify.** Record retained evidence and qualification
   limits in the canonical harness contract; update domain/provenance and
   design/plan execution state. Run cargo build/test/fmt, source reporter and
   desktop dependencies needed by the real Rust integration suite, then
   `bash scripts/verify-repo.sh`, docs checks/build, doc-check and worktree
   check. Complete `/code-review low` or the documented risk escalation for
   the actual diff; open one implementation PR and own its review lifecycle.

## Compatibility and rollback

No metadata schema migration, new persisted status vocabulary, user setting,
hook reinstall, ACP integration or approval ownership is required. Existing
Peon tuples survive rollout until an ordinary authorized transition. New
unqualified sessions retain documented hook behavior and never acquire native
attention from guessed app/version fields. Preserve Codex fixtures as regression
coverage. A source revert and fresh runtime restore previous behavior; do not
rewrite existing attention during rollback.

## Plan review

Disposition: **Investigate**, ready for retained qualification, not runtime
activation. Scope and module boundaries cover the approved contract; no further
architecture decision is delegated to the implementer. Dependencies 2 (shared
parser and launch/application boundaries); blast radius 3 (two runtime profiles,
Codex compatibility required); state changes 2 (additive provenance/runtime
selection, no migration); reversibility 1 (revert/relaunch); uncertainty 4 (native
wait/clear behavior not retained). Total 12, with the native gate explicit.
Retained second-run evidence now establishes app/field/root shape and the
question-answer/working/done sequence. The ordinary tool-denial and question-
cancellation criteria remain unqualified. Deterministic parser/application and
launch-preparation work may proceed behind a closed source activation gate;
this does not admit a native profile or claim those missing cases passed.
Activation still requires the original evidence gate or explicit owner
acceptance of the narrower empirical coverage. Update only materially changed
plan boundaries if native evidence contradicts the proposed route.


## Source implementation evidence

The shared parser and application wiring, source-owned launch preparation and
whole-tuple Peon preservation are implemented behind an empty qualification
table. Focused Antigravity tests pass, including the expired idle timer,
authenticated late hooks with distinct timestamps, and replacement-generation
fences. Synthetic executable/PTY fixtures exercise the positive path without
any native model request. Antigravity rejects malformed control framing and
skips bounded message bytes without decoding; Codex fixtures remain unchanged.

The first full repository run found the late-hook provenance bug, now corrected,
and three failures outside the changed logic. Their isolated disposition and
the final current-source verification are recorded in the implementation PR.
Native activation still requires the original retained-evidence gate or explicit
owner acceptance of partial empirical coverage. This implementation is not a
claim that the missing native denial/cancellation cases have passed.


Final source checks: `bash scripts/verify-repo.sh` passed with Node 22 and four
Rust test threads: 3,690 Rust unit tests passed (24 operator/manual tests ignored),
all Rust integration tests and 39 reporter tests passed, desktop type-check and
native ABI repair passed, 1,823 desktop tests passed (five skipped), and both
builds passed. The docs check passed 2,583 tests. The final parser cancellation
correction separately passed all seven parser tests and 28 shared terminal
regressions, including real synthetic PTY ingestion. Site maintenance tests
passed 42/42. `/code-review medium` approved the current code; its SHA and exact
review identity are recorded in the PR. Existing compiler and bundle/chunk
warnings are outside the behavioral diff. Doc currency passes. The pre-commit
worktree warning classified this new branch's unchanged base as merged; it did
not cover the uncommitted implementation. Other owners' worktrees were untouched.


## Limited activation follow-up, 2026-11-26

The owner accepted the [documented coverage limits](../../validation/antigravity-native-status.md#owner-accepted-limited-source-activation-2026-11-26)
for exactly macOS arm64 CLI 1.5.16. This follow-up populates the existing
source-owned gate and aligns hermetic executable/PTY fixtures with the shipping
version table. The updated admission tests failed against the empty table before
activation. It does not add permission policy, broaden other platform/version
eligibility or start another native model session. The original implementation
and review evidence above is historical; current verification/review evidence
is recorded in the activation PR.


## Rejection and automatic-command evidence follow-up

The owner-directed [2026-11-26 follow-up](../../validation/antigravity-native-status.md#explicit-rejection-and-automatic-command-follow-up-2026-11-26)
retains explicit native rejection and one automatic-command negative case.
Both are agent-operated captures, with failed startups and cleanup limitations
retained. No source eligibility, parser, application, hook or production launch
setting changed; the 1.5.16 profile remains limited. Verification for this
documentation-only increment is recorded in its PR.
