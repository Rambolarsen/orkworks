---
type: Architecture Reference
title: Runtime implementation contracts
description: Scoped architecture constraints, approval gates, and metadata-path contracts retained from the agent guide.
tags: [architecture, runtime, metadata, agents]
status: stable
---

# Runtime implementation contracts

Read the relevant section when changing runtime, metadata, or integration
behavior. These contracts preserve the approved boundaries and verification
gates; lighter repository planning does not grant product execution authority.

## Architecture constraints

Packaged desktop updating requires `electron-updater` as a production dependency.
Preserve the platform verification and shutdown guards described in
[desktop update architecture](architecture.md#packaging-and-release).
Electron workspace history uses the native `fs-ext` advisory lock on a retained
lock file; never evict it by age or delete its inode. Desktop dev/build/dist
rebuild native dependencies for Electron. Run `pnpm rebuild fs-ext` before Node
tests after an Electron build. See [architecture](architecture.md).

Provider process cleanup uses Windows Job APIs through the existing
`windows-sys` dependency; preserve suspended-child assignment and bounded cleanup
when changing inference transports. See [provider process ownership](architecture.md)
and the [ADR 0055 amendment](../adr/0055-json-taskmaster-inference-adapters.md).

The current Electron + React/TypeScript frontend (`apps/desktop/`) communicates with a Rust sidecar (`crates/orkworksd/`) over a dynamic localhost HTTP/WebSocket port. The source shell uses fixed React/CSS Grid regions with Sessions, Terminal, and one optional inspector (ADR 0082). Narrower widths use temporary utility pages while the terminal runtime remains attached; Review and Actions destinations remain follow-up work. The sidecar manages PTY sessions, Git context, the metadata protocol (under `~/.orkworks/workspaces/<hash>/`), Peon observation, and Taskmaster recommendation state. The desktop `pnpm dev` command builds the debug sidecar before launching Electron so development runs use the current Rust implementation.

The accepted ADR constraints below are retained from the root guide. Detailed
prose lives in [architecture](architecture.md), [harness contracts](harness-integration-contracts.md),
and the [ADR index](../adr/README.md). Keep this list limited to accepted
implementation constraints without a separate prose summary; collapse entries
to their canonical reference when that reference gains the full explanation.

- ADR 0022: PTY lifetime is session-runtime-owned in the sidecar; renderer terminal attachment is detachable and does not own process lifetime.
- ADR 0028: Harness version-probe results are cached with bounded TTLs and generation-aware invalidation, preserving the integration action's post-probe identity revalidation.
- ADR 0051/0093: Codex hooks retain identity and legacy-runtime attention. Once a runtime emits valid OSC 7501, best-effort terminal status supersedes its attention hooks while preserving explicit user/agent writes; see [Codex terminal status](harness-integration-contracts.md#codex-terminal-status). Native/completion gates remain closed.
- ADR 0037/0102: Plan/spec reports retain the dedicated path-only route and unchanged attention semantics. Shared branch design/implementation slots add bounded Git discovery alongside reports and conservative terminal fallback; explicit selections pin one slot. See [branch document association](../adr/0102-branch-document-association.md). This does not qualify a new native Codex hook signal.
- ADR 0025, 0026, 0031, 0032, 0042, 0072: prose lives in [`docs/agents/architecture.md`](architecture.md).
- ADR 0038: prose lives in [`docs/agents/harness-integration-contracts.md`](harness-integration-contracts.md).
- ADR 0052: one `orkworksd` process owns a workspace's metadata at a time through an OS advisory lease; workspace open/switch returns conflict before orphan reconciliation when another sidecar holds it.
- ADR 0060: independent OrkWorks instances each own at most one workspace and sidecar; installation-scoped history is path-only, with no peer registry or cross-instance attention/focus authority. Crash-surviving cleanup and replacement adoption remain blocked on native ownership proof in #545.
- ADR 0080: prose lives in [orchestration architecture](architecture.md#proposed-ordinary-child-orchestration).
- ADR 0065: hard-wrapped terminal rows are reassembled into logical lines at PTY ingestion, before the shared `output_buffer` (row-local chaining on `SessionRuntime::last_cols`, one held pending row flushed at `handle_runtime_exit`); raw physical rows stay authoritative in terminal history, `scan_buf`, and evidence grounding on `raw_persist_lines`, and read-time snapshot rejoins share the same row-local rule.
- ADR 0068: Codex CLI subagents remain within the owning OrkWorks session; identity replacement requires an authenticated root `SessionStart(source=clear)` after a recorded reset, and resume remains exact-ID-only.
- ADR 0074: alternate-buffer pixel wheel input is accumulated per live terminal through xterm's public wheel handler; the first movement is immediate, later events require a rendered line of movement, and normal scrollback and line/page wheel input keep their existing behavior. The xterm 6.0.0 patch remains required in both shipped bundles.

The approved Codex approval runtime extension is routed through
[ADR 0076](../adr/0076-codex-owned-native-approval-observer.md) and its
[written design](../superpowers/specs/2026-10-03-codex-native-approval-status-design.md).
The #690 serial capture gate passed and the owner approved implementation on
2026-10-05; the new launch and attention behavior remain disabled until their
version-specific production verification gates pass. Native startup and the
installed diagnostic also remain closed pending the owned-listener contract
in [#763](https://github.com/Rambolarsen/orkworks/issues/763).

The owner-approved [completion mitigation](../superpowers/specs/2026-10-23-codex-hook-completion-mitigation-design.md)
extends the written API-only eligibility contract to independently qualified
non-native Codex launches. The correlation/storage core remains dormant with
no production caller, capability, marker or qualified launch. Its conditional
mutation checks ownership and edits the latest record under the same storage
lane as revisioned patches, preserving unrelated descriptive updates. This
does not relax native startup/clearing or the producer/child-context gates.

The [ordered completion transport](../superpowers/specs/2026-10-24-ordered-completion-relay-design.md)
is an additive dormant protocol inside the private hook relay, not a metadata
mailbox. It has no production stream creator, preserves legacy reports, and
cannot adopt queued effects after consumer/runtime state loss. Source ordering
of published records does not prove complete upstream hook invocation.

## Metadata protocol

The accepted [ordinary-session storage contract](../../specs/session-storage.md)
changes authority only after qualified format-2 cutover; implementation is
pending. [ADR 0103](../adr/0103-sqlite-session-storage.md) and its delivery plan
preserve current native/hierarchy gates, content and retention settings. Existing
format-1 paths below remain current until activation.

The detailed paths, bounds, lifecycle, authentication, and ADR reference are in the [architecture concept](architecture.md#metadata-protocol).

- Metadata is workspace-scoped under `~/.orkworks/workspaces/<hash>/`; global harness definitions and stable hook reporters live under `~/.orkworks/`.
- Current source priority: user > agent > peon > backend_inference > process > unknown > debug. The written sidecar-mediated metadata contract for native-enabled Codex sessions is owner-approved: active metadata mutations use versioned sidecar writes serialized with native clears while preserving the Peon staleness rule. The sidecar implements the transactional API and split provenance; #789 producer migration and child-context verification remain pending. #761 supplied design approval, and native clearing remains disabled pending those dependencies and the independent #690/#763 verification gates. See [ADR 0076](../adr/0076-codex-owned-native-approval-observer.md) and the [mediated metadata design](../superpowers/specs/2026-10-08-codex-native-attention-layer-design.md).
- Peon reads terminal output and writes inferred metadata; it never types into terminals.
- Peon synchronously prepares attempts and repairs idle-timer origins under the
  existing projection fence shared with forgetting and retention. Only owned
  attempts escape it; reassembly, provider work/waits and idle-transition
  application run outside it. Deletion invalidates prior attempts, so late
  settlement cannot recreate their tracking. See [ADR 0104](../adr/0104-peon-scheduling-deletion-fence.md).
- Capacity analysis computes outside the input fence and commits only current runtime/capability/control observations; continuous output preserves newer counters. Password observations over 64 KiB or changed before enqueue conservatively suppress input-derived metadata without dropping terminal bytes. See [ADR 0102](../adr/0102-terminal-projection-input-fence.md).
- Within one independent instance, detached runtimes keep draining terminal output, persisting history, and feeding Peon while `orkworksd` remains alive; losing a renderer terminal attachment alone must not end a session. A workspace switch closes that instance's runtime before opening another workspace; it does not create a peer-runtime registry.
- Taskmaster proposes cross-session transitions, but ordinary v1 recommendations require explicit user approval for every action. `improve_workflow` may display without approval, but cannot focus a terminal, edit a file, or start a session; a user may dismiss it or accept it to send a scoped fix prompt to their active session. The proposed ordinary-child orchestration extension is separately gated: its UI-created run can prepare proposals, but each exact plan revision needs user approval before any plan-owned branch/worktree or child is created. Research completion retains planning authority, not execution approval; final completion revokes run authority. Child edits stay in approved worktrees for manual integration. The initial slice has no automated cleanup; any later separately reviewed removal must be clean, quiescent and plan-owned. Scope is accepted; detailed contract review, verified coding-tool profiles and scoped implementation-plan approval remain required; see [orchestration architecture](architecture.md#proposed-ordinary-child-orchestration).
The current metadata paths and behavior are also summarized below for quick
operational reference; the architecture concept remains authoritative for the
full protocol detail.

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state, including the current-summary snapshot (`summary`, `summarySource`, `summaryConfidence`, `summaryObservedAt`, all four updated or cleared together by the metadata API — ADR 0042). Broader summary handoff work remains tracked in #313
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size` — the PTY's `cols`x`rows`, used to render dead-session terminal replay at its recorded size instead of the current panel width. Written authoritatively at the moment a session reaches a terminal status (`killed`/`ended`/`error`), and best-effort after a successful live resize when the requested grid differs from the known durable grid. This also persists an initial grid when a successful same-size resize occurs before that grid is durable, so a daemon restart mid-session can leave a usable last-known size for orphan reconciliation (`metadata::reconcile_orphaned_session`), which has no in-memory runtime handle to read a size from and never reaches the terminal-status transition itself. Still absent for sessions that ended before this file existed and for sessions that never lived long enough to make a grid durable before an untimely daemon restart — both cases fall back to fit-to-container replay, which can misrender recorded output that used absolute-column cursor addressing computed for a different width than the container happens to fit to.
- `~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson` and `~/.orkworks/workspaces/<hash>/workflow-observations/sequence` — bounded (1,000 records/2 MiB per session), sequenced `WorkflowObservation` evidence recorded through one shared module (`workflow_observations.rs`) from the authenticated `POST /sessions/:id/workflow-observations` agent-report route (`http/workflow_observation_handlers.rs`); durable improvement evidence for Taskmaster, deliberately separate from the current-summary snapshot above (ADR 0042). The route authenticates with a per-session `ORKWORKS_REPORT_TOKEN` bearer capability, generated from OS randomness (`getrandom`) at session start/resume and never persisted, logged, or serialized; session creation/resume fails closed if OS randomness is unavailable rather than spawning with a weak or empty token. Peon-inferred recording and Taskmaster's `improve_workflow` correlation are implemented. Peon's generation-scoped idempotency keys would otherwise store one observation per scan re-detection, so the 5-minute retention loop also runs an anti-spam trim (`observation_spam_cleanup_once`): per session it groups Peon-origin records only (agent reports are deliberate durable evidence and are never trimmed), keeps each fingerprint's first and latest occurrence plus re-occurrences spaced >30 minutes after the previous kept hit, drops closer-gap re-detections (tombstoned like bounded-storage eviction; backward clock steps are exempt), and never removes an observation cited by an Accepted/Executing/RolledUp recommendation (Proposed cards are regenerated by the evaluator from stored observations each pass and carry their evidence as embedded snapshots, so their citations must not pin spam records forever).
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history, including the audit-derived `cleanup` cards (`POST /taskmaster/audit/recommendations` builds the proposed bulk-dismissal card; type-dispatched accept executes it atomically — see the "Recommendation audit" section of `specs/taskmaster.md`)
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/workspaces/<hash>/.sidecar.lock` — retained lock file whose OS advisory lock identifies the sidecar currently owning workspace metadata; lock ownership releases automatically when that sidecar exits (ADR 0052)
- `~/.orkworks/workspaces/<hash>/origin.json` — the canonical filesystem path that produced this workspace's hash, written once when the directory is first created. `workspace_hash` is a one-way SHA-256 of the canonical path, so this is the only way to trace a directory back to its source; it lets sidecar startup garbage-collect directories whose source path (e.g. a removed git worktree) no longer exists, while leaving directories without one (created before this file existed) untouched rather than guessed at. A directory currently held by another sidecar's `.sidecar.lock` is never removed regardless of path existence.
- `~/.orkworks/workspaces/<hash>/codex-hook-observation.json` — the last Codex hook fingerprint observed executing; Settings reports Codex activation only when it matches the currently installed hook definition
- `~/.orkworks/workspaces/<hash>/integrations/aider.json` — versioned OrkWorks-owned Aider notification-command preference
- `~/.orkworks/harnesses.json` — global harness definitions
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts (e.g. the Claude Code Notification hook), installed hook commands always point here rather than at the packaged/dev source, so they keep working across app updates and packaging schemes whose own paths aren't stable at runtime (Linux AppImage's per-launch mount point, in particular). The workspace-local harness hook configuration that invokes these reporters is gitignored and must not be committed.
- `GET /sessions/:id/summary-log` exposes checkpoints in append order as timestamp, summary, source, and nullable confidence; missing data returns `{ "entries": [] }`. Rendered in the session detail panel as "Task history," distinct from the session's `label` (title), which carries provenance and is separate from this turn-by-turn activity log (ADR 0063).

The owner-approved [explicit helper proxy transport](../superpowers/specs/2026-10-24-sandbox-metadata-proxy-design.md)
permits a caller-selected trusted loopback HTTP relay for the metadata helper.
The relay can observe its bearer token. Direct mode still ignores ambient proxy
variables except the narrow owner-approved worktree command: the qualified
Codex managed-runtime marker and strict loopback URL select its relay, with no
failure fallback. General metadata commands and all metadata/attention
qualification gates remain unchanged. See the [helper proxy contract](harness-integration-contracts.md#explicit-metadata-helper-proxy).

## Session organization

Session title ownership is independent of attention source priority. Desktop
title set/restore operations require the existing desktop token and expected
canonical workspace. Manual titles persist until explicitly restored to automatic
naming, including across conversation resets; late Peon/native work cannot
replace them. Output activity summaries never become live titles. Native Codex
refresh and one bounded contextual Peon refinement follow [ADR 0098](../adr/0098-contextual-session-titles.md).

The accepted [archive design](../superpowers/specs/2026-10-18-session-archive.md)
and [ADR 0088](../adr/0088-session-archive-state.md) add Active, Closed and Archived
as organization states, independently of runtime status and attention. The
sidecar owns bounded identity-bound `session-state/<id>.json` records. Missing
legacy records derive Active or Closed; unreadable state disables mutations and
protects retention. Active Archive and Delete close the exact captured runtime
and await cleanup plus persisted ending first. Restore returns to Closed.
Archived history is retained outside automatic age/count cleanup. Archive and
Restore preserve activity timestamps and native tool identity. This is OrkWorks
organization only; native archive synchronization is outside this increment.

## Explicit task-worktree reporting

The owner-approved [worktree binding](../adr/0094-explicit-session-worktree-binding.md)
extends the authenticated revisioned metadata API with `agentMetadata.worktreePath`
(absolute same-repository directory or null to clear). The sidecar persists the
canonical binding and preserves it across legacy snapshot replacements. Invalid
or missing bound worktrees are unavailable; launch `cwd` never changes. The
narrow `session-metadata.py worktree` command and Codex worktree-only SessionStart
context are independent of the general API-only marker. This does not enable
general metadata commands, native observers, or completion authority.
