# Brain-informed Taskmaster recommendations

Status: accepted
Date: 2026-09-09

## Purpose

Taskmaster uses a curated, independently updated knowledge bundle to enrich
observed-friction recommendations and discover improvements quietly. The user
approved this design and implementation in the 2026-09-09 planning session.
This extends [Taskmaster](taskmaster.md); it does not authorize coding, command
execution, session creation, or Git workflow actions in the background.

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

Before inference, retrieve relevant pages from workspace signals and assemble
bounded permitted context. Analyze only the currently open workspace. At most
one evaluation runs across the application. Defaults: eight calls per UTC day,
one hour minimum between evaluations of a workspace. Persist reservations before
calling the provider; failed calls count, restarts cannot reset usage, and an
unreadable ledger defers inference. These are usage limits, not dollar budgets.

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
Include both local evidence and relevant knowledge in its scoped prompt. No
background process edits files, types into terminals, or starts coding sessions.

## Settings

Recommendations has global defaults and workspace overrides. The daily usage
limit is application-wide; a workspace cannot enlarge it.

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
- Workspace/configuration switches discard stale results, and context exclusions
  apply to symlinks, ignored files, credentials, caches, and model requests.
- Changing Taskmaster selection leaves Peon configuration and inference intact.
- Settings, packaging, and explicit handoff integration have regression coverage.
