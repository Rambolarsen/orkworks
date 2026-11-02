---
type: Component Review
title: Role configuration component review
description: Decision-ready review of the existing immutable role configuration contract after the finite Copilot research handoff.
tags: [roles, configuration, review, orchestration]
status: stable
workflow_status: core-accepted
---

# Role configuration component review

## Decision in scope

#741 is a specification issue. Its logical core is already written in the
[canonical role contract](2026-10-04-agent-role-configuration-design.md).
The owner accepted that existing core and its existing defaults, with the approved [native-guardrails interpretation](2026-11-02-native-role-guardrails-design.md).
It does not ask for another capability probe, storage architecture or runtime
launch approval. The core acceptance is recorded below; independent technical
qualification and executable implementation remain gated.

Acceptance approves these design choices for a separately scoped
implementation/qualification plan:

1. **Freeze what the agent is asked to do.** Keep role/template, repository rules,
   assignment, selected skill/resource bytes, rubric, harness/model policy and
   requested/effective native settings in immutable versioned configurations.
   Exact-plan approval binds their canonical digest. Changes create a new
   configuration/proposal; no in-place mutation or transfer of old approval.
2. **Compose deterministically.** Render binding rules, role, assignment/rubric,
   then mandatory and optional skills/resources in the specified order. Preserve
   exact content bytes and mandatory requirements. Contradictory applicable rules
   or missing required content block the proposal; do not truncate or silently
   drop them. Natural-language applicability remains a reviewed planning judgment,
   not something structural validation claims to prove.
3. **Separate role intent from technical guarantees.** Six templates set intended
   work ceilings, while task-specific resources/actions are explicit. Apply
   available native controls and disclose instruction-only limits and normal
   user access. Ordinary profiles do not require universal bypass prevention.
   No unrestricted shell is labeled technically read-only. Skills and model
   self-reports cannot grant tools or authority.
4. **Bind approval and detect drift.** UI/server approval controls OrkWorks-owned
   grants. Launch/resume rechecks source/configuration identities and required
   delivery. Pinned and tool-managed model policies stay distinct; actual model
   visibility can be unknown only under the explicitly approved tool-managed
   policy. Native tool prompts do not revise the OrkWorks plan. Known prompt/
   policy-broadening risks are disclosed and detected drift requires attention,
   not a claim that native disobedience or immediate process stopping is impossible.
5. **Keep history and ordinary sessions honest.** Template/skill edits create new
   versions; user-authorized source-catalog retirement prevents new selection and
   preserves referenced history. Existing ordinary sessions and dormant plans
   are not silently upgraded into orchestrated role configurations. Mutable
   delivery/usage/evaluation records stay outside immutable definition digests.

## Current evidence and initial target

The [completed #740 handoff](../../validation/copilot-practical-research-handoff.md)
records exact runtime/settings/source identities, synthetic supplied startup
instruction/skill delivery and one successful owned-file read. The final case
collected no edit attempt/result, so native write-denial remains unverified.
The arithmetic-only synthetic skill is not a selected review skill. No complete
native invocation coverage or implemented role launch adapter is claimed.

The initial adapter target is a minimal Copilot review configuration with native
write/shell/URL/connector restrictions and explicit instruction-only file limits.
It is an implementation target, not an already eligible shipping profile.
The supported production-role set is currently empty. The missing real adapter
mapping and required delivery checks belong to that future integration; #740's
research closure neither supplies them nor needs to be reopened to finish #741's
written contract. No alternative harness or broader role is silently substituted.

This decision does not waive a functional requirement. A particular assignment
that requires a stronger technical guarantee cannot claim it from ordinary role
instructions. A continuous orchestrator separately needs #742's actual event/
wait/resume behavior; the ordinary review candidate does not establish that.

## Version and boundary compatibility

This review does not invent wire fields, change serializers/digest domains, or
downgrade accepted technical extensions. The canonical
[versioned records](2026-10-04-agent-role-configuration-design.md#configuration-identity-and-canonical-digest)
and [structured-output version routing](2026-10-04-agent-role-configuration-design.md#placement-versions-and-canonical-bytes)
remain authoritative. Base v1 examples do not authorize writing v1 records where
the applicable output-bearing child v3/root-bootstrap v2 contract is required.
Typed exact-result review inputs, output/reporting actions, admission identity,
quota/custody and source codecs retain their prior scoped decisions and separate
qualification status. No old definition is rehashed or promoted in place.

Current source seams at `0d43ed6770eddc60edbe13fa74cae8accdd9ca30` were inspected:
PlanNode and StoredPlan are data-only plan records; ordinary CreateSessionCommand
has harness/model/prompt inputs; HarnessDefinition/ResolvedHarness expose launch/
integration capabilities without the proposed immutable role configuration types.
This review does not claim those types or the launch workflow are implemented.

## Existing defaults included in the core review

These are the existing proposed logical admission limits, not new measurements
or certified coding-tool context/resource limits. Full details stay in the
[canonical bounds](2026-10-04-agent-role-configuration-design.md#bounds-compatibility-and-persistence).
A lower real adapter/context bound wins; mandatory content is never truncated.

| Subject | Existing default |
| --- | --- |
| Selected content | 16 skills; 32 rule/resource snapshots combined; 256 KiB rendered instructions |
| Definition sizes | 1 MiB per configuration; 2 MiB per complete plan, including referenced configurations |
| Role-template catalog | 64 versions/workspace, 1 MiB total; preserve pinned history |
| Retirement history | 4,096 entries and 1 MiB/workspace including the fail-closed marker; never evict tombstones to make room |
| Capability evidence | Ten surface checks, 16 references, 64 KiB inline snapshot; separately retained artifacts bounded by the canonical contract |

Native limitations and unknown coverage must fit the existing reason/reference
fields and be visible before approval. Empty/missing scope does not grant extra
OrkWorks actions. Saturated admission/retirement limits reject new selection or
input; this core review does not select a pruning/storage policy or approve a
migration. Previously accepted producer/storage numeric policies are unchanged.

## #741 written acceptance coverage

| Criterion | Existing canonical definition / disposition |
| --- | --- |
| Versioned records, model binding, digest | Configuration identity/canonical digest and bootstrap sections define the records, source/model identities and exact canonical bytes; implementation/shared-byte fixtures are not claimed. |
| Deterministic composition and mandatory content | Repository/role/assignment composition defines ordering, manifests, resource closure and conflict handling; natural-language interpretation is reviewed, not certified by a parser. |
| Six role defaults | Role templates and mandatory skills define orchestrator/research/implementation/review/verification/remediation; task-specific generated/check outputs must be declared. |
| Approval/launch/resume/drift | Approval binding section defines UI exact approval and revalidation; ordinary same-user native risks are explicit under the scoped guardrail amendment. |
| Eligibility/reasons/escalation | Native versus instruction-only/unknown evidence, support reasons, missing access and revised-proposal behavior are defined; no current adapter eligibility is invented. |
| Bounds, provenance, retirement, compatibility | Bounds/persistence and retirement definitions supply finite limits, source identity/history and ordinary-session handling. Storage mechanism/native feasibility remain separate. |
| Examples and verification cases | Existing permission projection, synthetic accepted path and rejection table define contract cases. #740's partial outcome fixes the current supported-role set as empty and identifies a review target; runtime launch planning remains gated. |
| Required graph-planning skill | The orchestrator manifest requires version/content-bound orchestrating-task-graphs for parallel planning; it does not grant implementation, review or code integration. |
| Immutable run bootstrap/plan binding | The separate root bootstrap and ParentPlanBinding preserve unchanged parent configuration across approved plans; they do not invent delivery or continuation evidence. |

The written definitions are present. A checked specification criterion would
mean that the design has been reviewed and accepted, not that its code/native
feature shipped. Do not reopen this specification issue for every independent
producer or native qualification gap after its reviewed core handoff is complete.

## Finite handoff and approval boundary

The owner accepted this logical core with “keep going” in response to the exact
acceptance question for draft PR #1298. Reviewed source:
`8759dded4fa312bb06110f98211684fa0e873ab1`. This includes the five choices above,
existing logical bounds, written graph-skill/bootstrap requirements and unchanged
version routing. It permits separately scoped planning, not runtime code, storage
changes, probes or a production adapter. The owner also authorized up to thirty
minutes after the reply for recording acceptance and completing this PR's CI,
review/merge and specification close-out (start 2026-11-02T06:56:04Z).

The decision resolves #741's outstanding written-core acceptance. Prior
technical amendments are not broadened, and no new producer/source/storage or
native guarantee is approved. The acceptance record and issue handoff complete
this specification; no further general research loop is required.

The next code-sized unit can be an isolated data-only configuration validator/
canonical renderer with deterministic fixtures and explicit version coverage.
It would not create sessions/worktrees, grant authority, call models, mutate user
settings, add endpoints, choose storage or enable a role. Its exact module/API,
version subset and tests require a separately reviewed executable plan and issue
before code. A launch integration additionally needs the real adapter mapping,
required delivery, #742/#610 authority and applicable storage/source qualifications.
No global storage/native proof campaign is a prerequisite to accepting this
logical core, and no runtime grant follows from that acceptance.

## Written review disposition

A fresh read-only builder/consistency review found no material blocker or missing
core decision. It confirmed the practical-risk boundary, all written criteria,
empty current production-role set, preserved version routing and separate
implementation/qualification gates. The subsequent owner acceptance is recorded above; it is not inferred from
the builder review or a proposal merge. Docs metadata/index/build and 776 component tests pass; source/runtime
files, old field definitions/digest domains and numeric defaults are unchanged.
