# Taskmaster Recommendation Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a manual Taskmaster audit pass that classifies every live proposed recommendation as `under_eligible` / `noise` / `duplicate` / `stale`, surfaces one `cleanup` card, and bulk-dismisses the flagged cards after a single explicit user approval.

**Architecture:** New pure classifier module (`taskmaster/audit.rs`) beside the existing evaluator; additive contract fields (`RecommendationType::Cleanup`, optional `Recommendation.audit`, optional `DismissalWatermark.reason`); the bulk execution reuses the store's graph-transaction batch primitive (CAS per record) inside the existing type-dispatched accept route. Desktop gets a third origin filter and a cleanup card renderer.

**Tech Stack:** Rust (sidecar, axum, serde, chrono), TypeScript/React (desktop).

**Spec:** `docs/superpowers/specs/2026-10-08-taskmaster-recommendation-audit-design.md` (worktree `../orkworks-taskmaster-audit-spec`) — the plan argues from the spec; executors read both.

## Global Constraints

- The audit never mutates anything until the user accepts the cleanup card (explicit-approval rule, root `AGENTS.md`).
- Transport contract changes are limited to: `Cleanup` discriminant on `RecommendationType`, optional `audit` field on `Recommendation`, optional `reason` field on `DismissalWatermark`, accept route's `sessionId` becoming optional (type-validated). Nothing else.
- serde style in `taskmaster/mod.rs` is `#[serde(rename_all = "camelCase")]` on structs, `#[serde(rename_all = "snake_case")]` on enums; old on-disk files must keep deserializing (new fields all `#[serde(default, skip_serializing_if = ...)]`).
- All timestamps in the store are RFC 3339 strings; use `chrono` and existing helpers.
- Rust commands: `cargo test --manifest-path crates/orkworksd/Cargo.toml`; format inside `crates/orkworksd` with `cargo fmt -- <file>` (never from the repo root).
- Work happens in worktree `../orkworks-taskmaster-audit-spec`, branch `taskmaster-recommendation-audit-spec`. Commit per task.

---

### Task 1: Contract additions (types only, TDD)

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/mod.rs` (enum `RecommendationType` ~line 23, struct `DismissalWatermark` ~line 82, struct `Recommendation` ~line 130)
- Test: same file, `mod tests`

**Interfaces:**
- Produces: `RecommendationType::Cleanup` (serde `cleanup`); `DismissalWatermark.reason: Option<String>`; `Recommendation.audit: Option<AuditCleanup>`; new types `AuditCleanup`, `AuditCleanupEntry`, `AuditCriterion` as below. Later tasks consume these exact names.

- [ ] **Step 1: Write the failing round-trip tests** in `mod.rs` `mod tests`:

```rust
#[test]
fn cleanup_card_contract_round_trips() {
    let mut recommendation = observation("one", 1, "session-a", 0.8, Impact::High);
    let proposals = evaluate_workflow_improvements(
        &[recommendation.clone()],
        &[],
        "workspace-1",
        "2026-08-21T12:00:00Z",
    );
    let mut card = proposals
        .into_iter()
        .chain(evaluate_workflow_improvements(
            &[observation("two", 2, "session-b", 0.8, Impact::High)],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        ))
        .next()
        .unwrap();
    card.recommendation_type = RecommendationType::Cleanup;
    card.audit = Some(AuditCleanup {
        entries: vec![AuditCleanupEntry {
            id: "recommendation-x".into(),
            title: "Improve Tooling".into(),
            criteria: vec![AuditCriterion::UnderEligible, AuditCriterion::Stale],
        }],
        scanned: 3,
        healthy: 2,
        stale_after_days: 14,
    });
    card.workflow_improvement.dismissal_watermark = Some(DismissalWatermark {
        dismissed_at: "2026-10-08T00:00:00Z".into(),
        dismissed_through_sequence: 1,
        observation_ids: vec!["one".into()],
        qualifying_count: 1,
        highest_impact: Impact::High,
        affected_session_ids: vec!["session-a".into()],
        reason: Some("audit:under_eligible@cleanup-1".into()),
    });
    let json = serde_json::to_string(&card).unwrap();
    assert!(json.contains("\"type\":\"cleanup\""));
    assert!(json.contains("\"reason\":\"audit:under_eligible@cleanup-1\""));
    let parsed: Recommendation = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, card);
}

#[test]
fn legacy_recommendation_files_load_without_audit_or_reason() {
    let proposal = evaluate_workflow_improvements(
        &[
            observation("one", 1, "session-a", 0.8, Impact::Low),
            observation("two", 2, "session-b", 0.8, Impact::Low),
        ],
        &[],
        "workspace-1",
        "2026-08-21T12:00:00Z",
    )
    .remove(0);
    let json = serde_json::to_string(&proposal).unwrap();
    assert!(!json.contains("\"audit\""));
    assert!(!json.contains("\"reason\""));
    let parsed: Recommendation = serde_json::from_str(&json).unwrap();
    assert!(parsed.audit.is_none());
    assert!(parsed.workflow_improvement.dismissal_watermark.is_none());
}
```

- [ ] **Step 2: Run and verify both fail** (`cargo test --manifest-path crates/orkworksd/Cargo.toml cleanup_card_contract legacy_recommendation_files_load`) — they must fail to compile because the types do not exist yet.

- [ ] **Step 3: Add the types** to `mod.rs`:

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditCriterion {
    UnderEligible,
    Noise,
    Duplicate,
    Stale,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditCleanupEntry {
    pub id: String,
    pub title: String,
    pub criteria: Vec<AuditCriterion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditCleanup {
    pub entries: Vec<AuditCleanupEntry>,
    pub scanned: usize,
    pub healthy: usize,
    pub stale_after_days: u32,
}
```

Then: `RecommendationType` gains `Cleanup,`; `DismissalWatermark` gains

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub reason: Option<String>,
```

and `Recommendation` gains

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub audit: Option<AuditCleanup>,
```

Fix all construction sites the compiler reports (`audit: None`, `reason: None` — grep `DismissalWatermark {` and `Recommendation {`; the evaluator at `mod.rs:436`, rollup parent builders, test fixtures). In the cleanup-card round-trip test, replace the convoluted two-evaluator `.next()` with directly building the card via the evaluator on two observations and then overwriting the fields (simpler than the snippet above — keep the assertions).

- [ ] **Step 4: Run the full suite** — `cargo test --manifest-path crates/orkworksd/Cargo.toml` must pass (some old tests may construct watermarks; give them `reason: None`).

- [ ] **Step 5: Format and commit**

```bash
cd crates/orkworksd && cargo fmt -- src/taskmaster/mod.rs
git add -A && git commit -m "feat(taskmaster): cleanup card contract types"
```

### Task 2: Store — widen dismiss, add watermark reason

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/store.rs` (`dismiss` ~line 272)
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs` (ordinary dismiss route passes `None`)
- Modify: `crates/orkworksd/src/session_application.rs` (its `dismiss_recommendation` call site, if it wraps `store::dismiss`)
- Test: `crates/orkworksd/src/taskmaster/store.rs` `mod tests`

**Interfaces:**
- Produces: `store::dismiss(&self, id: &str, dismissed_at: String, reason: Option<String>)` — cleanup-type cards allowed; watermark `reason` persisted. Task 6 calls this with `Some(...)`.

- [ ] **Step 1: Failing test** in `store.rs` tests (pattern after `dismisses_in_place_with_immutable_evidence_and_watermark`, ~line 2097):

```rust
#[test]
fn dismisses_cleanup_cards_with_watermark_reason() {
    let store = temp_store();
    let mut card = a_proposed_cleanup_card(&store); // build via put(), see below
    let dismissed = store
        .dismiss(&card.id, "2026-10-08T00:00:00Z".into(), Some("audit:stale@cleanup-1".into()))
        .unwrap()
        .unwrap();
    assert_eq!(dismissed.status, RecommendationStatus::Dismissed);
    let watermark = dismissed.workflow_improvement.dismissal_watermark.unwrap();
    assert_eq!(watermark.reason.as_deref(), Some("audit:stale@cleanup-1"));
    assert_eq!(store.get(&card.id).unwrap().unwrap().status, RecommendationStatus::Dismissed);
}
```

`a_proposed_cleanup_card(&store)` is a small helper in the test module: build a `Recommendation` with `recommendation_type: Cleanup`, `status: Proposed`, `workflow_improvement` placeholder (`target_surface: Documentation`, empty lists), `audit: None`, `put(&card)`, return it.

- [ ] **Step 2: Run to verify failure** — compile error (arity) or `InvalidTransition` assert.

- [ ] **Step 3: Implement** — widen the guard in `dismiss`:

```rust
if !matches!(
    recommendation.recommendation_type,
    RecommendationType::ImproveWorkflow | RecommendationType::Cleanup
) || !matches!(
    recommendation.status,
    RecommendationStatus::Proposed | RecommendationStatus::Executing
)
{
    return Err(StoreError::InvalidTransition);
}
```

Add `reason` to the `DismissalWatermark { .. }` construction; thread `reason: Option<String>` through the signature. Update every `store.dismiss(...)` caller (search `\.dismiss\(`) to pass `None` for the ordinary paths — the HTTP dismiss handler (`taskmaster_handlers.rs` ~line 325) and any session_application wrapper.

- [ ] **Step 4: Full suite green, format, commit** as in Task 1 Step 5 (`fix(taskmaster): dismiss accepts cleanup cards and records watermark reason`).

### Task 3: Pure audit classifier (TDD)

**Files:**
- Create: `crates/orkworksd/src/taskmaster/audit.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs` (`mod audit;` declaration)

**Interfaces:**
- Consumes: `Recommendation`, `DismissalWatermark` from `super`.
- Produces (Task 4 consumes):

```rust
pub(crate) const STALE_AFTER_DAYS: u32 = 14;

pub(crate) fn classify(
    recommendation: &Recommendation,
    family: &FamilyContext,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<AuditCriterion>;

/// Precomputed per-dedupe-key context the classifier needs, built once by the
/// caller so `classify` stays pure and cheap.
pub(crate) struct FamilyContext {
    pub has_newer_proposed_sibling: bool,
    pub has_unextended_terminal_sibling: bool,
}
```

- [ ] **Step 1: Failing tests** in `audit.rs` `#[cfg(test)]` — one test per criterion, building cards through `evaluate_workflow_improvements` where convenient and by hand otherwise:

1. `under_eligible`: card with two cited evidence entries where one is confidence `0.59` (fails the `0.6` floor) → `[UnderEligible]`.
2. `not_under_eligible`: card with two citations at `0.8` → criteria without `UnderEligible`.
3. `noise`: all citations `ObservationSource::Peon` with `problem_area: None` → includes `Noise`; one citation with `problem_area: Some(...)` or `ObservationSource::Agent` → no `Noise`.
4. `duplicate_newer_sibling`: `FamilyContext { has_newer_proposed_sibling: true, .. }` → includes `Duplicate`; `has_unextended_terminal_sibling: true` → includes `Duplicate`.
5. `stale`: card whose max evidence `observed_at` is 20 days before `now` → includes `Stale`; 5 days → not.
6. `healthy_card_matches_nothing`: fully qualifying, mixed-source, recent card → empty vec.
7. `criteria_multiply`: card matching three criteria lists all three in canonical order (under-eligible → noise → duplicate → stale).
8. `rollup_parents_never_classified`: a proposed card with non-empty `rollup_member_ids` produces no criteria and never appears in entries.
9. `cleanup_cards_do_not_block_brain_analyses`: `active_workflow_recommendation` (mod.rs ~line 57) returns `None` for a proposed cleanup card — pins the spec's evaluator-interaction guarantee.

Implementation sketch (write it out fully in the file):

```rust
fn qualifies(evidence: &WorkflowObservationEvidence) -> bool {
    evidence.confidence >= 0.6
        && (evidence.reported_impact != Impact::High || evidence.confidence >= 0.8)
}

// under_eligible: evidence.iter().filter(|e| qualifies(e)).map(|e| &e.observation_id)
//   collect into a HashSet; count < 2 → UnderEligible.
// noise: !evidence.is_empty()
//   && evidence.iter().all(|e| e.source == ObservationSource::Peon)
//   && evidence.iter().all(|e| e.problem_area.is_none())
// duplicate: from FamilyContext flags.
// stale: evidence.iter().map(|e| parse e.observed_at).max() < now - Days::new(STALE_AFTER_DAYS as i64)
//   parse with chrono::DateTime::parse_from_rfc3339(...).ok(); if no parseable
//   timestamps and evidence is non-empty, do NOT classify stale (unparseable
//   timestamps must never dismiss a card).
```

Order the returned vec canonically. Add `use` lines for `Impact`, `ObservationSource`, `WorkflowObservationEvidence` from `crate::workflow_observations`, and `super::{AuditCriterion, Recommendation}`.

- [ ] **Step 2: Verify red, implement, verify green** (same loop as Task 1). Register the module: `mod audit;` in `crates/orkworksd/src/taskmaster/mod.rs` next to the existing submodules (`store`, `rollup`, `evaluator`).

- [ ] **Step 3: Format, commit** (`feat(taskmaster): deterministic recommendation audit classifier`).

### Task 4: Audit run — build and persist the cleanup card

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/audit.rs` (card builder)
- Modify: `crates/orkworksd/src/session_application.rs` (`run_recommendation_audit` beside `refresh_workflow_recommendations` ~line 396)
- Test: `crates/orkworksd/src/taskmaster/audit.rs`, `crates/orkworksd/src/session_application.rs` tests

**Interfaces:**
- Produces: `SessionApplication::run_recommendation_audit(&self) -> Result<Option<Recommendation>, ...>` — persists via `apply_recommendation_graph_transaction` and returns the new/refreshed card (`None` when every card is healthy).

- [ ] **Step 1: Failing store-level test** in `audit.rs` for the builder:

```rust
#[test]
fn builds_cleanup_card_with_entries_and_counts() {
    // 3 proposed cards: under-eligible, noise, healthy; 1 terminal card (ignored).
    // Assert: entries == 2 in canonical order, scanned == 3 (proposed only),
    // healthy == 1, requires_approval == true, priority Medium,
    // evidence.is_empty(), recurrence_count == 0, workflow_improvement
    // placeholder target_surface == Documentation, proposed_improvement text
    // starts with "Dismiss 2 proposed recommendations",
    // dedupe_key == "cleanup:v1", chain_depth == previous card's + 0/1 rules.
}
```

- [ ] **Step 2: Implement the builder** in `audit.rs`:

```rust
pub(crate) fn build_cleanup_card(
    recommendations: &[Recommendation],
    workspace_id: &str,
    now: &str,
    prior: Option<&Recommendation>, // existing proposed cleanup card, if any
) -> Option<Recommendation> {
    // 1. Partition: proposed cards (status == Proposed, any type except
    //    Cleanup, and rollup_member_ids empty) vs terminal
    //    (accepted/completed/dismissed/superseded/failed/expired).
    // 2. Build FamilyContext per proposed card from same-dedupe_key siblings:
    //    has_newer_proposed_sibling = another Proposed with same dedupe_key and
    //    (created_at, id) greater than this card's;
    //    has_unextended_terminal_sibling = a terminal sibling whose id is not
    //    reachable by walking this card's supersedes_recommendation_id chain.
    // 3. classify() each; entries = non-empty ones (id, title, criteria);
    //    healthy = proposed.len() - entries.len(); return None if entries empty.
    // 4. Card:
    //    id: prior.map(id) else format!("recommendation-{}", stable_id(&dedupe_key, &[])),
    //    dedupe_key: "cleanup:v1".into(),
    //    chain_id: "cleanup:v1".into(),
    //    chain_depth: prior.chain_depth (replace-in-place) or 0,
    //    recommendation_type: Cleanup,
    //    status: Proposed,
    //    priority: Impact::Medium, confidence: RecommendationConfidence::Medium,
    //    title: "Recommendation cleanup".into(),
    //    summary/proposed_improvement: format!("Dismiss {} proposed recommendations audited as under-eligible/noise/duplicate/stale.", entries.len()),
    //    reason: vec![format!("{} proposed scanned; {} healthy; criteria counts: under_eligible {}, noise {}, duplicate {}, stale {}.", scanned, healthy, ...)],
    //    evidence: vec![], source_session_ids/target/suggested*: empty/None,
    //    requires_approval: true, created_at: prior's created_at or now,
    //    updated_at: now, expires_at: None,
    //    workflow_improvement: placeholder (Documentation, impact Medium,
    //      expected_benefit: "Fewer stale cards, less noise in the recommendations panel.".into(),
    //      supersedes_recommendation_id: prior-terminal cleanup id when regenerating),
    //    audit: Some(AuditCleanup { entries, scanned, healthy, stale_after_days: STALE_AFTER_DAYS }),
    //    completion_packet/rollup_*/proposed_change: None/empty.
}
```

`stable_id` is `pub(crate)` in `mod.rs` (verify visibility; if private, make it `pub(crate)`).

- [ ] **Step 3: Failing session_application test** — seed a temp workspace store (pattern from existing `refresh_workflow_recommendations` tests around `session_application.rs:409`): two proposed under-eligible cards + one healthy; call `run_recommendation_audit()`; assert persisted card exists with 2 entries; call again with one of the flagged cards now dismissed; assert same card id, refreshed entries (1), `updated_at` advanced; call with all healthy; assert `None`.

- [ ] **Step 4: Implement `run_recommendation_audit`** in `session_application.rs` (pattern: `refresh_workflow_recommendations`):

```rust
pub(crate) fn run_recommendation_audit(&self) -> Result<Option<Recommendation>, ...> {
    let workspace_guard = self.state.workspace.lock().unwrap();
    let Some(workspace) = workspace_guard.as_ref() else { return Ok(None); };
    let now = chrono::Utc::now().to_rfc3339();
    let (existing, existing_hashes) = workspace.recommendation_store.list_with_hashes()?...;
    let prior = existing.iter().find(|r| r.recommendation_type == Cleanup && r.status == Proposed);
    let Some(card) = audit::build_cleanup_card(&existing, workspace_id, &now, prior) else { return Ok(None); };
    let mut expected = BTreeMap::new();
    expected.insert(card.id.clone(), prior
        .and_then(|p| existing_hashes.get(&p.id).cloned()));
    workspace.recommendation_store
        .apply_recommendation_graph_transaction(&expected, &[card.clone()])?;
    Ok(Some(card))
}
```

Adapt error plumbing to the file's existing `Result` idioms (it logs and returns unit in places — choose `Result<Option<Recommendation>, StoreError>` and map at the handler).

- [ ] **Step 5: Green, format, commit** (`feat(taskmaster): audit pass builds the cleanup card`).

### Task 5: HTTP route

**Files:**
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs` (new handler + response type)
- Modify: `crates/orkworksd/src/main.rs` (router registration beside the other taskmaster routes)
- Test: `crates/orkworksd/src/http/taskmaster_handlers.rs` tests

**Interfaces:**
- Produces: `POST /taskmaster/audit/recommendations` → `200 { "recommendation": Recommendation | null }`.

- [ ] **Step 1: Failing handler tests** (pattern after `accept_returns_not_found_for_unknown_recommendation` ~line 1047):

1. Seeded workspace with one under-eligible card → 200, body has `recommendation` non-null, `type == "cleanup"`, `audit.entries.len() == 1`.
2. All healthy → 200, body has `"recommendation": null`.
3. No workspace → same status family as other workspace-missing taskmaster routes (copy the existing convention).

- [ ] **Step 2: Implement**:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditResponse {
    recommendation: Option<Recommendation>,
}

pub(crate) async fn run_recommendation_audit(
    State(state): State<Arc<AppState>>,
) -> Response {
    match SessionApplication::new(state).run_recommendation_audit() {
        Ok(recommendation) => Json(AuditResponse { recommendation }).into_response(),
        Err(error) => store_error(error),
    }
}
```

Register in `main.rs` where the taskmaster routes are mounted (`/taskmaster/audit/recommendations`).

- [ ] **Step 3: Green, format, commit** (`feat(taskmaster): audit route`).

### Task 6: Type-dispatched accept with bulk execution (TDD)

**Files:**
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs` (`AcceptRequest`, `accept_recommendation`)
- Modify: `crates/orkworksd/src/session_application.rs` (new `accept_cleanup_recommendation`)
- Modify: `crates/orkworksd/src/taskmaster/store.rs` (extract watermark-building helper from `dismiss` so the batch can reuse it; see Step 3)
- Test: `taskmaster_handlers.rs` tests, `session_application.rs` tests

**Interfaces:**
- Produces: `AcceptRequest.session_id: Option<String>`; cleanup accept executes the batch and returns `{ recommendation: <completed card>, skipped: [{ id, status }] }`.

- [ ] **Step 1: Failing HTTP tests**:

1. Cleanup accept without `sessionId` → 200; card `completed`; every flagged proposed card now `dismissed`; response contains `skipped: []`.
2. Cleanup accept **with** `sessionId` → `400` (spec: parse rejection — implement as explicit check, status `BAD_REQUEST`).
3. `improve_workflow` accept without `sessionId` → `400` (was: parse error 422/400 from serde; now the explicit check keeps rejection explicit — assert it does not reach the session flow).
4. Cleanup accept on unknown id → `NOT_FOUND`; on non-cleanup id via cleanup semantics (impossible — dispatch is by stored type; assert type-dispatch: request with `sessionId` targeting a cleanup card → `400`).

- [ ] **Step 2: Implement the HTTP dispatch**:

```rust
#[derive(Deserialize)]
pub(crate) struct AcceptRequest {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(flatten)]
    packet_mutation: Option<CompletionMutationRequest>,
}

// in accept_recommendation: load the recommendation first
// (SessionApplication::recommendation_query or store.get via a small
// session_application helper — reuse whatever `complete_recommendation`
// uses for read access), then:
match recommendation.recommendation_type {
    RecommendationType::Cleanup if request.session_id.is_none() => { /* cleanup path */ }
    RecommendationType::Cleanup => return BAD_REQUEST,
    _ if request.session_id.is_none() => return BAD_REQUEST,
    _ => { /* existing improve_workflow path, &request.session_id.as_ref().unwrap() */ }
}
```

- [ ] **Step 3: Extract the watermark builder** in `store.rs` so both `dismiss` and the batch build identical watermarks:

```rust
pub(crate) fn dismissed_record(
    mut recommendation: Recommendation,
    dismissed_at: &str,
    reason: Option<String>,
) -> Recommendation {
    // body of today's dismiss(): build watermark (now with reason),
    // set status Dismissed, clear packet approval — without put().
}
```

`dismiss` becomes `get` → guard → `put(dismissed_record(rec, &dismissed_at, reason))`. Existing store tests must stay green unchanged in behavior.

- [ ] **Step 4: Failing session_application test** for the batch: seed card + 2 flagged proposed + 1 entry already dismissed (skip case) + 1 healthy (untouched); call `accept_cleanup_recommendation(&card_id)`; assert: flagged → dismissed with watermark `reason: Some("audit:<criterion>@<card_id>")`, already-dismissed entry in `skipped`, healthy untouched, cleanup card `completed`, all in one store read-back. Then a fault test: set the store's fault-point (reuse the `#[cfg(test)] FaultPoint::Publication` thread-local pattern) and assert the batch leaves the cleanup card `proposed` and aborts atomically.

- [ ] **Step 5: Implement `accept_cleanup_recommendation`** in `session_application.rs`:

```rust
pub(crate) fn accept_cleanup_recommendation(
    &self,
    id: &str,
) -> Result<Option<(Recommendation, Vec<SkippedEntry>)>, StoreError> {
    let workspace_guard = self.state.workspace.lock().unwrap();
    let Some(workspace) = workspace_guard.as_ref() else { return Ok(None); };
    let now = chrono::Utc::now().to_rfc3339();
    let (existing, existing_hashes) = workspace.recommendation_store.list_with_hashes()?;
    let Some(card) = existing.iter().find(|r| r.id == id) else { return Ok(None); };
    if card.recommendation_type != RecommendationType::Cleanup
        || card.status != RecommendationStatus::Proposed
        || card.audit.is_none()
    {
        return Err(StoreError::InvalidTransition);
    }
    // For each audit entry: find current record.
    //   Proposed + (under-eligible-type still — no re-classification; spec says
    //   trust the audited list) → dismissed_record(rec, &now,
    //   Some(format!("audit:{}@{}", criterion, card.id)));
    //   else → skipped entry (id, status).
    // Card itself: status Accepted → Completed in the same batch (set status
    // Completed, updated_at now; no packet). Persist one
    // apply_recommendation_graph_transaction(expected from existing_hashes,
    // records). Return (completed_card, skipped).
}
```

Spec note (pinned): the batch re-dismisses by the *audited* list without re-classifying; drift between audit and accept is handled by CAS (`proposed` guard inside `dismissed_record`'s caller — verify status before mutating, and `expected` hashes catch any concurrent change with `StaleExpectedHash`).

- [ ] **Step 6: Green, format, commit** (`feat(taskmaster): cleanup accept executes bulk dismissal`).

### Task 7: Desktop panel

**Files:**
- Modify: `apps/desktop/src/api.ts:657-690` (`WorkflowRecommendation`)
- Modify: `apps/desktop/src/taskmaster.ts` (origin mapping + filter)
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx` (card rendering)
- Test: existing desktop component test setup (check `apps/desktop/package.json` scripts / existing `*.test.tsx` for the pattern; follow it)

**Interfaces:**
- Consumes: the sidecar contract from Tasks 1/5/6 (`type: "cleanup"`, `requiresApproval: true`, `audit` object, accept route without `sessionId`).

- [ ] **Step 1: Widen the API type** in `api.ts`:

```ts
type: "improve_workflow" | "cleanup";
requiresApproval: boolean;
audit?: {
  entries: Array<{ id: string; title: string; criteria: Array<"under_eligible" | "noise" | "duplicate" | "stale"> }>;
  scanned: number;
  healthy: number;
  staleAfterDays: number;
} | null;
```

- [ ] **Step 2: Origin mapping** in `taskmaster.ts`:

```ts
export type RecommendationOrigin = "analysis" | "observations" | "cleanup";
// in recommendationOrigin: dedupeKey.startsWith("cleanup:v1:") → "cleanup"
// panelEmptyMessage: "cleanup" → "Run an audit to review stale recommendations."
```

Panel filter: `cleanup` cards show under `all` and under a new `cleanup` chip; the existing status rule (`proposed` visible) applies as-is.

- [ ] **Step 3: Render the cleanup card** in `RecommendationsPanel.tsx` — extend the existing card component: when `recommendation.type === "cleanup"`, render the audit summary instead of the evidence/workflow blocks: counts line (`scanned`, `healthy`, `staleAfterDays`), expandable `<details>` listing `audit.entries` (title + criteria badges), accept button labeled `Run cleanup` calling a new `onRunCleanup(recommendation)` prop that POSTs accept **without** a `sessionId`; dismiss unchanged. Guard: disable `Run cleanup` while an accept is in flight (mirror the `dismissing` state pattern).

- [ ] **Step 4: Wire accept** in the accept-caller (`App.tsx:399-431` — the existing active-session resolution is for `improve_workflow` only; branch: cleanup type → call the accept route with `{}` body, no session resolution, no alive-session validation).

- [ ] **Step 5: Component tests** — follow the existing panel test pattern (find the existing test file for `RecommendationsPanel`; if none exists, add `RecommendationsPanel.test.tsx` following the desktop test conventions discovered in `apps/desktop/src`): cleanup card renders counts and entries; `Run cleanup` posts to the accept route without `sessionId`; visible under `all` and `cleanup` filters.

- [ ] **Step 6: Desktop verification** — `pnpm lint && pnpm test` (or the repo's actual scripts from `apps/desktop/package.json`) inside `apps/desktop`; commit (`feat(desktop): cleanup card in recommendations panel`).

### Task 8: Docs

**Files:**
- Modify: `specs/taskmaster.md` (new "Recommendation audit" section after the rollup section — port the design doc's contract sections verbatim-adapted: criteria, card shape, type-dispatched accept, immutability guarantees, non-goals)
- Modify: `docs/agents/domain-entities.md` (vocabulary: `Cleanup` type, `AuditCleanup`, watermark `reason`)
- Modify: `docs/agents/architecture.md` (route table row for `POST /taskmaster/audit/recommendations` + one-paragraph summary in the Taskmaster section)
- Modify: root `AGENTS.md` (metadata-protocol bullet list: one-line pointer to the audit route and cleanup card)

- [ ] **Step 1: Write the spec section** in `specs/taskmaster.md` (this is the authoritative contract — no "TBD").
- [ ] **Step 2: Prose updates** in the three docs; keep `docs/superpowers/specs/2026-10-08-...` as the design record.
- [ ] **Step 3:** `bash scripts/doc-check.sh` clean; commit (`docs: taskmaster recommendation audit contract`).

### Task 9: Verification and PR

- [ ] **Step 1:** `bash scripts/verify-repo.sh` from the worktree root — all green (fix narrowly if not).
- [ ] **Step 2:** Push, open PR referencing the spec, the design-doc decisions, and the verified noise pattern (~180 leftover single-evidence cards).
- [ ] **Step 3:** `/code-review low` gate (diff-scoped) — this PR touches concurrency-adjacent store code and a new protocol surface; per the review gate's risk conditions, if findings cluster around the batch execution path, escalate to medium.
- [ ] **Step 4:** Address findings; wait for required checks; maintainer merges with `gh pr merge <PR> --squash --admin`; cleanup via `bash scripts/finish-pr.sh <PR>`.
