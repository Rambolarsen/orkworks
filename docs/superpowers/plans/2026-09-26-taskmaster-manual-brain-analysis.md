# Taskmaster Manual Brain Analysis Implementation Plan

> **For agentic workers:** Use inline execution with the repository's scoped instructions and the approved design below.

**Goal:** Let users request a fresh Brain-backed Taskmaster analysis from Recommendations while preventing a second analysis when an improvement is already outstanding.

**Architecture:** Add an authenticated sidecar action that runs the existing evaluator immediately with a manual trigger mode. The manual mode bypasses only the per-workspace cooldown; the durable installation-wide daily ledger and evidence cache remain authoritative. Before scheduling and again before inference, block when an `improve_workflow` recommendation is proposed, accepted, or executing. The renderer offers the action, explains the active recommendation, and routes implementation through the existing Fix with AI flow.

**Tech Stack:** Axum/Rust sidecar, React/TypeScript renderer, existing Taskmaster runtime and recommendation store.

**Spec:** `specs/taskmaster-knowledge.md` and `specs/taskmaster.md` (updated for this approved behavior).

## Global Constraints

- At most one Taskmaster evaluation runs per OrkWorks instance.
- Manual analyses still consume the installation-wide daily evaluation allowance of eight.
- A manual analysis bypasses only the per-workspace one-hour minimum interval.
- Existing evidence caching, context exclusions, provider trust, workspace identity, and recommendation lifecycle rules remain in force.
- Taskmaster never edits files, types into terminals, or starts a session; Fix with AI remains an explicit user handoff.
- An active Brain recommendation is an `improve_workflow` item in `proposed`, `accepted`, or `executing` status.

## Planning Checkpoint

**Least confident:** Whether the manual action could bypass the one-hour interval without bypassing other evaluator safeguards. Inspection of `TaskmasterRuntime::reserve_snapshot` confirms the evidence cache, daily reservation, and workspace interval checks are separate; add an explicit manual mode that skips only the interval comparison.

**Biggest missing concern:** The existing Recommendations list omits accepted and executing non-rollup items from its actionable display. A renderer-only guard would therefore permit a duplicate run. The sidecar must enforce the active-recommendation gate and return the item so the desktop can explain that it must be handled first. The evaluator already has a process-wide single-flight guard and a durable analysis lease; reuse both.

---

### Task 1: Update the authoritative Taskmaster contract

**Files:**
- Modify: `specs/taskmaster-knowledge.md`
- Modify: `specs/taskmaster.md`
- Track: GitHub issue #503

- [ ] Document the manual trigger, cooldown bypass, daily reservation, cache behavior, and active-recommendation gate in both relevant specifications.
- [ ] Add the manual API action and Recommendations-panel control to the Taskmaster API and Phase 3 sections.
- [ ] Synchronize issue #503 with the approved acceptance criteria before implementation.

### Task 2: Add the manual sidecar action

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/runtime.rs`
- Modify: `crates/orkworksd/src/taskmaster/runtime/inference.rs`
- Modify: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`

- [ ] Add a manual reservation path that retains daily-limit and cache checks while skipping only the per-workspace interval check.
- [ ] Route manual requests through the existing evaluator immediately, preserving the requested workspace identity, single-flight guard, and analysis lease.
- [ ] Reject a manual run and return the active Brain recommendation when one is proposed, accepted, or executing; repeat the guard immediately before provider invocation.
- [ ] Return an explicit outcome for scheduled, active-recommendation, already-running, unavailable-ledger, and daily-limit cases.

### Task 3: Add the Recommendations-panel action

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Modify: `apps/desktop/src/App.css`

- [ ] Add a typed request helper for `POST /taskmaster/analyze`.
- [ ] Add an **Analyze now** button with in-flight feedback and clear messages for daily limits or an analysis already running.
- [ ] When the sidecar returns an active Brain recommendation, refresh the list, direct the user to its card, and offer the existing Fix with AI handoff rather than starting analysis.
- [ ] Keep the action disabled without a ready workspace/sidecar and preserve generation guards across workspace switches.

### Task 4: Review the complete change

**Files:** all files above.

- [ ] Review the final diff against both updated specifications and the approved design.
- [ ] Run `git diff --check`, the repository documentation check, and the desktop TypeScript check; report any checks not run.
