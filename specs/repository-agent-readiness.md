# Repository agent readiness

Status: accepted; owner approved the concrete design and runtime implementation in chat on 2026-10-25. Exact public rubric export review remains required before publication.
Tracking: [#1063](https://github.com/Rambolarsen/orkworks/issues/1063).

## Outcome and scope

Manually assess the selected workspace, show one overall agent-readiness number,
inspect its fourteen component ratings, apply improvements through ordinary work,
and rerun to see evolution. A run describes observed readiness at a particular
repository snapshot; it is neither a general code-quality score nor a guarantee
of future agent performance.

This proposal adds a distinct **Assess readiness** action and history within the
existing Recommendations surface. **Assess workflow**, **Analyze now**, their
reports, recommendations and admission rules retain their accepted behavior.
Readiness runs do not create recommendations or execute fixes in this first
version. Details describe possible next improvements; the user can use the
existing workflow assessment and coding-session paths separately.

The architecture rationale is in [ADR 0095](../docs/adr/0095-repository-agent-readiness-history.md).
The [implementation plan](../docs/superpowers/plans/2026-10-25-repository-agent-readiness.md)
is proposed, not runtime delivery evidence.

## Rubric and overall score

Use these stable concept identities, in this order, from the Brain's
[repository assessment playbook](https://github.com/Rambolarsen/brain/blob/main/playbooks/assess-repository.md):

1. `context-discovery`
2. `progressive-disclosure`
3. `instruction-scoping`
4. `architecture-awareness`
5. `domain-knowledge`
6. `environment-reproducibility`
7. `tool-accessibility`
8. `task-decomposition`
9. `verification`
10. `observability`
11. `change-safety`
12. `feedback-loops`
13. `durable-memory`
14. `human-oversight`

Each rated concept has an integer maturity from 0 through 4, qualitative
confidence (`low`, `medium`, `high`), citations, a concise rationale, limits and
a suggested next transition. The shared progression is implicit, discoverable,
usable, integrated and adaptive. The reviewed concept-specific descriptions,
including progressive disclosure's principle mapping, govern the rating.
Do not infer maturity 4 from a policy that merely promises future feedback.

OrkWorks adds an equal-weight overall summary to the Brain's individual ratings.
The sidecar computes `floor((100 * sum + 28) / 56)` only when all fourteen
concepts are rated. Examples: fourteen zeroes = 0; fourteen twos = 50; fourteen
threes = 75; fourteen fours = 100. Retain the integer raw total out of 56 so
rounding is inspectable. Never accept an overall score supplied by the model.

Confidence is independent of maturity and does not numerically weight it.
Missing evidence is not maturity zero. A concept with insufficient permitted
evidence has `maturity: null`, `confidence: null` and a specific reason; it is
**Unassessed**. A successful partial run shows coverage, for example **12/14
areas assessed**, and individual rated areas, without an overall number. Do not
normalize across fewer areas, carry forward previous ratings or substitute zero.
Zero requires evidence supporting the rubric's lowest level; an omitted file,
truncated excerpt or incomplete catalog alone cannot establish absence.

Having repository access generally permits many ratings, but does not prove that
builds, tests, external services, GitHub protections or every coding tool work.
An inspected workflow is evidence of configuration; an actual retained check
result is evidence of that check only at its recorded identity. A file may
contain a historical test result without proving that the current checkout
passes. Report these distinctions and adjust confidence rather than inventing
verification. Assessment itself runs no tests, scripts, tools or GitHub requests.

## Reviewed rubric delivery

Continue using the strict allowlisted, signed Brain distribution contract in
[Taskmaster knowledge](taskmaster-knowledge.md#knowledge-distribution). The
current packaged starter contains maturity models for only three concepts and
cannot support this feature by itself.

Require a new signed `taskmaster-readiness-v1` capability and an explicitly
validated rubric descriptor in the bundle. The descriptor contains rubric ID,
revision, all fourteen stable IDs, their signed page IDs/hashes and reviewed
level-0-through-4 descriptions. Capability without a complete valid descriptor
is ineligible. Descriptor identity hashes its exact canonical content. A wording,
level mapping or concept change changes that identity; the application validates
exactly this fourteen-concept contract. Do not add mandatory concepts silently.

The publisher must review and allowlist only general, distilled rubric guidance,
including the principle-to-concept mapping. No project assessments, projects,
experiments, private references, credentials or repository evidence may enter
publication. Private Brain access in this development session grants no runtime
access. Delivery uses bundle format 2, adding required `readinessRubric` to the exact
format-1 bundle fields. Keep manifest format 1, activation format 1, signature
algorithm/key and privacy policy 1 unchanged; manifest entries already carry a
bundle format. Retain immutable format-1 bundles. Old clients select supported
format 1 and ignore format 2; new clients support both, select the highest
compatible sequence, and require format 2 for readiness. A format-2 bundle also
preserves reviewed workflow-assessment guidance/capability so activation does not
disable the existing action. Unsupported or invalid new bundles preserve the
last verified compatible activation; new clients fail readiness unavailable if
only format 1 is available. Do not
patch the starter by hand or copy an unreviewed rubric into application prompts.
`readinessRubric` is a strict object with exactly `id` (literal
`brain-agent-readiness`), `revision` (positive safe integer) and `concepts` (the
fourteen ordered entries). Each entry has exactly `conceptId`, `pageId`,
`pageSha256` and `levels` (five nonempty strings ordered 0–4, each at most 500
UTF-8 bytes). Page IDs/hashes resolve to included reviewed signed pages; level
wording must be reviewed against those pages, not inferred by the client.
Cap the RFC 8785 canonical JSON descriptor at 40 KiB; its SHA-256 is
`rubricDigest`. Reject unknown fields, malformed Unicode, duplicate IDs, wrong
order, invalid level arrays, unresolved pages/hashes and unsupported rubric IDs.
New consumers validate this descriptor only after full existing bundle proof
verification. The public exporter allowlist gains an explicit reviewed rubric
mapping alongside reviewed general pages; links never expand that mapping.
Review publisher schema compatibility and the exact export before publication;
this specification does not authorize publishing Brain content.

Package the approved signed starter and matching attestation. Verified compatible
cache/starter operation remains available offline with no separate age cutoff.
Include all rubric definitions in the bounded provider input; retrieval cannot
silently omit low-signal concepts. Rubric capability is independent of the
existing `taskmaster-assessment-v1` capability.

## Collection, calls and authority

Reuse the selected Taskmaster provider/model/effort, transport trust, effective
context level/exclusions and the existing installation-wide single-analysis
lease as implemented today. The independent-workspace proposal's future
per-instance lease is not implemented by this feature; no peer-instance state
or new lease mechanism is introduced. Readiness is manual only and bypasses background cooldown and daily
allowance, not provider/trust/lease or workspace checks. It produces no
recommendation, so an active Brain improvement does not block a readiness run;
it still blocks existing analysis actions under their current rules.

Use the repository evidence owner from the
[repository-led assessment design](../docs/superpowers/specs/2026-10-18-repository-led-brain-assessment.md).
Reuse its safe collection machinery, not its workflow-specific `InputManifest`
validator or discovery-restoration semantics. Parameterize the existing owner
with a code-owned purpose/version; retain workflow versions and behavior while
readiness owns its manifest and discovery schema. Preserve traversal/depth,
UTF-8 path, catalog, instruction closure,
no-symlink, credential/ignore filtering and aggregate bounds: seed at most
16 files/24 KiB, one follow-up selection of at most eight catalog IDs, at most
32 files/8 KiB per file/64 KiB total, verbatim excerpts at most 2,000 characters,
prompts at most 128 KiB and responses at most 64 KiB. Rubric text counts against
the prompt bound. Do not enlarge access to obtain a complete score.

At session-observations context, return unavailable without inference. At
workflow-context/source-code levels, rate only supported areas; bounds or
exclusions may yield a partial result. Discovery selects supporting evidence
across all fourteen concepts rather than one candidate improvement. It does not
short-circuit at the first missing improvement. Use separate code-owned strict
readiness discovery/final schemas; never parse a workflow assessment as a score.
Native Codex receives the corresponding code-owned output schemas through the
existing constrained transport; application decoding remains authoritative.

At most two calls: one evidence-selection call and one final scoring call, no
retry, tool loop or fallback. If seed input cannot be assembled safely, return
unavailable before inference. If a permitted selection is interrupted or changes,
fail the run rather than score a partially read requested set. Existing invocation
timeouts apply: at most 240 seconds configured provider time plus existing
bounded collection/cleanup work. No raw terminal replay, command execution,
network fetch, file edits, session creation/resumption or permission widening.
Installed managed CLI policy limitations remain as documented for Taskmaster.

## Response and input identity

The final response is a strict version-1 envelope containing exactly fourteen
unique entries in canonical order and a bounded state summary. Each entry carries
concept ID, nullable maturity/confidence, supplied repository fact references,
its rubric page reference, rationale, evidence limits and a next-improvement
description. Use supplied fact IDs mapped to exact path/prefix/excerpt hashes,
not arbitrary paths. Every rated entry cites at least one supplied repository
fact and its actual rubric definition. An unassessed entry needs a reason and
may cite no repository facts; it still identifies its rubric definition.

Reject unknown fields/IDs, duplicate or missing concepts, noninteger/out-of-range
ratings, invalid null combinations, unsupported confidence, fabricated citations,
control characters, executable action output and claims of unperformed
verification. Limit state summary to 1,000 Unicode scalar values; per concept,
rationale and next transition to 500 each and limits to four 200-character
entries. Per concept allow at most eight distinct fact references; deduplicate
saved evidence across rows under the existing 32-fact bound. The complete saved
snapshot must fit 64 KiB; field bounds never waive the total. Invalid/oversized
output fails before history publication, preserving the previous history.

Bind the full existing captured inference identity, including custom definition
fingerprint/path, trust/runtime generations or native profile/document revision,
selected settings/workspace key and full verified knowledge proof. Reuse the
existing guarded dispatch/commit mechanism rather than comparing display model
strings. Stable comparison conditions include definition/profile identities but
exclude transient runtime generation; generation changes still interrupt an
in-flight run. A bundle update changes provenance; it suppresses comparison only
when the rubric or other supplied guidance hashes change, avoiding false
progress from changed guidance and pointless resets for unrelated bundle pages.

Capture canonical workspace path, current workspace instance, Git HEAD (nullable
for unborn/non-Git workspaces), dirty state (`true`, `false`, or unknown), exact
permitted evidence manifest, observation identity, rubric identity and bundle
version, provider/model/effort/transport identity, context/exclusions and
collector/prompt/response/scoring versions. Read Git identity through the
existing libgit2 boundary; do not spawn Git or infer clean state on read failure.
HEAD is context, not a substitute for evidence hashes in a dirty worktree.

Revalidate permitted inputs and all captured identities before each call and
atomic publication. Changed selected workspace, HEAD/dirty state, settings,
provider/trust, rubric or collected inputs interrupt the attempt. Unsupplied
source remains unknown; never claim this fingerprint covers every repo file.

## History, comparisons and freshness

Keep history scoped to the canonical workspace, including its particular
worktree. No remote URL joins, cross-worktree aggregation or dependency on the
proposed repository registry (#810). Reopening the same canonical workspace
restores its history; removing its local metadata removes its history.

Every completed full or partial result becomes an immutable snapshot with ID,
start/completion time, identities, bounded cited evidence and fourteen rows.
A manual rerun always invokes the provider after admission, even for unchanged
inputs; label identical-input reruns so model variability stays visible. Polls
and opening history never invoke inference. Failed/interrupted runs never become
score snapshots or replace the latest completed result.

Default comparison is against the preceding completed full snapshot with the
same rubric, provider/model/effort/transport, effective access settings and
collector/prompt/schema/scoring versions, in this workspace. The evidence and
commit are expected to differ. Show overall delta as the difference of displayed
rounded totals and each component delta. A zero delta is valid. A user may select
any retained older snapshot; incompatible pairs remain viewable side by side,
label the changed comparison conditions and suppress improvement/regression
arrows and an overall delta. Partial runs show their assessed rows and coverage,
with no overall delta. Confidence changes are shown separately from maturity.

Label historical snapshots **Assessed at ...**, with commit/dirty/unknown state.
Editing the repository does not erase history: it marks the last score **Changes
since assessment — rerun to update** when revalidation detects changed permitted
inputs. A current dirty snapshot may still be current if its captured evidence
is unchanged. Historical evidence is displayed as a saved excerpt, never a
live-file claim. A higher score describes the new assessment; do not claim a
specific fix caused it or attribute a point change to a session automatically.

## Persistence, narrowing and recovery

Add separate version-1 `readiness-history.json` and `readiness-run.json` in the
existing workspace metadata directory. Never rewrite/migrate `assessment.json`
or link readiness records into existing recommendation provenance.

The history is one atomic file with at most 100 completed snapshots, each at
most 64 KiB, and an 8 MiB total serialized/read cap. On successful append, evict
oldest snapshots only as needed to enforce the 100-entry/byte cap; show the
retention limit in history. Retain identity and snapshots together, without
silently truncating an individual entry. Deduplicate idempotent completion by
run ID. An atomic replacement failure leaves the last committed history intact.
Unknown/corrupt versions fail unavailable and are not overwritten as empty data.

The separate bounded 64-KiB run record carries identity, fixed progress phase,
state and sanitized failure, never full prompts, catalogs or uncited excerpts.
Durably mark a run active before dispatch. Restart marks queued/running attempts
interrupted without replaying calls. History commit is the success boundary:
if history contains the run ID after a crash, reconcile the run as succeeded;
otherwise interrupt it. No pending recommendation transaction is needed.

Historical retention does not override current access policy. Under the existing
settings/workspace lock ordering, extend access-narrowing reconciliation to every
known workspace readiness history, including unopened workspaces, before the
narrower setting is exposed. For each snapshot/run compare saved access policy against new effective settings
using the existing `assessment_access::access_narrowed` predicate: lower context
rank, or a new exclusion not covered by a saved exclusion. Redact the whole
snapshot even if its cited paths appear unaffected. Missing/unreadable policy or
workspace origin blocks exposure/settings completion; never presume no impact.
For any affected snapshot, remove all source-derived
prose, paths/hashes/excerpts, row ratings/confidence, totals and manifest details;
retain only fixed audit IDs/times/version and a redacted marker. Remove it from
comparisons. Cancel affected pending runs and redact their source-derived fields.
Conservatively redacting an entire affected snapshot is acceptable; retaining
its score after its basis became disallowed is not. Re-expanding access never
restores the removed data; a fresh run is required. Reads reconcile pending
narrowing before exposing history. Ordinary repo edits preserve historical data.

Missing readiness files mean no history on old workspaces. The new file formats
are additive, but older binaries cannot enforce readiness redaction. Before any
readiness metadata exists, ordinary rollback is possible. Afterward, use a
forward fix with readiness admission disabled and read/redaction fences active;
downgrading with retained readiness metadata or hand-editing it is unsupported.

## Concrete wire and serialization contract

New objects use camelCase and reject unknown/duplicate fields. Times are canonical
UTC RFC 3339 strings; run/snapshot IDs are server-generated UUIDs. `concepts` is
always in the fourteen-concept order above. Bounds in earlier sections apply.

The discovery response is exactly `{version: 1, requestedFileIds: number[]}`,
with up to eight distinct permitted catalog IDs. An empty selection proceeds to
final scoring from seed evidence; there is no one-candidate/no-candidate gate.
The final response is exactly `{version: 1, summary: string, concepts: Row[]}`.
A Row has exactly `conceptId`, `maturity`, `confidence`, `repositoryFactIds`,
`rubricPageId`, `rationale`, `limits`, `nextTransition`, `unassessedReason`.
`unassessedReason` is null for a rating and nonempty (at most 500 characters)
when maturity/confidence are null; other prose may explain that limitation.
`repositoryFactIds` uses unique per-run integer IDs for the ordered collected
facts; `rubricPageId` must match the descriptor for that concept. References are
resolved to hashes/snapshots before persistence. The model supplies no score,
run identity, commit, provider identity or timestamps.

A version-1 history object has exactly `formatVersion: 1` and `snapshots`.
Each snapshot has `id`, `startedAt`, `completedAt`, `redacted`, `data`.
For redacted snapshots `data` is null. Otherwise `data` has `workspacePath`,
`git` (`headOid`, `dirty`), `inputFingerprint`, `comparisonFingerprint`,
`inputManifest`, `inferenceIdentity`, `knowledgeIdentity`, `rubricDigest`,
`versions`, `contextLevel`, `excludedPaths`, `summary`, `concepts`,
`repositoryEvidence`, `knowledgeEvidence`, `coverage`, `rawTotal`, `overallScore`.
Nullable Git fields preserve unknown/unborn states. Stored rows use the same
fields as the final Row, replacing `repositoryFactIds` with
`repositoryFactHashes` into deduplicated saved evidence. Each saved repository
fact adds `factId`, the canonical hash of its path, prefix hash and exact
excerpt, so individual ratings resolve to their precise supporting snapshots. Knowledge evidence records its signed page hash and bundle provenance.
`rawTotal`/`overallScore` are null unless coverage is 14. Redacted records contain
only fixed IDs, times, version and marker, not provenance-derived fields.

Readiness manifest version 1 uses the existing bounded file entry layout and
catalog completeness/digest, but owns `purpose: "readiness"`, collector/prompt
versions, ordered selected IDs, discovery digest and permitted observation
IDs/content hashes (at most the existing newest 128 useful observations). Keep
observations and catalog contents memory-only; serialize their ordered digests.
All persisted hashes are lowercase SHA-256. Compute fingerprints from RFC 8785
canonical JSON: inputFingerprint includes all captured run inputs described
above, while comparisonFingerprint includes exactly the stable comparison
conditions above, excluding repository facts, Git identity, observations,
workspace-instance number, run timestamps and transient runtime generation.
Version fields name collector, discovery schema, final schema, prompt and scoring
versions separately. Include all uncited collected file prefix/excerpt identities
in inputManifest, without uncited contents. Supplied guidance hashes are part of
both fingerprints. Discovery digest binds its validated canonical response.

The run record is exactly `formatVersion: 1`, `id`, `state`, `phase`, `startedAt`,
`completedAt`, `inputIdentity`, `errorCode`. `inputIdentity` is null before capture
or after redaction, otherwise the bounded captured fingerprint/manifest and
workspace/access/transport/knowledge identity without excerpts. Fixed phases are
`collecting_evidence`, `reading_evidence`, `scoring_readiness`; no model prose.
States transition queued → running → succeeded/failed/interrupted. The state
may move directly queued → interrupted on restart. Terminal records have null
phase and a completedAt; failed/interrupted records use fixed error categories.
Successful partial results still have succeeded state. Never expose failed
attempt data as a completed snapshot.

Authenticated POST with a nonempty body returns 400; unauthorized requests use
the existing Taskmaster authorization status. Other admission responses use 200
with exactly `{outcome, runId, reasonCode}`. Outcome is `scheduled`,
`already_running` or `unavailable`; runId is the server UUID for a scheduled run,
the active readiness run ID when known for already_running, otherwise null.
ReasonCode is a fixed category, never repository text. A scheduled response
follows durable queued state and lease acquisition. Simultaneous requests do not
queue another job: the second returns already_running. No client ID is accepted;
explicit later reruns get new IDs, and completion retries deduplicate by run ID.

Authenticated GET accepts no target path and returns 200 with exactly
`{workspacePath, run, snapshots, freshness, retentionLimit, reasonCode}`.
The selected-workspace echo is mandatory; clients discard responses from an
obsolete selection. `run` is the sanitized run projection or null; snapshots
are newest first under the saved caps; retentionLimit is 100. Freshness is
`current`, `changed`, `unknown`, `none` or `redacted`, with a fixed reasonCode.
No workspace returns empty snapshots/null run and freshness none. Corrupt or
unreconciled metadata returns no source-derived payload and an unavailable
reason; it is not rewritten. Compare history client-side only using the
server-computed comparisonFingerprint/validated scores.

On opening Recommendations, changing workspace and each existing five-second
status polling cycle while the surface is visible, GET safely reconstructs the
latest snapshot's permitted manifest under current policy. It neither reruns
inference nor rereads historical snapshots' source files. Unchanged evidence and
Git metadata yield current; detected changes yield changed; inaccessible files,
missing providers or inability to revalidate yield unknown, never current. Saved
historical snapshots remain visible under current access. A failed new run does
not change this freshness calculation or erase the last successful result.

All publication, status/read reconciliation and settings narrowing use the
existing Taskmaster persistence guard before current-workspace lock order;
acquire the single analysis lease before admission, without waiting while
holding workspace/configuration locks. Existing transport guards retain their
harness lease ordering. No separate readiness mutex is acquired ahead of these
locks. Hold guards only for capture/validation/atomic writes, never provider I/O.
Publication rechecks access/selection/inference identity under those guards,
atomically appends history first, then marks the run succeeded. Narrowing uses
the same serialization, redacts history and run before publishing new settings,
and leaves its pending-narrowing fence on failure. A concurrent result can never
restore redacted data. Workspace switch or settings/trust changes invalidate the
attempt; cleanup releases the lease and records interruption without resuming
any user session. Recovery reconciles history/run by run ID as specified above.

## Presentation and acceptance

Use the existing Recommendations surface and fixed-shell navigation. Show the
latest overall number prominently, its date, coverage and current/historical
status, then expandable component ratings and evidence. History lists retained
runs and permits baseline selection. Progress uses fixed phases **Collecting
evidence**, **Reading evidence**, **Scoring readiness**. Failure leaves the prior
score visibly historical rather than clearing it or presenting it as a new run.
Partial and redacted states cannot display an overall number. No new central
context, shell tabs, floating panel or split terminal is introduced.

Acceptance requires deterministic scoring/protocol/privacy/recovery tests and
selected-model quality evidence as detailed in the implementation plan. The
feature is unavailable until complete signed rubric delivery and those gates
pass. Proposed specification merge alone does not establish runtime or installer
availability.
