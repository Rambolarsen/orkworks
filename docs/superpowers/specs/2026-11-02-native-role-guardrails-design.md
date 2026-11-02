---
type: Design Amendment
title: Native coding-tool role guardrails
description: Owner-approved practical role assurance for ordinary coding-tool sessions with explicit native controls and instruction-only limitations.
tags: [orkworks, roles, permissions, capability-evidence]
status: stable
---

# Native coding-tool role guardrails

## Decision and authority

On 2026-11-02 the owner clarified that coding tools retain the possibility of
reading or acting beyond assignment instructions when their ordinary host access
permits it, and approved continuing #740 under that constraint. The accepted
boundary is practical native guardrails plus clear instructions and disclosed
limitations. It does not require proving disobedience or bypass impossible.
This applies equally to Copilot, Codex, Claude Code, OpenCode and other harnesses;
their available controls and supporting evidence can differ.

This is a scoped amendment to the permission-assurance and research-completion
criteria in [#740](https://github.com/Rambolarsen/orkworks/issues/740), the
[role contract](2026-10-04-agent-role-configuration-design.md#requested-and-effective-permission-profiles)
and [hierarchy direction](2026-10-04-agent-hierarchy-and-configuration-learning-design.md#coding-tool-permissions).
It interprets ADR 0077's ordinary-session/coding-tool boundary without changing
the runtime architecture. It approves no runtime implementation, model probe,
new capture class, or new persisted wire schema. Broader component acceptance,
storage, orchestration authority and launch implementation remain separate.

## What a role promises

A role is an assignment and a reviewed native configuration. The adapter must
deliver the approved instructions/selected skill content, apply available native
controls correctly, describe actual access and record bounded practical evidence.
It must not claim a stronger security boundary than the coding tool supplies.
An instruction-only path/command restriction may remain part of the assignment
when the native tool cannot enforce it. Show that limitation before approval,
include it in the immutable configuration/evidence and preserve it in results.
A known native limitation is not automatically a failed role or unavailable tool.

| Basis | Evidence and user-facing meaning |
| --- | --- |
| Native control | Identify exact version/settings, establish that the adapter applies the control, and exercise a representative allowed/prohibited case where safely observable. Report what was actually observed and the control's residual limits. |
| Instruction-only limit | Confirm the intended rule was delivered; explicitly say the tool may still act beyond it using available host access. A normal compliant test is behavior evidence, not enforcement proof. |
| Unknown application or delivery | Missing required content, unverified settings/mapping or contradictory behavior blocks claiming that configuration supported. Do not turn an unknown native mapping into a silently accepted instruction-only rule. |
| Strong guarantee explicitly requested | If a particular assignment requires guaranteed containment or exact effect denial, ordinary role guardrails cannot satisfy it. Explain the unsupported requirement rather than silently weakening it. |

Use “native guardrails” and “instruction-only limits” in approval details.
“Read-only” must specify whether native write/command actions are disabled or
whether it describes requested behavior. An unrestricted shell can read, write,
run repository scripts and use network/credentials: never label that configuration
technically read-only. Normal user access and login behavior remain visible risks.

## Requested/effective profile and evidence compatibility

Requested paths/actions express intended task scope. Effective profiles record
the actual native settings/tools plus instruction delivery used to pursue that
scope; they are not an assertion that every scope entry is a security boundary.
Native access broader than the intended scope must be disclosed, not silently
introduced. User approval covers the exact reviewed configuration and its known
limitations. A detected change to applied native settings, instructions or
required capability still requires a revised proposal; a role cannot authorize
its own wider task, tools, connector, model, retry or delegation.

Retain the existing CapabilityEvidenceSnapshot fields and digest domains.
Each surface's existing reason and references must distinguish native control,
instruction-only limits, observed behavior and unknown coverage; these reasons
are part of the immutable approved evidence. Render limitations with the task
instructions and in approval details, so they cannot be omitted from review.
No new enum or wire field is introduced by this amendment.

A passed check means the declared native mapping/instruction delivery and its
bounded practical acceptance case passed. It does not mean all bypasses are
impossible. Known disclosed native limitations can coexist with passed checks;
unknown or failed required mapping/delivery cannot. `verified` describes that
exact practical configuration/evidence, not confinement. Existing `limited`,
`unverified` and `unsupported` remain non-eligible when their required mapping,
functional delivery or behavior evidence is incomplete. Historical no-go findings
are not promoted automatically; reassess them against these explicit criteria.

Exact OrkWorks-owned path/configuration validation, approval identity, instruction
and skill byte identity, evidence provenance, reporting honesty and drift checks
remain required. Canonicalize OrkWorks-bound launch/input/output paths and reject
an invalid or escaping binding. This does not require proof that the native agent
can never read another same-user path through a different operation.

For shell-enabled roles, declare intended commands/effects and native approvals,
working directory, environment/settings and credential inheritance limits. Use
native restrictions where available. An empty-start environment receipt before
every vendor task command and exact descendant argv/effect confinement are not
ordinary-role prerequisites. An environment explicitly constructed by OrkWorks
must still match its approved snapshot; do not pretend it attests to all native
subprocess environments. The command environment snapshot records bindings the
adapter explicitly sets; unknown or vendor-inherited bindings must be disclosed
in the surface reasons rather than asserted removed.

## Finite capability research

#740 delivers a version-bound evidence register and reviewed consumer handoff,
not exhaustive proof for every operation, every role or every coding tool.
Reuse common version/settings/delivery evidence across roles where the mapping
is identical; record role-specific differences instead of multiplying identical
probes. Report a supported practical slice, partial coverage, or a specific
functional blocker. Keep all uncertainty visible without making every unknown
an endless investigation prerequisite.

The next Copilot evidence boundary is finite: reuse the startup/source/settings
facts already retained; obtain one usable allowed-action observation for the
smallest review candidate and a representative native restriction check; then
classify the profile and hand off. Preserve instruction-only read-scope limits.
A missing exporter body/value explanation is not itself a permission blocker.
Use a separately reviewed narrow receipt or record a measurement limitation;
no broad telemetry or full prompt/reasoning capture is authorized here.

Native skill invocation/adherence is separate from delivered content. Automatic
parent continuation and exact resume are functional gates for that feature under
#742, not prerequisites for classifying an ordinary review child. Shell-enabled
role qualification and other harnesses can have scoped follow-ups with their own
finite acceptance criteria. Do not keep #740 open solely to prove confinement or
to finish every downstream feature. Do not close it before the promised coverage
classification and consumer handoff have actually been reviewed.

## Preserved boundaries

This amendment does not relax privacy or probe consent, authenticate model
self-reports as native evidence, invent delivery receipts, or equate a successful
start with a usable role. It does not change server/UI approval enforcement,
parent-only orchestration, bounded allocations, immutable sources, custody,
storage admission or the ordinary-session implementation boundary. No existing
historical probe authority is revived. The installed product gains no new
capability from this documentation change alone.
