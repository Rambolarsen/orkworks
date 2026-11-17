---
type: Design
title: Code-owned unverified status for improvement advice
description: Approved replacement for English verification-claim guessing in the new improvement producer, with explicit unverified presentation and compatible consumers.
tags: [orkworks, taskmaster, recommendations, validation, security]
status: stable
workflow_status: approved
---

# Code-owned unverified improvement advice

Tracking: [#1694](https://github.com/Rambolarsen/orkworks/issues/1694).
Related: [approved investigation](2026-11-12-brain-guided-improvement-design.md),
[inert text and fixed handoffs](2026-11-13-improvement-text-contract.md),
[ADR 0115](../../adr/0115-inert-improvement-text-and-fixed-handoffs.md), and the
[fifth comparison](../plans/2026-11-12-brain-guided-improvement.md#fifth-comparison-result-2026-11-17).

**Owner-approved, 2026-11-17.** The owner answered “keep going” to the explicit
question approving implementation of this reviewed design, including visible
false completion statements as unverified advice. This is separate from the
earlier 66-call trial authorization. Implementation is authorized; activation
and further provider calls remain separately gated. See [ADR 0116](../../adr/0116-unverified-improvement-advice.md).

## Problem and proposed decision

The fifth comparison used 28 of 60 calls. One positive app request failed without
model text; a second positive final was rejected despite valid citations. Its
155-character validation field says:

> Unverified; proposed validation includes a failing test case demonstrating a non-successful CI job and a passing test case demonstrating normal completion.

Exact production decoding identifies this field as the only text rejection. It
describes a proposed experiment, not completed verification. An attempted
adjective exception accepted it, but also accepted the actual execution claim
“Unverified; proposed validation includes a passing test case already executed.”
That experiment was withdrawn. The accepted policy remains unchanged.

Replace English execution-claim classification **only for a new improvement
producer** with code-owned verification status. All its generated descriptions
are bounded, unverified reference data. The server never infers execution or
successful checks from those strings. Every supported display and handoff states
that checks remain unverified. Structural validation, citations, privacy,
currentness, explicit actions and fixed handoffs remain mandatory.

This deliberately permits a model to emit a false sentence such as “Tests
passed.” as unverified advice. The product does not assert that sentence's truth
or promote it to an execution result. The warning does not make bad advice
accurate or safe; owner review and independent usefulness qualification remain
necessary. The owner explicitly approved this tradeoff on 2026-11-17.

## Policy and execution authority

- Add text policy 3 for the code-owned version-3 producer only. Retain existing
  character bounds, required/optional fields, Cc controls and the exact bidi set
  from policy 2. Do not classify commands or verification claims from English.
- Keep legacy, standalone readiness, ordinary recommendations and improvement
  policy 1/2 validation unchanged. Do not rewrite old records or retroactively
  credit any trial output. A request/model field never selects a text policy.
- The code-owned status is exactly `unverified`. The model result schema remains
  version 1 and cannot supply `verificationStatus` or protocol control fields;
  unknown/duplicate fields still reject the result.
- Derive the public `verificationStatus: "unverified"` only after validating the
  producer, capture, stored manifest and report/recommendation binding. Do not
  trust a standalone status field in stored or submitted JSON.
- No model prose, score, cache hit, recommendation completion or history record
  becomes evidence that checks ran or passed. Future verified status would need
  a separate trusted execution-evidence contract; it is outside this proposal.
- Preserve six calls/eight minutes, evidence limits/exclusions, signed guidance,
  shared analysis lease, provider policy, cancellation and explicit Fix with AI.
  There are no repair/judge calls, native-tool grants or automatic session starts.

## Presentation and handoff

Show fixed copy before generated version-3 advice, including report/history and
recommendation details: **“Checks remain unverified. Run the proposed validation
before relying on it.”** The status is not a generated limitation that can be
omitted, edited or hidden by model output. Expose it accessibly with the advice.
Keep escaped text nodes, without generated HTML, executable links or commands.

Version-3 implementation and investigation handoffs retain fixed ID-only
templates. Add the same fixed unverified warning and the protocol declaration
below. No generated prose or excerpt is interpolated into instructions. The
receiver fetches current authenticated data, treats it as untrusted reference,
checks the mechanisms, and establishes validation independently. Investigation
intent remains read-only. Overrides and completion-packet paths remain refused.
This is not native coding-tool confinement or a guarantee of advice accuracy.

## Producer identity and compatibility

Use one consistent producer identity, with all redundant fields agreeing. The
initial approved producer-3 identity was:

| Surface | Initial approved identity |
| --- | --- |
| Capture/evaluation discriminator | `brain-improvement-v3:collector-1:prompt-4:response-1:text-3` |
| Range manifest/report | Existing format 1 and ranges; purpose `brain-improvement-v3` |
| Model final response | Existing version 1; no policy/status selector |
| History manifest | Existing envelope version 1; purpose `brain-improvement-v3`, `Collector::Improvement` |
| History input wrapper | Version 3 around the unchanged range manifest |
| History identity versions | `purpose: brain-improvement-v3`, `collector: 3`, `schema: 1`, `text: 3` |
| Persisted history origin | Optional `improvementVersion: 3` on new snapshots, retained when privacy redacts data; absent on old snapshots |
| Recommendation reference | Existing intent enum with `improvementReference.version: 3` |

Prompt revision 5 clarifies the existing applicability/no-proposal decision:
compare inspected current coverage, respect guidance applicability and limits,
and require a meaningful local benefit before preferring a change. Both app and
controlled baseline receive the same selection guidance in both phases. Supported
hypotheses remain allowed without an observed failure; unknown or unassessed
behavior alone does not justify work. The capture discriminator becomes
`brain-improvement-v3:collector-1:prompt-5:response-1:text-3`; all response, storage,
history, text-policy and presentation versions above remain unchanged. Earlier
completed records still load in the same stored format, but cannot be projected
as current under the new prompt identity. This clarification does not qualify model compliance or enable
production admission.

Prompt revision 6 clarifies the positive evidence threshold for a supported
hypothesis. An apparent inconsistency across inspected artifacts governing a
workflow can remain a candidate after considering differing scope or intent and
meaningful local benefit. Unresolved explanations remain limitations or a bounded
permitted confirmation step; lack of execution results or a historical incident
alone does not disqualify the candidate. This does not turn unknown behavior into
a missing capability, require a proposal, or waive counterevidence/applicability.
Both phases and comparison arms keep the same shared selection guidance. The
capture discriminator advances to
`brain-improvement-v3:collector-1:prompt-6:response-1:text-3`; other versions and
stored formats remain unchanged. Earlier prompt results are preserved without
current projection or status-triggered inference. This is a prospective prompt
clarification, not empirical usefulness qualification or production admission.

Unknown or contradictory identities fail closed. Preserve policy-1/2 readers and
old bytes. Downgrade must report unsupported data without deleting it, rewriting
it as legacy, dispatching inference, or acquiring freshness/authority.

Older renderers may ignore unknown JSON fields, so adding a warning field alone
is insufficient. Introduce the fixed HTTP view declaration
`X-OrkWorks-Advice-Protocol: 3`. It negotiates supported presentation, not text
policy or authority: the producer always selects policy 3 in code. Missing
declaration means the legacy view; an explicit unsupported value is refused.

New narrow IPC methods use that fixed declaration in Electron main; keep legacy
methods on the legacy view. This prevents an older renderer connected to a new
main process from accidentally opting into new advice. The renderer receives no
arbitrary URL/header capability. Version-3 responses carry `protocolVersion: 3`,
and new clients require its matching code-owned unverified status before display.
The fixed handoff fetch also declares protocol 3 with existing authentication.
Main binds each new call to the selected workspace and backend generation before
and after awaiting the request. List/detail/audit may carry the expected backend
URL only as an equality selector; it never selects the fetch destination.
Before any new-protocol mutation, main negotiates support through a safe read
inside that same generation/abort scope and rechecks workspace identity. An old
server that ignores unknown headers cannot receive a mutation before rejection.
IPC carries HTTP failures in a fixed protocol/status envelope so stale-detail
recovery retains 404/409 semantics without transporting raw error prose.

Producer-3 mutation replies contain only lifecycle receipts: recommendation ID,
status and target session. Advice must be fetched separately through the current
privacy/currentness fence; completion can settle after evidence changes without
echoing stale prose or declaring checks verified.

Apply the declaration to **every producer-bearing surface**, not only the primary
panel: improvement status/start/cancel, recommendation lists/details/actions,
readiness/history projections and any copy of version-3 prose. Without support:

- Never return version-3 prose. Legacy lists/history omit version-3 entries while
  retaining their stored bytes; latest/history freshness is unknown when omitted
  new data would otherwise determine it. Do not invent a score or privacy reason.
- Version-3 detail or record-bound mutation returns `protocol_upgrade_required`
  before side effects. Legacy improvement admission cannot start producer 3.
  Existing standalone/legacy operations remain governed by their own contracts.
- Keep version-3 IDs/status in active-recommendation, dedupe, lease and currentness
  accounting. Exclude their prose from legacy rollup/audit synthesis and packet
  follow-ups until those producers can preserve version-3 origin and presentation.
  This initial scope supports direct improvement recommendations/history only.

Current server/client, new client/old server, old client/new server and downgrade
tests must cover these paths. A header or copied model status can never relabel a
legacy producer or bypass authentication, evidence access or handoff admission.

## Alternatives and limits

| Alternative | Tradeoff |
| --- | --- |
| More English exceptions | Small patches retain broad rejection, but repeated normal phrasing fails and exceptions can admit real completion claims. |
| Structured validation-plan model schema | More output/schema and publisher coordination; other prose fields still have the same ambiguity. |
| Code-owned unverified status (approved) | Removes the guessing boundary; false statements may remain visible as unverified data, with explicit consumer and authority controls. |
| Another model to judge or repair prose | Adds cost/latency and uncertain judgement; it does not establish execution evidence. |

The first provider failure's underlying cause is unknown: the old trial ledger
retained only a generic category. The separate fixed-code diagnostic correction
helps future failures; it does not explain, retry or reclassify that old request.

## Implementation and acceptance boundary

Owner approval is recorded above and the replacement decision in ADR 0116.
Synchronize the authoritative specifications and follow the concise plan for
backend identity/policy, compatible IPC/views and fixed UI/handoffs before
dependent implementation. ADR 0115 remains the contract for policy-2 records.

Acceptance must prove: correct version binding at decode/save/reload/publication;
the captured harmless CI final accepted only under policy 3; legacy/v2 fixtures
unchanged; false completed-claim text accepted only as explicitly unverified
version-3 data, never as verified state; bounds/citations/controls still reject;
unsupported/mixed clients receive no unlabelled version-3 prose or mutation;
fixed handoffs and packet/override refusal; privacy reduction and downgrade
preserve safety and bytes; status/history reads execute no checks or inference.

Run repository verification and current-code review before integration. A fresh
once-per-case paired usefulness comparison still gates activation and needs its
own finite authorization after implementation. Preserve all prior failures; an
offline decoder pass or warning label earns no usefulness credit. Rollback stops
new admission and keeps supported readers for existing records; use no destructive
migration. This approval authorizes no activation, release, private Brain access,
trust change, or additional provider calls.

## Proposal review

A fresh-context builder review found no material unanswered implementation
question. A separate safety review found no material trust or authority bypass
in the stated contract. Its required implementation acceptance check is a
complete inventory of producer-bearing HTTP, IPC and rendered surfaces, with
mixed-client and downgrade evidence for each. Both were read-only Luna/medium
reviews; neither supplies owner approval or implementation evidence.
