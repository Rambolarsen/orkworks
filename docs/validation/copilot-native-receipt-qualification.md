---
type: "Validation Record"
title: "Copilot 1.0.90 runtime and receipt qualification"
description: "Validation evidence and context: Copilot 1.0.90 runtime and receipt qualification."
tags: ["orkworks", "validation"]
---

# Copilot 1.0.90 runtime and receipt qualification

- Date: 2026-10-05
- Tracker: [#740](https://github.com/Rambolarsen/orkworks/issues/740), initiative #738
- Evidence: [inspection manifest](fixtures/copilot-1.0.90-native/manifest.json)
- Contract: [role configuration](../superpowers/specs/2026-10-04-agent-role-configuration-design.md)
- Execution gate: [bounded probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md)

## Decision

The installed native executable is byte-identical to the official npm
`@github/copilot-darwin-arm64` 1.0.90 executable. Its embedded runtime archive
contains the actual 1.0.90 schemas, and six selected runtime/cache files match
those embedded bytes. This resolves the earlier source-version uncertainty
without installing a package or starting a model.

The ordinary CLI `--output-format json` route is **no-go for mandatory content
delivery measurement**: its version-matched writer explicitly omits system
messages, skill invocations and skill-delivery receipts. Useful receipt schemas
exist internally, but do not make those receipts observable through this route.
OTel remains an unqualified alternative; experimental session RPC methods are
static research leads, not a reviewed CLI observer or approved SDK integration.
All six role profiles remain unverified/no-go. No inference, server, observer,
permission denial or continuation probe ran.

## Reproducible runtime identity

The version-specific [wrapper metadata](https://registry.npmjs.org/@github%2fcopilot/1.0.90)
and [native metadata](https://registry.npmjs.org/@github%2fcopilot-darwin-arm64/1.0.90)
identify the official archives. Both downloads passed their registry SHA-512
integrity checks. The native archive's executable SHA-256 is
`3ae21a3f00fcc216faaa1f062ee98451c7807066ee26f0fe1c85c6ed315e5458`,
matching the installed executable. The [package comparison](fixtures/copilot-1.0.90-native/package-comparison.json)
records archive sizes/digests and complete outer member inventories. This is
registry integrity and byte equality, not independent publisher attestation.
Shell registry networking was initially blocked; the approved escalated
read-only download succeeded. GitHub `gh` read access also succeeded.

The executable is a Node single-executable application. A read-only parser
locates its Mach-O `__NODE_SEA_BLOB`, checks the SEA header, consumes the entire
payload and reads the `copilot.tgz` asset. No extracted JavaScript or native
addon is executed. Node's [SEA documentation](https://nodejs.org/api/single-executable-applications.html)
and [deserializer source](https://github.com/nodejs/node/blob/v24.9.0/src/node_sea.cc)
explain the format; the Node source reference describes the parser layout,
not an observed Node runtime version for Copilot.

The [runtime identity record](fixtures/copilot-1.0.90-native/runtime-identity.json)
pins the embedded archive, loader and selected files: `package.json`, `index.js`,
`app.js`, both schemas, and `prebuilds/darwin-arm64/runtime.node`. The package
declares 1.0.90. All six match the existing cache at
`~/Library/Caches/copilot/pkg/darwin-arm64/1.0.90/`. This is a selected-file
comparison, not a complete transitive dependency or effective-startup audit.

The stale installed outer manifests and companion source still declare 1.0.28.
Their earlier packet remains historical evidence; use this embedded 1.0.90
source for native schema claims. Do not replace the installed outer loader's
identity with the newly downloaded wrapper's different bytes.

### A version string does not bind the loaded runtime

The [embedded loader excerpts](fixtures/copilot-1.0.90-native/sea-loader-excerpts.json)
show a version-only fast path that can return before importing the runtime.
Cache reuse checks a completion marker and readable required files rather than
content digests. Cache-root, selected-version and distribution-directory
overrides can select other files. Six relevant override variables were absent
in this inspection; only presence was checked, with no values or ambient
environment dump retained. The variable names are listed in the manifest.

A future probe must pin the resolved loaded distribution and relevant runtime
bytes alongside the executable chain and nonsecret effective configuration.
`--no-auto-update` plus `--version` is insufficient on its own. This clarifies
the existing actual-runtime identity requirement rather than adding a new
product control.

## What the matching schemas establish

Selected event definitions and their complete local reference closure are
retained in [event schema selection](fixtures/copilot-1.0.90-native/events-selected.schema.json).
They are reserialized selections from the hashed source, not raw event samples
or a copy of the full schema.

| Surface | Static evidence | Qualification limit |
| --- | --- | --- |
| `system.message` | Content, system/developer role, optional interaction ID and structured blocks | Emission, complete rule/resource coverage and delivery timing unobserved |
| `skill.invoked` | Skill path/content and optional model, source, trigger, allowed-tools metadata | Invocation alone cannot establish all resource bytes or later model input |
| `skill.context_delivered` | Exact model-facing wrapper and source; optional interaction ID | Internal/experimental; absent from ordinary JSON stdout |
| `skill.context_delivered_ref` / `skill.invoked_ref` | Schema descriptions specify same-session UTF-8 SHA-256 lookup, wrapper prefix/suffix and invocation UTF-16 length validation | Observer must resolve the earlier body and validate reconstruction/correlation; native resolver enforcement remains unverified |
| `session.skills_loaded` | Resolved name/path/source/enabled metadata | Discovery metadata, not content delivery |
| `session.tools_updated` | Ephemeral notification with model ID only | No tool inventory; missed ephemeral events are not replayable |

The reference fields themselves are a string and a nonnegative integer, not
JSON Schema constraints enforcing same-session lookup, digest/length matching
or dangling-reference rejection. Those behaviors are described in prose. No
resolver implementation or live negative case was inspected; native enforcement
remains unverified. A future observer must validate them independently and
reject unresolved/malformed references before claiming a content receipt.

Event envelopes carry UUID, timestamp, preceding event ID and optional subagent
ID. They do not uniformly carry session or required interaction identity.
The transport must independently bind session/runtime/generation and model
request; an event-chain parent ID is not that binding.

The [selected tool RPC schemas](fixtures/copilot-1.0.90-native/tools-selected.schema.json)
define `session.tools.initializeAndValidate` and
`session.tools.getCurrentMetadata`. The latter returns initialized session tool
metadata or null. Entries require name/description; input schema, MCP identity
and deferred-loading metadata are optional. Initialization can be a no-op for
unsupported sessions. A snapshot has no mandatory request/generation binding
and does not establish the complete definitions supplied on every later model
request. These experimental methods improve the lead beyond a global builtin
catalogue, but do not yet satisfy the inventory gate.

## Ordinary JSON stdout is an unsuitable receipt transport

The [version-bound writer excerpts](fixtures/copilot-1.0.90-native/json-output-excerpts.json)
retain the JSON-mode selection, subscriber/writer call path and exclusion set
with UTF-8 source offsets. Before writing, the exclusion set drops
`system.message`, `skill.invoked`, `skill.invoked_ref`,
`skill.context_delivered` and `skill.context_delivered_ref`. Secret filtering
occurs on the remaining serialized records. No runtime example is needed to
reject this writer as the required receipt route; this is a static exclusion
finding, not a runtime permission or delivery result.

The actual event schema also includes reasoning-bearing events. An event-log
RPC advertises type filtering and cursor-based reads, but no server/client was
started and its pre-storage behavior was not established. Filtering a later
read or redacting a copied log does not establish that forbidden content was
never stored. Likewise, native monitoring help's coupled OTel content gate
remains unresolved. Do not enable broad content capture to see what it contains.

## Investigated uncertainty

**Least confidence:** whether OTel or an existing native session channel can
provide complete request-bound receipts without forbidden capture. Static
schema presence cannot resolve live emission, storage or completeness.

**Project blind spot:** the executable version can be reported before importing
mutable cached runtime files. The selected-file comparison resolves today's
source binding, but a future probe needs fresh loaded-distribution identity and
the complete effective startup-source audit already required by Task 2.

## Handoff and bounded next action

Task 1's native source/schema binding has progressed; its safe runtime example,
capture-before-storage and per-request completeness gates remain open. The
retained [reproduction instructions](fixtures/copilot-1.0.90-native/README.md)
allow an independent static check without inference or a Copilot server.

Before proposing live role probes, establish a native observation route that
exposes necessary receipts while excluding forbidden capture before storage,
and binds a complete effective inventory to each relevant request. The ordinary
JSON stdout candidate is rejected. OTel and internal RPC remain unverified.
If neither satisfies the bounded contract, retain a measurement no-go rather
than creating an SDK integration, broker, alternate-tool fallback or wider
permissions. A schema-only result cannot satisfy the plan's retained-example
requirement or authorize an observer.

#741 receives the unchanged eligibility result and stronger runtime identity;
#743 receives version-bound receipt/ref semantics with unknown live coverage;
#742 receives no continuation/resume qualification. Reading events does not
prove that an approval/report wakes the same parent model loop. Keep #740 open
for the remaining evidence and reviewed handoff.
