---
type: Validation Evidence
title: Copilot exporter attribute-name diagnostic
description: One approved synthetic positive-view attempt identified gen_ai.output.messages at the lexical exporter guard without inspecting its value.
tags: [copilot, validation, telemetry, role-profiles]
---

# Copilot exporter attribute-name diagnostic

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
consumers #741–#743. Executes the
[bounded diagnostic plan](../superpowers/plans/2026-11-01-copilot-exporter-diagnostic.md)
after the [read-scope measurement stop](copilot-view-read-scope.md).

## Decision

**The exporter record contained the attribute key `gen_ai.output.messages`.**
The lexical guard stopped before decoding its value. This identifies a concrete
mismatch with the requested attribute-presence boundary for this new attempt.
Its value, type, emptiness and meaning remain unknown: it could contain permitted
tool-call metadata rather than forbidden response/reasoning body content. No
content leak, ignored response setting or permission outcome is established.
All six role combinations remain unavailable and #740 stays open.

This does not retroactively identify #1280’s missing triggering key. That
separate attempt’s raw records were deleted and its generic stop remains
ambiguous. No raw data from either attempt can now resolve the value’s meaning.

## Authority and exact candidate

The owner approved the exact reviewed configuration with “keep going”, including
existing authentication, one network-enabled native process, the disposable
synthetic-history/raw-exporter exception and acknowledged unknown managed
capture, remote and provider retention. The
[historical authorization](fixtures/copilot-1.0.90-exporter-diagnostic/authorization.json)
is exhausted, never reusable authority. Configuration SHA-256:
`1b80abcb90ac7645be92cb90d6b39cb89be34b47a8f63594d485c43d80a8aced`.
The immutable configuration retains its original proposed status; approval and
execution are recorded separately rather than changing approved bytes.

Requested candidate: pinned Copilot 1.0.90, gpt-6-luna, one view on owned in-root
allowed.txt, at most two observed chat spans, 120 seconds/process and 300 seconds
batch, no retry/fallback. Only view available; shell/write/URL denied, built-in
MCPs disabled, cwd verification and no extra roots. No negative/symlink read was
requested. The owned outside/symlink compatibility fixtures were snapshot inputs
only, with a guard against any second or other-path view.

Only toolArguments capture was requested true. Prompts, responses, toolOutput,
identity, policyDetail and the broad message-content flag were false. Exact
synthetic settings remained byte-identical. That is a requested configuration
plus post-state check, not a receipt of complete effective managed policy.

## Partial measurements

| Measurement | Retained observation |
| --- | --- |
| Live native attempts | 1 |
| First matched forbidden attribute key | gen_ai.output.messages |
| Forbidden value decoded or retained | No; lexical stop occurred at the key |
| Outcome / batch complete | unverified / false |
| Stop reason | Unexpected exported content class |
| Reported observed chat spans | 0; counter incomplete because parsing stopped first |
| Usable native view receipt | None collected; viewCalls empty |
| Duration | 6.749 seconds |
| Foreground stop recorded / exit status | true / 0; neither establishes diagnostic completion |
| Settings / both file and symlink snapshots | Unchanged |
| Disposable roots removed | true |

The projector stops inside an exporter attributes object before the complete row
passes record-type, operation, model/provider, definition and correlation checks.
Therefore this is an observed attribute key, not a fully validated chat-span or
model/provider receipt. gpt-6-luna is the requested model. The zero chat count is
not evidence of zero inference, retries, billed calls or token usage. No usage
receipt was retained, and unused observer headroom authorizes no retry.

Partial native metadata ends at assistant.turn_start. Empty viewCalls and
accessConfirmationObserved=false mean no usable observation was collected;
they do not prove absence of tool calls, reads or confirmations. Unchanged files
establish no mutation, not absence of reads. No native result/error body was
inspected, including the positive target’s result. The final outcome remains
unverified rather than a read-success or permission-denial classification.

## Source and observer boundary

Before the live process, static inspection matched the native executable and
six selected cache files to the embedded runtime archive. Source discovery
accepted only the synthetic repository instruction and two known vendor built-in
skill listings. These are guards, not complete startup/source attestation.

The [packet manifest](fixtures/copilot-1.0.90-exporter-diagnostic/manifest.json)
binds exact inputs/settings, configuration, historical authority, runtime
identity and [sanitized receipt](fixtures/copilot-1.0.90-exporter-diagnostic/summary.json).
The [observer binding](fixtures/copilot-1.0.90-exporter-diagnostic/observer-binding.json)
identifies the exact source used; it has not been modified after the run.

The lexical projector decodes permitted metadata and own view start arguments,
skips native result/error/assistant/reasoning bodies, and raises at the first
configured forbidden exporter key before decoding its value or later records.
The stop is latched so cleanup does not parse exporter records again. Only the
fixed matched name is retained; no value/shape/hash is extracted for that field.
The summary retains only the authorization configuration binding, with the safe
historical owner authorization stored separately in the packet.

Seven model-free tests cover diagnostic completion, forbidden-name stop, an
invalid forbidden value that must not be decoded or reparsed, arbitrary extra
authorization fields, missing telemetry, failed chat correlation and a second
view. Two privacy findings were reproduced RED then corrected GREEN before the
live attempt; all seven pass. Installation identities and native invocations
are synthetic in these checks; fixture/Git/projection/cleanup behavior is real.
Fresh written review confirmed the corrected boundary before exact approval.
These checks establish observer behavior, not native permission enforcement.

## Retention and handoff

All private native-history, settings, OTel, stdout/stderr and log roots were
removed. No raw vendor prompt, assistant/reasoning value, tool-result body,
credential or session identifier is published. Cleanup cannot establish
pre-storage exclusion or remote/provider deletion. Managed overrides, complete
requests, every persistence path and process-tree behavior remain unqualified.
The name receipt is observer evidence; deleted raw data cannot independently
corroborate it now.

This attempt is exhausted. A useful next research step is a separately reviewed
way to distinguish tool-call metadata from response body within this specific
attribute, preferably from static version-bound implementation evidence. This
proposal grants no new value inspection, capture class, model call or runtime
launch. Read-scope enforcement, native skill loading, continuation and complete
role eligibility remain separate unverified gates. No product code changed.
