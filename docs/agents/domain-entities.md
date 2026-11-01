---
type: Domain Reference
title: Session state model
description: Persisted session metadata, runtime state, terminology, and workflow-observation domain concepts.
tags: [domain-model, sessions, metadata, peon, taskmaster]
status: stable
---

# Session State Model

This document describes the current Rust session state model in `crates/orkworksd/src/`.

There is no separate `domain/`/`application/` DDD layer in this crate. Session
state uses `SessionMetadata` for persisted records and `SessionHandle` /
`SessionInfo` for live runtime and API state, with mutations coordinated by
`SessionApplication`. The historical typed scaffold and its integration remain
under the assessment described below.

## Scope

Today the state model is centered on **agent sessions**.

The code uses:

- `SessionMetadata` (`metadata.rs`) as the persisted record for one session
- `SessionInfo` (`main.rs`) as the in-memory/API DTO built from metadata plus live runtime state
- `Harness` for the internal coding-tool integration abstraction
- `provider_id` for the inference-service identity when it is known

The user-facing UI may call the selected CLI application a **Coding tool**, but the internal data still stores that concept as harness-related fields.

## SessionMetadata

The owner accepted [SQLite session storage](../../specs/session-storage.md) on
2026-11-01. Implementation is pending: format 2 will store the canonical metadata
and organization transactionally, with recorded historical facts separate from
current resource/native availability. The current source model below remains
format-1 JSON; identity, provenance and lifecycle vocabulary are preserved.

`labelContextRefined` defaults false and records one successful contextual Peon
title refinement per topic. Bootstrap handoff, conversation reset and explicit
restoration of automatic naming clear it. Manual title operations use existing
`labelSource: user`, preserve activity/attention/native identity, and invalidate
queued label work. Native Codex title refresh remains available on subsequent
authenticated exact-identity reports. See [ADR 0098](../adr/0098-contextual-session-titles.md).

File: `crates/orkworksd/src/metadata.rs`

`SessionMetadata` is the persisted record read from and written to `~/.orkworks/workspaces/<hash>/sessions/<id>.json`. It is a flat, serde-mapped struct — not an aggregate with behavior. Notable fields:

- `id`, `workspace`, `task`, `cwd`
- `label` — the session's display title, with persisted `labelSource` provenance, not the turn-by-turn activity in `summary`. New sessions begin with a placeholder, bootstrap prompt, or terminal-input source; Peon may refine its automatic topic, and authenticated Codex reports may replace automatic sources when the native thread provides a name/title. An explicit user rename wins until automatic naming is restored. Sessions written before `labelSource` existed deserialize as `legacy` and are preserved from native replacement because their provenance is unknown (ADR 0063). A descriptive New Session dialog prompt is an immediate bootstrap fallback; the first later descriptive terminal input may replace that fallback once (ADR 0047).
- `harness: String` (serialized `harnessId`, aliased from legacy `harness`)
- `model: String` (serialized `modelId`, aliased from legacy `model`)
- `status: String` — process/terminal state (see vocabulary below)
- `work_phase: String` (serialized `workPhase`) — `ideation` | `implementation` | `review` | `debugging` | `unknown`, normalized on read via `normalize_work_phase`
- `lifecycle_phase: String`, `lifecycle: String` — see vocabulary below
- `attention: Option<String>`, `plan_path: Option<PlanReference>` (serialized `planPath`), `connectivity: String`. Legacy string values remain readable; newly persisted references retain `worktreeRoot`, `relativePath`, and a source (`user_selected`, `hook_reported`, or `terminal_fallback`). `plan_path` is advisory agent context; only a revalidated existing Markdown file in the session repository's worktree family projects the non-persisted `hasOpenablePlan` API flag.
- `terminal_outcome: Option<String>`, `pending_terminal_status: Option<String>`
- `observed_status: Option<String>` plus `ending_observed_status_snapshot` / `final_observed_status_snapshot` (`ObservedStatusSnapshotMetadata { value, source, confidence, observed_at }`)
- `summary`, `next_action`, `needs_user_input`, `detected_question`, `suggested_options`, `blocker_description`, `failed_command`, `failed_test`, `capacity_hints`, `peon_last_inference` — Peon-inferred fields
- `provider_id`, `provider_label`, `provider_model`, `provider_state`
- `created_at`, `last_activity`, `last_output_at`, `metadata_source`, `metadata_confidence` — `last_activity` records meaningful situation changes; `last_output_at` records the latest non-empty PTY frame and is coalesced before persistence.
- `repo_root`, `branch`, `dirty`, `changed_files`, `is_worktree` — Git context, projected fresh on every `GET /sessions` response from `session_view::resolve_effective_cwds`'s priority chain (persisted `SessionMetadata` keeps the launch-time/last-known semantics unchanged; only the API projection reflects the live chain below):
  1. **Harness-reported cwd** (ADR 0032) — the harness's own logical working directory, forwarded from its hook JSON payload (currently Claude Code only, via `report-harness-event.sh`/`.ps1` → `POST /sessions/:id/attention`'s optional `cwd` field → `PeonState.reported_cwd`). Authoritative when present, since command-template harnesses (Claude Code, Codex, OpenCode, Aider) run as the PTY child directly and track "current directory" as their own internal state rather than calling `chdir()` on themselves — so the pid probe below can't see it.
  2. **Pid-probed live cwd** (ADR 0031) — the PTY child process's actual OS-level cwd, resolved via a cross-platform batched probe (`procfs::live_cwds`, backed by the `sysinfo` crate — Linux/macOS/Windows alike). Correctly tracks bare shell sessions (where the PTY child *is* the shell a `cd` mutates) but not harness sessions without hook support.
  3. **Frozen launch-time `cwd`** — the fallback when neither of the above is available.
  Aider, Codex, and OpenCode sessions currently only get tiers 2–3 — Codex's generated/local hooks capture `harness_session_id` and deterministic turn attention but do not report `cwd`; Aider's notifications-command integration reports attention but not `cwd`, and OpenCode's plugin reports session ID plus attention events (idle/permission/busy, issue #104) but not `cwd` (remaining gaps tracked as issues #103/#104).
- `resume`, `resume_options`, `resumed_from`, `harness_session_id_source/confidence/captured_at`.
  Desktop resume choices retain each option's `strategy` through the controller
  and optional `{ strategy }` request body. The sidecar revalidates that choice;
  no selected-strategy field is added to persisted session metadata. Bodyless
  legacy requests retain backend default selection. Exact resume preserves the
  captured native ID. A confirmed non-exact run clears the old ID, last-seen
  time and native-ID source/confidence/capture timestamp until the conversation
  actually opened reports its identity. Cancellation before spawn restores the
  prior ID and its provenance through the existing admission rollback.
- `lineChanges` is an optional **API-only** Git-context projection containing
  `additions` and `deletions` for live uncommitted text changes against `HEAD`,
  including staged/unstaged changes and non-ignored new files (empty-tree
  baseline before the first commit). It is not persisted in `SessionMetadata`.
  Binary files contribute no lines; failed statistics are omitted rather than
  presented as zero. Listings reuse Git context per discovered worktree root,
  including sessions in different subdirectories, and refresh it next listing.
  These are worktree totals, shared across sessions, not session authorship.
- `last_user_input`

`normalize_session_metadata` runs on every read to backfill defaults and reconcile `lifecycle`/`lifecycle_phase` drift between old and new records.

## Status vocabulary

Unlike a typed domain enum, `status`/`lifecycle`/`lifecycle_phase`/`attention`/`connectivity` are plain strings, and the vocabulary has grown organically across the HTTP and runtime layers rather than being centrally enumerated. Known values in current use:

- `status`: `creating`, `running`, `killed`, `ended`, `error`
- `lifecycle`: `creating`, `alive`, `stopping`, `dead` (ADR 0023 canonical form); `ending` also appears as an intermediate value in places
- `lifecycle_phase`: `creating`, `active`, `ending`, `ended` (`default_lifecycle_phase_for_status` derives this from `status` when absent)
- `attention` (raw observed values, before `canonical_attention` collapses them): `working`, `idle`, `blocked`, `failed`, `capped`, `waiting_for_input`, `stale`, `done`; canonicalized to `needs_you` / `idle` / passthrough for the UI-facing attention model
- `connectivity`: `online` (default) and other values set via `connectivity_for_status`
- `terminal_outcome`: `ended`, `killed`, `error`

Because these are untyped strings threaded through many call sites rather than a single enum, adding a new status value does not get compiler-checked exhaustiveness — grep for the field name across `http/` and `runtime/` before assuming a closed set.


## Transition ownership and observation windows

`SessionApplication` owns production application of lifecycle and attention
changes. `runtime/observed_status.rs` supplies the shared attention field
operations and `ProcessTransition` decisions described by ADR 0027. Hook/debug
attention, idle timeout and ordinary process-attention writes arbitrate against
the latest metadata in the retained store's session transaction lane before
publishing a live mirror. Caller-specific runtime generation, source priority,
prompt authority and lifecycle checks remain at the application boundary.
Committed terminal input keeps its generation/freshness bookkeeping and attention
update in the same critical section.

Physical lifecycle changes have a different order. A terminal status first
marks the owned live runtime `ending`, then attempts its durable transition;
failed persistence cannot undo the physical process event or prevent its
finalizer from being scheduled. Finalization writes `ended` metadata before
updating the live handle. During that latter window, `session_view::is_live_session`
must honor the durable terminal signal even if the handle still says `running`
with phase `ending`; resume admission separately rejects an ending live handle.
This reconciliation remains required rather than being a disposable workaround.

A failed finalization write reports failure and retains `ending`; the same
runtime generation can retry after storage recovers. That is retryability of the
existing application operation, not a promise of automatic retry or successful
durability during an I/O failure. Idle sweeps must not mutate either side of an
ending/ended window: production candidates carry the selected runtime generation,
and the apply boundary rechecks current lifecycle and authority before touching
metadata revisions or live attention.

## Terminology boundary

Current intended interpretation:

- `harness`: internal coding-tool integration identity
- `provider_id`: inference-service identity when known
- `model`: selected model when known
- a session is one running or remembered agent session, identified by `id`

This means:

- a harness is not a model provider
- a model provider is optional
- a model is optional
- a session may legitimately know only the harness and not the provider/model

Where provider/model identity cannot be determined, code should preserve `None`/empty rather than invent values.

## Session metadata revisions

Sidecar session replacements persist an opaque string `metadataRevision` in the
canonical session JSON. Storage reads that request a revision return legacy
records at `"0"` without rewriting them. Successful replacements advance the
checked counter, including identical writes and owned native replacements;
failed writes and rejected transactions do not advance it. `SessionMetadata`
remains the legacy field model; the storage `RevisionedSession` envelope owns
revision serialization so snapshot callers cannot select the next revision.

`MetadataStore::try_update_session` checks an expected revision and holds the
owning store's per-session lock through read, mutation, and replacement. It
rejects stale revisions and identity changes and distinguishes missing or
malformed records from revision conflicts. Callers must use the workspace's
retained store; separately constructed readers do not share its process-local
locks or attention ownership. Internal sidecar writers use scoped mutations of
the latest record; attention writes invalidate native ownership even when their
values are identical, while descriptive work, summary, and plan writes preserve it.

The isolated SQLite adapter reuses the same attention-owner identity/counter
rules for conditional hook writes. Its opaque proof binds the worker, session and
creation identity, survives descriptive writes and is revoked by accepted
attention writes. It is published only after commit and is never restored from
database fields after reopen. Shared lifecycle and hook reducers cover durable
metadata decisions; live runtime/source reservations and qualification remain
application-owned. Production continues using the JSON transaction lane.

Attention has separate `attentionSource`, `attentionConfidence`, `attentionOrigin`,
and `attentionUpdatedAt` fields. `metadataSource` and `metadataConfidence` describe
work metadata, whose freshness uses `workMetadataUpdatedAt`. Legacy reads hydrate
both scopes from the old fields and file modification time without rewriting the
record; unknown age preserves authority. Ending snapshots retain attention
provenance. Summary snapshots atomically set or clear `summary`, `summarySource`,
`summaryConfidence`, and `summaryObservedAt`. Agent-reported plan references use
the sidecar-assigned `agent_reported` source and cannot replace a `user_selected`
reference.

Codex UI program status uses `attentionSource: backend_inference` and
`attentionOrigin: codex_terminal`. It is best-effort terminal evidence, not
native approval authority. A runtime-local activation flag prevents late hooks
and competing inference from replacing it; a fresh runtime resets that flag.
Explicit user/agent attention remains higher priority. See [ADR 0093](../adr/0093-codex-terminal-program-status.md).

Authenticated live-session metadata GET/PATCH operations expose the revision and
strict field-scoped updates. Validation-only PATCH does not mutate the record or
ownership. Producer migration remains pending under
[#789](https://github.com/Rambolarsen/orkworks/issues/789); these operations do not
enable native clearing or coordinate external JSON replacements. See the
[approved metadata design](../superpowers/specs/2026-10-08-codex-native-attention-layer-design.md).

## Mapping to the API/UI layer

```text
SessionMetadata (on disk)
  -> SessionHandle / SessionInfo (main.rs, in-memory)
  -> HTTP JSON DTO (http/session_handlers.rs)
  -> apps/desktop/src/api.ts SessionInfo
  -> renderer components
```

Renderer helpers such as `sessionSort.ts` operate directly on the API DTO.
There is no separate renderer session model or attention vocabulary.

## Prior DDD scaffold (removed)

An earlier `domain/session/` + `application/session/` +
`infrastructure/session_*` scaffold was removed in PR #182. Its historical
integration state, deletion context, and useful recoverable types are being
reassessed in [#181](https://github.com/Rambolarsen/orkworks/issues/181); removal
alone does not establish that typed modeling failed or was rejected. The
current consistency assessment in [#402](https://github.com/Rambolarsen/orkworks/issues/402)
keeps the existing application/metadata/runtime owners. A recovered or adapted
typed boundary still requires its own reviewed production integration and
compatibility/recovery design before implementation.

## Workflow observation feedback loop

The durable `WorkflowObservation` contract and Peon/Taskmaster feedback loop
are implemented in `crates/orkworksd/src/`. The current-summary snapshot
(`summary`, `summarySource`, `summaryConfidence`, and `summaryObservedAt`) and
the replacement of summary-checkpoint history remain pending under
[issue #313](https://github.com/Rambolarsen/orkworks/issues/313).

`SessionMetadata` gains a current-summary snapshot: `summary` alongside
`summarySource` (`agent` | `peon`), `summaryConfidence`, and
`summaryObservedAt`, all four written or cleared together — replacing the
durable summary-checkpoint mechanism superseded ADR 0024 added to the event
log. `label` (ADR 0029, extended by ADR 0063) remains outside this snapshot
and outside the `metadata_source`/`metadata_confidence` precedence system;
its `labelSource` provenance lets automatic Peon and Codex title updates
coexist with explicit user labels.

A new, independent record type, `WorkflowObservation`, answers "what made
this work harder than necessary?" rather than "what is happening now?":

```text
WorkflowObservation
  id, sequence, sessionId, observedAt
  kind        repetition | obstacle | missing_context | assumption |
              correction | workaround | verification_gap
  description (<= 500 Unicode scalar values)
  evidence    (<= 2,000 Unicode scalar values)
  reportedImpact  low | medium | high
  source      agent (fixed confidence 0.9) | peon (own confidence)
  confidence, fingerprint, idempotencyKeyHash (not API-exposed)
```

Observations are immutable while retained, workspace-scoped but
session-segmented on disk, and never participate in the `SessionMetadata`
source-priority overwrite rules above — an agent report and a Peon
observation coexist rather than competing. See the "Workflow observations and the current-summary snapshot (partially implemented)"
section of [architecture.md](architecture.md) for storage
paths, routes, and the recording module's interface.

### Recommendation origins: analysis vs observations

Taskmaster produces `ImproveWorkflow` recommendations through two distinct
pipelines with distinct dedupe-key prefixes (the renderer classifies origins
with `recommendationOrigin` in `apps/desktop/src/taskmaster.ts`, mirroring
`is_brain_workflow_recommendation` in `crates/orkworksd/src/taskmaster/mod.rs`):

- **Analysis** (`Proactive`/`rollup` recommendations; `proactive:v1:` and
  `rollup:v1:` dedupe keys) — proposals from the model-analysis pipeline
  (`taskmaster/evaluator.rs`), which compare the workspace's agentic setup
  against signed reference knowledge and repository facts to find missing
  workflow capabilities. The background discovery poll and the
  user-requested `POST /taskmaster/analyze` run the same pipeline; on the
  current data model there is no recorded trigger provenance, so all of
  these recommendations surface as one "Analysis" category (origin badge
  and panel filter in `RecommendationsPanel`). Provenance caveat:
  `rollup:v1:` parents are model-assisted groupings of exact recommendation
  families, which may themselves derive from recorded workflow
  observations — an Analysis rollup reflects observed friction as much as
  knowledge-derived best practices, and the sidecar clears the parent's
  repository/knowledge evidence in favor of member evidence.
- **Observations** (deterministic `ImproveWorkflow` recommendations with
  `improve_workflow:v1:` dedupe keys) — proposals the deterministic
  evaluator (`evaluate_workflow_improvements`) builds by grouping accepted
  `WorkflowObservation` records by fingerprint, requiring recurrence (two
  distinct qualifying observations) before proposing.

### Manual workflow assessment

An assessment attempt has its own workspace-local ID and report; it is separate
from ordinary analysis. Running attempts expose only fixed `discovering`,
`reading_evidence` and `assessing` phases. These phases do not confer execution
authority and contain no discovery prose. A format-2 input manifest binds the
permitted catalog, seed/follow-up evidence identities and discovery selection.
Completed proposals retain the existing `proactive:v1:` recommendation family
and optional assessment provenance. Restart interrupts queued/running attempts
without replaying inference; pending publication uses the existing recoverable
recommendation transaction. See [the assessment architecture](architecture.md)
and [ADR 0086](../adr/0086-repository-led-taskmaster-assessment.md).

### Recommendation audit vocabulary

The recommendation audit adds a third `RecommendationType` discriminant,
`Cleanup` (serde `cleanup`, dedupe key exactly `cleanup:v1`). A cleanup card
is `ImproveWorkflow`-shaped (it still carries the required
`workflowImprovement` placeholder, `targetSurface: documentation`) plus an
optional `AuditCleanup` projection: `entries` (each `{ id, title, criteria }`
with criteria from the `AuditCriterion` set `under_eligible | noise |
duplicate | stale`, in canonical order), `scanned`, `healthy`, and
`staleAfterDays`. `AuditCleanup` exists only on `cleanup`-type records.

When a fresh audit finds no entries, any active proposed cleanup card becomes
`superseded` and remains as terminal history; its audited recommendations are
unchanged. A subsequent non-empty audit creates a new generation linked to
that terminal predecessor.

`DismissalWatermark` gains an additive optional `reason`. The ordinary user
dismiss route never writes it; the cleanup-accept batch writes
`audit:<first-criterion>@<cleanup-card-id>` so a bulk dismissal is auditable
from either side. The renderer surfaces a third origin, `cleanup`, in
`recommendationOrigin` (exact key `cleanup:v1`, or that prefix with a
generation suffix), shown as its own panel filter chip and as the audit
block on the card.

## Related files

- `crates/orkworksd/src/metadata.rs`
- `crates/orkworksd/src/main.rs` (`SessionHandle`, `SessionInfo`, `AppState`)
- `crates/orkworksd/src/http/session_handlers.rs`
- `docs/agents/architecture.md`

## Session organization

The accepted [archive design](../superpowers/specs/2026-10-18-session-archive.md)
and [ADR 0088](../adr/0088-session-archive-state.md) add Active, Closed and Archived
as organization states, independently of runtime status and attention. The
sidecar owns bounded identity-bound `session-state/<id>.json` records. Missing
legacy records derive Active or Closed; unreadable state disables mutations and
protects retention. Active Archive and Delete close the exact captured runtime
and await cleanup plus persisted ending first. Restore returns to Closed.
Archived history is retained outside automatic age/count cleanup. Archive and
Restore preserve activity timestamps and native tool identity. This is OrkWorks
organization only; native archive synchronization is outside this increment.

## Session image context

[Image context](../../specs/session-image-context.md) is a private local bundle,
not a new attention or session lifecycle state. Its ownership manifest binds it
to the generated session ID under workspace metadata `session-images/<id>/`.
Image bytes never enter SessionInfo, summaries, or Peon input. A complete bundle
precedes the creating metadata record. Runtime exit and renderer detachment keep
it available for remembered-session resume; Forget/retention removes it only
before deleting the owning record, while retaining the cleanup lease through
metadata deletion. Failed image removal preserves that record for retry. Recovery
retains uncertain metadata or ownership.

## Session task-worktree binding

Design/specification and implementation-plan selections are shared by canonical
Git common-directory identity and full branch ref within the workspace owner.
Additive `branch-documents/<key>.json` version-1 records retain two anchored
references, source, observation time, establishing HEAD evidence and explicit
pin state. Automatic `git_history` and `git_worktree` sources supplement existing
reports. They do not establish session authorship, attention or workflow approval.
`SessionInfo.planDocuments` projects ordered sanitized steps and sharing context;
`hasOpenablePlan` remains the compatibility aggregate. Legacy `planPath` records
without branch provenance stay session-local. See [ADR 0102](../adr/0102-branch-document-association.md).

`SessionMetadata.worktreeBinding` is an optional sidecar-resolved `{ root, commonDir }`
record written through revisioned `agentMetadata.worktreePath` reporting. It is
independent of launch/resume `cwd`, survives end/restart, and is authoritative
until explicitly changed or cleared. Git enrichment and Changes validate and
use this binding before hook/process/launch directories; unavailable bindings
do not fall back. `SessionInfo.gitContextSource` and capture provenance expose
agent, harness, process or launch origin. See [ADR 0094](../adr/0094-explicit-session-worktree-binding.md).

### Repository readiness snapshots

A readiness attempt is a manual selected-workspace evaluation with an immutable
server-generated ID. Its fixed phases are collecting evidence, reading evidence
and scoring readiness; restart interrupts unfinished work without inference
replay. A completed snapshot retains fourteen nullable maturity ratings, separate
confidence and exact cited evidence. The sidecar computes an equal-weight 0–100
score only at complete coverage. Run history is workspace-local and distinct
from workflow assessment recommendations and their completion provenance.
Historical input identity supports freshness; stable comparison identity permits
meaningful deltas across repository changes. Permission narrowing redacts an
affected snapshot's entire source-derived payload, including its score.
The [readiness specification](../../specs/repository-agent-readiness.md) defines
bounds, identities, retention and publication eligibility.
