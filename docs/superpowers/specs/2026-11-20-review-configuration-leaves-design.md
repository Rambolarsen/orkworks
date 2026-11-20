---
type: Design
title: Ordinary-review catalog and integration leaves
description: Proposed complete logical source-byte snapshots behind ordinary-review configuration digests, with read-only capture and explicit availability limits.
tags: [orkworks, review, configuration, source-contracts]
status: draft
---

# Ordinary-review catalog and integration leaves

**Proposed for owner review with the coupled offline plan.** The owner requested
the next prerequisite after [#1922](https://github.com/Rambolarsen/orkworks/pull/1922).
This authorizes investigation/preparation; new leaf, availability and budget
choices await acceptance before dependent implementation. Track
[#68](https://github.com/Rambolarsen/orkworks/issues/68),
[#66](https://github.com/Rambolarsen/orkworks/issues/66) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899), as prerequisites to
the separately gated #738/#741 role/configuration work.

Reuse the accepted [launch-input contract](2026-11-19-review-launch-input-design.md),
[preview binding](2026-11-19-review-handoff-binding-design.md) and
[admission source contract](2026-11-19-review-source-qualification-design.md).
The result here is a logical byte/coverage contract, not an actual collector,
resolver, integration adapter, native profile or Start implementation.

## Source findings

Audit base: `b4dfd63b54aed32df4112224d86a937d1686beb0`.
All paths below are under `crates/orkworksd/src/` unless stated otherwise.

| Source | Established behavior | Consequence |
| --- | --- | --- |
| `harness/definition.rs::EMBEDDED_BUILTINS`, `registry.rs::resolve_document` | Effective registry depends on the whole embedded builtin document, user overrides/custom/compatibility profiles and resolver logic. The resource is 10,537 bytes at the audit base. | A stored user-document hash alone is incomplete. Capture full applicable source bytes and bind the resolver policy/build. |
| `harness/store.rs::load`, `load_for_startup` | Missing user document means default version-3 empty inputs. Invalid startup document can fall back to builtins while preserving the invalid file. | Proposed first leaf requires proved absence; present/invalid/unreadable are unavailable, never treated as empty. |
| `HarnessStore::with_locked_snapshot`, `document_lock` | Store mutex/file lease surround a snapshot, but acquiring the file lease can create a directory/lock file. | Do not call this helper from preview inspection and assume it is read-only. A separate nonmutating capture seam is required. |
| `HarnessStore::mutate_at` | Writes the document before swapping the catalog Arc. A failed publication can leave disagreement; current revision is content-based. | Compare exact owners plus a new checked capture epoch; neither equal text nor override hash proves history/custody. |
| `harness_integration_application.rs::inspect`, `list_workspace` | Inspection can revoke stale Claude/Copilot prompt authority. Some workspace reads default after errors. | Do not reuse inspection as pure capture; unknown reads must deny. No install/reconcile/revoke during preview. |
| `IntegrationHandler`, `ReporterAssetResolver`, `ToolHookContract` | Adapter binding owns launch behavior and source/installed assets. Reconcile writes assets; status has broader semantics than byte capture. | Capture the code-owned read inventory and observations without invoking those actions. |
| `session_application.rs::create_session_workflow` | Integration read/augmentation errors are tolerated; workspace reconciliation and later startup work follow. | This path is not qualified unchanged for exact prepared creation. Preserve manual behavior separately. |
| `runtime/worktree_instructions.rs::PreparedScope::apply` | New scoped OpenCode launch changes argv[0], adds --session with a fresh selector and sets environment after preparation. | An ordinary-review profile promising frozen literal argv cannot silently reuse that path. |
| `runtime/terminal_runtime.rs::session_env_overrides_with_prompt_generation` | Session ID, reporting capability, port and optional prompt generation are supplied at startup. | Commit a closed allocation policy, not secret/session values that do not yet exist. |

## First leaf availability: stock inputs only

Propose the first catalog leaf for the **unmodified stock registry with an absent
user harness document**. All embedded builtin bytes are captured, including
unselected definitions/legacy snapshots. The requested ID must be its canonical
builtin ID, non-retired and non-generic; alias/custom/override resolution is not
included in this first version. A present file, even an empty valid document,
requires a later source profile. Nothing deletes, resets or rewrites that file.

This is an explicit narrowing of the first available capture profile under the
accepted #1922 configuration value; it does not remove accepted logical fields
or change ordinary/manual launch. The choice avoids inventing a credential
classifier or copying arbitrary user commands into the snapshot. No automatic
normalization, fallback catalog or migration makes a rejected source usable.
It is an availability decision for owner review, not a supported-tool claim.
The supported production-role set remains empty.

Choose whole embedded source bytes over a second projection of its fields:
unknown/new builtin fields then change the commitment rather than disappearing.
Existing Rust parsing/resolution must eventually qualify that the captured stock
inputs really produce the selected resolved command. An offline byte container
is not a replacement HarnessDefinition parser or proof of resolver correctness.

## Catalog leaf v1

Closed fields, in canonical order:

`{schemaVersion, kind, ownerInstanceId, ownerRevision, sidecarBuildId, platform,
requestedToolId, modelId, builtinDocumentBase64, userDocumentState, resolverPolicy,
resolvedCommand}`

- schemaVersion 1; kind ordinary_review_catalog; userDocumentState absent;
  resolverPolicy stock-registry-v2-default-user-v3-v1.
- ownerInstanceId is UUIDv4 backed by the exact live configuration owner;
  ownerRevision is its positive checked u64 epoch, including observed failed
  mutations/ABA. These are the same owner/epoch as #1922 configuration, not a
  second independent revision domain.
- sidecarBuildId is 64 lowercase hex from the source-owned immutable running
  build identity. It is not a caller assertion or proof of executable ownership.
  platform uses the existing macos_arm64/linux_x64/windows_x64 value tags.
- requestedToolId uses #1922's opaque ASCII ID domain; modelId is nonempty scalar
  UTF-8 at most 256 bytes, without controls. No default model substitution.
- builtinDocumentBase64 is canonical padded base64 of the **complete original
  embedded document bytes**, at most 64 KiB decoded. Raw bytes are not normalized,
  parsed/reformatted before commitment, or substituted by an earlier hash.
- resolvedCommand is `{program, argv, cwd}`: source-resolved canonical absolute
  UTF-8 program/cwd paths at most 4 KiB, ordered at most 64 argv strings at most
  1 KiB each, preserving empty entries. Its producer must prove exact selected
  stock tool/model resolution. Native canonicality/eligibility remains separate.

Whole canonical catalog leaf limit: 128 KiB. Structural component acceptance
only proves the wrapper/byte domains; it does not prove that arbitrary base64
bytes form the actual stock catalog, that a requested tool exists, or that the
command follows from it. Those joins require independent source-owned facts and
actual Rust resolver qualification, not another client-controlled boolean.

## Integration leaf v1

Closed fields, in canonical order:

`{schemaVersion, kind, ownerInstanceId, ownerRevision, workspaceId,
workspaceGeneration, catalogDigest, selectedToolId, adapter, activeSelection,
inventory, resources, runtimePolicy, finalCommand}`

schemaVersion 1; kind ordinary_review_integration. Owner fields equal the catalog
and #1922 configuration owner/epoch. Workspace identity/generation equal the
prepared input's owner-derived 64-hex ID and positive u64 generation.
catalogDigest equals the complete catalog leaf digest defined below;
selectedToolId equals its requestedToolId and the selected draft/configuration.

adapter is `{binding, representativeToolId, policyVersion}`. binding is none,
claude, codex, open-code, gemini, copilot or aider (the existing code-owned binding
spellings); representativeToolId is a canonical builtin opaque ID or null only
for none; policyVersion is literal owned-integration-inputs-v1. A custom name
cannot select the representative, handler, hook path or resource inventory.

activeSelection is `{revision, toolIds}`: workspace active-selection
revision (canonical u64 decimal allowing zero, matching the existing source)
and at most 64 unique canonical stock IDs, sorted by unsigned UTF-8 byte
order. The selected tool must be active. Unreadable state, unknown IDs or retained
retired IDs deny this first profile; there is no unwrap_or_default fallback.
This does not rewrite the user's selection.

inventory is at most 64 `{id, role, presence}` records, ordered by id.
presence is required or observed: required inputs must be regular, while observed
inputs may be absent as explicitly captured by the policy. IDs are opaque ASCII
1..128 bytes. Roles are workspace_preference, native_owned_fragment, source_asset
or installed_asset. resources is the exact same ordered ID/role set, with each
record `{id, role, path, state, bytesBase64}`. Path is an owner-selected canonical
absolute scalar UTF-8 path at most 4 KiB. state is absent (bytesBase64 null) or
regular (canonical padded base64, including an empty file). Each decoded resource
is at most 256 KiB. Symlink, unreadable, nonregular, unknown, duplicate or extra
observations deny capture; they must not be reported as absence. Required absence
also denies availability; weakening a required inventory item is not a refresh.

The inventory must come from the selected adapter's **independently qualified
complete code-owned read set** for that platform/policy. It covers all owned
preferences, owned native fragments and source/installed reporter/helper assets
that affect resolution, readiness or delivery, including expected absences.
This design fixes the record/coverage relation, not each adapter's actual list:
actual inventory/projector completeness remains an explicit qualification gate.
An offline caller supplying both arrays cannot establish completeness. The plan
compares them to separate literal expected-inventory fixtures and calls the result
conditional; it does not infer a real adapter inventory from those examples.

For native configuration, resources contain only the complete OrkWorks-owned
fragment, validated against its exact code-owned template. They do not copy the
whole user/vendor configuration or try to classify secrets in arbitrary text.
Unknown/customized owned fragments deny this profile. Other native/repository
configuration remains ambient context outside the immutability guarantee already
stated in #1922; none of it is declared filesystem confinement or complete native
context. No runtime credential/report token may enter either leaf.

Whole canonical integration leaf limit: 512 KiB. Raw/encoded aggregate limits
both apply; per-resource maxima do not promise combined fit. Resource bytes and
paths are internal capture data, not returned to the renderer, logged or persisted.

## Runtime policy and frozen command

runtimePolicy is a closed fixed record:

`{terminalEnvironment:terminal-v1, sessionReporting:fresh-session-report-v1,
promptHook:disabled, worktreeInstructions:none, initialInstruction:exact-one-turn-v1}`

The first profile excludes launch paths requiring scoped post-capture argv or
native prompt-generation authority. If the selected tool/profile needs those
features, capture is unavailable. Do not silently disable an installed feature
or change the existing manual launcher to satisfy this prototype. In particular,
current scoped OpenCode creation is not qualified merely by selecting these tags.
A symbolic late-argv extension would need a separate reviewed contract.

The reporting policy reserves fresh server-generated session ID/report capability
and the current sidecar port for the admitted session. Actual values are allocated
through the future private creator after #1919 reservation; no capability, port,
attempt/session ID or native selector is copied into these source bytes. The
policy/build and workspace owner are committed; late identities are independently
bound to the reserved attempt/session under the existing admission contract.
This does not prove that the current creator implements that boundary.

finalCommand has the same closed shape/limits as resolvedCommand. It is the exact
result after all qualified, read-only integration preparation and canonical
executable resolution. Program/argv equal #1922 configuration executable/argv;
cwd equals prepared workspace root/source effective worktree. If an adapter
cannot produce/revalidate that complete literal command without later mutation,
this profile is unavailable. No shell/model/cwd fallback, hidden suffix, image,
effort, resume or custom environment path is added.

## Exact commitments and retention bounds

Use #1922's scalar UTF-8, fixed-key compact JSON and byte-equality decoding rules.
Base64 is RFC 4648 standard alphabet with canonical padding; decode then re-encode
must match exactly. No whitespace/alternate alphabet, lossy path/text conversion,
unknown keys, sparse arrays, duplicate IDs or omitted state fields. Null is allowed
only for none representativeToolId and absent bytesBase64. Counter/UUID/ID domains
reuse #1922. Exhausted owner epoch denies new capture, never wraps/resets.

```text
catalogDigest = lowercase_hex(SHA256(
  UTF8("orkworks.ordinary-review.catalog-input.v1\n") || C(catalogLeaf)))
integrationDigest = lowercase_hex(SHA256(
  UTF8("orkworks.ordinary-review.integration-input.v1\n") || C(integrationLeaf)))
```

The full source bytes, resource coverage and runtime policy are included, not
just existing hashes. These two digests join the complete frozen #1922 input;
capability evidence and actual owners remain independent requirements. Equal
content or commitment alone does not grant currentness, secrecy or authority.

Combined encoded leaf capture limit: 640 KiB, the sum of the individual maxima.
This bounds logical byte work, not a measured total resident/scratch-memory or
physical-operation envelope. Leaves are ephemeral capture/hash inputs and are
not retained in a second preview cache. The already-owned immutable build/catalog
sources and exact live references remain with their existing/future qualified
owners; preview retains only its bounded configuration/digests and live bindings.
The separate 64 KiB preview entry/response limits remain unchanged. No raw leaf
export/log/store or automatic installation is authorized.

## Read-only capture and publication boundary

Proposed future configuration owner is shared across the global catalog and
workspace-specific effective integration inputs. It binds the exact catalog,
workspace, native-fragment and asset owners, plus this one monotonic epoch.
Every observed controlled mutation, failed publication, recovery or effective
input change invalidates old captures before the owner can present another usable
snapshot; the same visible A→B→A cannot restore an old entry. Workspace/build/
owner replacement invalidates all old references. Epoch exhaustion denies.

Slow bounded reads/hashes precede the short final validation. Capture then
rechecks the exact owner references/epoch, workspace/active selection and observed
resource set; mismatch returns changed/unavailable, with no implicit repair or
retry. Status and Start repeat accepted validation without renewing the 30-second
preview. The eventual admission consumer must serialize its final controlled-
source check and first creation mutation through qualified fences. No scans,
disk writes, asset reconcile or queued I/O under a hot terminal-input lock.

Existing store/integration lanes do not yet form that owner. Before actual
capture, qualify coverage of all mutations, lock ordering, absence races and
uncertain publication between disk and in-memory catalog. Neither inspect/list,
lease-file creation, startup fallback, successful readback nor a process scan
constitutes this proof. Unobserved external file ABA remains outside the accepted
history guarantee; observed mismatches permanently invalidate their preview.

## Qualification disposition

The [coupled offline plan](../plans/2026-11-20-review-configuration-leaf-components.md)
proposes finite byte/digest/coverage and selected-command joins. Its expected
inventories and current-owner facts are synthetic assumptions. It may qualify
those logical values, never a real resolver, collector or adapter's complete
read set. No actual user configuration, hook installation, provider/model call,
native probe, new metadata, storage activation or production import is involved.

Investigated blind spots changed the design: read-looking integration inspection
can revoke authority; file leases can write; ordinary creation swallows errors;
OpenCode can append a new native selector; global override bytes may contain
arbitrary user data; effective catalog is more than override hash. The stock-only
scope, explicit late-argv refusal, separate pure-capture seam and retained native/
source qualification gates address these without inventing capability evidence.
Independent correctness review verified the cited source claims and accepted
parent-contract consistency. A fresh builder check found no important planning
ambiguity. The coupled plan was self-reviewed once under the local plan policy.
No executable leaf or actual-capture qualification is claimed.
Disposition: **Ready for scoped owner review; production Investigate.**
