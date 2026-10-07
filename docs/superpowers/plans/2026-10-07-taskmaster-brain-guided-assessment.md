# Taskmaster Brain-guided Assessment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a manual, selected-workspace Taskmaster assessment that returns at most one evidence-backed workflow improvement, or a clear no-proposal result.

**Architecture:** Add a distinct assessment evaluator and report identity while reusing Taskmaster's selected provider, bounded context/evidence collection, signed knowledge bundle, installation-wide analysis lease, and existing `improve_workflow` recommendation lifecycle. Persist one bounded report per workspace, expose a narrow authenticated manual request and status, and gate all Brain-backed inference on an eligible verified bundle delivered by #529.

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
- All Brain-backed provider analysis, including background discovery, **Analyze now**, and **Assess workflow**, remains unavailable until the active verified bundle carries a supported privacy policy version and meets the strict exclusion policy; #529 delivers the reviewed export, compliant starter snapshot, and signed publication. Deterministic observation recommendations continue during this gate.
- A compatible eligible cached bundle remains usable offline; network availability does not gate analysis.
- Bundle age alone does not make an eligible verified offline bundle unavailable. Eligibility requires signed `privacyPolicyVersion: 1`, a valid signature and digest, compatible format, required reviewed pages, and strict privacy exclusions; there is no separate wall-clock freshness cutoff.
- Brain guidance is reference data only. It cannot override repository instructions or user decisions, grant permissions, or add execution authority.
- Keep reports under the selected workspace's metadata root, retain one latest report per workspace, cap each serialized report at 64 KiB, and do not store the full provider prompt or uncited workspace files. Workspace metadata deletion removes its report. Never persist repository evidence in the global Taskmaster ledger.
- The cache key and input identity hash the complete bounded request: permitted observations, every collected repository fact including uncited facts, current recommendation snapshot, selected pages, context settings/exclusions, workspace generation, prompt/schema version, provider/model, and full bundle identity. Any input change invalidates and deletes the report, including no-proposal results.
- Any workspace/evidence identity mismatch, effective access reduction, new exclusion, provider/harness change, bundle change, or cited-page mismatch invalidates and deletes the stored report. Status reads revalidate identity and delete stale snapshots before returning a report.
- Invalidation supersedes a still-proposed assessment-derived recommendation, removes it from active recommendations, and prevents **Fix with AI** from using stale evidence. Redact invalid snapshots from the superseded record and accepted/executing/completed records while preserving lifecycle and outcome history.
- Implementation tasks are gated on written-spec review and explicit implementation authorization. #529 separately gates runtime availability; its completion is not implied by approval of implementation work.
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
| `crates/orkworksd/src/taskmaster/runtime.rs` | Persist bounded per-workspace assessment status/report and validate bundle integrity, supported privacy policy version, and required signed knowledge pages. |
| `crates/orkworksd/src/taskmaster/evaluator.rs` | Gate all Brain-backed inference on bundle eligibility; schedule assessment under the shared analysis lease and revalidate complete live input identity before applying output. |
| `crates/orkworksd/src/http/taskmaster_handlers.rs` | Apply the bundle eligibility gate to Analyze now and expose authenticated assessment outcomes for unavailable, blocked, already-running, and scheduled requests. |
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

## Required gates before implementation

1. Obtain written-spec review and record acceptance of the proposed assessment contract.
2. Obtain explicit authorization to begin implementation and approval of the protocol ADR approach.
3. Recheck live dependencies, including #529's Brain-inference eligibility prerequisite and #745's recommendation-lifecycle alignment.
4. Only then begin Tasks 1–6. Before #529 supplies an eligible bundle, all Brain-backed inference must fail closed.

## Task 1: Record the protocol decision

**Files:** Create `docs/adr/00xx-taskmaster-assessment-protocol.md`; modify `docs/adr/README.md`.

**Interfaces:**
- Decide a versioned assessment result/status type distinct from ordinary analysis status.
- Record the signed bundle `privacyPolicyVersion` eligibility contract from the knowledge spec.
- Record the spec's narrow authenticated `POST /taskmaster/assess-workflow` request and distinct read-only `GET /taskmaster/run-status` assessment projection.
- Store the report under the canonical workspace metadata root, with one latest report per workspace and a 64 KiB serialized cap; workspace metadata deletion removes it.
- Specify compatibility for workspace metadata without assessment fields, unreadable metadata, interrupted runs, and invalidation when stored input identity or effective access changes.

- [ ] Write the ADR from the accepted spec; do not broaden it into new workspace deletion UX or a second provider/network path.
- [ ] Verify the record cap covers all stored fields and define deterministic rejection/truncation behavior before any runtime code uses the schema.
- [ ] Index the ADR and check its links and status.
- [ ] Obtain the required architecture decision review before proceeding to protocol implementation.
- [ ] Do not begin this task until written-spec acceptance and explicit implementation authorization are recorded.

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
- [ ] Build cache identity from the complete bounded input snapshot, including uncited facts and current recommendations, plus prompt/schema version, effective settings, provider/harness identity, workspace generation, selected pages, and full bundle identity; keep it separate from Analyze now.
- [ ] Prove changes to an uncited fact, recommendation snapshot, or any other supplied input invalidate proposal and no-proposal cache entries.
- [ ] Run focused assessment evaluator tests; preserve the existing recommendation application path while refusing Brain-backed inference when the bundle is ineligible.

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
- [ ] Invalidate the stored report when effective context narrows or an exclusion is added; status reads revalidate identity and delete stale reports before returning them.
- [ ] Supersede proposed recommendations derived from invalidated reports and redact stale excerpts from other lifecycle states while preserving transition/outcome history; ensure stale evidence cannot reach **Fix with AI**.
- [ ] Discard stale results without changing a newer report, recommendation, or run outcome.
- [ ] Run focused runtime/evaluator tests, including existing analysis lease and stale identity tests.

## Task 4: Expose the authenticated sidecar contract

**Files:** Modify `crates/orkworksd/src/http/taskmaster_handlers.rs`, `http/taskmaster_settings_handlers.rs`, and `main.rs`.

**Interfaces:**
- `POST /taskmaster/assess-workflow` accepts no workspace or provider override from the renderer; sidecar state selects both.
- The existing Taskmaster status read includes a distinct assessment projection.
- Responses classify scheduled, unavailable, already-running, and active-recommendation outcomes and never expose uncited repository content.

- [ ] Add handler/evaluator tests for authentication, no workspace, missing policy version, ineligible legacy bundle, missing required pages/provider, active Brain recommendation, active analysis, and successful scheduling. Cover background and Analyze now gates as well as assessment.
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
- [ ] Add component tests for the ineligible-bundle gate, available, unavailable, no-proposal, proposed, running, failed, stale-workspace, and active-recommendation states; deterministic observation recommendations remain visible while Brain inference is gated.
- [ ] Keep Analyze now state/formatting independent; reuse the existing recommendation card and **Fix with AI** action for a proposal.
- [ ] Verify no background popup, focus change, or second approval vocabulary is added.
- [ ] Run targeted desktop tests and type-check from `apps/desktop/` using pnpm.

## Task 6: Verify bundle gating and end-to-end contract

**Files:** Modify or add test fixtures only where required; no Brain repository or publisher changes are part of this issue.

- [ ] Test eligible online and offline cached bundles, old-timestamp bundles, missing/unsupported `privacyPolicyVersion: 1`, malformed/unverified bundles, missing required pages, and bundles failing the exclusion policy. An eligible signed offline bundle remains usable regardless of age; an ineligible bundle blocks all Brain-backed provider analysis.
- [ ] Verify deterministic observation recommendations continue while Brain-backed provider analysis is gated, and verify Analyze now/background analysis resume only with an eligible bundle.
- [ ] Verify a proposal reaches the existing `proactive:v1:` lifecycle and respects duplicate/dismissed/active recommendation state without duplicating mutations.
- [ ] Verify insufficient evidence, unsupported citations, changed cited or uncited facts, changed recommendation inputs/configuration/workspace, and unavailable provider produce no ungrounded or cached-stale recommendation.
- [ ] Verify report replacement, local-only persistence, workspace metadata deletion, access-narrowing invalidation on status reads, proposed-recommendation supersession, snapshot redaction on superseded and other audit records, bounded size, and separation from Analyze now cache/report state.
- [ ] Run Rust tests, formatting, focused desktop tests/type-check, documentation link/build checks, and the repository-required `/code-review low` before code PR merge.
- [ ] Keep all Brain-backed inference explicitly gated until #529's reviewed distilled export, supported privacy policy version, generated starter snapshot, and verified signed publication are complete; report any remaining #745 lifecycle alignment before implementation handoff.

## Planning checkpoint

- **Agent uncertainty investigated:** whether the existing evaluator could serve as the assessment implementation. It currently supports multiple proposals and has ordinary analysis cache/application identity. The plan therefore calls for a distinct evaluator and result identity, reusing only the provider/context/evidence/recommendation seams.
- **Project blind spot investigated:** a valid signature alone does not mean the active bundle obeys the privacy policy or contains assessment guidance. The sidecar must check both; eligible cached guidance remains usable offline with no independent age cutoff, while unreviewed legacy, malformed, or incomplete bundles block Brain-backed inference.
- **Dependency facts:** #529 is open and its issue requires the strict privacy policy, generated starter snapshot, and verified signed publication. #503 is the accepted Taskmaster knowledge baseline. #745 is open; this feature reuses the existing `proactive:v1:` recommendation identity and must reconcile its active/dismissed lifecycle without waiting for unrelated configuration-history work.
- **Unresolved external gate:** Brain-backed inference cannot resume until #529 supplies an eligible signed bundle. A proposal also stays on the existing user-approved Fix with AI path; no assessment result grants execution permission.
