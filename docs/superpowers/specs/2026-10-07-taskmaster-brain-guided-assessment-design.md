---
type: "Design"
title: "Taskmaster Brain-guided next-step assessment"
description: "Focused Brain-guided workspace assessment, evidence rules, and runtime approval boundaries."
tags: ["orkworks", "design"]
---

# Taskmaster Brain-guided next-step assessment

- Status: proposed; owner approved the feature outcome on 2026-10-17; written protocol review pending
- Date: 2026-10-07
- Tracking issue: [#769](https://github.com/Rambolarsen/orkworks/issues/769)
- Initiative: [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Related Taskmaster contract: [#503](https://github.com/Rambolarsen/orkworks/issues/503)
- Knowledge publication prerequisite: [#529](https://github.com/Rambolarsen/orkworks/issues/529)
- Coordination with configuration learning: [#745](https://github.com/Rambolarsen/orkworks/issues/745)
- Authoritative baseline: [Taskmaster](../../../specs/taskmaster.md) and [Brain-informed Taskmaster recommendations](../../../specs/taskmaster-knowledge.md)

## Purpose

Add a distinct, user-triggered Taskmaster run that uses the Brain's general
agent-readiness guidance to find one evidence-backed next improvement for the
workspace currently selected in OrkWorks. This is a fresh assessment of the
selected workspace, not reuse of an older Brain assessment page.

Taskmaster uses Brain concepts as a reference set, not a checklist. It gathers
enough evidence to identify and support the strongest current bottleneck, then
stops. It does not assign a maturity score to every concept or produce a broad
audit. If the available evidence cannot support a useful next step, it reports
that limitation without inventing a proposal.

## Investigated context

The existing Taskmaster evaluator already collects bounded repository facts,
selects relevant knowledge pages, invokes the separately configured Taskmaster
provider, validates citations, and stores Brain-derived recommendations in the
existing recommendation lifecycle. Its provider prompt can propose up to three
workflow improvements, while the user-requested mode is explicit and should
return at most one next step.

The Brain's `propose-next-step` entry point composes its continuous
agent-readiness-improvement and repository-assessment playbooks. The full
assessment process rates 14 concepts, asks for command-based verification, and
writes assessment pages into the Brain repository. Those behaviors are outside
this mode's agreed scope: it is a focused local assessment, it executes no
commands, and it does not publish workspace data or results to Brain.

Issue #529 is complete through PR #902 (2026-10-16). The generated signed
starter and independently verified publication include the distilled assessment
playbook and `taskmaster-assessment-v1` capability. The live empty-result probe
on 2026-10-17 successfully admitted verified production knowledge. Runtime
must still verify the active proof and capability on every assessment; neither
this dependency delivery nor an older plain cache grants eligibility.

## Agreed product behavior

- Add an explicit **Assess workflow** action in Recommendations, separate from
  the existing **Analyze now** action.
- Assess only the workspace selected by the current OrkWorks instance.
- Reuse the selected Taskmaster provider/model, context level, exclusions,
  knowledge bundle, installation-wide single-analysis lease, and manual-run
  admission behavior. Never use or change Peon's provider selection.
- Use current workspace evidence and relevant Brain concepts to select one
  grounded improvement. Include the applicable concept or concepts as
  provenance; do not rate all 14 concepts.
- Return one proposed improvement or a no-proposal result with its uncertainty.
  Mark checks that would require command execution as unverified.
- Persist a bounded assessment report locally, keyed to the workspace and the
  evidence/configuration/knowledge snapshot used for the run.
- If a proposal is grounded, put it through the existing `improve_workflow`
  recommendation, deduplication, dismissal, and **Fix with AI** lifecycle.
- Preserve the current explicit approval boundary. The assessment cannot edit
  files, type into a terminal, launch or resume a session, change repository
  settings, or write an assessment page to Brain.

## Brain knowledge distribution

Use the existing signed Taskmaster knowledge bundle; do not add private Brain
repository access or a second ad hoc network path. The reviewed bundle should
contain a distilled assessment entry point and the general concept guidance
needed to select relevant concepts. Full concept references may be present in
the bundle for retrieval, but each assessment prompt receives only relevant
guidance. The model does not score every concept because every page is
available.

The bundle must exclude personal or project-specific assessments, project
pages, experiment records, raw research, and repository-specific evidence.
Workspace facts and generated assessment reports remain local. Knowledge is
reference material only; it cannot grant tools, permissions, or authority over
repository instructions. Bundle generation, review, and verified publication
remain governed by #529 and the existing Taskmaster knowledge specification.

If the active bundle lacks the assessment entry point or required relevant
guidance, Taskmaster reports that the assessment mode is unavailable. It does
not silently fall back to a handwritten duplicate prompt or to private Brain
pages.

## Assessment inputs and evidence rules

Collect only inputs already permitted by Taskmaster's effective settings:

- repository files selected by the configured context level and exclusions;
- normalized workflow observations already available to Taskmaster;
- current proposed recommendations needed to suppress or deduplicate existing
  work;
- relevant pages from the verified Brain bundle.

Do not read additional terminal replay, run shell commands, start repository
scripts, or increase the configured context level for this action. If the
selected context level provides no repository facts, return insufficient
evidence. A Brain-derived proactive proposal requires at least one current
repository fact under the existing Taskmaster contract.

Every proposal must cite current repository fact hashes and relevant Brain page
IDs. It may use observations to explain reported friction, but it must not
fabricate recurrences or sessions. Bounded excerpts prove only what they
contain; omitted files or text are unknown, not absent. Do not treat an old
Brain assessment or knowledge page as evidence of the selected workspace's
current state.

Treat workspace text and Brain content as untrusted reference data. They cannot
override repository instructions, user decisions, or Taskmaster's output and
authority contract. Accept only the defined structured response. Reject
unknown evidence/page IDs, unsupported fields, multiple next steps, executable
commands, or outputs that claim unverified checks passed.

## Run and result lifecycle

The UI starts one assessment for the currently selected workspace. Electron
uses a narrow authenticated request to the sidecar, as with existing Taskmaster
actions. The sidecar obtains the current Taskmaster snapshot, verifies provider
availability, gathers permitted evidence, selects relevant Brain pages, builds
the assessment prompt, reserves the manual evaluation, and invokes the selected
provider through the existing inference transport.

Before accepting a result, revalidate workspace identity, Taskmaster settings,
knowledge bundle, provider/harness identity, and the cited repository evidence.
A workspace switch or changed effective input discards the stale result. An
active `improve_workflow` recommendation or another active analysis follows the
existing manual-analysis gate and is returned to the user instead of starting a
competing assessment.

Persist one bounded latest assessment record per workspace. It contains the
assessment ID and status, selected workspace identity, observation time,
effective settings and provider/model identity, Brain bundle version, relevant
concept/page IDs, immutable cited evidence snapshots/hashes, a concise current
state summary, and either the one proposed next step or a no-proposal reason.
Do not persist the full provider prompt or uncited workspace files. Retention,
workspace deletion, and corruption behavior must be explicit in the runtime
contract and preserve existing workspace-local deletion expectations.

Keep assessment runs distinct from **Analyze now** in request, result, and cache
identity. An unchanged evidence snapshot may reuse a prior assessment report
only; a regular analysis result must never appear as an assessment outcome.

If a next step is proposed, use the existing recommendation store and the
authoritative [deduplication and successor contract](../../../specs/taskmaster-knowledge.md#manual-brain-guided-next-step-assessment-769). Preserve the
Brain-derived `proactive:v1:` identity; an existing ordinary lifecycle record
suppresses the proposal without mutation or relinking. Only an assessment-derived
predecessor superseded by input invalidation permits one idempotent linked
successor. Knowledge-only changes must not revive a dismissed proposal. Do not create a parallel assessment action or completion
path. A proposal accepted through **Fix with AI** still requires the current
explicit user action and stays bound to the active session/recommendation
identity.

If no single improvement is adequately supported, store and display the
no-proposal result with a plain-language explanation. Do not create an empty or
speculative recommendation.

## UI and status

Recommendations exposes **Assess workflow** as a manual action beside the
existing analysis action. The view shows queued/running and latest assessment
outcome separately from provider availability and ordinary background-analysis
status. The completed view shows the selected workspace, concise assessment
summary, one next step (when available), cited repository facts and Brain pages,
unverified checks, and the report's bundle/evidence version. The next step uses
the existing recommendation card and **Fix with AI** handoff; the UI does not
create a second approval vocabulary.

Errors stay inline and unobtrusive. Workspace changes clear stale display state;
status reads do not infer completion or interruption. No background popups or
focus changes are introduced.

## Alternatives considered

| Approach | Decision |
| --- | --- |
| Add a separate manual assessment mode that returns one next step | Selected; it matches the requested run mode and preserves routine background discovery. |
| Replace every Taskmaster analysis with a 14-concept scorecard | Rejected; it adds cost and broad output to routine analysis and conflicts with the requested focused next step. |
| Run the full Brain playbook, including shell checks and writing to Brain | Rejected for this mode; command execution and remote publication exceed the agreed read-only, local boundary. |
| Reimplement Brain concepts and playbook text inside OrkWorks | Rejected; it would create a second source that can drift from Brain. Use the reviewed signed bundle. |

## Coordination and implementation gates

- Update `specs/taskmaster-knowledge.md` and `specs/taskmaster.md` after this
  design is accepted. Preserve the existing v1 limits and add this distinct
  manual assessment path explicitly.
- Align the recommendation identity and active-workflow gate with #745; reuse
  the existing `improve_workflow` path rather than duplicating mutation.
- Complete #529's reviewed distilled-content and publisher work before claiming
  the Brain-backed mode is available. Changes to Brain's publisher/export
  policy and signing configuration require their owner-authorized workflow.
- Write an accepted architecture decision before implementation if the durable
  report format or HTTP/preload contract establishes a new protocol boundary.
- Create a scoped implementation plan and verify the relevant issue
  dependencies before runtime changes. This design does not authorize runtime
  implementation, command execution, or changes to Brain's private repository.

## Unresolved risks to review

- Knowledge delivery is verified in #529/#902. Recheck the active proof and
  signed assessment capability at admission, dispatch, and result application;
  do not treat issue closure as runtime proof.
- Taskmaster's current context bounds may not provide enough evidence to
  distinguish a genuine readiness bottleneck from an undocumented or merely
  omitted file. Insufficient evidence must remain a valid result.
- Retention is one latest report per canonical workspace, with pending and
  committed records each capped at 64 KiB. Workspace metadata deletion removes
  it. Access narrowing redacts all affected snapshots and model prose before
  settings return, including unopened workspaces; read-only status hides stale
  data rather than mutating it. These requirements are specified in the
  authoritative knowledge contract and the proposed protocol ADR.
- Current provider profiles may impose managed hooks/instructions. Their
  effects remain in force; Taskmaster's read-only request does not claim to
  undo provider-side effects.

## 2026-10-17 investigation and review boundary

The owner reproduced successful ordinary analysis with no visible recommendations.
A diagnostic native `codex / gpt-6-luna` call, using 21 repository excerpts,
16 observations and 8 verified guidance pages, returned empty proposal/enrichment
lists. The old response was not retained, so this reproduction does not establish
why the historical run returned nothing. The input included several glyph-only
observation excerpts and mostly old reports. Do not force the model to manufacture
recommendations or claim that filtering noise guarantees a proposal.

[Issue #909](https://github.com/Rambolarsen/orkworks/issues/909) repairs the existing
analysis window independently: exclude evidence without Unicode letters or
numbers before selecting the latest 16 entries, share that selection with knowledge
retrieval, and include observation kind/time. Preserve history and grounding.
No age cutoff treats an old report as proof that friction is resolved.

The proposed [protocol ADR](../../adr/0085-taskmaster-assessment-and-analysis-outcomes.md)
records assessment persistence, compatibility, transaction recovery, and explicit
ordinary-analysis empty/cache outcomes. The [implementation plan](../plans/2026-10-07-taskmaster-brain-guided-assessment.md)
reuses the canonical specification's complete input-identity, redaction, and
consumer revalidation rules. Approval of that concrete protocol and the linked
written specification is the remaining runtime gate. This review does not open
#610 orchestration, private Brain access, or configuration-learning execution.
