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

The brain publisher exports only pages explicitly named in an application
allowlist. It preserves hierarchy, page identities, maturity, applicability,
provenance, and relationships. Links never implicitly include excluded pages.
An application index is generated from included pages. Personal assessments,
project pages, experiment details, and research are excluded unless individually
selected for distribution. No private-repository credential ships in the app.

After validation, changes publish automatically to the brain's existing Pages
site as immutable signed JSON bundles. A signed versioned manifest identifies
compatible bundles by format version, publication sequence, SHA-256 digest,
and relative download path. Older compatible releases remain available.
The signing private key is a publishing secret; the app pins the public key.
Missing signing configuration blocks publication, never verification.

The app packages a reviewed starter snapshot and checks for updates at startup
when due, then every six hours while running. Verify signature, digest, format,
size bounds, and content before atomic activation; retain the previous working
snapshot. Failures and offline operation preserve cached knowledge. Knowledge
is reference data, never executable tools, permission policy, or authority over
repository instructions. Application-binary auto-update remains out of scope.

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
but still consumes the installation-wide daily evaluation allowance. At most
one analysis may run at a time, and the existing evidence cache still suppresses
a provider call when the evidence and effective settings have not changed.

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
Knowledge updates alone cannot resurface dismissed suggestions. Accepted and
completed recommendations are not rewritten by analysis. Store explicit
dismissal decisions and completion outcomes locally, distinguishing completed
work from evidence of benefit. Do not publish local outcomes in v1.

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
| Daily evaluation limit | 8 |
| Minimum workspace interval | 60 minutes |
| Automatic knowledge updates | Enabled |

Context levels are session observations, workflow context (also instructions,
documentation, manifests, and CI configuration), and relevant source code (also
selected source files). Support excluded paths. Ignored files, credential files,
and files resolving outside the workspace are excluded. Symlinks cannot bypass
these limits. Explain that allowed context may be sent to the chosen provider.

Show knowledge version, last successful update, remaining daily evaluations,
model/configuration availability, and expandable recommendation provenance.
Errors appear unobtrusively in Settings; no background popups or focus changes.
Personal/team brain connections and exporting local lessons are deferred.

## Validation

- Publication cannot include an excluded page, including through links.
- Tampered signatures/digests, incompatible bundles, interrupted updates, and
  offline startup retain the verified fallback.
- Legacy stored recommendations remain readable; proactive evidence never
  changes session recurrence counts.
- Ungrounded model citations are rejected; a knowledge-only change cannot
  bypass dismissal or rewrite accepted work.
- Daily limits survive restarts; provider failures consume a reservation.
- Manual analysis requests work with background discovery disabled, bypass only the workspace cooldown, still consume the durable daily allowance, and return any active Brain recommendation without invoking a provider.
- Workspace/configuration switches discard stale results, and context exclusions
  apply to symlinks, ignored files, credentials, caches, and model requests.
- Changing Taskmaster selection leaves Peon configuration and inference intact.
- Settings, packaging, and explicit handoff integration have regression coverage.
