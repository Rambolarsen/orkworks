---
type: "Implementation Plan"
title: "Taskmaster Brain-guided Assessment Implementation Plan"
description: "Implementation plan: Taskmaster Brain-guided Assessment Implementation Plan."
tags: ["orkworks", "plans"]
---

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
- All Brain-backed provider analysis, including background discovery, **Analyze now**, and **Assess workflow**, remains unavailable until the active verified signed bundle payload carries `privacyPolicyVersion: 1`, the manifest policy version matches the payload, and the strict exclusion policy passes. **Assess workflow** additionally requires signed `taskmaster-assessment-v1` capability. #529 delivers the reviewed export, a signed starter envelope with matching signed manifest attestation, and signed publication. Deterministic observation recommendations continue during this gate.
- A compatible eligible cached bundle remains usable offline; network availability does not gate analysis.
- Bundle age alone does not make an eligible verified offline bundle unavailable. Eligibility for all Brain-backed inference requires signed payload `privacyPolicyVersion: 1`, matching signed-manifest policy version, a valid signature and digest, compatible format, and strict privacy exclusions; Assess workflow also requires signed `taskmaster-assessment-v1` capability. The packaged starter uses the same verification path and pinned publisher key with a bundled signed manifest attestation; no unsigned starter exception is allowed. There is no separate wall-clock freshness cutoff.
- Brain guidance is reference data only. It cannot override repository instructions or user decisions, grant permissions, or add execution authority.
- Keep reports under the selected workspace's metadata root, retain one latest report per workspace, cap each serialized report at 64 KiB, and do not store the full provider prompt or uncited workspace files. Workspace metadata deletion removes its report. Never persist repository evidence in the global Taskmaster ledger.
- The cache key and input identity hash the complete bounded request: permitted observations, every collected repository fact including uncited facts, the pre-run recommendation snapshot, selected pages, context settings/exclusions, workspace generation, prompt/schema version, provider/model, and full bundle identity. If this assessment creates a recommendation, store its stable ID and exclude only that assessment-created record when revalidating the recommendation snapshot. If ordinary deduplication suppresses the proposal, store no derived recommendation ID and keep the existing record in the snapshot. A successor exception applies only to an assessment-derived predecessor terminally superseded by input invalidation; all other input changes logically invalidate the report, including no-proposal results.
- Any workspace/evidence identity mismatch, effective access reduction, new exclusion, provider/harness change, bundle change, or cited-page mismatch logically invalidates the stored report. The read-only status route revalidates identity and omits stale snapshots without changing state. Before returning or using a proposal, revalidate it on every recommendation list/get, active-recommendation response, acceptance, and **Fix with AI** path; reject stale use even if no status poll occurred. Access-setting mutations redact disallowed evidence before replying; other stale reports are removed on the next state-changing assessment or workspace cleanup.
- Any input invalidation supersedes a still-proposed assessment-derived recommendation, removes it from active recommendations, and prevents **Fix with AI** from using stale evidence. For this assessment-derived subset only, `superseded` is terminal and valid only after `proposed`; it cannot later be accepted or executed. A later valid assessment may create one linked successor only when the same dedupe key belongs to an assessment-derived record superseded by input invalidation; bind the successor ID to the dedupe key, predecessor, and unique assessment ID, and set `supersedesRecommendationId`. Other existing lifecycle states continue to suppress duplicate proposals. On effective-access narrowing, redact snapshots and all model-generated display fields from every lifecycle record retaining them, including dismissed records, while preserving identity, transitions, dismissal decisions, and outcomes. Replace required prose with a generic access-redacted placeholder. Other input changes do not redact audit snapshots.
- Global context-level or exclusion reductions compare old and new effective settings for every known workspace and redact all affected workspace-local reports and recommendations before returning the settings update. Workspace overrides affect only their workspace. Reconcile unopened workspace stores before any later status, recommendation, acceptance, or Fix with AI response can expose or use their data; test inherited defaults and unaffected explicit overrides across multiple workspaces.
- A novel proposal uses a crash-recoverable write in the single capped workspace assessment record: persist the validated report with an internal `pending_recommendation` marker and stable recommendation ID, durably upsert the existing recommendation, then atomically mark the report committed. Reconcile pending mutations idempotently under the analysis lease before the sidecar is ready to expose report/status. If the final report commit fails while the sidecar remains live, reconcile immediately; until successful, read-only status and recommendation list/get must omit the pending result or return unavailable without writing, and active-response/accept/Fix with AI must not expose or act on it. If reconciliation still fails, return unavailable/service failure and do not begin another provider assessment; a pending record does not count as an active recommendation. If existing deduplication suppresses the proposal, return a no-proposal result and do not stage a recommendation write; other no-proposal results need only an atomic report write. Proposals require at least one current repository fact and one relevant selected Brain page; no-proposal results may cite no Brain pages.
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
| `crates/orkworksd/src/taskmaster/runtime.rs` | Persist bounded per-workspace assessment status/report and validate bundle integrity, signed policy/capability fields, and crash-recovery state. |
| `apps/desktop/electron/knowledgeUpdates.ts` | Validate and retain the signed bundle payload's policy and assessment-capability markers across cache/restart. |
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
3. Recheck live dependencies, including #529's Brain-inference eligibility prerequisite and #745's recommendation-lifecycle alignment. The signed bundle payload from #529 must carry supported `privacyPolicyVersion` and `taskmaster-assessment-v1` capability fields, with the manifest policy value matching the payload. Verify its tracked acceptance criteria cover these fields and the signed packaged-starter envelope/manifest attestation before implementation, and resolve any gap in #529 first.
4. Only then begin Tasks 1–6. Before #529 supplies an eligible bundle, all Brain-backed inference must fail closed.

## Task 1: Record the protocol decision

**Files:** Create `docs/adr/00xx-taskmaster-assessment-protocol.md`; modify `docs/adr/README.md`.

**Interfaces:**
- Decide a versioned assessment result/status type distinct from ordinary analysis status.
- Record signed-payload `privacyPolicyVersion`, matching-manifest, and `taskmaster-assessment-v1` capability requirements from the knowledge spec.
- Record the spec's narrow authenticated `POST /taskmaster/assess-workflow` request and distinct read-only `GET /taskmaster/run-status` assessment projection.
- Store the report under the canonical workspace metadata root, with one latest report per workspace and a 64 KiB serialized cap; workspace metadata deletion removes it.
- Specify proposal transaction recovery using an internal pending marker in the single capped workspace assessment record, stable recommendation ID, idempotent recommendation upsert, committed report state, and reconciliation before exposing a report/status.
- Specify compatibility for workspace metadata without assessment fields, unreadable metadata, interrupted runs, read-only stale status projections, and invalidation when stored input identity or effective access changes.

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
- [ ] Require at least one current repository fact and at least one relevant selected Brain page for a proposal; a session-observations-only context may return no proposal but cannot assert a repository gap, and a no-proposal output may have no Brain-page citations.
- [ ] Build cache identity from the complete bounded input snapshot, including uncited facts and the pre-run recommendation snapshot, plus prompt/schema version, effective settings, provider/harness identity, workspace generation, selected pages, and full bundle identity; when this run creates a recommendation, record and exclude only its stable ID during revalidation; keep it separate from Analyze now.
- [ ] Prove changes to an uncited fact, any recommendation other than this assessment's own created recommendation, or any other supplied input invalidate proposal and no-proposal cache entries; prove the created proposal itself does not invalidate its report, and dedupe suppression leaves the existing record in the snapshot with no derived ID.
- [ ] Reject a proposed result with no Brain-page citations or citations to pages outside the selected eligible bundle; allow no-proposal output without page citations.
- [ ] When a proposal's `proactive:v1:` dedupe key exists in an ordinary lifecycle state, return a duplicate-suppression no-proposal result without changing or relinking it. Allow one idempotent linked successor only when the matching assessment-derived record is terminally superseded by input invalidation; bind its stable ID to the predecessor and unique assessment ID, and test repeated attempts.
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
- [ ] Mark an admitted assessment running before collection/prompt construction; persist failure for errors after admission. During recovery under the analysis lease, reconcile a pending recommendation mutation to a committed result before marking an attempt interrupted; attempts without a pending mutation recover as interrupted.
- [ ] For a novel proposal, atomically persist the validated bounded report in the single workspace assessment record with internal `pending_recommendation` state and stable recommendation ID before changing the recommendation store; do not publish a succeeded report while pending, and keep the serialized record under 64 KiB.
- [ ] Idempotently upsert the recommendation by that ID, then atomically mark the assessment report committed and clear the pending marker. On startup, before opening the status/report route, acquire the analysis lease and reconcile pending writes by repeating the upsert before commit.
- [ ] Add fault-injection tests for interruption before pending persistence, after pending persistence but before recommendation persistence, after recommendation persistence but before report commit, and during retry; every retry produces one recommendation and a matching committed report.
- [ ] Simulate final report-commit failure after the recommendation upsert while the sidecar stays alive; verify read-only status/list/get omit or fail closed on the pending recommendation without writes, active-response/accept/Fix with AI neither expose nor use it, it does not count as an active-recommendation gate, and new provider work waits for reconciliation.
- [ ] Revalidate workspace instance, effective settings, provider/harness identity, bundle version, fact hashes, and page IDs immediately before report and recommendation writes, and before every recommendation list/get, active-recommendation response, acceptance, and Fix with AI use path; add a direct-acceptance test without an intervening status read.
- [ ] Logically invalidate reports when any input identity changes. Keep status reads read-only: revalidate and omit stale report content without modifying state; settings mutations redact disallowed evidence before replying, and other stale reports are deleted by the next state-changing assessment or workspace cleanup.
- [ ] Supersede proposed assessment-derived recommendations on invalidation, using the explicit terminal `superseded` state allowed only for this subset; ensure stale evidence cannot reach **Fix with AI**. On effective-access narrowing, redact report/recommendation snapshots and all generated display text that could copy or paraphrase repository content in every lifecycle state, including dismissed records, while preserving identity and lifecycle/outcome history; ordinary evidence/provider/bundle changes do not redact audit snapshots.
- [ ] After invalidation, prove a new valid assessment can create one generation-aware successor linked to a superseded assessment recommendation, while dismissed/accepted/completed or otherwise active records continue to suppress duplicates.
- [ ] When global context/exclusion defaults narrow, compare old and new effective settings across known workspace stores and redact all affected evidence before returning the mutation. Reconcile unopened workspace stores before later exposing or using their recommendations; verify explicit overrides that do not narrow access remain unchanged.
- [ ] Discard stale results without changing a newer report, recommendation, or run outcome.
- [ ] Run focused runtime/evaluator tests, including existing analysis lease and stale identity tests.

## Task 4: Expose the authenticated sidecar contract

**Files:** Modify `crates/orkworksd/src/http/taskmaster_handlers.rs`, `http/taskmaster_settings_handlers.rs`, and `main.rs`.

**Interfaces:**
- `POST /taskmaster/assess-workflow` accepts no workspace or provider override from the renderer; sidecar state selects both. Its `active_recommendation` outcome returns only stable ID/status fields, never a serialized recommendation or repository excerpts.
- The existing Taskmaster status read includes a distinct assessment projection.
- Responses classify scheduled, unavailable, already-running, and active-recommendation outcomes and never expose uncited repository content.

- [ ] Add handler/evaluator tests for authentication, no workspace, missing policy version or capability, ineligible legacy bundle, missing provider, active Brain recommendation, active analysis, and successful scheduling. Cover background and Analyze now gates as well as assessment, and assert `active_recommendation` responses contain no evidence excerpts.
- [ ] Register the route and ensure status reads remain read-only, suppress stale report content after identity mismatch, and perform persistent invalidation only from state-changing handlers/reconciliation.
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

- [ ] Test eligible online and offline cached bundles, restart with cached payload after manifest/feed loss, fresh offline install with the signed packaged starter envelope and manifest attestation, mismatched starter/feed version or digest, missing/unsupported/mismatched `privacyPolicyVersion: 1`, missing/unsupported `taskmaster-assessment-v1` capability, malformed/unverified bundles, and bundles failing the exclusion policy. An eligible signed offline bundle remains usable regardless of age; an ineligible bundle blocks all Brain-backed provider analysis.
- [ ] Verify deterministic observation recommendations continue while Brain-backed provider analysis is gated, and verify Analyze now/background analysis resume only with an eligible bundle.
- [ ] Verify a proposal reaches the existing `proactive:v1:` lifecycle and respects duplicate/dismissed/active recommendation state without duplicating mutations.
- [ ] Verify insufficient evidence, missing or unsupported Brain citations, changed cited or uncited facts, changed recommendation inputs/configuration/workspace, direct list/get/accept/Fix with AI after invalidation, and unavailable provider produce no ungrounded or cached-stale recommendation.
- [ ] Verify report replacement, local-only persistence, workspace metadata deletion, stale-report suppression on read-only status reads, access-narrowing redaction of evidence and generated recommendation/report text before settings replies including inherited global defaults across multiple workspaces, proposed-recommendation supersession and linked successors, live commit-failure suppression, redaction of dismissed evidence after access narrowing, preservation of audit snapshots for non-access changes, bounded size, crash recovery, and separation from Analyze now cache/report state.
- [ ] Run Rust tests, formatting, focused desktop tests/type-check, documentation link/build checks, and the repository-required `/code-review low` before code PR merge.
- [ ] Keep all Brain-backed inference explicitly gated until #529's reviewed distilled export, supported signed-payload policy/capability fields, generated starter snapshot, and verified signed publication are complete; report any remaining #745 lifecycle alignment before implementation handoff.

## Planning checkpoint

- **Agent uncertainty investigated:** whether the existing evaluator could serve as the assessment implementation. It currently supports multiple proposals and has ordinary analysis cache/application identity. The plan therefore calls for a distinct evaluator and result identity, reusing only the provider/context/evidence/recommendation seams.
- **Project blind spot investigated:** a valid signature alone does not mean the active bundle obeys the privacy policy or contains assessment guidance. The sidecar must check both, and the packaged starter must have the same signed attestation as feed content; eligible cached guidance remains usable offline with no independent age cutoff, while unsigned, unreviewed legacy, malformed, or incomplete bundles block Brain-backed inference.
- **New review blind spots investigated:** recommendation invalidation must permit a later valid successor without bypassing dismissal/acceptance; access narrowing must redact generated prose as well as evidence excerpts; and live write failure must keep a pending proposal hidden and non-actionable until reconciliation. The spec and plan now define narrow successor identity, full generated-text redaction, and pending-transaction handling across all response/use paths.
- **Dependency facts:** #529 is open and requires the strict privacy policy, generated starter snapshot, and verified signed publication, but its current acceptance criteria do not name `privacyPolicyVersion`, the assessment capability marker, or a signed starter envelope with matching manifest attestation. Resolve those tracked dependency gaps before runtime implementation; #529 remains a prerequisite for Brain inference availability. #503 is the accepted Taskmaster knowledge baseline. #745 is open and includes a requirement for compatibility with existing `improve_workflow` recommendations; reconcile its lifecycle contract with this assessment-only terminal `superseded` transition before implementation, without waiting for unrelated configuration-history work.
- **Unresolved external gate:** Brain-backed inference cannot resume until #529 supplies an eligible signed bundle. A proposal also stays on the existing user-approved Fix with AI path; no assessment result grants execution permission.
