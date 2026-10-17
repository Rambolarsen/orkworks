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

The owner approved the intended user outcome on 2026-10-17. This authorizes
independent evidence-quality repair under #909 and preparation of the concrete
review artifacts; the explicitly proposed assessment protocol still needs owner
review before dependent runtime implementation. Do not request approval again
for unchanged accepted portions after that review.

1. Obtain written-spec review and record acceptance of the proposed assessment contract and [ADR 0085](../../adr/0085-taskmaster-assessment-and-analysis-outcomes.md).
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

- [x] Write the ADR from the accepted spec; do not broaden it into new workspace deletion UX or a second provider/network path.
- [x] Verify the record cap covers all stored fields and define deterministic rejection/truncation behavior before any runtime code uses the schema.
- [x] Index the ADR and check its links and status.
- [x] Obtain the required architecture decision review before proceeding to protocol implementation.
- [x] Do not begin this task until written-spec acceptance and explicit implementation authorization are recorded.

## Task 2: Build the distinct assessment evaluator

**Files:** Create `crates/orkworksd/src/taskmaster/assessment.rs` and its tests; modify `taskmaster/mod.rs` and `taskmaster/evaluator.rs`.

**Interfaces:**
- Input: immutable Taskmaster snapshot, canonical workspace identity, permitted observations, bounded repository facts, current recommendations, and selected relevant signed knowledge pages.
- Output: typed `Proposed { ... }` or `NoProposal { reason, ... }`, each with assessment input identity and validated fact/page references.
- A proposal contains exactly one next improvement and uses the existing recommendation identity/application functions.

- [x] Add failing tests for one proposal, no proposal, multiple proposals, unknown fact/page IDs, unknown fields, executable command output, and unverified checks claimed as passed.
- [x] Add prompt fixtures proving only relevant distilled knowledge pages are supplied and every input is marked untrusted reference data.
- [x] Implement strict structured-output decoding; reject more than one proposed next step rather than applying a subset of multiple model proposals.
- [x] Require at least one current repository fact and at least one relevant selected Brain page for a proposal; a session-observations-only context may return no proposal but cannot assert a repository gap, and a no-proposal output may have no Brain-page citations.
- [x] Build cache identity from the complete bounded input snapshot, including uncited facts and the pre-run recommendation snapshot, plus prompt/schema version, effective settings, provider/harness identity, workspace generation, selected pages, and full bundle identity; when this run creates a recommendation, record and exclude only its stable ID during revalidation; keep it separate from Analyze now.
- [x] Prove changes to an uncited fact, any recommendation other than this assessment's own created recommendation, or any other supplied input invalidate proposal and no-proposal cache entries; prove the created proposal itself does not invalidate its report, and dedupe suppression leaves the existing record in the snapshot with no derived ID.
- [x] Reject a proposed result with no Brain-page citations or citations to pages outside the selected eligible bundle; allow no-proposal output without page citations.
- [x] When a proposal's `proactive:v1:` dedupe key exists in an ordinary lifecycle state, return a duplicate-suppression no-proposal result without changing or relinking it. Allow one idempotent linked successor only when the matching assessment-derived record is terminally superseded by input invalidation; bind its stable ID to the predecessor and unique assessment ID, and test repeated attempts.
- [x] Run focused assessment evaluator tests; preserve the existing recommendation application path while refusing Brain-backed inference when the bundle is ineligible.

## Task 3: Add durable assessment lifecycle and admission

**Files:** Modify `crates/orkworksd/src/taskmaster/runtime.rs` and `taskmaster/evaluator.rs`; add focused tests beside runtime/evaluator tests.

**Interfaces:**
- Runtime operations queue, mark running, read status, and finish an assessment report for one canonical workspace.
- Assessment and ordinary analysis share the existing analysis lease and do not overlap.
- Assessment status distinguishes queued/running from the latest succeeded/failed/interrupted outcome without changing ordinary analysis status.

- [x] Add tests for a fresh report, replacement by a later report, the 64 KiB cap, legacy-ledger compatibility, unreadable-ledger refusal, and reopening persisted state.
- [x] Add tests proving one report is keyed to one canonical workspace and local Taskmaster-data deletion removes it.
- [x] Add tests for one active analysis/assessment only, active Brain recommendation blocking, active observation-only recommendations not blocking, and no background quota/cooldown consumption.
- [x] Mark an admitted assessment running before collection/prompt construction; persist failure for errors after admission. During recovery under the analysis lease, reconcile a pending recommendation mutation to a committed result before marking an attempt interrupted; attempts without a pending mutation recover as interrupted.
- [x] For a novel proposal, atomically persist the validated bounded report in the single workspace assessment record with internal `pending_recommendation` state and stable recommendation ID before changing the recommendation store; do not publish a succeeded report while pending, and keep the serialized record under 64 KiB.
- [x] Idempotently upsert the recommendation by that ID, then atomically mark the assessment report committed and clear the pending marker. On startup, before opening the status/report route, acquire the analysis lease and reconcile pending writes by repeating the upsert before commit.
- [x] Add fault-injection tests for interruption before pending persistence, after pending persistence but before recommendation persistence, after recommendation persistence but before report commit, and during retry; every retry produces one recommendation and a matching committed report.
- [x] Simulate final report-commit failure after the recommendation upsert while the sidecar stays alive; verify read-only status/list/get omit or fail closed on the pending recommendation without writes, active-response/accept/Fix with AI neither expose nor use it, it does not count as an active-recommendation gate, and new provider work waits for reconciliation.
- [x] Revalidate workspace instance, effective settings, provider/harness identity, bundle version, fact hashes, and page IDs immediately before report and recommendation writes, and before every recommendation list/get, active-recommendation response, acceptance, and Fix with AI use path; add a direct-acceptance test without an intervening status read.
- [x] Logically invalidate reports when any input identity changes. Keep status reads read-only: revalidate and omit stale report content without modifying state; settings mutations redact disallowed evidence before replying, and other stale reports are deleted by the next state-changing assessment or workspace cleanup.
- [x] Supersede proposed assessment-derived recommendations on invalidation, using the explicit terminal `superseded` state allowed only for this subset; ensure stale evidence cannot reach **Fix with AI**. On effective-access narrowing, redact report/recommendation snapshots and all generated display text that could copy or paraphrase repository content in every lifecycle state, including dismissed records, while preserving identity and lifecycle/outcome history; ordinary evidence/provider/bundle changes do not redact audit snapshots.
- [x] After invalidation, prove a new valid assessment can create one generation-aware successor linked to a superseded assessment recommendation, while dismissed/accepted/completed or otherwise active records continue to suppress duplicates.
- [x] When global context/exclusion defaults narrow, compare old and new effective settings across known workspace stores and redact all affected evidence before returning the mutation. Reconcile unopened workspace stores before later exposing or using their recommendations; verify explicit overrides that do not narrow access remain unchanged.
- [x] Discard stale results without changing a newer report, recommendation, or run outcome.
- [x] Run focused runtime/evaluator tests, including existing analysis lease and stale identity tests.

## Task 4: Expose the authenticated sidecar contract

**Files:** Modify `crates/orkworksd/src/http/taskmaster_handlers.rs`, `http/taskmaster_settings_handlers.rs`, and `main.rs`.

**Interfaces:**
- `POST /taskmaster/assess-workflow` accepts no workspace or provider override from the renderer; sidecar state selects both. Its `active_recommendation` outcome returns only stable ID/status fields, never a serialized recommendation or repository excerpts.
- The existing Taskmaster status read includes a distinct assessment projection.
- Responses classify scheduled, unavailable, already-running, and active-recommendation outcomes and never expose uncited repository content.

- [x] Add handler/evaluator tests for authentication, no workspace, missing policy version or capability, ineligible legacy bundle, missing provider, active Brain recommendation, active analysis, and successful scheduling. Cover background and Analyze now gates as well as assessment, and assert `active_recommendation` responses contain no evidence excerpts.
- [x] Register the route and ensure status reads remain read-only, suppress stale report content after identity mismatch, and perform persistent invalidation only from state-changing handlers/reconciliation.
- [x] Verify the renderer cannot submit a workspace path, provider, context override, arbitrary knowledge page, or report body.
- [x] Run focused sidecar handler tests and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.

## Task 5: Add the Electron bridge and Recommendations UI

**Files:** Modify the listed Electron preload/main and desktop API/types/component files; update focused desktop tests.

**Interfaces:**
- Main-process IPC exposes `requestTaskmasterAssessment()` and typed status retrieval.
- Every request is bound to the current ready backend generation; a workspace switch aborts or rejects stale responses.
- Recommendations presents Assess workflow beside Analyze now, and displays assessment-specific progress, failure, outcome, evidence, page provenance, and unverified checks inline.

- [x] Add tests for bridge payload validation, API response decoding, and generation changes during request/status calls.
- [x] Add component tests for the ineligible-bundle gate, available, unavailable, no-proposal, proposed, running, failed, stale-workspace, and active-recommendation states; deterministic observation recommendations remain visible while Brain inference is gated.
- [x] Keep Analyze now state/formatting independent; reuse the existing recommendation card and **Fix with AI** action for a proposal.
- [x] Verify no background popup, focus change, or second approval vocabulary is added.
- [x] Run targeted desktop tests and type-check from `apps/desktop/` using pnpm.

## Task 6: Verify bundle gating and end-to-end contract

**Files:** Modify or add test fixtures only where required; no Brain repository or publisher changes are part of this issue.

- [x] Test eligible online and offline cached bundles, restart with cached payload after manifest/feed loss, fresh offline install with the signed packaged starter envelope and manifest attestation, mismatched starter/feed version or digest, missing/unsupported/mismatched `privacyPolicyVersion: 1`, missing/unsupported `taskmaster-assessment-v1` capability, malformed/unverified bundles, and bundles failing the exclusion policy. An eligible signed offline bundle remains usable regardless of age; an ineligible bundle blocks all Brain-backed provider analysis.
- [x] Verify deterministic observation recommendations continue while Brain-backed provider analysis is gated, and verify Analyze now/background analysis resume only with an eligible bundle.
- [x] Verify a proposal reaches the existing `proactive:v1:` lifecycle and respects duplicate/dismissed/active recommendation state without duplicating mutations.
- [x] Verify insufficient evidence, missing or unsupported Brain citations, changed cited or uncited facts, changed recommendation inputs/configuration/workspace, direct list/get/accept/Fix with AI after invalidation, and unavailable provider produce no ungrounded or cached-stale recommendation.
- [x] Verify report replacement, local-only persistence, workspace metadata deletion, stale-report suppression on read-only status reads, access-narrowing redaction of evidence and generated recommendation/report text before settings replies including inherited global defaults across multiple workspaces, proposed-recommendation supersession and linked successors, live commit-failure suppression, redaction of dismissed evidence after access narrowing, preservation of audit snapshots for non-access changes, bounded size, crash recovery, and separation from Analyze now cache/report state.
- [x] Run Rust tests, formatting, focused desktop tests/type-check, documentation link/build checks, and the repository-required `/code-review low` before code PR merge (medium effort under the documented lifecycle/protocol/size escalation).
- [x] Keep all Brain-backed inference explicitly gated until #529's reviewed distilled export, supported signed-payload policy/capability fields, generated starter snapshot, and verified signed publication are complete; report any remaining #745 lifecycle alignment before implementation handoff.


## Planning checkpoint

- **Agent uncertainty investigated:** whether the existing evaluator could serve as the assessment implementation. It currently supports multiple proposals and has ordinary analysis cache/application identity. The plan therefore calls for a distinct evaluator and result identity, reusing only the provider/context/evidence/recommendation seams.
- **Project blind spot investigated:** a valid signature alone does not mean the active bundle obeys the privacy policy or contains assessment guidance. The sidecar must check both, and the packaged starter must have the same signed attestation as feed content; eligible cached guidance remains usable offline with no independent age cutoff, while unsigned, unreviewed legacy, malformed, or incomplete bundles block Brain-backed inference.
- **New review blind spots investigated:** recommendation invalidation must permit a later valid successor without bypassing dismissal/acceptance; access narrowing must redact generated prose as well as evidence excerpts; and live write failure must keep a pending proposal hidden and non-actionable until reconciliation. The spec and plan now define narrow successor identity, full generated-text redaction, and pending-transaction handling across all response/use paths.
- **Dependency facts:** #529 is open and requires the strict privacy policy, generated starter snapshot, and verified signed publication, but its current acceptance criteria do not name `privacyPolicyVersion`, the assessment capability marker, or a signed starter envelope with matching manifest attestation. Resolve those tracked dependency gaps before runtime implementation; #529 remains a prerequisite for Brain inference availability. #503 is the accepted Taskmaster knowledge baseline. #745 is open and includes a requirement for compatibility with existing `improve_workflow` recommendations; reconcile its lifecycle contract with this assessment-only terminal `superseded` transition before implementation, without waiting for unrelated configuration-history work.
- **Unresolved external gate:** Brain-backed inference cannot resume until #529 supplies an eligible signed bundle. A proposal also stays on the existing user-approved Fix with AI path; no assessment result grants execution permission.

## 2026-10-17 execution update and scoped review

### Current dependency evidence

- #529 is closed, with #902 recording signed production publication and packaged
  acceptance. A local diagnostic admitted the active production proof and
  included the distilled assessment playbook. The runtime eligibility/capability
  checks in the specification remain mandatory.
- #745 is closed. Assessment retains the existing `improve_workflow` identity,
  dedupe/dismissal and Fix with AI seams; no learning, configuration mutation or
  orchestration runtime is added.
- Ordinary analysis was reproduced returning empty lists, not losing proposals
  in the UI. Glyph-only evidence can occupy its newest-observation window.
  [#909](https://github.com/Rambolarsen/orkworks/issues/909) is the independent
  bounded fix. It changes only supplied observation selection/context, not
  durable observation history or lifecycle.

### Delivery order and acceptance mapping

1. Complete #909 with a shared observation selector for prompt and retrieval,
   filtering before the 16-entry bound and including kind/time. Tests exercise
   noisy newest entries, Unicode, ordering, empty input and unchanged history.
2. Obtain one owner review of the current written specification and ADR 0085.
   This is the remaining dependent-runtime gate. Record approval in the same
   artifacts; do not reopen approved scope or require another plan approval.
3. Execute Tasks 1–3 using ADR 0085's exact version-1 response envelope and
   complete capped report. Add strict output/citation tests, separate cache
   identity, shared-lease admission, pending-upsert/commit crash recovery and
   legacy/corrupt/unknown-version cases. Success means either one grounded
   recommendation or a committed explained no-proposal report.
4. Carry Task 3's consumer revalidation, global/workspace narrowing across
   unopened workspaces and stale derived-recommendation successor rules through
   Task 4's authenticated routes and Task 5's bridge/UI. Add ordinary-analysis created/suppressed counts
   and fixed manual skip dispositions per ADR 0085; preserve the latest actual
   outcome on cache hits. Keep all model prose workspace-local. UI tests verify
   no-proposal reasons, cache reuse, failure, workspace switching and existing
   Fix with AI behavior.
5. Complete Task 6 and repository verification, current-code review, PR checks
   and merge/cleanup only within the reviewed scope. Keep #769 open until all
   assessment acceptance criteria are implemented and verified; #909 can close
   independently when its fix lands.

Every original task remains governed by the authoritative specification,
especially complete-input revalidation, transaction cap, privacy redaction and
all recommendation-consumer gates. No runtime assessment work starts merely
because this plan exists.

### Plan review disposition

**Ready for owner protocol review; dependent runtime not started.** The original
plan covers evaluator, persistence, admission, consumer fencing, transport/UI and
verification. This update resolves outdated dependency assumptions and adds the
reported observation-quality and ordinary-outcome gaps without weakening the
canonical contract. The production code diff currently covers only #909.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 2 | #529 and #745 are closed; active signed capability still requires runtime proof. |
| Blast radius | 4 | Recommendation list/get, settings narrowing, admission and handoff consumers all need assessment fencing; exercise every surface. |
| State changes | 3 | Additive optional capped report/status with pending upsert and redaction; test legacy reads and fail-closed corruption. |
| Reversibility | 5 | Before any report exists, revert is local. After rollout, keep consumer fences and disable new assessment admission for recovery; forward-fix instead of binary downgrade. Old binaries cannot enforce the new lifecycle. |
| Uncertainty | 3 | Existing transport/store seams are verified; full assessment crash and access-reduction behavior is not implemented yet. |

Total: 17/25. This records risk and treatment, not approval. Before first report
creation, ordinary rollback is possible. After report creation, the supported
recovery is to disable new assessment admission while preserving report consumers,
then forward-fix; do not downgrade or hand-edit metadata. Confirm this operational
boundary in the owner review. It does not block independent nonpersistent #909.

## Execution ledger — 2026-10-17

Owner approval: after being directed to ADR 0085 and the written assessment
specification as the remaining gate, the owner replied "go ahead". Record this
as approval of that unchanged concrete protocol and its linked specification;
no repeat approval is required within scope. #909 is merged through #911.

- Task 1: accepted protocol and specification recorded.
- Task 2: response validation/evaluator in progress.
- Tasks 3–6: pending implementation and verification.

Ownership: primary session owns lifecycle, report persistence, consumer fencing,
status/UI integration and the final PR. A bounded parser/validation implementation
may use one Luna worker on an isolated worktree with a fixed API; integration
consumes its reviewed commit. Maximum three implementation agents concurrently;
reviews are read-only and scoped. No worker launches coding harnesses, applies
live recommendations, or changes files owned by another task.

Desktop routing: one Luna worker owns desktop assessment DTOs, narrow IPC and
Recommendations UI/tests on its own worktree. Its fixed contract is independent
of implementation timing: `requestTaskmasterAssessment()` returns the manual
admission status and only `{id,status}` for an active recommendation;
`getTaskmasterRunStatus()` gains optional assessment `{state,report,errorSummary,
reused}`. Report display fields are id, workspacePath, observedAt, provider/model,
bundleVersion, outcome, summary, noProposalReason, unverifiedChecks,
repositoryEvidence, knowledgeEvidence and recommendationId. Ordinary outcomes
gain optional `proposalCounts:{created,suppressed}` and status gains optional
`latestManualRequest:{id,requestedAt,completedAt,disposition,reason}`. The parent
owns all Rust protocol producers and integrates the reviewed desktop commit.

- Store foundation: four bounded atomic-store tests passed after three expected
  failures against the stub. Unknown/corrupt/oversized records fail closed;
  oversized replacement preserves the old bytes; workspace deletion removes it.
- Model task review: c447688a requires a scoped correction to command/claim
  filtering: generic Git/run descriptions and hypothetical or negated verification
  text must remain valid. The worker is fixing it with red regression tests;
  no decoder commit has been integrated before that disposition.


### Integrated implementation evidence (2026-10-17)

The owned `assess-workflow` worktree integrates parser commits `c447688a`,
`6ac6f2cb`, `1836b8de`, `4060a898`, and `71c625d6`; ordinary analysis outcomes
`a45229b8` and count corrections `8a44f37`; desktop bridge/UI `67cd0179`;
HTTP composition `408ed80b`; and assessment engine/recovery tests `bde0545`,
`417678f`, `00bb578`, and `98ce2c3`. Only the root integration branch is intended
for PR handoff. Worker copies of dependency files are not independent changes.

Root regression evidence covers capped atomic report replacement, publication
ordering and crash replay, privacy narrowing in all retaining recommendation
states (including nested completion packets), unopened/global versus overridden
workspace reconciliation, pending list/detail reads without transaction writes,
stale direct acceptance, read-only status, and workspace-open recovery. The
terminal-capacity regression observed stale input reaching the queue before the
admission guard, then passed with revalidation through synchronous enqueue.

Scoped reviews found and corrected overly broad prose validation, missed command
forms, duplicate and rollup proposal counts, recovery on transient identity-read
failures, and nested completion-packet retention. The post-enqueue and denied-capacity reservation races have real-fixture
red/green regressions and scoped approval. Assessment handoff history uses fixed
text, delivered reservations finalize as accepted, and undelivered redacted
reservations roll back before terminal supersession.

`bash scripts/verify-repo.sh` passed on the integrated production code: 2,066
Rust tests passed (5 ignored), all Rust integration executables passed, 9 OpenCode
reporter tests passed, and 1,334 desktop tests passed (1 skipped). Type-checks,
Rust/desktop builds, native ABI repair, docs validation/build, diff checks,
documentation currency and worktree currency passed. A subsequent test-only
addition exercises the real durable capability gate with signed missing,
unsupported and supported markers, restart, and proof corruption.

Build warnings are existing dead-code/test helper diagnostics, the exercised
host/Electron native ABI repair, and unchanged documentation chunk sizing/cycles;
none requires a behavior change in this feature. User guides, architecture and
README claims are updated; homepage positioning remains accurate.

Final current-diff manual review will use medium effort because this one protocol
availability unit includes lifecycle, concurrency, privacy and more than eight
code files. Splitting admission from its consumer/recovery fences would leave a
partial protocol exposed. The current lower-cost Luna tier remains first. The
final review gate and PR/CI lifecycle remain pending.


### PR #913 review cycle 1

The initial current-code manual review approved `f0757cec`; the subsequent
GitHub Codex review identified seven actionable lifecycle/UI/validation edge
cases. This is substantial feedback cycle 1 of 3, within the original approved
protocol scope. Corrections make skipped/cached manual completion atomic,
persist assessment reuse under current-input guards, allow reassessment when a
prior report becomes stale, and identify a report's recommendation as history
rather than promising an actionable card below it. The global settings scan
holds the active-workspace lock and skips that workspace before its guarded
reconciliation. Successor eligibility uses durable recommendation provenance
and the unique connected invalidated chain tip, surviving intervening failed or
no-proposal assessments without bypassing ordinary lifecycle suppression.

Separate review also found that failed actual-outcome persistence could fall
back to a false pre-dispatch skip. The guard now preserves that active attempt
for interrupted-run recovery. Command-like checks, including unknown tools and
polite/article wrappers, are rejected; the native provider fixture uses a check
description. Regression tests reproduced reuse loss, intervening-report
suppression, repeated-chain suppression, the stale desktop admission gate, and
false skip after a transient outcome write failure before their fixes. Full
verification and current-code review are repeated before the cycle is pushed;
current-head external reviews and CI remain merge gates.


Cycle 1 final integrated verification passed with `bash scripts/verify-repo.sh`:
2,072 Rust tests passed (5 ignored), all Rust integration executables passed,
9 OpenCode tests passed, 1,335 desktop tests passed (1 skipped), and all builds,
type checks, docs checks/build, diff and currency checks passed. Host Node ABI
was restored before this run; the desktop native ABI repair then passed as part
of normal verification. The separate current-diff medium manual review approved
the corrected production code with no remaining findings. Review evidence is
recorded on PR #913 for the committed code SHA; external re-review and CI follow
the feedback-cycle push before merge. Source/user-guide claims were reviewed:
the corrections refine the documented existing workflow and require no new
installer-availability claim or homepage wording.


### PR #913 review cycle 2

The current-head Codex review of `95416027` identified five additional concrete
edge cases. Cycle 2 of 3 preserves a user's filter during repeated assessment
polls, permits an explicit recovery retry for deferred queued/running state,
rejects executable-style instructions using additional verbs such as “use,”
advances successor depth from the selected predecessor, and lets the authenticated
target session record completion after an already-delivered fix is redacted.
The completion exception grants no new action authority: packet attachment and
replacement stay blocked, existing approved packet identities remain unchanged,
matching completed retries remain idempotent, and supplied completion prose is
replaced by a generic event. Tests cover wrong-target rejection and privacy
retention. Canonical architecture/user/spec wording now explains these existing
recovery and completion boundaries. Full verification and fresh medium manual
review are repeated on the integrated code before pushing cycle 2.


The fresh cycle-2 review identified one further privacy race in request/startup
reconciliation. Both callers now use the existing current-settings persistence
guard followed by the active-workspace lock and path/metadata identity check.
A staged regression failed on the old unguarded reconciliation, then passed with
a concurrent delivered lifecycle update and second access reduction. The fresh
medium manual re-review approved the final code with no remaining findings;
full repository verification is repeated after this last guard change.


Cycle 2 final integrated `bash scripts/verify-repo.sh` passed: 2,075 Rust tests
(5 ignored), all Rust integration executables, 9 OpenCode tests, 1,337 desktop
tests (1 skipped), builds, type checks, native ABI repair, docs validation/build,
diff and currency checks. Manual medium review approved the identical code diff.
The README warning was reviewed: its source-build assessment/outcome claims remain
accurate; the user guide now documents retry recovery and the canonical
architecture/spec describe the bounded completion and reconciliation guards.
The repo-wide worktree warning names an unrelated owner's merged inspector
worktree and does not authorize cleanup by this session.
