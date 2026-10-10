---
type: "Validation Record"
title: "Copilot 1.0.90 selective-capture research"
description: "Validation evidence and context: Copilot 1.0.90 selective-capture research."
tags: ["orkworks", "validation"]
---

# Copilot 1.0.90 selective-capture research

Date: 2026-10-05. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
initiative #738; consumers #741–#743. Extends the
[native receipt qualification](copilot-native-receipt-qualification.md).

## Decision

**A narrower native capture-control candidate exists; safe capture and complete
request-bound tool evidence remain unverified. All six profiles remain no-go.**
The pinned native addon contains descriptions of independent content classes,
including a response class that explicitly covers reasoning. The single broad
environment flag in installed monitoring help is therefore not evidence that
selective capture is impossible. No setting was applied, no runtime payload was
captured, and the compiled descriptions do not prove enforcement or precedence.

The [retained packet](fixtures/copilot-1.0.90-capture/manifest.json) binds static
selection to the actual 1.0.90 runtime files already matched against its embedded
package. It also retains the installed update-disabled configuration help.
No model request, CLI server or imported native addon was run. Existing user
configuration, authentication and managed policies were unchanged.

## What changed the investigation

The [capture settings artifact](fixtures/copilot-1.0.90-capture/capture-settings.json)
contains two independently selected compiled JSON structures: setting descriptions
and canonical telemetry keys. Both identify the following candidate controls.
The table paraphrases their descriptions; it does not certify behavior.

| Compiled setting | Described content class | Qualification consequence |
| --- | --- | --- |
| `telemetry.capture.identity` | Account, OS user and machine identity | Keep disabled; other identifiers still need review |
| `telemetry.capture.prompts` | Prompts and other model input | Candidate for role/rule/skill delivery; later inputs may contain prior response content |
| `telemetry.capture.responses` | Assistant responses and reasoning | Must be disabled and shown absent before storage |
| `telemetry.capture.toolArguments` | Raw arguments, definitions and identifying tool metadata | Candidate for tool definitions; synthetic inputs do not prove metadata contains no credentials |
| `telemetry.capture.toolOutput` | Tool results and execution-error details | Keep disabled; exceptions must not bypass the gate |
| `telemetry.capture.policyDetail` | Expanded filesystem-policy paths and similar details | Keep disabled in the first no-shell batch |
| `telemetry.captureContent` | Legacy shorthand for all five content classes, excluding identity | Broad shorthand is unsuitable for this batch |

The latest [vendor command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#content-capture)
and retained [1.0.90 monitoring help](fixtures/copilot-1.0.90-transport/monitoring.txt)
describe a broad content flag. The [configuration-directory reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
also lists enterprise telemetry defaults and a capture lock. These current docs
are primary references, not evidence pinned to the installed runtime's control
flow. Neither the installed configuration help nor the consulted docs documents
the granular controls above. Absence from those references does not invalidate
the compiled lead; it leaves supported configuration and effective semantics open.

An outcome-changing uncertainty is precedence: an ambient broad environment
flag, legacy config overlay, repository setting or enterprise capture policy
might override a narrow requested value. The retained API description of
`UserSettingsGetResult` says its values exclude repository and enterprise
managed overrides. A user-settings snapshot therefore cannot stand in for an
effective, per-session capture receipt. No real settings were read to answer
this question.

## Per-request coverage and persistence

The [model-call payload selection](fixtures/copilot-1.0.90-capture/model-call-data.schema.json)
shows that `ModelCallStartData` identifies the initiating turn, while
`ModelCallFinishedData` describes a logical dispatch that can include reconnect
or fallback attempts. `ModelCallFinalResultData` records the final model/result.
These payloads do not include exact instructions or effective tool definitions.
They cannot independently confirm every provider request or fill missing content
receipts. This is schema inspection, not an observed count or ordering guarantee.

The current vendor reference lists tool definitions in its `invoke_agent` table
for both top-level and subagent invocations; its separate `chat` attribute table
does not list them. That omission is not proof that
1.0.90 never emits them on requests. It makes full per-request coverage an open
check: retries, fallback, subsequent skill loading and tool updates must be
bound to the actual model-facing inventory, not joined to a stale invocation
snapshot merely by session or turn ID.

The [API projection](fixtures/copilot-1.0.90-capture/api-observations.json)
retains the `session.eventLog.read` type filter, cursor and live wait contract.
The declaration explicitly reads persisted history; filtering the read does
not establish filtering before persistence. `eventsLogDirectory` relocates logs,
and `eventsLogIncludesSubagents` controls subagent callback forwarding; neither
is a documented content-class persistence filter. `reasoningSummary` is a model
option, not proof that raw reasoning cannot enter events or later inputs.
No explicit content-class persistence control is identified in the retained
`SessionOpenOptions` property names; controls elsewhere remain unknown.

Event cursor waiting is also observer-side behavior. It supplies no evidence
that a report, approval, blocker or capacity event continues the same parent
model loop. #742's continuation/runtime/generation gate remains independent.

## Next qualification checks

The approved research question remains whether a native path can supply required
content/tool evidence while excluding forbidden content before storage. Advance
only after the following checks establish a concrete candidate; do not enable
broad capture to discover its contents.

1. Locate the supported configuration route and version-bound semantics for the
   granular keys, including defaults, broad-env/legacy/managed precedence and
   locking. A settings catalogue alone is insufficient.
2. Establish the effective combination for the exact session: prompts and tool
   definitions only; responses/reasoning, output, policy details and identity
   disabled. Verify previous response/reasoning is absent from captured later
   inputs, tool arguments, errors and every export or persistence path. A
   transcript read filter or receiver-side redaction cannot provide this proof.
3. Qualify actual schema and a safe native sample with complete request coverage,
   byte/content identity, runtime/turn/request correlation and loss/overflow
   checks. Keep global tools and optional tool-update metadata separate.
4. Only then review the exact synthetic no-shell `review` profile, model,
   permitted effects, auth use and finite budget in a separately authorized
   probe session, as required by the existing
   [probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md).

If these checks cannot be established, retain the precise capture/measurement
no-go. Do not invent a broker, new SDK integration, broad capture exception,
provider substitution or wider role permissions to obtain a passing result.

## Consumer handoff

#741 receives a native selective-capture lead and the unchanged unavailable
role matrix; there is no verified profile/settings/evidence snapshot. #743
receives schema-level content/usage leads only: discovery, confirmed delivery,
native invocation and agent-reported usage remain separate. #742 receives an
observer cursor schema lead, with model-loop continuation, generation fencing
and exact resume still unverified. #740 remains open for the required evidence
and reviewed final handoff; this research does not authorize implementation.
