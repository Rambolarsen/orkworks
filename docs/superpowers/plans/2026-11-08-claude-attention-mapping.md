---
type: Implementation Plan
title: Claude prompt and turn mapping
description: Deliver the accepted Claude event mapping and conservative elicitation clears for issue 682.
tags: [claude, attention, hooks, implementation]
status: draft
workflow_status: active
---

# Claude prompt and turn mapping

Implement [#682](https://github.com/Rambolarsen/orkworks/issues/682) under
[ADR 0073](../../adr/0073-other-harness-prompt-attention-authority.md), the
[MVP event contract](../../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals)
and [accepted design](../specs/2026-09-27-other-harness-prompt-attention-design.md).
The [owner's #643 disposition](https://github.com/Rambolarsen/orkworks/issues/643#issuecomment-5956163125)
explicitly permits this mapping with feature-probed, limited coverage. No new
architecture, generation protocol, version verification or harness/model call.

Pass condition: only accepted prompt types promote authority; authenticated
Claude submission and allowed notification clears work after promotion;
ambiguous or unrelated clears preserve waits; terminal input and Peon boundaries
remain intact. Root owns implementation and verification in one isolated worktree.

## Decisions and boundaries

- Reuse the existing registration/token/generation/reset/metadata transaction
  path. Replace its two boolean event classification with a private typed
  classification for prompt open, submission, delayed idle and the two existing
  allowed elicitation-clear notifications. Copilot behavior is unchanged.
- Claude receipts remain ordered by receipt, without promoting a caller-provided
  timestamp into source-time proof. No turn/clear event activates authority.
- Refine private RuntimePromptKind to retain a sole Claude MCP or URL elicitation
  and an ambiguous elicitation state. A second unresolved prompt involving an
  elicitation becomes ambiguous until committed input, submission or session
  clearing; no request ID exists to deduplicate it. Keep that state in the kind
  so existing None clears, SQL currentness checks and runtime replacement retain
  their established lifecycle behavior. Held reset notifications preserve this
  same bounded ambiguity rather than collapsing to the last prompt.
- A matching sole elicitation clear reports working. A mismatched, zero or
  ambiguous elicitation clear makes no attention change. Delayed idle cannot
  clear any elicitation. Permission single-key clearing remains available;
  free-form or mixed ambiguous waits cannot clear from an unsubmitted key.
- POSIX and PowerShell reporters require the raw Claude Notification event name,
  safely typed native ID and notification type; register before every accepted
  report. Add UserPromptSubmit transport without prompt contents. Reconciliation
  removes only owned legacy PreToolUse hooks, adds UserPromptSubmit and preserves
  synchronous Write|Edit plan-path reporting and foreign hooks. No Stop hook.

## Delivery and checks

- [x] Add failing route/state fixtures using real registration and application
  seams: pre/post promotion, delayed idle, permission and MCP/URL zero/one/multiple
  and mixed elicitations, mismatched identity/type/token/generation, malformed
  payloads, receipt order, user-owned tuple and independent summary/phase/evidence.
- [x] Implement the smallest classification and bounded runtime-kind changes,
  including reset-held ambiguity and existing terminal/SQL currentness consumers.
  Extend metadata's existing authority-only transaction with idle, preserving
  attention ownership and persistence-failure behavior.
- [ ] Add failing reporter and installer fixtures, then implement both platforms
  and owned-hook migration. Run actual POSIX transport and hosted PowerShell
  cases; synthetic fixtures do not become live/version qualification.
- [ ] Update canonical source behavior and fixture provenance. Run scoped
  regressions, full repository verification, docs checks/build, current-code
  manual review and applicable native CI. Resolve feedback and merge via PR;
  keep #643 open for outstanding live evidence and clean owned worktree.

## Review

Ready after self-review against #682 and ADR 0073. Dependencies 3: existing
shared authority and reporter/installer seams. Blast radius 3: attention shared
with Copilot, protected by regression coverage. State changes 2: private runtime
kind only, no persisted schema/DTO. Reversibility 2: source revert and explicit
owned-hook reconciliation. Uncertainty 3: bounded source/protocol evidence;
live/version limits already dispositioned. Total 13/25.

Review focus: before-activation clears, ambiguous/mixed elicitations, delayed
Claude receipt order, queued reset prompts, source-priority/persistence failures,
legacy owned-hook removal and both-platform transport. Each has a delivery test
above; final reviewer checks only this implementation against the accepted scope.

## Execution evidence

- Root worktree `claude-attention-mapping` selected at 02431980407875d6d55f7cd5844f84adeaf8510a;
  OrkWorks binding reported before edits (metadata revision 38655). Other owners'
  worktrees are untouched. Frozen desktop/docs dependencies installed with Node 22;
  fs-ext rebuilt for host ABI before full Rust verification.
- Route regression: five missing-turn/clear cases failed and two existing guard
  cases passed before implementation; all seven then passed. POSIX transport
  regression failed because only three prompt openings were sent; extended
  reporter then passed the whole script, including missing/mismatched event names,
  unknown types, registration denial and private prompt exclusion.
- Installer regression failed on absent UserPromptSubmit. The shared integration
  filter then passed 90 tests after owned legacy migration; foreign-hook and
  synchronous plan-path behavior remain covered. Removed the old test's private
  generic-kind assertion; its real unsubmitted-input/attention assertions remain.
- Reset handoff regression failed when the second held prompt replaced the sole
  prompt kind; retaining bounded kind history passed that case. An unrelated SQL
  reservation test in the broad authority filter timed out waiting to enter its
  worker queue; it is not counted as green and remains in verification evidence.
- Native PowerShell transport fixture compiles for x86_64-pc-windows-gnu, exercising
  actual reporter bytes with isolated HOME/USERPROFILE and a fake HTTP function.
  Its hosted execution is still required; this is not native execution evidence.
- Full repository verifier and one fresh final code review are running. The first
  full verifier has reported an unchanged SQL observer case failure; final status
  and targeted recheck must be recorded before completion.

- Fresh `/code-review medium` identified one queue edge: a prompt superseded by
  independently committed input must not contribute ambiguity to the next held
  prompt. The regression failed (Ambiguous versus sole Dialog), then passed after
  filtering previous queued state with the existing committed-input predicate,
  retaining its reset-submission exception. All 44 Claude-focused tests pass,
  including user ownership and clear-persistence failure/retry. Scoped review of
  that fix is approved; no Critical/Required findings remain.
- Final reviewed code fingerprint: `4c5977b6d497c5f8a9ec060247c34a75eb94ef251e8fec6850091ceda16ed307`.
  Framing is SHA-256 over all 14 changed/untracked Rust source, reporter and
  workflow paths sorted lexically, each path + NUL + contents + NUL. Base is
  02431980407875d6d55f7cd5844f84adeaf8510a. Docs-only edits do not alter that code.
- First full verifier: 3213 Rust tests passed, 18 ignored, one unchanged native SQL
  observer case returned Conflict. It is not counted as green. Final verifier
  reruns the full repository with one Rust test thread to avoid shared SQL test
  interference. Docs build passes; native Windows execution remains a CI gate.

- Documentation-currency warning reviewed: the user sessions guide now explains
  Claude submission/clarification behavior and explicit Settings reconciliation,
  with limited-coverage/source-versus-installer bounds. The homepage's general
  onboarding and release claims remain unchanged and accurate.
