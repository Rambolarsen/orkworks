# Taskmaster Brain-guided Assessment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a manual, selected-workspace Taskmaster assessment that returns at most one evidence-backed workflow improvement, or a clear no-proposal result.

**Architecture:** Add a distinct assessment evaluator and report identity while reusing Taskmaster's selected provider, bounded context/evidence collection, signed knowledge bundle, installation-wide analysis lease, and existing `improve_workflow` recommendation lifecycle. Persist one bounded report per workspace, expose a narrow authenticated manual request and status, and gate action availability on the verified bundle containing the reviewed guidance delivered by #529.

**Tech Stack:** Rust, Axum, serde/JSON, existing Taskmaster provider and evidence services, Electron IPC, React/TypeScript, pnpm.

**Spec:** [Taskmaster contract](../../../specs/taskmaster.md#brain-guided-next-step-assessment), [Brain-informed Taskmaster recommendations](../../../specs/taskmaster-knowledge.md#manual-brain-guided-next-step-assessment-769), and [#769 design](../specs/2026-10-07-taskmaster-brain-guided-assessment-design.md).

## Global Constraints

- **Peons report facts; Taskmaster recommends transitions.**
- The user remains the authority and explicitly accepts or dismisses recommendations.
- Taskmaster uses a separate, explicitly selected provider/model and never changes or inherits Peon's selection.
- The eight-call daily limit is installation-wide and governs background discovery only; manual analyses bypass it and the one-hour workspace minimum interval.
- At most one analysis runs per instance; all manual and background analyses share the installation-wide single-analysis lease.
- Analysis context levels are session observations, workflow context, and relevant source code; excluded paths, ignored files, credential files, and files resolving outside the workspace remain excluded.
- Background collection is read-only and never runs repository scripts or commands. The assessment action also runs no commands, reads no additional terminal replay, and does not raise the configured context level.
- **Assess workflow** remains unavailable until #529 delivers the reviewed distilled assessment entry point and required general guidance in a verified signed bundle and generated starter snapshot.
- Brain guidance is reference data only. It cannot override repository instructions or user decisions, grant permissions, or add execution authority.
- Keep reports local, retain one latest report per workspace, cap each serialized report at 64 KiB, and do not store the full provider prompt or uncited workspace files.
- Reuse the existing Brain-derived `improve_workflow` identity (`proactive:v1:`), deduplication, dismissal, active-recommendation, acceptance, and **Fix with AI** lifecycle. Do not add another recommendation mutation or completion path.
- Keep provider-managed policies and effects authoritative; rejection of unexpected tool output does not undo provider-side effects.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `docs/adr/00xx-taskmaster-assessment-protocol.md` | Decide and record the durable report, authenticated request, status projection, and compatibility contract before runtime changes. |
| `docs/adr/README.md` | Index the accepted assessment protocol ADR. |
| `crates/orkworksd/src/taskmaster/assessment.rs` | Assessment prompt, typed output, citation/schema validation, proposal/no-proposal result, and assessment cache identity. |
| `crates/orkworksd/src/taskmaster/mod.rs` | Export assessment types and adapt a validated proposal through the existing recommendation path. |
| `crates/orkworksd/src/taskmaster/runtime.rs` | Persist bounded per-workspace assessment status/report and validate the required signed knowledge pages. |
| `crates/orkworksd/src/taskmaster/evaluator.rs` | Schedule a manual assessment under the existing analysis lease and revalidate live workspace/configuration/evidence before applying output. |
| `crates/orkworksd/src/http/taskmaster_handlers.rs` | Authenticated assess request and responses for unavailable, blocked, already-running, and scheduled outcomes. |
| `crates/orkworksd/src/http/taskmaster_settings_handlers.rs` | Expose read-only assessment status through the existing authorized Taskmaster status boundary. |
| `crates/orkworksd/src/main.rs` | Register the narrow assessment route. |
| `apps/desktop/electron/main.ts` | Generation-bound main-process request/status forwarding. |
| `apps/desktop/electron/preload.ts` | Expose the narrow assess/status IPC methods. |
| `apps/desktop/src/orkworksWindow.d.ts` | Type the Electron bridge methods. |
| `apps/desktop/src/api.ts` | Type assessment outcome and status payloads. |
| `apps/desktop/src/components/RecommendationsPanel.tsx` | Render the separate manual action, progress, provenance, outcome, and inline failures. |
| `apps/desktop/src/taskmasterSettings.ts` | Format typed assessment state without conflating it with Analyze now status. |
| `crates/orkworksd/src/taskmaster/assessment_tests.rs` | Prompt, validation, identity, and no-proposal fixtures. |
| Existing Taskmaster HTTP/evaluator/runtime tests and desktop component/API tests | Cover persistence, admission, stale-result fencing, bridge shape, and UI behavior. |

## Task 1: Record the protocol decision

**Files:** Create `docs/adr/00xx-taskmaster-assessment-protocol.md`; modify `docs/adr/README.md`.

**Interfaces:**
- Decide a versioned assessment result/status type distinct from ordinary analysis status.
- Decide the narrow authenticated `POST /taskmaster/assess-workflow` request and status projection over the existing Taskmaster run-status boundary.
- Store the report in the existing global Taskmaster ledger, keyed by canonical workspace, with one latest report per workspace and a 64 KiB serialized cap.
- Specify compatibility for ledgers without assessment fields, unreadable ledgers, interrupted runs, and deletion of local Taskmaster data.

- [ ] Write the ADR from the accepted spec; do not broaden it into new workspace deletion UX or a second provider/network path.
- [ ] Verify the record cap covers all stored fields and define deterministic rejection/truncation behavior before any runtime code uses the schema.
- [ ] Index the ADR and check its links and status.
- [ ] Obtain the required architecture decision review before proceeding to protocol implementation.

## Task 2: Build the distinct assessment evaluator

**Files:** Create `crates/orkworksd/src/taskmaster/assessment.rs` and its tests; modify `taskmaster/mod.rs` and `taskmaster/evaluator.rs`.

**Interfaces:**
- Input: immutable Taskmaster snapshot, canonical workspace identity, permitted observations, bounded repository facts, current recommendations, and selected relevant signed knowledge pages.
- Output: typed `Proposed { ... }` or `NoProposal { reason, ... }`, each with assessment input identity and validated fact/page references.
- A proposal contains exactly one next improvement and uses the existing recommendation identity/application functions.

- [ ] Add failing tests for one proposal, no proposal, multiple proposals, unknown fact/page IDs, unknown fields, executable command output, and unverified checks claimed as passed.
- [ ] Add prompt fixtures proving only relevant distilled knowledge pages are supplied and every input is marked untrusted reference data.
- [ ] Implement strict structured-output decoding; reject more than one proposed next step rather than applying a subset of multiple model proposals.
- [ ] Require at least one current repository fact for a proposal; a session-observations-only context may return no proposal but cannot assert a repository gap.
- [ ] Build cache identity from assessment prompt/version, effective settings, provider/harness identity, workspace generation, cited input evidence, and the full verified bundle identity; keep it separate from Analyze now.
- [ ] Run focused assessment evaluator tests and preserve the existing Analyze now output/application path.

## Task 3: Add durable assessment lifecycle and admission

**Files:** Modify `crates/orkworksd/src/taskmaster/runtime.rs` and `taskmaster/evaluator.rs`; add focused tests beside runtime/evaluator tests.

**Interfaces:**
- Runtime operations queue, mark running, read status, and finish an assessment report for one canonical workspace.
- Assessment and ordinary analysis share the existing analysis lease and do not overlap.
- Assessment status distinguishes queued/running from the latest succeeded/failed/interrupted outcome without changing ordinary analysis status.

- [ ] Add tests for a fresh report, replacement by a later report, the 64 KiB cap, legacy-ledger compatibility, unreadable-ledger refusal, and reopening persisted state.
- [ ] Add tests proving one report is keyed to one canonical workspace and local Taskmaster-data deletion removes it.
- [ ] Add tests for one active analysis/assessment only, active Brain recommendation blocking, active observation-only recommendations not blocking, and no background quota/cooldown consumption.
- [ ] Mark an admitted assessment running before collection/prompt construction; persist failure for errors after admission, and recover a persisted running attempt as interrupted under the existing lease discipline.
- [ ] Revalidate workspace instance, effective settings, provider/harness identity, bundle version, fact hashes, and page IDs immediately before report and recommendation writes.
- [ ] Discard stale results without changing a newer report, recommendation, or run outcome.
- [ ] Run focused runtime/evaluator tests, including existing analysis lease and stale identity tests.

## Task 4: Expose the authenticated sidecar contract

**Files:** Modify `crates/orkworksd/src/http/taskmaster_handlers.rs`, `http/taskmaster_settings_handlers.rs`, and `main.rs`.

**Interfaces:**
- `POST /taskmaster/assess-workflow` accepts no workspace or provider override from the renderer; sidecar state selects both.
- The existing Taskmaster status read includes a distinct assessment projection.
- Responses classify scheduled, unavailable, already-running, and active-recommendation outcomes and never expose uncited repository content.

- [ ] Add handler tests for authentication, no workspace, missing/old bundle, missing provider, active Brain recommendation, active analysis, and successful scheduling.
- [ ] Register the route and ensure assessment status reads remain read-only.
- [ ] Verify the renderer cannot submit a workspace path, provider, context override, arbitrary knowledge page, or report body.
- [ ] Run focused sidecar handler tests and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.

## Task 5: Add the Electron bridge and Recommendations UI

**Files:** Modify the listed Electron preload/main and desktop API/types/component files; update focused desktop tests.

**Interfaces:**
- Main-process IPC exposes `requestTaskmasterAssessment()` and typed status retrieval.
- Every request is bound to the current ready backend generation; a workspace switch aborts or rejects stale responses.
- Recommendations presents Assess workflow beside Analyze now, and displays assessment-specific progress, failure, outcome, evidence, page provenance, and unverified checks inline.

- [ ] Add tests for bridge payload validation, API response decoding, and generation changes during request/status calls.
- [ ] Add component tests for available, unavailable, no-proposal, proposed, running, failed, stale-workspace, and active-recommendation states.
- [ ] Keep Analyze now state/formatting independent; reuse the existing recommendation card and **Fix with AI** action for a proposal.
- [ ] Verify no background popup, focus change, or second approval vocabulary is added.
- [ ] Run targeted desktop tests and type-check from `apps/desktop/` using pnpm.

## Task 6: Verify bundle gating and end-to-end contract

**Files:** Modify or add test fixtures only where required; no Brain repository or publisher changes are part of this issue.

- [ ] Test current, old, offline, malformed, and missing-required-page bundles; the action is unavailable unless the required verified guidance is present.
- [ ] Verify a proposal reaches the existing `proactive:v1:` lifecycle and respects duplicate/dismissed/active recommendation state without duplicating mutations.
- [ ] Verify insufficient evidence, unsupported citations, changed facts/configuration/workspace, and unavailable provider produce no ungrounded recommendation.
- [ ] Verify report replacement, local-only persistence, deletion behavior, bounded size, and separation from Analyze now cache/report state.
- [ ] Run Rust tests, formatting, focused desktop tests/type-check, documentation link/build checks, and the repository-required `/code-review low` before code PR merge.
- [ ] Keep runtime availability explicitly gated until #529's reviewed distilled export, generated starter snapshot, and verified signed publication are complete; report any remaining #745 lifecycle alignment before implementation handoff.

## Planning checkpoint

- **Agent uncertainty investigated:** whether the existing evaluator could serve as the assessment implementation. It currently supports multiple proposals and has ordinary analysis cache/application identity. The plan therefore calls for a distinct evaluator and result identity, reusing only the provider/context/evidence/recommendation seams.
- **Project blind spot investigated:** a valid signature alone does not mean the active bundle has the assessment guidance. The sidecar and UI must check required reviewed page IDs/content role and stay unavailable for missing, old, or offline bundles.
- **Dependency facts:** #529 is open and its issue requires the strict privacy policy, generated starter snapshot, and verified signed publication. #503 is the accepted Taskmaster knowledge baseline. #745 is open; this feature reuses the existing `proactive:v1:` recommendation identity and must reconcile its active/dismissed lifecycle without waiting for unrelated configuration-history work.
- **Unresolved external gate:** runtime availability cannot be claimed until #529 is complete. A proposal also stays on the existing user-approved Fix with AI path; no assessment result grants execution permission.
