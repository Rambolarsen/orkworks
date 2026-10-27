---
type: Design Proposal
title: Remaining root bootstrap source schemas
description: Proposed root-only permission, requirement, settings, model and rendering codecs for review before dependent source-byte fixtures.
tags: [orkworks, hierarchy, bootstrap, codecs, approval]
status: draft
---

# Remaining root bootstrap source schemas

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741), with
[#742](https://github.com/Rambolarsen/orkworks/issues/742) as the run consumer.
Inspection base: `130c8621daca6209c0228758de09cc8b80f0a6b4`, after merged
[PR #1093](https://github.com/Rambolarsen/orkworks/pull/1093).
**Proposed for owner review. No dependent fixtures, checker changes, capability
probe, runtime implementation, migration or launch approval are delivered.**

## Purpose and boundary

Close the remaining selected nested source representations for output-bearing
root bootstrap v2. Reuse the accepted
[root assignment, provenance and executable codecs](2026-10-26-root-preparation-source-codecs-design.md)
and the existing [#741 record fields and limits](2026-10-04-agent-role-configuration-design.md#approved-logical-records).
The proposal selects new shapes and finite defaults below; the instruction to
prepare it is not acceptance of those choices. The broader component issue stays
open. This document takes precedence only for the named root source slice after
acceptance; it does not redefine child assignments or later mutable lifecycle rows.

The initial root is coordination-only. A bootstrap cannot acquire codebase reads,
commands, external research or arbitrary MCP by choosing a convenient nonempty
entry. Define those lists as empty for this slice, and close the nonempty
coordination representation that a functioning parent actually needs. General
command/effect/environment, external-source and connector codecs remain a
separate child-profile task. Their absence cannot be hidden by testing an empty
all-denied profile as a functional orchestrator.

Alternatives considered: generic permission JSON would leave authority and
canonical bytes writer-dependent; closing every child's command/network profile
would expand this root source task; a fake verified snapshot would misstate
#740 evidence. Choose exact root-only schemas and explicitly ineligible source
examples until real qualification exists.

## Common rules and complete envelope

All listed properties are required and all objects are closed. Reject duplicate
or unknown keys, unsupported tags/versions, invalid UTF-8/unpaired surrogates,
wrong types and bounds before hashing or persistence. Use the existing #741
recursive compact canonical JSON, strict integer and Digest/ID encodings.
Digest is 64 lowercase hexadecimal characters; ID is 1–128 ASCII bytes from
`[A-Za-z0-9._-]`. String bounds count UTF-8 bytes, inclusively. Preserve bytes;
no Unicode, line-ending, case or whitespace normalization. Sets are sorted by
UTF-8 bytes of the stated key; reject duplicates and reject noncanonical stored
order. Incoming sets may be normalized only as already allowed by #741.

The exact top-level root envelope is:

```text
{schemaVersion:2,bootstrapId,repositoryId,repositoryBinding,
 sourceWorktreeBinding,workspaceId,parentSessionId,runId,rootAssignmentId,
 goalDigest,roleTemplate,taskCategory,assignment,rules,requirementManifests,
 skills,skillResources,rubric,harness,capabilityEvidence,model,permissions,
 renderedInstructions,renderedInstructionsDigest,outputContract,
 bootstrapConfigurationDigest}
```

Use the accepted full #745 repository/worktree descriptors, RootAssignmentV1,
SnapshotProvenanceV1 and ExecutableIdentityV1, and the accepted #741 v2
outputContract. No future plan/task, reviewInput, grant, bearer or effective
settings object is added to this envelope. Existing v2 digest remains SHA-256 of
`orkworks.orchestrator-bootstrap.v2` + one LF + canonical envelope excluding only
bootstrapConfigurationDigest. Keep the 1 MiB inclusive envelope and 256 KiB
rendered-context limits and every smaller nested bound. No old seed is upgraded.

All top-level identities use their existing owners/encodings: in particular
runId is the #742 epoch/sequence string, repositoryId is #745's registered ID,
and goalDigest hashes the exact retained goal bytes. taskCategory is a nonempty
label through 512 bytes. roleTemplate.role and capabilityEvidence.role are
exactly `orchestrator`. The rubric evaluatorRole is one of the existing six
roles; its appropriateness still requires #744 owner validation.

## Requirement links and snapshot details

Keep existing RoleTemplateSnapshot, RuleSnapshot, SkillSnapshot,
SkillResourceSnapshot, RequirementManifest, CriterionSnapshot and RubricSnapshot
field sets and digest ownership. Complete their root-selected leaves as follows:

| Value | Exact representation / bound |
| --- | --- |
| rubric.dimensions entry | `{id,description}`; ID unique within 1–16 dimensions; nonempty description through 2 KiB; preserve declared dimension order |
| sourceLocations entry | Nonempty UTF-8 locator through 512 bytes, identifying a reviewed location within that manifest's exact source snapshot; a label is not a fetch request |
| skill.requirementSources entry | `{sourceId,sourceDigest,sourceLocations}`; 1–32 distinct sorted locators when this entry exists |
| applicabilityReason, selectionReason | Nonempty explanatory UTF-8 through 2 KiB |
| RuleSnapshot.sourcePath, SkillResourceSnapshot.sourceReference | Existing relative rule-path rules / reviewed resource locator, respectively; nonempty through 2 KiB |
| instructions, skill/rule/resource content | Nonempty exact UTF-8, each within the shared 256 KiB supplied-context ceiling; no truncation |

Each root rule and the role template has exactly one manifest; no extras.
sourceId resolves unambiguously across the combined rule/template namespace
(reject cross-namespace ID collisions here). sourceDigest equals RuleSnapshot's
raw contentDigest or RoleTemplateSnapshot's canonical contentDigest, respectively.
Every manifest.taskId equals rootAssignmentId. Sort manifests by sourceId;
requiredSkillIds is a sorted unique set of 0–16 selected IDs; sourceLocations
is a sorted unique set of 1–32 locators. Even a no-required-skill manifest explains
its applicability and identifies its reviewed source locations. Locators must
resolve against retained exact source text during review; structural spelling
alone does not establish a correct natural-language interpretation.

For mandatory skills, requirementSources is the exact sorted set of contributing
manifests projected to `{sourceId,sourceDigest,sourceLocations}` (same full locator
sets); 1–32 entries, sorted by sourceId. A skill requiring 33 contributors exceeds
the inherited 32-source bound and blocks this slice; never drop a contributor.
Optional skills have exactly `[]`. Mandatory IDs equal the union of all manifests.
Planning parallel work still requires the version/content-bound
`orchestrating-task-graphs` skill with compatible guidance. Unresolved rule/skill
conflicts block approval; serialization does not settle them.

Skills sort by ID in the envelope, resources by ID, each resourceIds list by ID,
and rules retain contiguous precedence order. Rendering uses mandatory then
optional skills as below. Preserve the existing 16 skills, 32 combined rules/
resources, 33 manifests, snapshot version rules, resource closure and provenance
joins. No new standalone requirement/provenance digest is introduced.

## Root permission profile

permissions is exactly `{requested,effective}`. Each profile is exactly:

```text
{tools,readPaths:[],writePaths:[],commands:[],externalSources:[],connectors:[],
 coordinationActions,nativeDelegation:"denied",accessChanges:"request-only"}
```

Those five lists must be literally empty for this slice. A reference input, rule
file, skill resource or repository descriptor is retained source context, not a
permission to read the live repository. Inputs requiring live reads/research need
a different reviewed assignment/profile. Startup content delivery uses the
qualified adapter mechanism, not a model-accessible shell or filesystem grant.
RootAssignmentV1.worktreeScope.readPaths and writePaths must also be empty:
the approved source worktree is identity, not live access. Root outputContract
may select only existing `server-artifact` sources for coordination outputs;
path outputs require a different reviewed profile. Explanatory output text must
agree with that restriction. No CommandEnvironmentSnapshot is present because
no task command is allowed;
ordinary provider login/startup behavior remains under the existing baseline.

Each coordinationActions entry is exactly:

```text
{actionId,workspaceId,runId,parentSessionId}
```

The three identities equal the enclosing bootstrap. Entries sort by actionId,
are unique and have at most 16 entries. The proposed logical IDs below name
existing #742 operations; they do not define an endpoint or request codec:

| actionId | Existing operation and additional gate |
| --- | --- |
| `orkworks.coordination.run.read.v1` | Read own authorized run coordination state and its declared plan/report references; no arbitrary source/artifact listing |
| `orkworks.coordination.event.wait.v1` | Existing waitForRunEvent with exact cursor/version, at most 30 seconds; events remain hints |
| `orkworks.coordination.clarification.request.v1` | Bounded clarification request; denied while stageContinueRequired |
| `orkworks.coordination.plan.propose.v1` | Bounded research/execution proposal under current input revision; no approval; denied while stageContinueRequired |
| `orkworks.coordination.task.launch.v1` | Request launch of an exact task only with a separate current server-held exact-plan execution grant and all reservation/capacity checks |
| `orkworks.coordination.child-result.record.v1` | Existing explicit outcome/report operation for an already-launched exact assignment owned by this run; no #744 candidate/evaluation impersonation |

For the initial continuous-parent source slice require all six entries in both
requested and effective profiles: an all-empty or narrowed nonfunctional profile
cannot qualify this assignment. An adapter incapable of even one action remains
ineligible, rather than exposing general HTTP/MCP as a substitute. These names
are new proposed permission identities requiring consumer agreement and evidence;
no current adapter is asserted to recognize them.

Each coordination actionId must also occur once in tools. tools is a sorted
unique set of at most 64 IDs; in this slice its only other legal entries are the
already proposed `orkworks.result.submit.v1` and
`orkworks.result.receipt.read.v1`, selected together when the root assignment
requires publishing its own result. Both profiles must cover the declared
assignment's required reporting. Result actions retain their separate #744
capability and own root subject binding; coordination rights cannot supply it.
No user-evaluation, evaluation, review-input, UI create/approve/continue/finish,
bootstrap mutation, Git or cleanup action is permitted. Every allowlist entry
is only a ceiling, intersected with current role, bearer, grant, run-state,
generation and operation-specific authority at each use. Stored identity gives
no bearer and cannot revive a closed run or bypass pause/revocation.

## Platform, settings and model source records

AdapterBinding and CapabilityEvidenceSnapshot retain their existing field sets.
Their shared platform is exactly PlatformV1:

```text
{os,architecture,osVersion}
```

os is `macos`, `linux` or `windows`; architecture is `arm64` or `x64`;
osVersion is nonempty exact UTF-8 through 512 bytes, not a compatible-version
range. Evidence must establish the actual native version and architecture,
including any translation mode; an unrepresentable combination is unavailable
in this initial slice. Match platform and executable identity byte-for-byte
between adapter binding and evidence. A representable platform is not qualified.

Two complete retained artifact records close digest-only source inputs. They
are referenced through the existing capability references, not inserted as new
bootstrap properties or made available through an uncounted object store:

```text
RootStartupSettingsV1 =
 {schemaVersion:1,kind:"root-startup-settings",harnessId,adapterId,
  adapterVersion,argv,nonsecretBindings,configurationSources,
  discoveredInstructions,deliveryOrder,instructionMechanism}
RootModelPolicyV1 =
 {schemaVersion:1,kind:"root-model-policy",harnessId,adapterId,adapterVersion,
  policyId,mode,modelId,settings,unknownModelAllowed}
```

argv is the actual ordered nonsecret launch argument vector after executable,
0–64 elements through 2 KiB each. Reject secret-bearing arguments instead of
hashing a redacted surrogate. Tokens are injected separately under the existing
verified startup contract. nonsecretBindings and settings are sorted unique
arrays of `{name,value}`: 0–32 entries, names matching `[A-Za-z_][A-Za-z0-9_.-]{0,127}`, exact UTF-8 values
through 2 KiB (empty allowed). They contain no credentials. Settings names are
adapter-recognized; arbitrary text is not evidence of supported semantics.

configurationSources is 0–32 entries exactly
`{sourceIdentity,sourceRevision,contentDigest}`, sorted by sourceIdentity with
unique identities, nonempty identity/revision labels through 512 bytes. Each
pins canonical RootEffectiveConfigurationV1 bytes, exactly
`{schemaVersion:1,kind:"root-effective-configuration",sourceIdentity,sourceRevision,bindings}`.
Its identity/revision equal the referring entry; bindings uses the same sorted,
unique 0–32 `{name,value}` nonsecret representation and bounds as above. Empty
values are retained as empty strings; absent bindings mean absent settings, not
an empty-value setting. The adapter's version-bound parser/mapping defines each
recognized name and exact string-value semantics; unrecognized settings block
qualification rather than being dropped. The complete record is at most 64 KiB.
This is the complete nonsecret adapter-resolved effective projection, not a raw
secret-bearing file or just its display path. Its bytes resolve by contentDigest
among protected capability references. Evidence
must prove the adapter enumerated all applicable sources and separated secret
values without losing behavior-changing settings. Unknown sources or an unsafe
projection block complete source qualification. Discovery must not infer a
complete effective configuration merely from an empty list.

discoveredInstructions is 0–32 entries exactly
`{snapshotKind,snapshotId,contentDigest,precedence}`, ordered by contiguous integer
precedence starting at 0. snapshotKind is `rule`, `role-template`, `skill` or
`skill-resource`; snapshotId resolves to that exact selected snapshot and its
existing contentDigest. Reject duplicate (snapshotKind,snapshotId). This records
tool-automatic discoveries and their order, not every explicitly supplied block;
qualification must establish that there are no unlisted automatic instruction
sources. Unselected global/ancestor instructions block the slice until included
through the appropriate reviewed rule/source mechanism. Unknown private vendor
instructions remain outside capture under the existing policy.

deliveryOrder is a 1–33 element ordered array with exactly one
`{kind:"supplied"}` and one `{kind:"automatic",precedence}` for every
discoveredInstructions entry (no extra or duplicate occurrence). precedence
selects its exact discovery entry; automatic entries preserve their relative
precedence order. This fixes how the one complete renderedInstructions payload
and automatically loaded sources are interleaved at startup for either
instructionMechanism. An adapter that splits supplied content or cannot prove
this ordering is unsupported in this slice. An automatically loaded selected
snapshot is still included in the supplied payload: record both occurrences,
never silently deduplicate or claim the automatic copy replaces a supplied
block. Review must establish that duplication/interleaving preserves all binding
rules and applicable instruction precedence; otherwise block approval. This
source declaration is not a delivery receipt. A source-byte case may declare
an unverified order; qualified delivery needs independent adapter evidence.

unknownModelAllowed is a boolean. Model mode/modelId use the existing pinned/tool-managed null rules; policyId,
harnessId and adapterId are IDs. All adapter/version/model/assessor identity
labels not already constrained as IDs are nonempty through 512 bytes. A pinned
policy requires unknownModelAllowed=false; a tool-managed policy may set it true
only if approval explicitly accepts unknown actual-model visibility. Its settings
must encode the exact adapter-recognized nonsecret policy, and be covered by the
model-policy evidence check. ModelBinding.policyId/mode/modelId and adapter
identity must equal this artifact. RootStartupSettingsV1 harness/adapter/version
and instructionMechanism equal AdapterBinding and capabilityEvidence; its
harness identity is not a display name. Recompute both requestedProfileDigest
and effectiveProfileDigest as raw SHA-256 of their respective normalized
canonical profile bytes and require exact equality in capabilityEvidence.
All inherited evidenceId/evidenceDigest, definition/tool/executable/settings,
role, generation and approved-model joins remain mandatory. No model observation becomes a policy input.

Each complete artifact is at most 64 KiB. effectiveSettingsDigest is raw SHA-256
of canonical RootStartupSettingsV1 bytes; ModelBinding.policyDigest is raw
SHA-256 of canonical RootModelPolicyV1 bytes. This selects exact payloads for
previously unspecified settings/policy hashes, without changing a named existing
digest domain. Both records must appear as retained `fixture` references with
matching contentDigest, even for an unverified snapshot. Their presence proves
bytes, not verified startup or model behavior. Other configuration projection
artifacts count within the same 16-reference / 1 MiB evidence artifact budget;
no recursive reference closure or separate allowance is added. Missing retained
bytes make a complete source unavailable; native remeasurement stays a separate
gate. Settings/model drift requires a new proposal, never a digest repair.

## Capability record leaves and eligibility

Keep the exact CapabilityEvidenceSnapshot fields, ten mandatory surfaces,
reference kinds, evidenceDigest domain, shared owner joins and decision rules
from [#741](2026-10-04-agent-role-configuration-design.md#immutable-capability-evidence-snapshot).
Checks are exactly `{surface,result,reason,referenceIds}`; references exactly
`{id,kind,sourceReference,contentDigest}`. No executable or provenance projection
may replace the accepted full value. Sort checks by surface, references by ID,
and each unique referenceIds set by ID; each check names 1–16 retained references.
reason and sourceReference are nonempty through 2 KiB. assessedAt is exactly a
valid UTC Gregorian timestamp `YYYY-MM-DDTHH:mm:ssZ`, years 0001–9999, no leap
seconds or fractions; assessorIdentity is nonempty through 512 bytes. Preserve
all existing evidence freshness/revocation checks; a timestamp proves none.

Both complete source artifacts above, every reference artifact and any selected
reference-input bytes must be retained and rehashed. Passed checks need a primary
reference and reproducible fixture; not-applicable needs source-backed denial/
absence plus its fixture. An unverified check cites retained gap/source evidence;
a synthetic positive outcome is never evidence. All six role profiles stay
no-go; #740 capability work remains deferred until November. A fully hash-valid
unverified root can be a source-byte test case only, never an approvable draft,
admitted run or qualified startup. Missing evidence must remain a specific gap,
not an invented minimal verified record.

## Exact instruction rendering

The existing composition order lacks separators/escaping rules for reproducible
renderedInstructions bytes. Select this root-only framing for review. Rendering
starts with exactly `ORKWORKS ROOT INSTRUCTIONS v1` followed by one LF. Append
each block as `kind` + one ASCII space + `id` + one ASCII space + decimal UTF-8
content byte length + LF + exact content bytes + LF. No zero padding, CR insertion,
Unicode normalization or terminal trimming. Header kind/id use the stated ASCII
literals/IDs; content is length-delimited and may contain any header-like text.

Blocks, in exact order:

1. kind `boundary`, id `root-v1`, content exactly the following one-line text
   without a terminal LF:

   ```text
   Coordinate only within the approved root assignment. Binding repository rules remain mandatory. Skills and task text grant no tools or authority. Request changed scope from the user. Approval, current capabilities and exact-plan grants remain separate gates. Do not implement, review code, mutate Git, delegate natively or integrate edits.
   ```

2. One kind `rule` block per rule in precedence order, ID=rule.id,
   content=rule.content.
3. One kind `role` block, ID=roleTemplate.id, content=roleTemplate.instructions.
4. One kind `assignment` block, ID=rootAssignmentId, content=canonical JSON of
   exactly `{assignment,outputContract,rubric}` copied from the bootstrap.
5. Mandatory skills in ID order, then optional skills in ID order: one kind
   `skill` block with ID=skill.id and content=skill.content, immediately followed
   by each owned resource in ID order as kind `resource`, ID=resource.id,
   content=resource.content.

This supplies all exact selected instruction content; framing itself adds no
permissions or rule precedence. Recompute and compare the whole rendered string,
then its existing raw SHA-256 renderedInstructionsDigest. Include framing bytes
in the 256 KiB limit and the full envelope limit. Any content or framing drift
requires a new bootstrap proposal. Adapter/native automatic delivery is still
subject to exact content/precedence evidence; the rendered string alone is not
a receipt, confirmation of model adherence or ongoing context retention.

## Written acceptance and rejection oracles

These are review cases, not executed fixtures or runtime results:

| Case | Required result |
| --- | --- |
| Complete root source with retained artifacts and unverified checks | Hashable ineligible source case; no approval/admission/launch |
| Empty all-denied profile labeled functional orchestrator | Reject required coordination coverage |
| Nonempty commands, read/write paths, externalSources or connectors | Reject this root slice; no generic JSON fallback |
| Action refers to another run/parent or lacks matching tools entry | Reject profile |
| Current bearer but missing exact execution grant | task.launch remains denied |
| stageContinueRequired or revoked/closed run | Preserve existing state-specific denial; static profile cannot restore rights |
| Root result action treated as evaluation or child result authority | Deny cross-subject/action use |
| Missing/extra manifest, optional mandatory skill or missing contributor | Reject requirement binding |
| Source locator exists but natural-language applicability is unresolved | Block review; structural validation cannot resolve intent |
| Wrapped executable, native platform or startup settings differ across owners | Reject exact binding; no version-label equivalence |
| Missing settings/policy/projection artifact or unknown automatic instruction | Complete source unavailable, never substitute an empty object/hash |
| Fixture artifact is present but permission control has not been tested | Remain unverified/no-go |
| Composed/decomposed Unicode, embedded LF or header-like content changes | Preserve exact bytes and lengths; changed render/digest |
| Framing, artifact count or reference content exceeds inherited bounds | Reject whole source; do not truncate content or discard evidence |
| Old synthetic bootstrap seed has a matching-looking digest | Keep historical UNVERIFIED status; no source reconstruction |

## Bounded review disposition

One read-only correctness review found no material contradiction with the cited
#741/#742 fields, inherited limits, digest rules or authority gates. A separate
builder review identified undefined configuration-projection artifact bytes and
unspecified interleaving of automatic and supplied instruction content. The
author added RootEffectiveConfigurationV1 and deliveryOrder to settle those
source choices. Its sorting question was already answered by coordinationActions
sorting by actionId. Author checking also makes root scope path lists empty and
restricts outputs to existing server-artifact sources, consistent with the
coordination-only ceiling. No post-edit independent rereview or native evidence
is claimed. These definitions remain proposed for owner acceptance.

## Review choices and bounded fixture handoff

Owner review should accept or revise the root-only empty access lists; the six
exact own-run action identities and tools pairing; requirement-source projections;
PlatformV1; the retained settings/model/configuration records, delivery order and raw hash payloads; the
capability leaf constraints; and the framing/default boundary text. These are
author choices proposed here, not permissions inferred from earlier acceptance.
No new technical choice is recorded as already approved.

After acceptance, a separate documentation fixture unit can pin independent
canonical bytes/hashes for these components and one complete **unverified** root
bootstrap, test each written rejection, and join the accepted run/draft/request/
receipt envelopes without claiming admission. Preserve prior fixture catalogs
byte-for-byte. Nonempty reference inputs additionally require closed retained
custody; a first case may use inline inputs or no inputs without pretending
reference custody is qualified. Native repository/worktree identity, all retained
source custody and lower adapter limits must be explicit qualification gaps.

This handoff is not a runtime execution plan. Complete #742/#744 storage-row
membership/changed effects, source-history/production aggregate quotas, native
commit/revocation ordering, shared custody and session attachment proof remain
independent work. Preserve current authentication before retry lookup, frozen
draft-byte equality, original receipts, burned sequences, current fence checks,
no reconstruction at/below high-water and existing shared-ledger usage. No #744
claim-first exception is extended to root draft/approval/preparation.

No runtime/database/migration/model/native-probe work, eligible profile or launch
is authorized. Broader #741/#742 component acceptance, #740 evidence, #745/#746
custody/consumer qualification and separately reviewed executable plans remain
required. Keep #741/#742/#744/#746 open. Dependent fixtures wait for this proposal's
owner acceptance; storing or merging the proposal is not that acceptance.
