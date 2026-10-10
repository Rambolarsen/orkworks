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
- ADR 0051: Codex's generated/local six-event hook bundle captures `harness_session_id` at `agent`-tier confidence and reports deterministic turn attention through the existing `JsonHookHandler`/reporter-script framework; `SessionStart` remains identity-only, `PreToolUse` and `PostToolUse` are capture-only for the #690 redacted payload verification gate, and `UserPromptSubmit`, `PermissionRequest`, and `Stop` drive validated per-session authority. `PermissionRequest` also fires for `auto_review`-resolved approvals with no human prompt; the capture gate changes no attention behavior.
- ADR 0037: Plan/spec paths can be reported through a dedicated path-only sidecar route (`POST /sessions/:id/plan-path`) that canonicalizes the file and stores its workspace-relative form without changing session attention, superseding terminal-text inference when a harness reports a canonical file path. Codex remains on the conservative terminal fallback because its hook payload provides patch text, not a canonical file path.
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

## Metadata protocol

The detailed paths, bounds, lifecycle, authentication, and ADR reference are in the [architecture concept](architecture.md#metadata-protocol).

- Metadata is workspace-scoped under `~/.orkworks/workspaces/<hash>/`; global harness definitions and stable hook reporters live under `~/.orkworks/`.
- Current source priority: user > agent > peon > backend_inference > process > unknown > debug. The written sidecar-mediated metadata contract for native-enabled Codex sessions is owner-approved: active metadata mutations use versioned sidecar writes serialized with native clears while preserving the Peon staleness rule. This resolves #761's specification prerequisite only; it is not implemented or enabled, and #761 does not authorize runtime changes. Native clearing remains disabled pending implementation, producer migration, and the independent #690/#763 verification gates. See [ADR 0076](../adr/0076-codex-owned-native-approval-observer.md) and the [mediated metadata design](../superpowers/specs/2026-10-08-codex-native-attention-layer-design.md).
- Peon reads terminal output and writes inferred metadata; it never types into terminals.
- Within one independent instance, detached runtimes keep draining terminal output, persisting history, and feeding Peon while `orkworksd` remains alive; losing a renderer terminal attachment alone must not end a session. A workspace switch closes that instance's runtime before opening another workspace; it does not create a peer-runtime registry.
- Taskmaster proposes cross-session transitions, but ordinary v1 recommendations require explicit user approval for every action. `improve_workflow` may display without approval, but cannot focus a terminal, edit a file, or start a session; a user may dismiss it or accept it to send a scoped fix prompt to their active session. The proposed ordinary-child orchestration extension is separately gated: its UI-created run can prepare proposals, but each exact plan revision needs user approval before any plan-owned branch/worktree or child is created. Research completion retains planning authority, not execution approval; final completion revokes run authority. Child edits stay in approved worktrees for manual integration. The initial slice has no automated cleanup; any later separately reviewed removal must be clean, quiescent and plan-owned. Scope is accepted; detailed contract review, verified coding-tool profiles and scoped implementation-plan approval remain required; see [orchestration architecture](architecture.md#proposed-ordinary-child-orchestration).
The current metadata paths and behavior are also summarized below for quick
operational reference; the architecture concept remains authoritative for the
full protocol detail.

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state. (design, not yet implemented — see issue #313) Gains a current-summary snapshot (`summary`, `summarySource`, `summaryConfidence`, `summaryObservedAt`, all four updated or cleared together — ADR 0042)
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
