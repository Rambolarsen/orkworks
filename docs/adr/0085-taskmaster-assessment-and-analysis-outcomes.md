---
type: Architecture Decision
title: Taskmaster assessment reports and explicit analysis outcomes
description: Local assessment protocol, evidence lifecycle, recovery, and empty or cached analysis status.
tags: [orkworks, architecture, taskmaster, assessment]
status: stable
---

# Taskmaster assessment reports and explicit analysis outcomes

- Status: accepted; owner approved the written protocol and linked specification on 2026-10-17
- Deciders: repository owner
- Date: 2026-10-17
- Tracking: [#769](https://github.com/Rambolarsen/orkworks/issues/769), [#909](https://github.com/Rambolarsen/orkworks/issues/909)

## Context

Ordinary Taskmaster analysis may validly return no proposals. Its generic success
status does not distinguish an empty result from duplicate suppression, and an
accepted manual request can be skipped by the evidence cache while the previous
actual outcome remains displayed. The owner approved clearer outcomes and a
separate manual assessment that selects at most one grounded workflow improvement.

The [assessment specification](../../specs/taskmaster-knowledge.md#manual-brain-guided-next-step-assessment-769)
is authoritative for evidence identity, lifecycle, redaction and recovery. This
ADR extends [ADR 0071](0071-taskmaster-model-refresh-and-run-status.md) additively;
its existing actual-analysis status, locking and recovery decisions stand.
Signed knowledge eligibility follows [ADR 0081](0081-signed-taskmaster-knowledge-activation-proof.md)
and #529/#902. No new network transport or inference authority is introduced.

## Decision

### Distinct manual assessment

Add Electron-authorized `POST /taskmaster/assess-workflow` and a distinct
assessment projection on read-only `GET /taskmaster/run-status`, with narrow
preload methods and independent renderer/main types. Reuse the selected
Taskmaster provider, context/exclusions and installation-wide analysis lease.
Require server-reverified policy-1 knowledge proof and signed
`taskmaster-assessment-v1` capability. Manual assessment bypasses only background
cooldown/accounting; provider availability and active Brain recommendation gates
remain binding. Observation recommendations do not block assessment.

Decode a versioned strict provider result with a concise state summary and one
of `proposed` or `no_proposal`. A proposal uses existing target-surface/title/summary
and evidence-reference fields, with at least one supplied repository hash and
one selected Brain page ID. A no-proposal result carries a required reason and
may have no citations. Bound all model prose; reject unknown fields, unknown
citations, multiple proposals, executable commands and claims that unperformed
verification passed. Include unverified checks as descriptions, never commands.
The exact provider envelope has required fields `version: 1`, `outcome`,
`summary`, `repositoryFactHashes`, `knowledgePageIds`, `unverifiedChecks`,
`proposal`, and `noProposalReason`. `proposal` is either null or one object with
exactly `targetSurface`, `title`, and `summary`. For `proposed`, that object and
nonempty fact/page lists are required and `noProposalReason` is null. For
`no_proposal`, `proposal` is null and the reason is nonempty; reference lists may
be empty. Limit references to 32 repository hashes and 8 page IDs, each unique;
limit unverified checks to 4 descriptions of 200 characters each. Summary,
proposal summary, and reason are at most 1,000 Unicode scalar values each;
title is at most 240. Reject control characters and a complete response above
64 KiB before parsing. These field bounds do not waive the complete stored-record
cap. Never manufacture a recommendation to avoid an empty outcome.

### Bounded local report and compatibility

Use one optional version-1 `assessment.json` record under each canonical
workspace's metadata root. Include assessment ID, pending/committed state, outcome,
provider/model, bundle identity, observation time, input identity, cited immutable
snapshots, summary, proposal or no-proposal reason, and the exact derived
recommendation ID when one was created. Cap the complete serialized pending and
committed records at 64 KiB. Enforce per-field bounds and reject a record that
still exceeds the cap before writing or exposing either mutation. Never truncate
identity, citations, or a pending recommendation silently. Do not persist the
full prompt or uncited workspace excerpts. Retain until replacement or workspace
metadata deletion; no wall-clock expiry.

Missing report fields mean no assessment for legacy workspaces. Unknown versions,
corruption or unreconciled pending mutations produce unavailable, not an empty
success. Preserve the previous committed record on failed atomic replacement.
Existing recommendations remain readable with optional assessment provenance,
but an older binary cannot enforce assessment freshness semantics. Before the
first report is created, ordinary binary rollback is possible. Afterward, the
supported recovery disables new assessment admission while retaining consumer
fences and forward-fixes the implementation; binary downgrade and manual metadata
repair are unsupported. Document this operational boundary before availability.

### Complete identity and evidence access

Fingerprint the canonical workspace/generation, effective settings/exclusions,
provider/model and captured harness/transport identity, full verified bundle
identity, prompt/schema version, selected pages, permitted observation content,
all collected facts including uncited facts, and the pre-run recommendation
snapshot. Persist bounded input identities/hashes, not uncited excerpts.
Exclude only the exact assessment-created recommendation ID when rebuilding the
recommendation-input snapshot. A dedupe-suppressed proposal has no derived ID and
excludes no pre-existing record. An ordinary-analysis cache entry cannot satisfy
an assessment request.

Revalidate before dispatch, apply, every recommendation list/get and active gate,
acceptance and Fix with AI. Stale reports are hidden by read-only status; stale
proposed assessment recommendations become terminally superseded and cannot be
accepted or handed off. Ordinary changes preserve audit snapshots. Access
narrowing redacts disallowed snapshots and every source-derived display field
in all retaining lifecycle states, preserving identity and outcome history.
Global settings reductions reconcile every affected known workspace, including
unopened ones, before replying; workspace overrides affect their workspace.
Reconcile persisted pending narrowing before any later consumer can expose data.

### Existing recommendation transaction and recovery

Use `proactive:v1:` and the existing recommendation lifecycle and completion
route. Existing ordinary lifecycle identities suppress a duplicate without
mutation/relinking. Only an assessment-derived record terminally superseded by
input invalidation permits a successor whose stable ID binds the dedupe family,
predecessor and unique assessment ID; retries are idempotent.

Under the shared lease, atomically save a hidden pending report with the stable
recommendation ID and bounded pending upsert, durably upsert the recommendation,
then commit the report. Startup replays an unfinished upsert idempotently before
serving status. A failed final commit triggers immediate reconciliation. Until
it succeeds, report/status and recommendation consumers fail closed for pending
data, and no new assessment dispatches. An unrelated interrupted running attempt
uses existing run recovery. A pending record is not an active recommendation.

### Explain ordinary empty and cached outcomes

Keep the latest actual analysis outcome unchanged when no inference dispatches.
Add optional fixed-category created/duplicate-suppressed counts to new ordinary
success records. Older outcomes without counts remain generic success; never
infer zero proposals from today's empty recommendation list. Store no model
prose or repository evidence in the installation-wide ledger.

Record a bounded separate latest-manual-request disposition for a cache hit or
pre-dispatch skip, keyed to that request ID and time with a fixed code, so the UI
can clear the request banner and say that inputs were unchanged only when the
server confirms the cache hit. New manual admission clears the prior disposition;
background polls do not replace it or the last actual outcome. Distinguish
no new proposals, all proposals suppressed, cached reuse, failure and interruption.
Do not label a skipped request a successful new analysis.

Assessment cached reuse displays its own prior committed report and explicit
reuse status. Model summaries and no-proposal reasons remain workspace-local,
are treated as untrusted display text, and follow access redaction.

## Consequences

A useful assessment can depend on current repository facts without requiring a
recurring session problem. Insufficient evidence remains a valid, explained
result. Users can tell whether work ran, returned no new proposals, or reused
cached inputs. Existing admission, provider selection and explicit Fix with AI
boundaries remain intact.

The feature adds durable protocol and consumer revalidation work. Transaction,
redaction, restart and stale-consumer tests are required before availability;
UI-only status tests cannot prove those properties. No assessment edits files,
runs repository commands, starts/resumes sessions, or publishes local results
to Brain. Permitted context still goes only to the selected inference provider
under the existing user-configured transport.
