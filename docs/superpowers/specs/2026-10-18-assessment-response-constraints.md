---
type: Design Proposal
title: Native Codex assessment response constraints
description: Proposed shape constraints for the existing assessment envelopes without changing inference authority or weakening validation.
tags: [orkworks, taskmaster, assessment, codex, inference]
status: stable
workflow_status: approved
---

# Native Codex assessment response constraints

Tracking: [#931](https://github.com/Rambolarsen/orkworks/issues/931),
[draft runtime PR #938](https://github.com/Rambolarsen/orkworks/pull/938).
**Owner approved this concrete design on 2026-10-18 in this task.**
Selected-model compatibility still gates production integration.
This supplements the [approved two-stage assessment design](2026-10-18-repository-led-brain-assessment.md)
and does not change its usefulness threshold or response envelopes.

## Problem and decision

Retained [quality evidence](../../validation/repository-assessment-runtime.md)
shows malformed JSON, unknown fields and duplicate keys from the configured
Codex `gpt-6-luna` / medium selection. Prompt-only exact-key guidance has not
established compliance. Separately, prose heuristics have false positives;
broader grammar exceptions introduced command/claim bypasses and were removed.
Neither problem warrants response repair, a fallback model or more inference.

Constrain the native Codex final message to the code-owned JSON shape of each
existing stage. The current local Codex CLI 0.163.0 advertises `--output-schema`
in `codex exec --help`. [Official OpenAI documentation](https://developers.openai.com/blog/eval-skills)
describes this option as a constraint on the final response's JSON shape.
That documents the mechanism, not proof that the selected model meets this
product's semantic or prose requirements.

Alternatives are to continue prompt-only tuning or broaden prose acceptance.
Repeated failures and reproduced bypasses make neither the next implementation.
This proposal addresses wire compliance; it makes no promise to solve prose
classification or usefulness. The existing strict prose guard remains active.

## Boundary and ownership

The assessment owner supplies an explicit typed response-shape discriminator
(`discovery-v1` or `assessment-v1`) with each native invocation. It is code-owned,
not derived from repository text, a model response, arbitrary JSON schemas or a
custom adapter declaration. Other native consumers request no constraint.

`providers/inference.rs` owns a small closed set of static schema documents and
passes the appropriate document through `--output-schema` only for the existing
native Codex profile. Each schema uses the existing required keys, primitive
and nullable types, enum values and closed object properties, including the
nested candidate/proposal shape. Outcome-dependent null/citation rules, bounds,
unknown/duplicate-field checks, ID membership and prose validation remain the
application decoder's responsibility. Do not rely on provider schema support
for those application guarantees.

The schema file lives in the existing private inference temporary directory,
owned by `PreparedInference` until process and pipe-reader cleanup finishes.
It contains no repository paths, facts, catalog entries, Brain content, prompts,
model output or dynamic citation enums. Its size has a fixed code-owned bound.
No new retained response file is needed; existing JSONL extraction stays active.

The selected provider/model/effort, login preferences, timeout, admission/dispatch
gate, sandbox, disabled tools/plugins and managed-policy behavior are unchanged.
There are still at most two stage invocations and no application retry, response
repair or fallback. An unsupported flag/schema/model is an explicit provider
failure; do not retry without the constraint. Claude, Ollama, custom adapters,
ordinary analysis and Peon do not gain or claim schema enforcement in this scope.
No new coding session, public endpoint or user capability is introduced.

## Identity and recovery

Include a versioned response-constraint contract marker in the assessment's
code-owned prompt/input identity, even though the schema file contains no prompt
contents. Reconstruction must distinguish constrained Codex inputs from the
previous prompt-only inputs. Preserve decoding of existing format-2 records;
do not make old records unreadable by changing accepted manifest versions.
Existing freshness fences supersede stale proposals; accepted/executing records
retain the existing lifecycle protections. There is no record-format migration.

The constraint is immutable for the captured assessment stages and cannot be
changed by settings or workspace mutations between capture and dispatch.
Existing settings/workspace/knowledge changes interrupt as before. Restart
interrupts in-flight inference without replaying it; pending publication recovery
uses its existing exact transaction, never another inference call.

## Delivery and verification

1. After owner approval, record the protocol decision in a new ADR and link the
   accepted constraint from the authoritative assessment specification before
   dependent implementation. Add the typed internal constraint and static schema
   owner. Before production wiring, an isolated selected-model compatibility
   check under the existing inference-only profile must accept the static shape;
   unsupported/refused constraints stop integration without an unconstrained
   retry. Fixture tests assert exact stage selection, closed nested shapes, static-only file contents,
   lifetime through process cleanup and unchanged unconstrained callers.
2. Integrate only native Codex assessment stages and bind the contract marker
   into reconstruction. Test selected model/effort and dispatch gates unchanged,
   unsupported-schema failure without an unconstrained retry, and stale identity
   fencing without mutating accepted/executing records.
3. Run the complete fresh selected-provider matrix with distinct workspaces,
   actual dispatches and semantic adjudication of every surfaced proposal.
   Keep #938 draft if prose rejection, grounding or usefulness still fails.
4. Run scoped assessment/transport checks, full repository verification,
   documentation/worktree checks and the current-diff review before PR handoff.

Use an isolated synthetic evaluation; do not change user settings or private
Brain. Required evidence includes CLI version, selected model/effort and bundle,
stage results, failure categories and exact current code identity.

## Risk and disposition

| Dimension | Rating | Treatment |
| --- | --- | --- |
| Dependencies | 3 | Existing Codex CLI boundary; unsupported constraints fail explicitly |
| Blast radius | 3 | Shared inference owner, opt-in only for Codex assessment |
| State changes | 1 | Input identity changes; no stored schema or record migration |
| Reversibility | 1 | Remove opt-in; normal freshness fences handle the prior identity |
| Uncertainty | Unknown | Selected-model schema compliance and usefulness remain unproved |

Score incomplete. Disposition: **Approved for implementation; compatibility
investigation gates production integration.** This is not a usefulness claim.
The first integration checks must establish selected-model compatibility;
a failure remains a blocker
and does not authorize another transport/model or broader prose acceptance.
