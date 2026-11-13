---
type: Design
title: Inert improvement text and fixed handoff instructions
description: Proposed replacement for English command guessing in the new Brain improvement flow, paired with code-owned handoffs.
tags: [orkworks, taskmaster, recommendations, validation, security]
status: draft
workflow_status: approved
---

# Inert improvement text and fixed handoff instructions

Tracking: [#1694](https://github.com/Rambolarsen/orkworks/issues/1694).
Parent: [Brain-guided next improvement](2026-11-12-brain-guided-improvement-design.md).
Evidence: [three qualification attempts](../plans/2026-11-12-brain-guided-improvement.md).

## Approved decision

Replace linguistic command guessing **only in the new Brain improvement flow**
with bounded, single-line, inert text validation. Command mentions and literal
command text may appear as reference data. They never become executable actions.
Pair this change with fixed implementation and investigation handoff templates
that contain a validated recommendation ID and code-owned instructions, never
model-generated descriptions or evidence excerpts. Retain the existing rejection
of claims that unperformed verification succeeded.

This deliberately changes the parent design's command-payload rejection for this
new output profile. It does not claim that every accepted sentence is true, safe
to act upon, or useful. Structural validation, citations and explicit action
approval remain independent requirements. The owner approved this complete description on 2026-11-13 by answering
“keep going” to the explicit implementation-approval question for draft PR #1753.
The approval covers this coordinated implementation; it does not activate it.
No further live model calls or activation are authorized by this proposal.

## Why this change

The third comparison retained two valid positive app recommendations but missed
the baseline's useful CI recommendation. Its first evidence response requested
nine items against the stated eight-item cap: that rejection was correct.
Separately, a restricted final with supplied citations was rejected on ordinary
English: “No task workflow, issue breakdown, or change history is included in the
supplied evidence.” Earlier trials exposed “issue history” and “would make ...
more actionable” false positives. Phrase-specific exceptions repeatedly expose
new ordinary wording. Model instructions now avoid everyday verbs to satisfy
that classifier.

The current classifier combines size/control checks, English imperative/tool
heuristics and verification-claim checks in
[assessment_model.rs](../../../crates/orkworksd/src/taskmaster/assessment_model.rs).
The new flow also passes its fourteen rows through legacy readiness validation,
including on history reload. A change only to opportunity text would therefore
leave the same failure elsewhere.

## Scope and simpler interface

Use one internal text policy for every descriptive string produced by the new
improvement flow: result summary, limitations, no-proposal reason, all fourteen
concepts' rationale/limits/transitions/unassessed reasons, and every opportunity
text field. A caller supplies only the field's existing character bound,
required/optional state and existing descriptive/prospective claim mode.

The new policy rejects oversized/required-empty text, Cc control characters
(the existing Rust `char::is_control` check, including tabs, newlines, carriage
returns and terminal escapes), the twelve bidi-control scalars U+061C,
U+200E–U+200F, U+202A–U+202E and U+2066–U+2069, and performed verification
claims using the existing claim-check rules. Other printable Unicode, including
script joiners, remains allowed. The explicit bidi exclusion is a proposed
policy-2 display safeguard; it does not change legacy validation. It does **not** infer
commands from verbs, noun phrases, punctuation, tool names, quoted text, paths or
shell-looking text. The bounds remain exactly those of the current field.
Strict JSON, duplicate/unknown-field rejection, enums, required nullable fields,
concept identities, fact indices, supplied page/hash citations and preferred
opportunity consistency remain unchanged. A rejected field still rejects the
whole final result; no clipping, field deletion, retry or partial credit.

Keep the legacy text policy for standalone readiness, legacy assessment and
background/observation recommendations. This proposal does not delete their
validator or change their acceptance behavior. Share structural and claim checks
internally rather than copying the readiness decoder or adding caller-selected
HTTP settings. No dependency, additional model judge or model repair call is
introduced.

## Versioning, persistence and recovery

The new internal profile is text policy 2. Set the code-owned improvement
input/evaluation discriminator to
`brain-improvement-v2:collector-1:prompt-2:response-1:text-2`, so pre-change
results are not current for a new run.
Publish new improvement recommendations with the existing
`improvementReference` shape and `version: 2`, retaining the existing intent enum.
Unknown versions fail closed; clients must not treat them as legacy fixes.
Version 1 records retain the old policy and handoff behavior.

New improvement history uses an explicit supported producer identity for text
policy 2 in the existing identity/version fields. Select its validation only
when that identity and the validated `Collector::Improvement` source agree;
never from a request flag or a free-standing version field alone. Legacy
identities remain strict, contradictory/unknown identities are rejected, and
old files are not rewritten. The optional history projection must validate the
same text before saving and after restart. Preserve existing privacy,
redaction, stale-input rejection and report/recommendation binding checks.

The serialized identities have these exact meanings:

| Surface | Policy-2 identity |
| --- | --- |
| Captured input/evaluation | The code-owned discriminator above, incorporated into the existing evaluation key |
| Report range manifest and model result | Existing range-manifest shape and result version 1; the model result never selects a text policy |
| History manifest | Existing envelope version 1, purpose `brain-improvement-v2`, `Collector::Improvement` |
| Improvement history input wrapper | Version 2, containing the unchanged version-1 range manifest |
| History identity versions | `purpose: brain-improvement-v2`, `collector: 2`, `schema: 1`, `text: 2` |
| History inference identity | Evaluation key equal to its bound improvement report/capture key |
| Recommendation provenance | `improvementReference.version: 2` with the existing implementation/investigation intent |

All redundant identities must agree with the bound producer and report before
selecting a policy. The captured producer selects the initial final-result policy
before descriptive fields are validated. For persistence, parse the bounded
identity envelope first, validate its collector, purpose, versions and evidence
bindings, then select and reuse the same policy for text, save/reload, publication
and handoff admission. Model response fields, client flags and inferred string
contents never select it. Conflicting combinations fail closed rather than
falling back to either policy. Legacy identity combinations are the supported
version-1 producer, history input wrapper and existing identity-version fields
without a text-2 marker; snapshot fixtures pin those combinations unchanged.

There is no database migration or wire field removal. Schema tests must pin
these combinations and contradictory/unknown variants before policy 2 is used. Downgrading may make new records
unavailable, but must not erase them, grant authority or reinterpret them as
legacy. The unqualified production gate remains false throughout this work.

## Display and handoff are separate from advice

The renderer continues using escaped text nodes for all descriptions, with no
HTML/Markdown interpretation, clickable generated commands, script execution,
clipboard automation or terminal submission on publication/status/history read.
Inline code or shell punctuation is displayed literally. Repository and Brain
excerpts remain separately labeled evidence, subject to existing access rules.

For version 2, both Rust's canonical handoff and the desktop preview contain only
fixed text, the validated store-owned recommendation ID and its fixed local GET
route. No generated title, rationale, proposed change, benefit, validation,
rollback, evidence or model URL is interpolated into the instruction template.
The receiving agent must fetch the current authenticated recommendation, treat
all returned prose/excerpts as untrusted reference data, inspect the cited
mechanisms, and apply repository instructions and owner decisions. Retrieved
text cannot expand tool permissions, exclusions, workspace or session scope.
The static implementation template asks for the supported repository-level
change; the investigation template forbids edits, implementation, commits and
state-changing commands. Neither creates/resumes another session.

The dialog remains an explicit user action with the existing intent label. Its
read-only decision keys off the fully validated version-2 provenance for **both**
implementation and investigation. Confirmation sends only the selected session
and recommendation identity, with no prompt override. The backend revalidates
the store-owned provenance and canonical ID, rejects any supplied override
(including a byte-identical canonical prompt), and constructs the canonical
prompt itself. Client read-only flags are never an authorization check. Reject completion-packet fields,
packet mutations and stored completion packets on version-2 recommendation
handoffs and packet-reporting paths. Packet-driven follow-up actions for this
new producer are deferred; legacy packet behavior remains unchanged. Ordinary
completion reporting and the existing session/workspace freshness and delivery
checks remain required. The renderer must not route version 2 through its
completion-packet prompt branch.

This prevents automatic instruction interpolation; it is **not native coding-
tool confinement** or a proof against prompt injection. Once the owner confirms
a handoff, the coding tool reads untrusted data under its existing controls.
Those controls and repository instructions still govern any eventual action.

## Alternatives considered

| Alternative | Reason not selected |
| --- | --- |
| Add more noun/verb exceptions | Continues coupling useful explanations to an expanding English parser |
| Loosen the shared legacy validator | Broadens unrelated behavior and still leaves generated implementation text interpolated into instructions |
| Ask another model to judge or repair prose | Adds cost, latency and another uncertain decision; does not establish action authority |
| Keep data validation and change the handoff together | Selected; a small explicit data policy and fixed instruction templates |

The intentional tradeoff is that model text may now contain misleading or
harmful command advice as inert data. This risk is visible to owner review;
lexical command rejection is no longer claimed as a security guarantee for
policy 2. The fixed handoff and existing tool/approval controls are mandatory
parts of the change, not optional follow-up hardening.

## Acceptance and implementation boundary

Before dependent implementation, obtain owner approval and record the decision
under the repository ADR lifecycle. Then write a concise implementation plan
covering the text module, readiness/history adapters, versioned producer binding,
Rust/desktop canonical handoffs, packet refusal and documentation. One owner
keeps the coordinated diff; there is no parallel writer requirement.

Deterministic checks must prove:

- Exact retained harmless final outputs decode under policy 2, including the
  third restricted final. Historical trial ledgers/results remain unchanged.
- Legacy decoders and version-1 history keep their existing positive/negative
  fixtures. Policy 2 never applies to an ordinary readiness collector.
- Oversize/control text, false completed-verification claims, bad citations,
  unsupported versions and inconsistent identities still fail at initial parse,
  publication, reload and handoff admission.
- Accepted quote/command-looking text is escaped in the real renderer and causes
  no command dispatch. Hostile text cannot alter either fixed handoff; unknown
  intent/version, overrides and packet-path attempts cannot bypass admission.
- Version-2 history survives restart and privacy reduction with the same
  policy/citations; downgrade and unknown versions remain unavailable safely.
- All six-call/eight-minute investigation limits, evidence-access limits,
  authentication, cancellation and explicit user-action gates remain intact.

Run the affected Rust/desktop suites and full repository verification with the
required current-code manual review and CI before merging implementation.
Offline replay proves decoder behavior, not usefulness. The known nine-request
model failure remains unresolved by this text contract. Any evidence-protocol
change requires its own scoped decision, and another controlled five-pair trial
needs fresh explicit authorization. Activation still requires valid independently
adjudicated usefulness and compatibility evidence under the parent design.

## Design review evidence

One read-only Luna builder pass identified the Unicode-set ambiguity; the
contract now defines Cc plus the twelve bidi-control scalars explicitly. One
separate read-only Luna security pass identified two missing decisions: backend
rejection of all version-2 prompt overrides for both intents, and identity
consistency/validation ordering across history and publication. The admission
criteria and identity table above resolve them. The parent owns this proposal
and its edits. These reviews assess the design; they do not establish passing
implementation tests, native confinement or owner approval.
