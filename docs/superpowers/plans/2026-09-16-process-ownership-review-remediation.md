# Process Ownership Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the actionable review findings on PR #568 while preserving the proof-only scope and keeping unproven native/platform behavior explicitly unresolved.

**Architecture:** Strengthen the fixture protocol and evidence contract rather than adding production runtime ownership. Supervisor-issued executable identity and authenticated, retry-safe adoption remain the authority; platform-specific tests prove only the platforms they actually execute on.

**Tech Stack:** Rust fixture crate, serde/JSON protocol tests, Markdown design/evidence records, Cargo test, GitHub PR review threads.

**Spec:** `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`

## Global Constraints

- The fixture does not implement multi-workspace recovery, production runtime replacement, or the production supervisor.
- Persisted PIDs, executable names, working directories, and released metadata leases are never ownership authority.
- Adoption succeeds only after a live authenticated response proves a generation-bound complete-exit receipt; unavailable, stale, malformed, or ambiguous state remains unresolved.
- Windows native claims require independent identity/containment observation and forced parent termination; macOS/Linux gaps remain unsupported or unresolved when the native runtime is unavailable.
- Review comments must be answered in their inline threads with evidence or a precise parked-status explanation.

---

### Task 1: Make launch identity and adoption retry semantics coherent

**Files:**
- Modify: `crates/process-ownership-fixture/src/protocol.rs`
- Modify: `crates/process-ownership-fixture/src/supervisor.rs`
- Test: `crates/process-ownership-fixture/tests/protocol.rs`
- Test: `crates/process-ownership-fixture/tests/cleanup_matrix.rs`
- Modify: `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`

**Interfaces:**
- `SupervisorProtocol::issue_launch_ticket` derives executable identity from the trusted role/image binding; callers supply role and request nonce, not authority-bearing identity.
- Adoption responses remain challenge-bound and authenticated. Repeating the same successor-bound adoption request after a lost response returns the same complete-exit receipt without allowing a different successor or generation to consume it.

- [ ] **Step 1: Write failing tests** for supervisor-derived executable identity and retrying the same successor-bound adoption request after a simulated lost response.
- [ ] **Step 2: Run the focused protocol and cleanup tests** and verify the new tests fail for the current caller-selected identity / one-use challenge behavior.
- [ ] **Step 3: Implement the smallest protocol change**: remove caller authority over executable identity, add an explicit successor binding to adoption challenges/responses, and make the exact successor retry idempotent while rejecting a different successor.
- [ ] **Step 4: Run the focused tests** and the full fixture suite; verify all existing ticket, receipt, stale-generation, and replay tests remain green.
- [ ] **Step 5: Update the design spec** so the protocol signature and adoption retry semantics match the implementation.
- [ ] **Step 6: Commit** with `fix: tighten process ownership adoption protocol`.

### Task 2: Split cleanup escalation success from unresolved termination failure

**Files:**
- Modify: `crates/process-ownership-fixture/src/supervisor.rs`
- Test: `crates/process-ownership-fixture/tests/cleanup_matrix.rs`
- Modify: `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`
- Modify: `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`

**Interfaces:**
- A graceful-resistant but force-terminable child produces an acknowledged receipt after escalation.
- A separately injected termination failure produces unresolved survivors; the matrix does not require ordinary force termination to fail.

- [ ] **Step 1: Write failing tests** for force-terminable escalation acknowledgement and injected termination failure unresolved status.
- [ ] **Step 2: Run the focused cleanup tests** and verify the new expectations fail against the current single unresolved-survivor case.
- [ ] **Step 3: Implement the minimal fixture behavior and test hooks** needed to distinguish force escalation success from an injected termination failure.
- [ ] **Step 4: Run cleanup tests and the full fixture suite**; confirm bounded phase timing and receipt validation remain intact.
- [ ] **Step 5: Rewrite the matrix row and evidence wording** to describe both outcomes precisely.
- [ ] **Step 6: Commit** with `fix: distinguish process cleanup escalation outcomes`.

### Task 3: Close the platform/evidence review loop

**Files:**
- Modify: `crates/process-ownership-fixture/tests/windows_job.rs`
- Modify: `crates/process-ownership-fixture/src/platform/windows.rs`
- Modify: `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`
- Modify: `.superpowers/sdd/2026-09-15-process-ownership-proof/task-6-report.md`

**Interfaces:**
- Windows fixture coverage explicitly exercises supervisor-ticketed roots, inherited-endpoint resistance, and registration failure.
- Production seam audit remains an inventory/gap record; it does not claim that the fixture routes real PTY, inference, discovery, or harness launches.

- [ ] **Step 1: Add a failing Windows fixture assertion** that the forced-parent target was admitted through the supervisor ticket and that endpoint inheritance is rejected before target release.
- [ ] **Step 2: Run the Windows-specific test locally where supported or record the exact hosted command as unavailable locally.**
- [ ] **Step 3: Implement only the fixture-side assertion/diagnostic needed to make the admission path observable; do not modify production launch seams.**
- [ ] **Step 4: Run the Unix and Windows fixture tests available in the current environment.**
- [ ] **Step 5: Update evidence and Task 6 audit language** to mark the cross-platform endpoint and actual production seam findings as addressed-by-audit or unsupported, not passed.
- [ ] **Step 6: Commit** with `docs: reconcile process ownership review evidence`.

### Task 4: Review, respond, and verify

**Files:**
- Modify: PR #568 inline review threads through GitHub API
- Modify: `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md` if verification changes counts or refs

- [ ] **Step 1: Run verification-before-completion checks:** fixture tests, evidence validator, `git diff --check`, and relevant Rust formatting.
- [ ] **Step 2: Dispatch a fresh whole-branch reviewer** against the review-remediation diff.
- [ ] **Step 3: Reply in each inline thread** with the exact commit/evidence reference, marking fixed items fixed and platform-limited items explicitly parked.
- [ ] **Step 4: Push the branch and confirm PR checks.**
- [ ] **Step 5: Leave issue #545 open** until native Linux/macOS mechanism proof, production seam routing, and the forced A/B relaunch proof are complete.
