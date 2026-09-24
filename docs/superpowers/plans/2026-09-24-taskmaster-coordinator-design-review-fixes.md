# Taskmaster Coordinator Design Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the outstanding design-review findings for the bounded Taskmaster coordinator without authorizing coordinator runtime implementation.

**Architecture:** Strengthen the design contract across the coordinator design spec, Taskmaster authority section, ADR 0063, and user-facing architecture summary. The contract will make execution policy, resource/revision binding, bounded evidence, and recovery semantics explicit while retaining the separate design gate and implementation-plan boundary.

**Tech Stack:** Markdown, repository documentation checks, GitHub PR review threads.

**Spec:** `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`

## Global Constraints

- Coordinator code, child APIs, and runtime launch paths remain prohibited until the design gate and a separate implementation plan are approved.
- The existing Taskmaster v1 explicit-approval, single-active-context, user-escalation, and completion-packet rules remain unchanged.
- Every capability, command, report, and result must be server-validated, revision-bound, and fail closed on ambiguity.
- Documentation links must remain valid and the repository documentation checks must pass.

### Task 1: Define enforceable execution and resource authority

**Files:**
- Modify: `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`
- Modify: `specs/taskmaster.md`
- Modify: `docs/adr/0063-bounded-taskmaster-coordinator.md`

**Interfaces:**
- The design spec is authoritative for the future coordinator contract.
- `specs/taskmaster.md` and ADR 0063 summarize the same boundary without authorizing implementation.

- [ ] **Step 1: Add the tool-broker contract.** State that every child command/tool invocation crosses a server-owned broker that checks executable, arguments, cwd, environment, declared resource effects, lease, capability revision, and hard denials; prohibit direct shell/process access and make child prose non-authoritative.
- [ ] **Step 2: Bind grants to the target node.** Require every runtime grant to be a subset of the active parent lease, the target node’s pre-approved maximum envelope, and an unconsumed slot.
- [ ] **Step 3: Make resource ceilings mandatory.** Add required per-attempt wall-clock, CPU, memory, process-count, output-size, token/cost, and tool-invocation ceilings.
- [ ] **Step 4: Make launch ambiguity fail closed.** State that lost/uncertain launch responses retain the reservation and become orphaned until explicit reconciliation proves whether work exists; no refund or retry is allowed before that point.
- [ ] **Step 5: Make concurrent scope policy explicit.** Require all write/write and write/read overlaps to be rejected unless an approved exclusive resource lease serializes them.

### Task 2: Define lifecycle, workspace, and evidence invariants

**Files:**
- Modify: `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`
- Modify: `specs/taskmaster.md`
- Modify: `docs/adr/0063-bounded-taskmaster-coordinator.md`

- [ ] **Step 1: Add required-node semantics.** Add a required/optional or deterministic activation field to each immutable node and require completion to evaluate only the declared required set.
- [ ] **Step 2: Define pause/resume.** Add `paused -> active` only through fresh user approval bound to the same immutable revision, current workspace subject, expiry, and revocation generation; scope or budget changes require a new revision.
- [ ] **Step 3: Bind attempts to workspace revisions.** Require server-observed input and output workspace revisions/change subjects on every attempt and atomically reject a result when the observed state is stale or conflicting.
- [ ] **Step 4: Define cancellation completion.** Require bounded process-tree termination and tool-channel revocation; if termination cannot be proven, classify the work as orphaned and do not refund or relaunch it.
- [ ] **Step 5: Require server-attested evidence.** Define broker/verifier receipts, observed command identity/result, and scope-bound output hashes as the only evidence capable of advancing graph state; child-authored claims remain context only.

### Task 3: Bound prompts, payloads, and retained lineage

**Files:**
- Modify: `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`
- Modify: `specs/taskmaster.md`
- Modify: `docs/adr/0063-bounded-taskmaster-coordinator.md`

- [ ] **Step 1: Bind effective prompt/context.** Include the server-rendered prompt/context template, derivation inputs, and resulting digest in the immutable revision; parent edits require a new revision and approval.
- [ ] **Step 2: Add hard portable limits.** Specify finite maximums for graph nodes/depth, text fields, prompt/context, command requests, evidence items/bytes, reports, and retained audit/lineage records; reject over-limit requests before mutation.
- [ ] **Step 3: Add bounded audit eviction.** Preserve immutable plan/approval/lineage digests while redacting and tombstoning evicted evidence/audit records.

### Task 4: Update naming and project documentation

**Files:**
- Modify: `specs/taskmaster.md`
- Modify: `README.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/adr/0063-bounded-taskmaster-coordinator.md`

- [ ] **Step 1: Rename the coordinator design gate.** Use `Coordinator design gate` consistently and reserve `Phase 2` for the existing deterministic evaluator rollout.
- [ ] **Step 2: Update the README architecture summary.** Explain that coordinator work is proposed, separately gated, and not implemented or authorized by the design ADR.
- [ ] **Step 3: Keep the ADR index status and cross-links accurate.** Point readers to the proposed design gate and preserve the historical status.

### Task 5: Verify and report the review fixes

**Files:**
- Verify: all modified Markdown files and repository documentation checks.

- [ ] **Step 1: Self-review the design.** Search for stale `Phase 2` coordinator references, contradictory authority claims, unbounded payload language, and missing review requirements.
- [ ] **Step 2: Run documentation verification.** Run `bash scripts/doc-check.sh` and `git diff --check`.
- [ ] **Step 3: Review the complete diff.** Confirm no runtime code, child API, or launch path was added and all 14 review findings are addressed or explicitly answered.
- [ ] **Step 4: Commit the documentation changes.** Use a focused commit message describing the hardened coordinator design contract.
- [ ] **Step 5: Reply in each GitHub review thread.** Reference the exact sections changed and request a fresh review of the new head.
