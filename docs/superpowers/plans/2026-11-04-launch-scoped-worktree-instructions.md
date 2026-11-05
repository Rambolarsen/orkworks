---
type: Implementation Plan
title: Launch-scoped worktree instruction delivery
description: Gated delivery of the shared worktree helper context to a launch-selected OpenCode root.
tags: [sessions, harnesses, git, worktrees, plan]
status: stable
workflow_status: implementing
---

# Launch-scoped worktree instruction delivery

Outcome: the eligible owning OpenCode root receives the shared helper context;
children/peers and uncertain profiles receive none. Tracking: [#1347](https://github.com/Rambolarsen/orkworks/issues/1347).
Contract: [approved design](../specs/2026-11-04-launch-scoped-worktree-instructions-design.md).
This is instruction delivery, not a native identity/attention fix. The owner approved the concrete protocol on 2026-11-05. Native activation
qualification remains separately gated.

## Boundaries and ownership

One implementer owns runtime preparation, reporter delivery and coupled tests.
Use an owned worktree. Preserve all existing native identity, attention, worktree
PATCH, source-priority, sandbox/proxy, installer ownership and schema contracts.
Do not launch another coding harness without a separately scoped owner exception.
Do not add a second Git resolver, durable root record, user-editable capability,
or generic registry layer for speculative future adapters.

Expected surfaces (confirm exact seams during implementation):

- `session_application.rs`: explicit new/exact/unscoped intent at launch/resume.
- `runtime/session_runtime.rs` and a focused private preparation helper: run-local
  intent/scope, qualified adapter selection, argv/env coherence and cancellation.
- `runtime/terminal_runtime.rs`: inherited-marker filtering and fresh env export.
- `harness/integrations/opencode.rs` and existing asset reconciliation: current
  owned reporter/helper prerequisite; no new Settings mutation route.
- `scripts/opencode-session-reporter.js` and its test file: independent root
  context checks, bounded helper rendering, no changes to event capture/attention.
- Authoritative docs and the existing qualification record: source/live distinction.

The key private contract is optional intent -> optional scope plus matched argv;
root_id is an instruction recipient selector, never observed native identity.
The application supplies intent; the closed adapter owns installed-version and
command-template qualification; the shared runtime owns generation/lifetime.

## Deliverables and checks

- [x] Record owner approval. Amend the session Git specification, harness
  integration concept and ADR 0094 before runtime code. Preserve the documented
  legacy identity-capture limitation instead of implying this proposal fixes it.
- [x] Add failing launch/runtime regressions before implementation. Cover a new
  root's argv/env equality; exact-resume unchanged argv; latest/custom/unsupported
  profiles unscoped; ambient markers stripped; missing/drifted assets; failed
  randomness; cancellation/generation changes during version/asset preparation.
  Compare full installed bytes against bundled assets initially and again without
  an intervening await before spawn; drift discards scope and its argv adjustment.
  Implement the smallest private scope preparation seam needed by those cases.
- [x] Add failing real-reporter subprocess/context tests. Root context must
  receive actual helper-generated narrow text; child, peer, wrong/missing API
  parent/ID and misleading plugin cwd must not. Reorder legacy capture events
  without changing instruction eligibility. Exercise cleanup, delayed reads,
  one-outstanding-lookup bound, helper timeout/output bounds and invalid JSON.
  Check the helper digest immediately before execution; replacement after spawn
  must suppress delivery. Keep the documented cooperative loader-race boundary.
  Assert context delivery generates no identity or attention HTTP effects and
  preserves tools/settings. Implement the bounded context hook to pass them.
- [x] Verify normal owned install/probe/reconcile/uninstall and helper asset
  drift behavior, without changing foreign files or running-session env. Verify
  API-only/general metadata and other harness instruction channels are unchanged.
- [ ] Run scoped suites and `bash scripts/verify-repo.sh`; complete current-code
  `/code-review low`. Prepare one concrete actual-source zero-inference native
  fixture, obtain its separate bounded run exception, and record root/child/peer
  facts and zero transport. Activation stays disabled until this gate passes.
- [ ] Record qualified platform/version and source-vs-installed limits. Follow
  the branch/PR/comment/check workflow through terminal state; use guarded cleanup.

## Review focus and failure treatment

A legacy child capture, peer root created later, or missing native ID must not
become an instruction selector. Exact-resume existence is not root proof; context
parent validation remains mandatory, and resume availability is unchanged.
Latest/custom/native TUI root switches are not automatically adopted. Late helper
or API work after teardown cannot inject into another run. Read-only operations
elsewhere never update the explicit Git binding. Do not claim universal harness
or Windows support from macOS arm64 2.2.5 evidence.

Treat unavailable eligibility as no new instruction assistance; preserve existing
coding-tool operation. Treat cancellation or eligible-launch randomness failure
as startup failure before spawn, without changing the selected root or retrying
under another profile. Native verification failure blocks activation, not the
creation of a fallback location or an identity/attention update.

## Plan review

Disposition: Owner-approved for implementation on 2026-11-05. Scope is bounded to one
instruction channel and its run-local selector. The failed capture audit is
resolved by not depending on that mutable state; identity-store fencing and
resume-root migrations are outside this delivery and remain separately tracked.

Risk assessment: dependencies 3 (native hooks/CLI and current assets), blast
radius 2 (qualified OpenCode argv/context only), state changes 1 (run-local only),
reversibility 2 (disable preparation; ordinary native IDs remain valid),
uncertainty 3 (actual changed reporter qualification still required). Total 11/25
is a review aid, not implementation or activation approval. Native Windows and
other versions remain off. Review revisions only for material contract changes.
