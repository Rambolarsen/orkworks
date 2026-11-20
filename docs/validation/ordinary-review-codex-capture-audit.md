---
type: Validation Evidence
title: Ordinary-review Codex capture source audit
description: Pinned Codex integration inputs, late startup mutations and missing capture-owner boundaries before ordinary-review qualification.
tags: [orkworks, review, codex, source-contracts]
status: stable
---

# Ordinary-review Codex capture source audit

Disposition: **Investigate; no Codex ordinary-review producer is qualified.**
This is a static source audit at
`145b7371ef360ad3ab69c692f557eb1370670f41` after
[#1933](https://github.com/Rambolarsen/orkworks/pull/1933), not an approved new
capture architecture, native probe, runtime implementation or installed-state
inspection. It advances the tracked prerequisites in
[#68](https://github.com/Rambolarsen/orkworks/issues/68),
[#66](https://github.com/Rambolarsen/orkworks/issues/66) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899), related to #738/#741.
No local user configuration, credentials or native conversation was read.

The accepted [leaf contract](../superpowers/specs/2026-11-20-review-configuration-leaves-design.md)
and [prepared-input contract](../superpowers/specs/2026-11-19-review-launch-input-design.md)
remain unchanged. Their logical fixtures do not prove an actual adapter read set.
Codex is an audit candidate because its JSON hook adapter does not override
`IntegrationHandler::augment_launch`; that fact alone does not make the later
runtime command literal or its owner ready.

## Pinned source trace

Paths in this section are under `crates/orkworksd/`. Line numbers refer to the
audit commit; inspect the named symbol when source subsequently moves.

| Boundary | Source and observed behavior | Capture consequence |
| --- | --- | --- |
| Stock command | `src/harness/registry.rs::ResolvedHarness::build_launch` and `src/harness/definition.rs::EMBEDDED_BUILTINS` resolve the requested model through the stock template. | Keep the complete embedded document and exact resolved command; the absent user-document gate still applies. |
| Adapter identity | `src/harness/integrations/codex.rs:121` declares the Codex handler, six event groups, marker, reporter and additional `session-metadata.py` asset. `src/harness/integration.rs:666` supplies the no-op default launch augmentation. | Representative and policy are code-owned; a tool name cannot select arbitrary assets. This covers the adapter method, not all startup behavior. |
| Hook document | `src/harness/integrations/mod.rs::JsonHookHandler::load` reads `.codex/hooks.json`, parses the shared JSON document and selects a stable reporter path. POSIX permits tracked or ignored-untracked targets; Windows uses the stricter gate. | The carrier is shared configuration. Do not put its complete user/vendor contents in the leaf. A separately qualified owned-fragment projection is missing. |
| Fragment test | `codex.rs::groups`, `marker_state`, `probe` check six event arrays, marker commands, duplicate owned groups, matcher absence and reporter existence. | This status test is not a closed fragment codec: it does not reject every extra group/hook field, and `groups` visits only the six declared event keys. Reusing its Installed label does not prove the leaf's exact template requirement. |
| Asset status | `integrations/mod.rs::reporter_assets_current` and `integration.rs::ReporterAssetResolver::is_current` compare source/installed reporter and helper bytes. | Both pairs are inputs; existence alone is insufficient. Missing/unreadable installed files collapse to a false comparison here, not a typed capture observation. |
| Asset roots | `src/harness_integration_application.rs::resolve_scripts_source_dir`, `scripts_source_dir`, `stable_hook_scripts_dir` select packaged scripts beside the executable, otherwise manifest scripts; stable assets are under the resolved home. | Source root, stable root and home identity are owner inputs. The checkout's script hashes cannot attest the running package or installed copy. |
| Active tools | `workspace_harness_enabled` reads `WorkspaceMemory.active_harness_ids`; metadata owns `active_harness_revision`. `src/metadata/workspace_memory.rs::Owner` serializes complete edits. | Capture the exact live workspace owner and typed active selection. Do not copy the entire workspace-memory file as an integration preference or mistake its edit flag for the configuration epoch. |
| Trust observation | `JsonHookHandler::status_from_document` reads `codex-hook-observation.json` through `src/metadata/hook_observation.rs::Store::read`, comparing its fingerprint to the expected bundle. Missing or invalid reads return None. | Trust is an independent readiness input if Active is required. A matching historical fingerprint is not executable custody, a current-session hook receipt or native role qualification. |
| Settings mutation | `src/harness_integration_codex.rs::OwnedAction` retains workspace lease, metadata and integration claim for install/repair/uninstall/cleanup. `hook_observation.rs` separately serializes observation writes/clears. | These existing owners serve different exclusions. Their combination is not the shared catalog/integration checked epoch required by #1933. |
| Ordinary creation | `src/session_application.rs::create_session_workflow` tolerates integration launch-state/augmentation errors. | A future prepared creator must deny unavailable exact inputs rather than inherit these fallbacks. This audit changes no manual launch behavior. |
| Runtime isolation | `src/runtime/session_runtime.rs:2550` calls `codex_launch::isolate_session` before PTY creation when the native plan is absent. `src/runtime/codex_launch.rs:14` resolves/probes the executable with `--help`; it may insert `--no-daemon` at argv index zero. | The actual ordinary Codex path can mutate argv after preparation. A no-op integration adapter does not satisfy literal finalCommand. |
| Late environment | `session_runtime.rs:2613` forwards filtered ambient variables; later it allocates reporting capability and a Codex mailbox, with HTTP fallback on mailbox failure, and reads prompt generation. | The fixed fresh-session reporting policy does not by itself prove these late choices implement the prepared creator's reservation/identity boundary. No runtime values belong in leaves. |

The isolation probe has a three-second deadline and 64 KiB output cap, requires
success, combines stdout/stderr with a separator, strips ANSI and checks the
exact first whitespace token `--no-daemon`. An existing flag only counts before
`--`. Failed/oversized output denies startup; older successful help without the
option preserves argv. These are source behaviors with synthetic tests in
`codex_launch.rs`, not fresh native capability evidence from this audit.

## File inputs and logical owners must be separated

The following is the observed candidate inventory for a future Codex projector,
not a registered `owned-integration-inputs-v1` expected inventory or a completeness
claim. IDs and encoded projections still need a concrete reviewed design.

| Source input | Relation to the existing leaf |
| --- | --- |
| `<workspace>/.codex/hooks.json` | Carrier for `native_owned_fragment`. Future capture must extract exactly the six owned groups without retaining unrelated configuration, validate the complete supported template and distinguish absent, invalid, ambiguous and unknown observations. No projection bytes are defined here. |
| `<source-scripts>/report-harness-event.sh` and `<stable-scripts>/report-harness-event.sh` on POSIX | Required `source_asset`/`installed_asset` pair if this integration is eligible. Include complete bytes, exact roots, regular-file identity and executable-readiness evidence. The `.ps1` pair is the Windows counterpart, not an additional POSIX runtime dependency. |
| `<source-scripts>/session-metadata.py` and `<stable-scripts>/session-metadata.py` | Required additional pair declared by the handler, including the SessionStart context producer. The reporter alone is an incomplete inventory. |
| Workspace active IDs/revision | Already typed in `activeSelection`; join the actual workspace-memory owner and current publication. No whole `workspace.json` export is needed or authorized. |
| Codex hook observation | Separate typed readiness/observation owner; the existing leaf has no explicit trust field. Decide whether/how this source is bound before claiming Active-dependent availability. Do not invent a resource containing arbitrary metadata bytes. |
| Resolved home and script roots | Inputs to portable `$HOME` commands, installed asset locations and helper context. Pin owner-selected canonical paths and enforce the eventual policy; content hashes alone cannot bind different homes. |
| Exact executable/platform/model and isolation result | Independent native/configuration eligibility and custody. Neither the hook fragment nor a file-inventory item supplies it. A new arbitrary flag or caller-supplied capability boolean is not a repair. |

At this base the repository-owned source assets measure:

| Source asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `report-harness-event.sh` | 29,508 | `d554765491d8d58f85b9bb9357e38ea2b73dd5872303a4f1490bd4c6ee3b71b6` |
| `report-harness-event.ps1` | 30,127 | `718eb3e627715f49abf1f2332fc17660337fbe921940744d0f1af6fe48ef148b` |
| `session-metadata.py` | 14,445 | `5a8a588e60831a6ddd64bc1f60612aa67cb47659b9a83f088731b20c14eb1755` |

Each is below the accepted 256 KiB raw resource cap. That observation does not
prove combined leaf fit, installed bytes, permission safety, bounded whole-file
parsing or complete physical read work. Only repository files were measured.

## The helper and runtime are part of the boundary

The POSIX reporter's SessionStart path invokes the adjacent metadata helper:
`instructions` when the API/storage marker is present, otherwise
`worktree-instructions`. `session-metadata.py::worktree_instructions` requires
session ID, port and report token, resolves its own path and emits additional
context. This is separate from the explicit initial review turn and from the
Claude/Copilot prompt-generation authority gate. The accepted `promptHook:disabled`
tag does not mean installed Codex SessionStart context is absent; silently
turning it off would change existing behavior. The future producer must qualify
this path under the accepted initial-instruction/context boundary.

The reporter also consumes runtime mailbox/reporting inputs and writes bounded
native diagnostic/identity reports. Those files are runtime outputs, not assets
to snapshot into the preview. The shell, Python, curl, native user configuration,
repository instructions and credentials remain ambient dependencies under the
accepted instruction-only assurance. This audit neither hashes that environment
nor upgrades it to confinement. An uncontrolled dependency that changes the
OrkWorks-resolved command still cannot be dismissed as ambient to evade the
prepared-input contract.

## Capture-owner prerequisites

There is no single qualified pure capture entry point in the inspected source.
Do not assemble one by calling status, install, reconcile, a store lease or a
native probe during preview. In particular, existing status success does not
supply strict read-error classification, closed projection or checked epochs.

Before a producer can use these leaves, a concrete design must resolve:

1. **Final command before preview.** Either establish a source-owned, independently
   qualified pre-capture isolation result and freeze its literal command, or
   propose a separate reviewed contract for late command decisions. Preserve
   manual isolation and deny unknown/changed evidence. Skipping `--no-daemon`,
   keeping a shared daemon or probing at Start after showing different argv is
   not qualified by the accepted contracts. This audit selects no new mechanism.
2. **Pure projection.** Separate bounded observation from deterministic template
   construction/projection. Define exact owned-group bytes, unknown-key and
   duplicate-key behavior, the independent expected set, typed read failures,
   path/executable checks and contextual helper policy. Reuse the actual source
   template rather than introduce an independently maintained JSON lookalike.
3. **Shared configuration owner.** Account for catalog mutations/publication,
   active-tool edits, Settings actions, asset reconciliation/replacement,
   observation writes/clears and eligibility invalidation. Qualify lock ordering,
   no queued I/O under terminal-input locks, failed publication, owner replacement
   and epoch exhaustion. Existing claims are useful seams, not coverage evidence.
4. **Admission and late allocation.** Bind exact owners/epochs through the final
   controlled-source check and first creation mutation, and bind fresh reporting
   allocation/fallback policy to that admission. No physical registry or private
   prepared creator is established by this audit.

Required future negative cases include a late isolation insertion, a flag after
`--`, simultaneous weakening of supplied inventories, a marker-valid group with
extra executable fields, malformed/duplicate shared JSON keys, one missing helper,
source/installed mismatch, home/root replacement, read error reported as absence,
observation clear during capture, failed catalog publication and observed A→B→A.
These are acceptance oracles for later design/testing, not tests executed here.
Unobserved external ABA remains outside the accepted history guarantee.

Next proposed task: design the pre-capture Codex isolation/command seam first.
It decides whether the first literal profile can use Codex and prevents a fragment
projector from being mistaken for end-to-end launch qualification. Obtain owner
approval of that concrete architecture before implementation. The supported
production-role set remains empty; parent issues remain open.
