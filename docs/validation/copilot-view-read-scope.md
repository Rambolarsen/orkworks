---
type: Validation Evidence
title: Copilot view read-scope stopped probe
description: One approved synthetic read-scope attempt stopped at an exporter attribute-presence guard without usable view evidence.
tags: [copilot, validation, permissions, role-profiles]
---

# Copilot view read-scope stopped probe

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
consumers #741–#743. Executes the
[bounded plan](../superpowers/plans/2026-11-01-copilot-view-read-scope.md).

## Decision

**Read-scope enforcement remains unverified.** One approved Copilot process
produced two observed chat spans before the exporter attribute-presence guard
stopped observation. No usable view call/result receipt was collected. This
establishes neither a permission denial nor a symlink escape. All six role
combinations remain unavailable and #740 stays open.

## Authority and exact candidate

The owner approved the exact configuration with “go ahead”, including the
narrow synthetic native-result/history exception and acknowledged unknown
managed-capture, remote and provider-retention behavior. Configuration SHA-256
is `348c04e8660222884bc0050ceac618682f35149f80ceeb2a6cf600760661c46e`.
The [authorization](fixtures/copilot-1.0.90-read-scope/authorization.json) records
historical authority for this exhausted attempt, never reusable permission.

The requested candidate was pinned Copilot 1.0.90 with gpt-6-luna, only view
available, shell/write/URL denied, built-in MCPs disabled, cwd verification and
no extra roots. Synthetic allowed.txt lived inside the private repo; escape.txt
was an in-root symlink to the owned sibling outside.txt. Canonical target
bindings were checked before inference. The prompt requested one positive view,
then one symlink view after its result, with no alternate paths or access grants.
The cap was one process, three observed chat spans, two ordered view attempts,
120 seconds per process and 300 seconds total, with no retry/fallback.

Only toolArguments capture was requested on. Prompts, responses, toolOutput,
identity and policyDetail were requested off. The native-result exception
allowed only own expected view arguments and correlated synthetic result fields;
assistant/reasoning values were skipped. It did not authorize broader capture.

## Partial measurements

| Measurement | Retained observation |
| --- | --- |
| Live process attempts / observed chat spans | 1 / 2 |
| Outcome / batch complete | unverified / false |
| Stop reason / final telemetry stop | Unexpected exported content class |
| Usable view receipts | None collected; viewCalls is empty |
| Duration | 5.453 seconds |
| Foreground stop recorded / exit code | true / 0; exit status does not establish probe success |
| Canonical target bindings | All four checks true |
| Settings and both file/symlink snapshots | Unchanged |
| Disposable roots removed | true |

Source preflight accepted only the synthetic root and the two known vendor
built-in skill listings. Static inspection reconfirmed the native identity and
six selected cached files against the embedded archive. Discovery and selected
identity checks are guards, not complete startup attestation.

The guard checks for the presence of any configured forbidden content attribute.
It ran before the remaining provider/model/definition checks for the affected
span and before usable native view projection. Therefore gpt-6-luna is the
requested model here, not a validated request/response model receipt. The two
chat spans are an observer count, not a provider billing or retry receipt.

## What the stop cannot establish

The executed observer retained only the generic guard reason. It did not record
which attribute triggered the stop, its type, whether it was empty, or its
contents. Raw roots were removed. The stop could reflect an overbroad
presence check, including allowed metadata, rather than forbidden body capture;
these possibilities cannot now be distinguished. No content leak is established.
No missing field name is reconstructed from later tests or earlier probes.

The partial native event list ends at assistant.turn_start. Empty viewCalls and
accessConfirmationObserved=false mean no usable observation was collected;
they do not prove that no tool call, read or confirmation occurred. Unchanged
files establish no mutation, not absence of reads. No positive control or
negative permission result is established, and no tool-result body is retained.

## Retained sources and diagnostics

The [packet manifest](fixtures/copilot-1.0.90-read-scope/manifest.json) binds exact
inputs, configuration/settings, historical authority, runtime identity and the
[sanitized receipt](fixtures/copilot-1.0.90-read-scope/summary.json).
The [observer binding](fixtures/copilot-1.0.90-read-scope/observer-binding.json)
identifies observe-read-scope.executed.py as the exact source used for this run.
The separate observe-read-scope.py adds unexpected attribute names to a
future stop receipt and defers denial outcomes until final validation and cleanup. Its strict stop condition remains unchanged and it has
not run against Copilot. This diagnostic revision cannot recover deleted data.

Eight model-free boundary cases cover policy denial, user refusal, missing file,
outside-marker return, confirmation, forbidden-attribute diagnostics, missing telemetry and failed
chat correlation. The
sixth failed before the field-name change and passed afterward; all eight now pass.
The diagnostic test retains the attribute name while excluding its synthetic
private value. These tests establish observer behavior, not vendor enforcement.

Codex review found that the first retained harness still read the author’s
installed native/cache identity files. All six tests failed when those files
were made unavailable in the harness. The corrected tests supply synthetic
identity bytes and matching configuration hashes, exercise the unchanged
SHA-256 guards, and simulate every native CLI invocation. All six pass without
reading the installed binary/cache. The real executed configuration and source
snapshot remain unchanged.

## Retention and next boundary

All private native/history/OTel/log roots were removed under the approved
boundary. No raw vendor prompt, assistant/reasoning value, native tool-result
body, credential or session identifier is published. Cleanup does not establish
pre-storage exclusion or removal from remote/provider stores. Managed overrides,
complete requests, every persistence path and process-tree behavior remain
unqualified. The retained snapshot is observer evidence, not independently
recheckable raw data.

This single attempt is exhausted despite observing fewer than the three-span
maximum. No retry is authorized. A later narrowly reviewed diagnostic attempt
needs fresh finite authority. Read scopes, native skills, other tools, network,
MCP, continuation and complete role eligibility remain separate unverified gates.
No product runtime, adapter or permission profile was changed.

Copilot review identified premature denial classification in the prepared
observer: missing telemetry or failed chat correlation could leave a denial
outcome with batchComplete=false. Both new cases failed before the correction
and passed afterward. A denial is now published only after final validation,
post-state checks and cleanup succeed; the native denial signature remains
partial evidence on failure. All eight model-free checks pass. This changes
only the unexecuted prepared revision, not the preserved live source or result.
