# Brain-informed Taskmaster recommendations

The [independent workspace instances proposal](multi-workspace.md) defines a
future installation-scoped path history and one selected workspace per
OrkWorks instance. Taskmaster analysis and diagnostics remain scoped to that
instance's selected workspace; instances do not coordinate focus, attention,
or analysis with peers. It is pending written-spec review and does not describe
implemented multi-workspace behavior.

Status: accepted
Date: 2026-09-09

## Purpose

Taskmaster uses a curated, independently updated knowledge bundle to enrich
observed-friction recommendations and discover improvements quietly. The user
approved this design and implementation in the 2026-09-09 planning session.
This extends [Taskmaster](taskmaster.md); it does not authorize coding, command
execution, session creation, or Git workflow actions requested by Taskmaster in
the background. Installed CLI-managed policy effects are qualified below.

## Knowledge distribution

The Brain publisher exports only reviewed, distilled guidance pages explicitly
named in an application allowlist. It preserves hierarchy, stable page
identities, maturity, applicability, safe provenance, and relationships. Links
never implicitly include excluded pages. An application index is generated
from included pages. Personal or project-specific assessments, project pages,
experiment records, raw research, and repository-specific evidence are always
excluded; there are no per-page exceptions to these exclusions. Review both
paths and contents so names, project details, excerpts, URLs, identifiers,
credentials, or sensitive provenance cannot enter indirectly through links,
metadata, assets, or relationships. No private-repository credential ships in
the app.

After validation, changes publish automatically to the brain's existing Pages
site as immutable signed JSON bundles. A signed versioned manifest identifies
compatible bundles by format version, publication sequence, SHA-256 digest,
relative download path, and `privacyPolicyVersion`. The signed bundle payload
contains the integer `privacyPolicyVersion` and repeats the manifest value so
an offline cache can verify the policy without the manifest; clients require
the manifest and payload values to match. Version `1` is the current strict
allowlist policy, and the packaged starter snapshot carries the same version.
Clients accept only a supported policy version. Bundles without it or with an
unsupported/mismatched version are not eligible for Brain-backed inference,
even when their signatures and digests are valid. Older compatible releases
remain available.
The signing private key is a publishing secret; the app pins the public key.
Missing signing configuration blocks publication, never verification.

The app packages a reviewed starter snapshot in a signed envelope with its
signed manifest attestation, verified by the same pinned public key as feed
bundles. The starter's manifest entry must match its payload version, sequence,
digest, format, and `privacyPolicyVersion`; it must also carry the supported
assessment capability when the publisher's reviewed content qualifies. There
is no unsigned-JSON or app-packaging trust bypass. On a fresh offline install,
the verified packaged starter is eligible without contacting the feed. The app
checks for updates at startup when due, then every six hours while running.
Verify signature, digest, format, size bounds, and content before atomic
activation; retain the previous working snapshot. Failures and offline
operation preserve cached knowledge. Knowledge is reference data, never
executable tools, permission policy, or authority over repository instructions.
Application-binary auto-update remains out of scope.

Bundle eligibility has no independent wall-clock age cutoff. A compatible
active bundle remains usable offline while its signature, digest, format, and
content validate, its privacy policy version is supported, and it includes only
guidance allowed by this distribution contract. The updater activates a newer
verified bundle on its normal schedule; an unavailable feed or failed update
retains the last eligible bundle. An old timestamp alone does not make a
verified offline fallback ineligible.

## Analysis

Preserve deterministic workflow-observation evaluation. Enrichment and proactive
discovery share a separate, explicitly selected Taskmaster provider/model using
existing provider infrastructure. They never change or inherit Peon's selection.
Without a configured available Taskmaster model, deterministic recommendations
continue. There is no automatic provider fallback or model routing in v1.

### Existing CLI logins and managed policy (approved 2026-09-10)

JSON-defined custom inference adapters extend this contract under
[ADR 0055](../docs/adr/0055-json-taskmaster-inference-adapters.md) and the
[reviewed adapter design](../docs/superpowers/specs/2026-09-10-custom-inference-adapters-design.md).
An optional independent `inference` capability preserves custom provider
configuration and model identifiers. User definitions require separate explicit
executable trust before background use; importing JSON cannot approve execution.
Custom adapters are not certified side-effect-free. Trust/definition identity
gates scheduling, cache reuse, and atomic result acceptance; failures never
change Peon or select another provider. Existing built-in safeguards remain.

CLI adapters reuse the installed coding tool's existing login and credential
storage. OrkWorks never copies credentials or requires separate API keys for
Taskmaster. Fixed invocation-local profiles request recommendations only, disable
optional tools, hooks, plugins, and project discovery where supported, and do not
reuse interactive launch/resume arguments. Unsupported or incompatible profiles
fail without fallback or changing Peon.

Administrator-managed CLI policies remain authoritative. Required hooks may run,
managed instructions may add context, and managed routing/restrictions remain in
force. OrkWorks does not modify managed configuration, bypass a policy conflict,
or retry with weaker permissions. Its context limits govern what OrkWorks
collects and supplies, not additional effects imposed by the installed CLI's
managed policy. Settings and documentation disclose this distinction.

Taskmaster does not request coding actions even when managed policies expose
tools. Unexpected tool-action output is rejected, not converted to an accepted
recommendation; rejecting output is not a claim to undo any CLI side effects.

Before inference, retrieve relevant pages from workspace signals and assemble
bounded permitted context. Analyze only the workspace selected by the current
instance. At most one evaluation runs per instance; the eight-call daily limit
remains installation-wide and is enforced by the existing durable reservation
ledger, without peer-instance focus or analysis coordination. The minimum
interval is one hour per workspace. Persist reservations before calling the
provider; failed calls count, restarts cannot reset usage, and an unreadable
ledger defers inference. These are usage limits, not dollar budgets.

The Recommendations panel offers an explicit **Analyze now** action. A manual
analysis uses the same Taskmaster provider, selected workspace, context limits,
knowledge retrieval, evidence validation, and durable reservation ledger as
background discovery. It remains available when automatic background discovery
is disabled, provided a supported provider remains configured. It bypasses the
per-workspace minimum interval because the user explicitly requested the run,
and it does not consume or count against the installation-wide daily evaluation
allowance, which governs background discovery only. At most
one analysis may run at a time, and the existing evidence cache still suppresses
a provider call when the evidence and effective settings have not changed.

Before any Brain-backed provider call, the active bundle must be eligible under
the strict privacy rules above. Apply this gate to background analysis,
**Analyze now**, and **Assess workflow**. Until #529 delivers the reviewed
allowlisted export, generated compliant starter snapshot, and verified signed
publication, fail closed for every Brain-backed analysis; do not send pages from
the legacy starter or cached bundle that lack the supported privacy policy
version. The current starter snapshot is ineligible until replaced by the
compliant snapshot from #529.
Deterministic workflow-observation recommendations continue without Brain
inference. Once an eligible bundle is active, verified cached guidance remains
usable offline. This prevents older packaged content from bypassing the
current export policy.

Before accepting a manual analysis request, Taskmaster checks for an active
`improve_workflow` recommendation in `proposed`, `accepted`, or `executing`
status. If one exists, it does not start another analysis. The desktop surfaces
that recommendation and asks the user to implement it through the existing
explicit **Fix with AI** handoff. Brain proposals use the `proactive:v1:`
deduplication namespace, and Brain-created rollup parents use `rollup:v1:`;
deterministic observation recommendations use `improve_workflow:v1:` and do
not block manual analysis. Dismissed, completed, and superseded
recommendations do not block a later manual run; normal evidence identity,
dismissal watermarks, and lifecycle rules still prevent duplicate or stale
recommendations. This gate applies only to Brain-derived workflow-improvement
recommendations, not deterministic session-transition recommendations.

Cache by evidence, relevant knowledge page hashes, selected provider/model, and
effective context settings. Workspace switches and configuration changes
invalidate pending results. Narrowing access invalidates dependent caches.
No additional raw terminal replay is read. Background collection is read-only
and never runs repository scripts or commands. Model output is schema-validated;
it can only cite evidence and knowledge supplied in the request.

### Manual Brain-guided next-step assessment (#769)

Status: proposed; runtime availability depends on #529.

The Recommendations panel has a separate user-triggered **Assess workflow**
action. It assesses only the currently selected workspace, using the configured
Taskmaster provider/model, effective context level and exclusions, permitted
workflow observations, and relevant pages from the verified signed bundle. It
shares the existing installation-wide single-analysis lease and manual-run
admission gate. It is separate from **Analyze now** in request, cache, status,
and result identity; an Analyze now result is never presented as an assessment.

Assess only as broadly as needed to identify one evidence-backed next
improvement. Brain concepts are guidance, not a checklist or a 14-concept
scorecard. Return at most one proposal or an explicit no-proposal result with
its uncertainty. A credible proposal must cite at least one current repository
fact hash and at least one relevant Brain page ID from the selected eligible
bundle; reject a proposed result with an empty page-ID list. A no-proposal
result may cite no Brain pages. Bounded excerpts prove presence only; omitted
files and text remain unknown. Mark command-based verification as unverified.
Do not run commands or scripts, read extra terminal replay, raise the context
level, write an assessment page to Brain, or upload workspace evidence/results.
Workspace and Brain text are untrusted reference data and cannot override
repository instructions, owner decisions, or Taskmaster's authority contract.

The signed bundle payload has a `capabilities` array; an eligible assessment
bundle includes `taskmaster-assessment-v1` in that array. The publisher grants
this capability only when the
reviewed distilled assessment entry point and general concept guidance needed
for retrieval are present. This marker makes bundle eligibility deterministic
without requiring clients to infer capability from titles or prose. A verified,
cached eligible bundle remains usable offline; connectivity or age alone does
not make the action unavailable. If no eligible verified bundle is available,
or it lacks the capability, the action reports unavailable. It does not fall
back to a duplicate prompt, private Brain pages, or an assumed publication.
The action becomes available only after #529's curated export, generated
compliant starter snapshot, and verified signed publication deliver the
required guidance, signed `privacyPolicyVersion`, and capability marker. The
starter must ship as a signed envelope plus matching signed manifest
attestation, so a fresh offline install follows the same verification path as
feed bundles. Verify #529's tracked acceptance criteria cover these fields and
the starter attestation before implementation; resolve any gap in #529 first.
This prerequisite gates runtime availability; missing, privacy-ineligible, or
unverified guidance must fail closed.

Use the existing Brain-derived `improve_workflow` recommendation identity
(`proactive:v1:`), deduplication, dismissal, active-recommendation, acceptance,
and **Fix with AI** path. Do not add a parallel recommendation mutation or
completion path. If an active Brain recommendation or another analysis holds
the existing admission gate, return it to the user without invoking the
provider. Observation-only recommendations do not become Brain assessment
results. Manual assessments do not consume the background daily evaluation
allowance or workspace cooldown, but remain subject to provider availability,
the shared single-analysis lease, and the existing active Brain
recommendation gate.

If the existing deduplication key belongs to a proposed, dismissed, executing,
accepted, or completed recommendation, return a no-proposal result with a
duplicate-suppression reason and do not mutate or relink that record. If the
matching record is an assessment-derived recommendation terminally superseded
because its inputs became stale or access narrowed, a later valid assessment
may create one successor in the same `proactive:v1:` family. Bind its stable ID
to the dedupe key, predecessor ID, and unique assessment ID, and link it through
`supersedesRecommendationId`. Retries of that assessment remain idempotent; no
other lifecycle state may use this exception.

Persist one latest assessment report per canonical workspace under that
workspace's local metadata root (`~/.orkworks/workspaces/<hash>/`), capped at
64 KiB serialized. Do not put repository-derived evidence or excerpts in the
installation-wide Taskmaster ledger. Include the assessment ID and
outcome, canonical workspace/evidence/configuration identity, observation time,
provider/model identity, Brain bundle version, relevant concept/page IDs,
immutable cited evidence snapshots and hashes, a concise state summary, and
the single proposal or no-proposal reason. Do not persist the full prompt or
uncited workspace files. Replace the report on a new assessment; do not expire
it by age. Deleting that workspace's local metadata deletes its report. Reports
never leave the local OrkWorks installation.

Before returning or using an assessment-derived proposal, revalidate workspace
identity, effective settings, provider/harness identity, bundle version, the
complete bounded input snapshot, and every cited repository fact and page. This
applies before recommendation list/get responses, active-recommendation
admission responses, acceptance, and the **Fix with AI** handoff, without
requiring an intervening status poll. Suppress stale proposals and refuse their
acceptance or handoff. The cache key and stored input identity
cover all data supplied to the model: permitted observations, every collected
repository fact (including uncited facts), the pre-run recommendation snapshot,
selected knowledge pages, context settings and exclusions, workspace generation,
prompt/schema version, provider/model, and full bundle identity. If this
assessment creates a recommendation, the report records its stable ID;
revalidation excludes only that exact assessment-created record from the
current recommendation snapshot, so its own output cannot invalidate the
report while other recommendation changes still do. If existing deduplication
suppresses the proposal, the report has no derived recommendation ID and does
not exclude the pre-existing record. Output-selected citations alone are not a
sufficient cache key. Discard
stale results after a workspace, any input evidence, or relevant configuration
change. A reduction in effective context access, a new exclusion, or a mismatch
in any stored input identity logically invalidates the report and prevents it
from being returned. The read-only status request never deletes or rewrites
state: access-setting mutations redact disallowed evidence before replying.
Global context or exclusion reductions redact every affected workspace-local
report and assessment-derived recommendation before the settings response;
workspace override changes affect only the matching workspace. Persisted data
for unopened affected workspaces must be reconciled before any later status,
recommendation, acceptance, or Fix with AI response can expose or use it. Other
stale reports are removed by the next state-changing assessment or workspace
cleanup.

When a novel proposal creates a recommendation, persistence is crash-consistent
with the existing recommendation store. Before upserting the recommendation,
atomically persist the bounded workspace assessment record with an internal
pending-recommendation marker and the recommendation's stable ID. Keep the
record hidden from status until the existing recommendation is durably upserted
and the assessment record is committed. Under the shared analysis lease, startup
recovery idempotently repeats that upsert and commits the report before the
sidecar serves status; persisted running attempts without pending proposal
mutations follow the existing interrupted-run recovery. If the final report
commit fails while the sidecar remains live, reconcile immediately before
returning from the request. Until reconciliation succeeds, list/get,
active-recommendation admission, acceptance, and Fix with AI must not expose or
act on the pending record; if reconciliation still fails, return unavailable or
service failure and do not start another provider assessment. A pending record
does not count as an active recommendation. Keep pending and committed report
records within the same 64 KiB serialized cap.

When invalidation affects an assessment-derived Brain recommendation, supersede
any still-proposed recommendation, remove it from the active recommendation
surface, and prevent **Fix with AI** from using stale evidence. For this
assessment-derived subset only, `superseded` is terminal and valid only after
`proposed`; it cannot later be accepted or executed. When effective
access narrowing makes evidence disallowed, redact its immutable snapshot from
the report and every lifecycle record that retains it, including proposed,
superseded, dismissed, accepted, executing, and completed recommendations.
Also redact model-generated display fields that may copy or paraphrase source
content, including titles, summaries, reasons, `proposedImprovement`, expected
benefit, and report summary/proposal text. Clear the affected excerpts and
source-derived display text, replacing required strings with a generic
access-redacted placeholder. Preserve record identity, lifecycle transitions,
dismissal decisions, and outcome history.
Ordinary evidence, provider, bundle, or recommendation-input changes still
invalidate reports and supersede proposed recommendations, but do not redact
audit snapshots unless they also narrow permitted access.
Validate the response schema and reject unknown evidence/page IDs, unsupported
fields, more than one next step, executable commands, or claims that unverified
checks passed. If evidence is insufficient, persist and show a no-proposal
result without creating an empty recommendation.

## Evidence and lifecycle

Retain existing session-observation evidence and its recurrence policy. Add
separate repository-fact and knowledge-reference evidence with immutable
snapshots, hashes, timestamps, and bundle version. Do not fabricate session IDs
or recurrences for proactive findings. Every proactive suggestion requires a
current repository fact; old assessments or knowledge alone cannot prove a gap.
Bounded excerpts prove their presence, not that omitted text or files are absent.

Hypotheses remain visibly experimental. Repository instructions and explicit
owner decisions govern applicability. Recommendation identities are based on
the target and underlying evidence, not generated prose or knowledge version.
Knowledge updates alone cannot resurface dismissed suggestions. Analysis does
not otherwise rewrite accepted or completed recommendations. When
effective-access narrowing makes their evidence disallowed, redact the affected
snapshots while preserving lifecycle transitions and outcome history. Store
explicit dismissal decisions and completion outcomes locally. Distinguish
completed work from evidence of benefit. Do not publish local outcomes in v1.

Use the existing explicit Fix with AI handoff into the user's active session.
Include both local evidence and relevant knowledge in its scoped prompt.
Taskmaster never initiates file edits, terminal input, or coding sessions during
background discovery; managed CLI policy effects follow the exception above.

## Settings

Recommendations has global defaults and workspace overrides. The daily usage
limit is installation-wide; a workspace cannot enlarge it.

| Setting | Default |
| --- | --- |
| Background discovery | Enabled once Taskmaster is configured |
| Taskmaster provider/model | Unconfigured, independent of Peon |
| Analysis context | Workflow context |
| Daily background evaluation limit | 8 |
| Minimum workspace interval | 60 minutes |
| Automatic knowledge updates | Enabled |

Context levels are session observations, workflow context (also instructions,
documentation, manifests, and CI configuration), and relevant source code (also
selected source files). Support excluded paths. Ignored files, credential files,
and files resolving outside the workspace are excluded. Symlinks cannot bypass
these limits. Explain that allowed context may be sent to the chosen provider.

Show knowledge version, last successful update, remaining background
evaluations, provider/configuration availability, and expandable recommendation
provenance. The Taskmaster model picker shows suggestions for the selected
provider and continues to allow manual model IDs. Opening Settings does not
discover models or execute a provider command. A user may explicitly refresh
models for built-in Taskmaster providers with live discovery: Codex uses its
app-server model list and Ollama lists models at the Taskmaster selection's
draft URL. Claude Code uses its built-in static catalog. Refresh performs model
listing only; it does not run inference, apply settings, or mutate Peon
selection or runtime state. Use the shared provider-discovery implementation
where safe behind a separate Electron-authorized Taskmaster request, bound to
Taskmaster's selected provider and draft connection settings. Do not send the
Taskmaster UI through Peon's discovery endpoint. Custom providers retain static
suggestions and manual model IDs; Taskmaster does not run custom model-list
commands. Select a typed, code-owned discovery operation from the validated
built-in provider: Codex uses the fixed app-server model-list operation and
Ollama queries the supplied draft URL. Do not resolve a mutable provider
definition to choose a model-list command, including for built-in IDs with a
user-defined `models` command override. Failed or stale refresh results leave
manual entry and the saved model usable and report the refresh failure
separately from analysis status. Peon's existing model refresh remains
unchanged.

Separately show Taskmaster's analysis run status in Recommendations and
Settings. The status distinguishes queued/running work from the latest
succeeded, failed, or interrupted result, and includes provider/model,
timestamps, and a useful failure summary. Persist one bounded run record per
workspace, containing an `activeAttempt` (queued/running, when present) and a
`latestOutcome` (succeeded/failed/interrupted, when present); the status
projection shows the active attempt first, then the latest outcome, then idle.
No selected workspace or an unreadable ledger is unavailable, not idle.
A restart marks a persisted running attempt interrupted and clears a queued
attempt while preserving the prior result, but only after acquiring the same
installation-wide analysis lease used by evaluators. If another instance holds
that lease, leave the record unchanged and retry on the next workspace open or
evaluation admission. Status reads are read-only and do not infer interruption.
Mark an attempt running before
context collection or prompt construction; failures after that point are failed
outcomes even if the provider was not invoked. Pre-evaluation skips clear
queued attempts and preserve prior outcomes, removing empty records. Each
workspace retains one bounded record with only its active attempt and latest
terminal outcome; total record count grows with the number of workspaces used,
and global retention/cleanup are outside this increment. A successful analysis
clears or supersedes only that workspace's previous analysis error; the
workspace-scoped run record is the sole source for displaying analysis outcomes.
Retain the installation-wide legacy `lastError` only to read older ledger files;
stop writing or displaying it as an analysis error. Knowledge-update errors
remain separate. Background cooldown, cache, or eligibility checks that do not
dispatch analysis do not overwrite the last actual outcome. Status is scoped to
the instance's selected workspace. Expose it through a narrow
`getTaskmasterRunStatus()` preload method and Electron-authenticated
`GET /taskmaster/run-status` request. Recommendations polls it alongside its
recommendation list; Settings polls every five seconds and refreshes on window
focus. Do not add peer-instance status or analysis coordination. Errors remain
inline and unobtrusive: no background popups or focus changes. Personal/team
brain connections and exporting local lessons are deferred.

## Validation

- Publication cannot include an excluded page, including through links.
- Tampered signatures/digests, incompatible bundles, interrupted updates, and
  offline startup retain the verified fallback.
- Legacy stored recommendations remain readable; proactive evidence never
  changes session recurrence counts.
- Ungrounded model citations are rejected; a knowledge-only change cannot
  bypass dismissal or rewrite accepted work.
- Daily limits survive restarts; background provider failures consume a reservation.
- Taskmaster suggestions change with the selected provider. Codex and Ollama
  live refresh is explicit, uses Taskmaster draft settings, and never applies
  Peon settings or runs inference. Claude presents its built-in catalog; custom
  providers remain static/free-text and do not run discovery commands.
- Taskmaster reports queued/running/latest outcome separately from provider
  availability, surfaces failures in Recommendations and Settings, scopes
  outcomes to the current workspace, and recovers interrupted attempts after a
  restart without changing focus.
- Manual analysis requests work with background discovery disabled, bypass the workspace cooldown and the daily allowance, and return any active Brain recommendation without invoking a provider.
- All Brain-backed provider analysis, including background discovery, **Analyze now**, and **Assess workflow**, remains unavailable until the active verified bundle carries signed payload `privacyPolicyVersion: 1`, matching the signed manifest, and meets the strict exclusion policy; #529 delivers the compliant starter/export/publication. **Assess workflow** additionally requires the signed `taskmaster-assessment-v1` capability. Deterministic observation recommendations continue. A compatible eligible cached bundle remains usable offline with no independent age cutoff.
- Assessments use only the selected workspace and current permitted context, share the single-analysis lease and active Brain recommendation gate, and do not consume the background daily allowance or workspace cooldown.
- Assessment reports are capped at 64 KiB serialized with one latest report per workspace under its workspace metadata root; their input identity covers every supplied observation, fact, pre-run recommendation, selected page, and effective setting while excluding only their own derived recommendation by stable ID. Stale inputs suppress reports and derived proposals on every recommendation list/get/active-response/acceptance/Fix with AI path, without requiring a status poll; invalidated proposed assessment recommendations transition to terminal `superseded`, and a later valid assessment may create one linked successor with an ID bound to the predecessor and unique assessment. Access-setting mutations redact disallowed evidence and generated assessment text before replying, including every workspace affected by a global default reduction; unopened workspace records are reconciled before later exposure or use. Pending recommendations stay hidden and non-actionable until live or startup reconciliation succeeds. Other stale reports are deleted by the next state-changing assessment or workspace cleanup. Workspace metadata deletion removes reports, and no prompt, uncited files, or report is sent to Brain.
- Workspace/configuration switches discard stale results, and context exclusions
  apply to symlinks, ignored files, credentials, caches, and model requests.
- Changing Taskmaster selection leaves Peon configuration and inference intact.
- Settings, packaging, and explicit handoff integration have regression coverage.
