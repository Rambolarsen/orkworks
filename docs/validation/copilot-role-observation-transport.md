---
type: "Validation Record"
title: "Copilot role observation transport research"
description: "Validation evidence and context: Copilot role observation transport research."
tags: ["orkworks", "validation"]
---

# Copilot role observation transport research

- Date: 2026-10-05
- Tracker: [#740](https://github.com/Rambolarsen/orkworks/issues/740), initiative #738
- Status: candidate identified; measurement qualification and role eligibility remain unverified
- Inputs: [role contract](../superpowers/specs/2026-10-04-agent-role-configuration-design.md), [probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md)
- Evidence: [transport inspection manifest](fixtures/copilot-1.0.90-transport/manifest.json)

## Subsequent qualification

The [native 1.0.90 inspection](copilot-native-receipt-qualification.md) now binds
actual embedded schemas/source to the installed executable. The 1.0.28
companion excerpts below remain historical leads. Ordinary CLI JSON stdout
excludes the necessary receipts; OTel still lacks safe capture and a retained
runtime example. No transport or role has qualified.

## Decision at this inspection

Investigate Copilot's native OpenTelemetry file exporter as the first candidate
for measuring delivered instructions and model-visible tool definitions. The
installed 1.0.90 monitoring help advertises both under content capture. This
advances the earlier search for a mechanism; it does **not** qualify the
transport. No event stream, exporter output, model request, denial test or
content receipt was produced in this research session.

All six role profiles remain unavailable. The next gate is an exact-version
schema and a bounded synthetic capture establishing completeness, content
identity, session/turn correlation and permitted retention. Until then, do not
write a production observer, start permission probes or promote discovered
skills into confirmed delivery.

## Retained facts and provenance

`copilot --no-auto-update --version` reports 1.0.90. The retained loader and
native executable hashes match the earlier packet. Every captured invocation
now includes `--no-auto-update`, which avoids treating a cached downloaded
update as the inspected executable. The manifest records commands, cwd,
platform, exit status, byte counts and SHA-256; stderr is not evidence.

| Evidence | Established fact | Qualification limit |
| --- | --- | --- |
| [Monitoring help](fixtures/copilot-1.0.90-transport/monitoring.txt) | Native help describes a local JSON-lines file exporter, content capture, system instructions and tool definitions | Help supplies neither an exact event example nor proof of complete capture |
| [Logging help](fixtures/copilot-1.0.90-transport/logging.txt) | Logging levels exist; OTel is a separate monitoring surface | Debug logging is not a delivery schema or a permission receipt |
| [Companion event schema excerpts](fixtures/copilot-1.0.90-transport/companion-1.0.28-event-schema-excerpts.json) | The installed companion schema defines `system.message`, `skill.invoked` and `session.tools_updated` | Both npm package manifests report **1.0.28**; these are leads, not a 1.0.90 runtime schema |
| [Companion tools-list schema](fixtures/copilot-1.0.90-transport/companion-1.0.28-tools-list-schema.json) | `tools.list` describes a model-dependent built-in tool catalogue without a session ID parameter | A catalogue does not establish the exact custom-agent/session/MCP inventory |
| [Companion exporter source excerpt](fixtures/copilot-1.0.90-transport/companion-1.0.28-file-exporter-excerpt.txt) | Companion JavaScript serializes spans and metrics as separate JSON-lines records | The loader prefers a native executable; this source is not proven to implement its exporter |

The excerpts are selected source/schema subtrees, reserialized where identified
in the manifest. They are not captured runtime events. Full source-file hashes
allow reproduction of each selection; excerpt hashes identify the retained
bytes. No SDK client, ACP server, MCP server or inference session was started.

Current [GitHub monitoring documentation](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#opentelemetry-monitoring)
names instruction and tool-schema attributes plus a session identifier. The
manifest records a dated documentation source commit/blob; neither that pointer
nor current documentation is matched to the 1.0.90 release. Installed help is
the version-specific advertised interface. Source pointers cannot substitute
for retained runtime evidence in a future capability snapshot.

## Candidate measurement surfaces

| Surface | What to investigate | What it cannot currently establish |
| --- | --- | --- |
| OTel file exporter | Native help advertises system instructions, tool definitions, prompts/responses and tool arguments/results under one capture gate | Exact 1.0.90 JSON shape, loss/truncation behavior, complete per-request inventory or byte-bound rule/skill/resource delivery |
| CLI `--output-format json` | Installed help exposes JSONL; companion schemas contain content-bearing events | Which events reach CLI stdout, whether content is complete, and whether schemas match the native binary |
| Companion session events | `system.message.data.content` describes system/developer text; `skill.invoked.data.content` describes injected skill content | Actual emission/coverage in 1.0.90, applicability of scoped rules, or delivery of referenced resources |
| Companion `session.tools_updated` | The retained event carries a `model` field | It carries no tool inventory; do not interpret it as proof that forbidden tools are absent |
| ACP | Current [ACP reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server) describes a client protocol and session notifications | No inspected version-bound receipt establishes complete system content or effective tools; command availability is a different inventory |

Prefer the native telemetry candidate because it is advertised by the exact
executable without changing the product to an SDK integration. Keep CLI events
as a possible corroborating source if their actual schema/coverage is established.
Do not transfer SDK configuration fields or companion schemas into CLI launch
claims. None of these observations establish same-parent model-loop continuation.

## Qualification checks before an observer or role probe

These checks refine Task 1 of the existing plan. They do not authorize a live
batch or replace Task 2's exact configuration review.

1. **Pin an actual schema and sample.** Obtain a primary schema applicable to
   the native executable and a safely retained synthetic runtime example.
   Identify whether file output uses exporter-specific records or an OTLP
   envelope; do not assume the companion source's `type: span` shape. Preserve
   the schema/source version and raw-to-retained transformation. No fabricated
   event can satisfy this check.
2. **Establish capture safety first.** Installed help's content gate couples
   inputs with responses and tool content. Before enabling it, establish how
   forbidden content, especially hidden reasoning and credential values, is
   excluded before storage. Post-capture redaction cannot prove it was never
   stored. Use only synthetic instruction/task bytes and enumerated settings;
   production prompts/transcripts are excluded. A route unable to satisfy the
   plan's retention rules remains no-go even if it exposes useful fields.
3. **Bind content, scope and time.** Associate every necessary observation with
   the exact native session, observer-owned runtime/generation and model call
   or turn. Independently hash the approved role, root/scoped rules, selected
   skills and referenced resource closure. Compare complete delivered text
   after decoding its documented envelope, preserving UTF-8, LF/CRLF, spaces
   and terminal newlines; marker echoes or filenames cannot pass. Establish
   when nested rules/resources become applicable and arrive before the request
   that needs them. A tool result containing text is insufficient without
   evidence of its inclusion in subsequent model input.
4. **Prove complete effective tools.** Compare actual model-facing definitions
   against the reviewed allowlist, including all builtin, MCP, delegation,
   shell-session and permission-changing routes. Distinguish absent tools
   from available tools requiring permission. Determine when inventory is
   captured: an invocation-level snapshot cannot certify later requests after
   skill loading, model changes or dynamic tool retrieval unless completeness
   is separately established. One used tool or a global catalogue is not an
   inventory. Even a complete inventory does not prove filesystem/network
   denial or instruction adherence.
5. **Fail on incomplete measurement.** Missing fields, unmatched sessions,
   uncorrelated requests, unknown startup sources, altered bytes, parse errors,
   lost spans, exporter failure, truncation and overflow keep the case
   unverified. Inspect shutdown/flush and timing behavior; do not treat an
   empty file as evidence of absence. Keep the existing 128 KiB artifact and
   2 MiB batch ceilings; exceeding them cannot be fixed by silently dropping
   evidence. Demonstrate wrong-byte and incomplete-inventory negative cases
   before relying on the observer.

Successful measurement qualification would permit review of the first exact
synthetic role batch, not establish the role itself. Read-scope denial,
skill-induced tool widening, startup/settings/model binding, exact resume and
the other ten #741 evidence surfaces still require their own fixtures.
Command-enabled roles additionally require the empty-start environment
construction receipts. Orchestrator continuation remains a separate batch.

## Investigated uncertainty and handoff

**Least confidence:** whether a complete content/tool receipt exists in the
actual native release. Native monitoring help supplies a credible candidate;
the installed companion source/schema cannot resolve its runtime coverage.
The uncertainty is now a specific qualification gate rather than an assumed
absence of monitoring support.

**Project blind spot:** a content capture flag is broader than the needed
receipt. It can expose response content while an invocation snapshot can miss
later tool changes. The existing plan already forbids hidden reasoning and
incomplete evidence. The capture-safety and per-request completeness checks
above operationalize those constraints; no new product scope is proposed.

#741 consumes the unchanged unverified eligibility decision and explicit
version/source distinctions. #743 may investigate native invocation events,
but a companion `skill.invoked` schema or OTel skill name alone does not confirm
delivered resource bytes or justify observed usage in production. #742 still
has no qualified wait/event/resume transport. No consumer may infer support
from this research packet's hashes.

Keep #740 open. Review these findings and resolve the schema/example and safe
capture gates before approving the exact model/argv/settings/authentication,
observer and finite budget in the separate probe session. If the candidate
cannot meet those checks, record a precise transport no-go and stop rather
than changing #741's contract or introducing a fallback integration.
