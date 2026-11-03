---
type: Design
title: Harness-independent session workspace signals
description: Proposed evidence and resolution contract for reliable Git location across coding harnesses.
tags: [sessions, harnesses, git, worktrees, signals]
status: draft
---

# Harness-independent session workspace signals

Owner approved this direction on 2026-11-03 (“sounds good”). New native signal
activation still requires the evidence below.
Tracking: [#1347](https://github.com/Rambolarsen/orkworks/issues/1347).
The owner requested investigation of hooks and required a generic solution for
all harnesses on 2026-11-03. This document does not claim all harnesses expose
an automatic task-worktree signal.

## Outcome and existing authority

Details, Changes and branch documents resolve the same selected session location.
An agent working in checkout B after launching in A must not leave Git inspecting
A. Temporary inspection in A and child work in C must not redirect the parent
away from B.

Reuse [explicit task-worktree reporting](2026-10-25-session-worktree-location-design.md),
[ADR 0094](../../adr/0094-explicit-session-worktree-binding.md), and the
[selected-session Changes contract](../../../specs/session-git-changes.md).
Keep the existing shared resolver and authenticated revisioned worktree helper.
Do not add a parallel Git resolver, infer location from changed files, parse
shell command strings, or use the newest worktree on disk.

## Evidence collected

Source inspection at base `70d8e272`:

- `session_view::resolve_effective_locations` already chooses explicit binding,
  harness directory, process directory, then launch directory.
- `session_location` validates explicit bindings and supplies the Changes
  subject; projection uses the same precedence for Git and branch documents.
- `report-harness-event.sh` and `.ps1` extract Claude and Copilot directories.
  Codex omits its common `cwd`, and its tool hooks remain capture-only under
  the independent attention qualification gate. Forwarding common `cwd` alone
  would not establish a tool-specific task directory.
- `opencode-session-reporter.js` captures native identity and attention but no
  per-session directory. A plugin's own directory is insufficient evidence.
- No location-specific capability is currently declared in harness definitions.
  This proposal does not require adding a public registry field before an
  adapter actually needs one.

In the current owning Codex tool environment, an authenticated read found no
explicit binding and recorded A/main. The installed helper successfully reported
this session's owned worktree B through the qualified managed relay, returning
B's canonical root and common directory. No inherited credentials were logged.
This proves reporting access in this environment, not automatic discovery or
live rendered Changes behavior. No other coding harness was launched.

## Three meanings of directory

A **task worktree** is the owner's durable selection. The existing
`agentMetadata.worktreePath` report sets it. It outranks all observations,
remains selected through temporary commands, and becomes unavailable if its
repository disappears or changes. Only the owner changes or clears it.

A **harness session directory** is structured native state for the exact root
session. A qualified adapter may update the existing harness-directory fallback.
It never overwrites or clears the task binding. A plugin directory, server
process directory or child session directory does not establish root-session
state.

A **tool execution directory** describes one invocation, including read-only
inspection. It is diagnostic evidence only. Neither a tool-directory observation
nor a file-edit path automatically establishes task ownership. The location
policy never promotes arbitrary tool activity into a durable task binding.

For agent-chosen task worktrees that do not update native session state,
cooperative reporting remains necessary. Use the same helper for any harness
whose agent/tool context can invoke it. Deliver its narrow instructions through
each harness's already-qualified instruction channel; lack of such a channel
must remain an explicit coverage gap, not a claim of automatic support.

## Adapter evidence and qualification

The matrix covers every current builtin, including legacy definitions. Custom
harnesses use the same helper and process/launch fallback unless independently
qualified; no vendor name is special-cased in the shared resolution policy.

| Harness | Candidate signal | Current evidence and activation boundary |
| --- | --- | --- |
| Claude Code | Main-conversation `CwdChanged.new_cwd`; existing hook `cwd`; explicit owner report | [Hooks reference](https://code.claude.com/docs/en/hooks#cwdchanged) documents `old_cwd`/`new_cwd`. Qualify installed-version event semantics, identity and temporary navigation before using it for session fallback. `WorktreeCreate` replaces creation and must not be installed as passive observation. |
| Codex | Common hook `cwd`; tool arguments for diagnosis; explicit owner report | [Hooks reference](https://learn.chatgpt.com/docs/hooks) describes common `cwd` as session directory. Tool input shape varies by tool. Observe supported shell, patch and nested/code-mode calls before declaring coverage; no command-string parsing. Child hooks can share the parent ID, so ID equality alone cannot authorize a new location signal. Existing narrow helper instructions are Codex-specific delivery today. |
| GitHub Copilot | Existing hook `cwd`; qualified session/tool events; explicit owner report | [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference) supplies `cwd`, native identity and timestamps. Existing forwarding is not proof that `cwd` follows a task selected through a per-tool directory. Qualify root identity, timestamp ordering and installed version. |
| OpenCode 2 | Exact captured session state or a scoped plugin event; explicit owner report | [Server plugin reference](https://opencode.ai/v2/docs/build/plugins/server) says `ctx.location` belongs to the plugin instance, not every session. Pin a session-specific directory schema and root identity before enabling updates; do not use V1 plugin assumptions. |
| Aider | Explicit owner report where invocable; process/launch fallback | [Notifications](https://aider.chat/docs/usage/notifications.html) describes response completion, without a structured location payload. Do not infer workspace from callback invocation alone. No automatic location qualification. |
| Antigravity | Explicit owner report where invocable; process/launch fallback | No qualified integration contract in this repository. Require primary documentation, exact fixtures and version evidence before a native adapter. |
| Generic shell | Existing process directory, then launch | Preserve existing process tracking. This is shell cwd, not a claim that a coding tool's subprocess cwd follows its task. |
| Gemini, retired | Historical facts and existing fallback | Preserve readable history. Do not activate a retired client or install new hooks. |

These are documentation/source findings, not live payload qualification for any
new hook. Unsupported adapters must not fabricate a signal or silently choose
another checkout.

## Shared acceptance rules

Location acceptance must be independent of attention updates. Do not synthesize
working/idle/Needs You just to deliver a directory. Preserve all current native
identity, launch-generation, hook trust and capability gates. A source's
attention qualification neither grants nor proves location qualification.

Before enabling a new native source, define and test its exact root-session
correlation and ordering against the active runtime. Missing or ambiguous child
identity is a no-op. Reject delayed observations from ended/replaced runtimes.
Do not treat receipt order as source order when events can arrive asynchronously.
The adapter qualification record must name the ordering mechanism; if none is
available, do not activate automatic updates from that source.

Canonicalize and validate any location used for Git. Explicit reports retain
existing same-repository validation, revision conflicts, retained unavailable
binding, and no-direct-file-fallback behavior. Existing unbound fallback behavior
is not changed by this document. Any new native location update must be an
absolute existing directory in the launch repository and must not bypass the
binding validator. A source that cannot prove those conditions is unavailable
for automatic updates.

Collect only the location fields, event identity/ordering and supported-version
evidence needed by the qualification fixture. Do not retain prompts, command
text, tool results, transcript contents or credentials. Existing diagnostics
remain unchanged until a separately reviewed bounded capture change exists.
Use disposable fixture paths or comparisons against expected A/B/C; no global
raw hook-payload logging. Do not launch another coding harness from an active
session; live runs need an owner-run fixture or separate authorized session.

## Bounded delivery and checks

1. Qualify directory semantics with A/main, B/task and C/other. Work in B, inspect
   A temporarily, and exercise child work in C. Compare structured session cwd,
   tool directory and explicit binding without conflating them. Record version,
   source, actual transport and unknown coverage. Hookless harnesses explicitly
   retain fallback. This evidence pass can conclude that no automatic task signal
   exists for a given harness.
2. Reuse the helper and shared resolver. Implement only the adapters whose exact
   location, root identity and ordering passed qualification. Deliver generic
   helper instructions through already-qualified channels; scope each new
   instruction channel separately where none exists. Amend authoritative
   contracts and add an ADR before new signal authority or protocol work.
3. Add regression fixtures before product code: explicit B beats session A and
   tool C; unbound qualified root moves follow native session state; ambiguous
   children and stale events do not update; removed B remains unavailable;
   clear restores existing fallback; end/restart preserve binding. Exercise
   generated POSIX/PowerShell commands, install/probe/reconcile/uninstall and
   trust drift for every changed adapter.
4. Verify Details, both Changes comparisons, base enumeration and branch documents
   agree on B. Test A-to-B-to-C intentional reports and old capture rejection.
   Run the repository verification and scoped suites for changed surfaces.
   Live installed behavior must be recorded separately from source tests.

No database migration, new durable location record, automatic clear, launch/resume
change, worktree management, or universal hook schema is proposed. Rollback of a
new adapter disables that source and retains explicit reports and the existing
fallback. Rollback to a binary predating worktree bindings retains ADR 0094's
mixed-version limitation.

## Review and approval

Open questions are evidence tasks, not permission to guess: exact per-tool/root
signals, installed-version support and ordering still require qualification.
The [delivery plan](../plans/2026-11-03-harness-workspace-signals.md) starts with
bounded evidence tooling and records any unqualified native adapter as deferred. Do not promise automatic
tracking for all harnesses merely because the shared resolver supports them.

Risk assessment: dependencies 3 (several external contracts); blast radius 3
(shared Git consumers); state changes 1 (proposal adds no stored schema);
reversibility 1 (no runtime change); uncertainty 5 (new native signals unqualified).
Total 13/25 describes this proposal/evidence phase, not permission to enable
runtime behavior. Builder review: Ready for owner review of the evidence/qualification phase. A fresh
read-only reviewer on 2026-11-03 reported “nothing important left”; automatic
adapter activation remains dependent on the explicit evidence gates above.

## Running the qualification probe

`scripts/qualify-workspace-signals.py` is a developer fixture, not an installed
reporter. It reads one JSON object, at most 64 KiB, from stdin and emits a small
JSON record containing only fixed field names, `launch`/`task`/`other`/`outside`/
`invalid` directory classifications and optional native-ID equality booleans.
It never posts to the sidecar, writes metadata, configures hooks or changes
location authority. All output says `diagnostic-only` and `rootScope: unverified`.
Even matching native IDs cannot establish root scope for shared-ID child hooks.
Missing fields remain absent; encoded/string tool arguments and unknown nested
fields are not interpreted. The fixed field allowlist includes candidate native
and tool directory fields; recognition does not qualify their semantics.

Use three distinct existing checkout roots A, B and C. Roots must be absolute,
canonical and non-overlapping. In a disposable, separately owner-run harness
fixture, pass the hook's JSON stdin to this command (substitute actual paths):

```bash
python3 /absolute/orkworks/scripts/qualify-workspace-signals.py \
  --harness codex --launch-root /absolute/A \
  --task-root /absolute/B --other-root /absolute/C
```

Supported harness labels are `claude-code`, `codex`, `copilot`, `opencode`,
`aider`, `antigravity`, `generic-shell`, `gemini` and `custom`. These labels
identify the fixture; they do not assert hook availability. For an optional
comparison, set `ORKWORKS_QUALIFY_NATIVE_SESSION_ID` in the disposable fixture's
environment from its known root identity; the probe never prints that ID.

Do not attach diagnostic JSON directly as hook stdout: the harness may interpret
it as control output. For a command-hook fixture, redirect stdout to a disposable
local result file, overwriting it on each invocation, and make the wrapper exit
success regardless of probe failure. Keep stderr local too. For example, append
`> /absolute/fixture/result.json 2> /absolute/fixture/error.txt; exit 0` to a
POSIX command wrapper. In PowerShell use `python ... 1> result.json 2> error.txt; exit 0`.
Use absolute paths and the platform's normal quoting for paths with spaces.
The probe itself never opens either output file. Do not append indefinitely or
retain raw payloads. Copy the small derived result after each sequential fixture
step; this probe does not claim to measure asynchronous event ordering.

Do not replace an existing OrkWorks hook or automatically install a capture
hook in a live session. Configure and approve only a separate disposable fixture
under the harness's supported trust flow. OpenCode needs a V2 adapter that passes
the actual scoped event object to the probe; plugin location alone is insufficient.
Hookless tools have no payload to feed; record unsupported rather than inventing
one. The probe does not synthesize events or run another coding harness.

For each harness, record the exact version, event/tool path, root correlation,
ordering source and transport, then perform A launch, B task work, temporary A
inspection, child C work, intentional B-to-C switch and delayed-event scenarios.
Keep these external run facts beside the derived results. Separately verify the
explicit helper report and both Git comparisons. Synthetic tests of the probe
only prove classification and privacy; they do not qualify native hooks.

### Evidence status on 2026-11-03

The standalone probe passes synthetic subprocess fixtures for Claude, Codex,
Copilot, OpenCode and unknown/hookless payloads, including invalid, oversized,
duplicate-key and privacy cases. CI runs these probe tests in the existing
Documentation site job. No new native source has live root-identity, directory
and ordering evidence. Therefore no adapter is enabled by this change. The only
live transport result is this owning session's successful explicit B report;
rendered Git and native-hook qualification remain outstanding under #1347.
