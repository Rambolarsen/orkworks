---
type: Validation Evidence
title: Copilot settings-only capture comparison
description: Two approved hosted turns contrast metadata-only and narrow instruction/tool capture without qualifying native persistence or a role.
tags: [copilot, validation, role-profiles, telemetry]
---

# Copilot settings-only capture comparison

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
consumers #741–#743. Executes the
[approved one-batch boundary](../superpowers/plans/2026-11-01-copilot-synthetic-capture-boundary.md)
after the [legacy settings migration finding](copilot-settings-and-persistence.md).

## Decision

**The two-case settings-only contrast passed for this pinned synthetic batch.**
With all capture classes off, neither observed span carried input-message,
system-instruction or tool-definition attributes. Enabling only prompts and
toolArguments supplied those attributes, with the exact synthetic root-rule
text and expected observed tool names on the chat span. Both cases omitted
response-content attributes in the checked span-attribute projection.
This is bounded OTel selection evidence, not complete capture safety, effective
permission enforcement or role eligibility. All six roles remain unavailable.

## Authority and exact inputs

The owner explicitly approved the exact two-call batch in draft
[PR #1276](https://github.com/Rambolarsen/orkworks/pull/1276), including ordinary
native history inside private disposable homes and the declared unknown managed
capture/remote/provider-retention risks. The
[historical authorization](fixtures/copilot-1.0.90-capture-comparison/authorization.json)
binds the response to configuration SHA-256
`c9d27f1752467e12baf9ac26e19ea9e46645e2171c9af8464a6b8423def09777`.
It authorizes no replay or later batch. The immutable configuration's original
proposed status is historical; authority and outcomes are separate artifacts.

Each case used fresh private synthetic repo/settings/log directories with the
retained `AGENTS.md` bytes. Only `settings.json` was seeded; `config.json` was
absent before both live launches. Exact independently materialized settings
files differ only in the prompts/toolArguments capture booleans. Both remained
byte-identical after source preflight and after their hosted turn.

Four bounded discovery commands accepted only the synthetic root instruction
and two named package built-in skill listings. This guarded sources without
claiming a complete startup attestation. Existing authentication used the approved
native resolution route; the observer copied no credentials or real user
configuration. Remote control/export were requested off through both settings
and flags; their enforcement and remote/provider retention were not measured.

The native executable and selected runtime/cache bytes matched the pinned
1.0.90 package via the retained
[identity recheck](fixtures/copilot-1.0.90-capture-comparison/embedded-runtime-identity.json).
No package, policy, trust-store or login changes were made.

## Results

| Measurement | Metadata only | Instructions and tools |
| --- | --- | --- |
| CLI exit / stderr bytes | 0 / 0 | 0 / 0 |
| Duration | 6.314 seconds | 5.690 seconds |
| Expected response | `42` matched | `42` matched |
| Observed chat spans | 1 | 1 |
| Request and response model | gpt-6-luna | gpt-6-luna |
| Input / output / reported reasoning tokens | 4080 / 43 / 36 | 4082 / 83 / 76 |
| Input, system, definitions attributes | Absent on chat and invocation spans | Present on chat and invocation spans |
| Exact root-rule text | No content receipt expected | Present in decoded chat system instructions |
| Observed definitions | No definition receipt expected | view, skill, rg, glob |
| Settings after launch | Exact before-bytes unchanged | Exact before-bytes unchanged |
| Raw OTel size, unretained | 10,545 bytes | 52,644 bytes |

The observer required provider `github` on every span and rejected unknown span
operations. Both request/response model checks passed. No observed tool span
occurred. Two live attempts produced exactly two observed chat spans; internal
provider attempts, retries, billing and full request-generation coverage remain
unverified. No application retry or provider fallback was requested.

The arithmetic completion is only a success check. Delivery rests on the
transport-backed rule-text comparison, not on the model echoing a marker.
Metadata-only inventory remains unknown; absence of definition attributes is
expected for that case and cannot prove absence of tools. The requested grep
versus observed rg relationship remains unresolved; the comparison grants no
permission equivalence.

Both native event files matched the chat conversation/session, turn and
interaction identifiers internally. Retained receipts contain only correlation
booleans and counts, never the identifiers. Native-history inspection scanned
structure, field names and whitelisted metadata IDs while skipping content and
reasoning values. Assistant content/reasoning-related key presence is recorded;
values, emptiness, meaning and absence were deliberately not tested.

## Evidence limits and cleanup

The [packet manifest](fixtures/copilot-1.0.90-capture-comparison/manifest.json)
binds exact inputs, authorization, settings, observer source, runtime identities
and [sanitized measurements](fixtures/copilot-1.0.90-capture-comparison/summary.json).
Raw content attributes are represented only by byte counts and SHA-256 values.
Raw OTel, vendor instructions, model output/reasoning, identifiers, native
history/SQLite state, process logs and temporary configuration were removed
with the owned disposable roots. Both foreground processes exited normally;
cleanup succeeded. No process-tree or physical-erasure guarantee is asserted.

The retained observer source has only trailing whitespace removed from the
script used; its original hash/size and verified Python AST equivalence are
recorded. It is not a native dependency or provider-request attestation. Its helper guards were checked using synthetic
metadata and a mocked preflight failure without invoking Copilot. A static
review identified cleanup/provider/correlation gaps; these were fixed before
launch. The final observer rejects unknown operations, bounds evidence, stops
after the first failed case and cleans up in an unconditional outer finalizer.
No inference ran during those observer checks.

Because raw files were deleted, a reader cannot independently recompute their
content receipts. These are observer measurements, not replayable raw evidence.
The comparison checks selected span attributes only, not every exporter event,
metric, later input, native persistence path, managed-policy variant, remote
store or credential side channel. A pseudonymous identity attribute name remains
in the invocation projection even with identity capture off; no value is retained.
Deletion afterward is not proof of pre-storage exclusion. Rerunning the retained
observer requires a fresh reviewed authority/budget; the historical receipt is
an audit input and cannot enforce or grant authorization.

## Consumer handoff

#741 receives one exact settings-only OTel contrast and unchanged unavailable
role decisions. It does not receive a verified effective permission profile.
#743 receives exact root-rule delivery evidence for the narrow case; no selected
skill body/reference or native skill invocation was tested. #742 receives no
resume or same-parent continuation evidence. #740 stays open.

The approved two-call batch is exhausted. A useful later experiment is a
separately reviewed skill-body delivery case using this settings-only candidate,
with a new explicit budget and equally narrow retention rules. No later case,
new capture class, wider permission or runtime role launch is authorized here.
