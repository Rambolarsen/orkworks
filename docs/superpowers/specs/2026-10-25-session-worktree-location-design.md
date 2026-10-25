---
type: Design
title: Explicit session worktree reporting
description: Proposed authenticated task-worktree binding for session Git context and Changes.
tags: [sessions, git, worktrees, metadata]
status: draft
---

# Explicit session worktree reporting

Proposed for owner approval; no implementation approval is claimed.
Tracking: [#1056](https://github.com/Rambolarsen/orkworks/issues/1056).
Extends [selected-session Changes](../../../specs/session-git-changes.md) and
[ADR 0032](../../adr/0032-harness-reported-cwd-via-hook-payload.md).

## Outcome and evidence

A session launched in checkout A can deliberately work in linked worktree B.
Details and both Changes comparisons must inspect B, even when the coding
agent process and subsequent read-only commands remain in A.

The existing `session_view::resolve_effective_cwds` chooses hook-reported,
process, then launch CWD. `http/git_changes_handlers.rs::subject` uses that
resolver. ADR 0032 already records why harness process CWD cannot identify
individual tool directories, and uncertainty about hook CWD. Codex's reporter
does not currently forward CWD. Forwarding it alone cannot resolve that
uncertainty. Parsing command strings or choosing the most recently changed
worktree would introduce guesses about another session's work.

Observed in this session on 2026-10-25: a real tool execution reports directory
`/Users/froomiebot/workspace/orkworks-session-worktree-location` and branch
`session-worktree-location`, while the launch checkout
`/Users/froomiebot/workspace/orkworks` remains on `main`.
This reproduces the directory distinction, not a completed viewer fix.

## Decision

The owning agent explicitly reports the task worktree after creating/selecting
it and before editing there. It reports again on an intentional task-worktree
switch. A temporary command in another checkout does not change that binding.
One OrkWorks session has one reported task worktree at a time. Subagents do
not replace their owning session's binding; independent worktrees need separate
OrkWorks sessions under the existing ownership rules.

Reuse the authenticated, revisioned metadata API rather than adding a second
HTTP transport or inferring location from terminal output. Extend its closed
patch schema with `agentMetadata.worktreePath`: an absolute path to an existing
working directory, or null to explicitly withdraw the binding. Omission leaves
it unchanged. The sidecar resolves a subdirectory to its canonical Git worktree
root and validates that its canonical common Git directory equals that of the
session's launch repository. Reject non-Git, bare, relative, invalid, oversized,
control-character, unavailable and unrelated-repository paths atomically.
The existing 8 KiB patch body bound also applies.

Persist optional `worktreeBinding: { root, commonDir }` in SessionMetadata,
with a missing field equivalent to no binding. These are sidecar-resolved
values, not caller-provided Git identity. Keep launch `cwd` unchanged.
Projection carries the binding to the shared resolver. An explicit binding
wins over hook/process/launch CWD; hook launch reports cannot override it.
Metadata read errors cannot silently remove a known live binding.

Revalidate root and common-directory membership for Git inspection. If a bound
worktree disappears, becomes another repository, or is unreadable, report
unavailable; do not fall back to A. Retain the binding through session end and
sidecar restart for historical inspection. A new/resumed runtime retains the
same session's binding until the owner changes or clears it, and reports it
again before work. Session deletion removes it with the metadata record.
Clearing the binding restores the existing hook/process/launch precedence.

Use the current revision and report token under the existing workspace/session
locks and serialized transaction lane. A stale revision, ended session or
wrong token cannot change the binding. A worktree-only patch changes neither
attention nor task/summary/plan fields or their provenance timestamps. Existing
accepted metadata writes may advance activity/revision as today. Updates from
other writers must preserve the binding from the latest canonical record;
add tests for projection/write-back and lifecycle writers.

## Agent delivery and existing capability gates

Extend the stable `session-metadata.py` helper with `worktree ABSOLUTE_PATH` and
`worktree --clear`, including its existing explicit proxy option. This command
reads the current revision and submits only the narrow worktree patch. It uses
the caller's actual tool/sandbox context, never the host hook as evidence of
access. Conflict or transport failure is reported without blind retries,
credential output, sandbox changes or direct-file fallback.

The session identity, port and per-runtime report token already reach spawned
PTY children through `runtime/terminal_runtime.rs::session_env_overrides_with_prompt_generation`
(lines 42–60 at the investigated head); they are independent of the stripped
API-only marker. No new credential or launch marker is introduced. A sandbox
that withholds these capabilities or blocks loopback fails explicitly; do not
relaunch the session or change its sandbox to recover reporting.

General metadata commands remain gated exactly as today by
`ORKWORKS_SESSION_METADATA_API_VERSION=1`. The new narrow worktree command is
independently available with the inherited session capability. Factor shared
transport/environment validation out of the marker-gated general Client; only
the fixed `worktree` operation uses the marker-free path. It cannot accept
arbitrary patch JSON or grant attention writes. Codex SessionStart receives
worktree-only additional context independently of that API-only marker. When
the marker is present, combine contexts into one valid hook response. This
proposal does not enable the general metadata launch marker, native observers,
completion producers, or any #789/#690 gates.

Update both shell and PowerShell reporters and their stable-asset verification.
For existing live sessions, integration reconciliation installs the helper;
the agent can call it explicitly without restarting or reopening that session.
New/resumed Codex sessions receive the worktree-reporting instruction. The
first implementation targets this observed Codex flow. Other harnesses retain
their existing signals; this is not a claim of automatic worktree discovery.

## Visible behavior and capture freshness

Expose the selected Git location source (agent, harness, process or launch)
through session projection and Changes captures. Details and Changes display
`Reported task worktree`, `Harness-reported directory`, `Process directory` or
`Launch directory; task worktree not reported` next to their Git facts.
This is explanatory copy in the existing panel, with no new navigation or
worktree-management controls. Non-Git and unavailable locations retain an
explicit unavailable state.

Both comparisons and base enumeration resolve the same binding. Root changes
invalidate old file snapshots via the existing subject validation. A displayed
capture keeps its captured identity and source until explicit Refresh; a session
binding change clears mismatched data and requires a new capture. Failed
refresh cannot retain data from a previous worktree. Workspace/session changes
keep their existing reset and stale-response rules. Git changes do not acquire
file-editing or Git mutation authority.

## Compatibility, recovery and verification

This is an additive optional record field and additive accepted patch member.
No bulk migration or rewrite is required. Legacy records keep existing lookup
behavior with explicit source labeling. Older servers reject the new patch
instead of silently accepting it. Older writers/binaries may drop unknown
fields on rewrite: rollback removes the feature and its binding guarantee;
do not describe mixed-version writers as supported. No binding changes resume
CWD, exact native session identity or process launch behavior.

Acceptance uses two real linked worktrees: A/main remains untouched, B/feature
has distinct uncommitted and committed changes. Report B through the actual
helper and authenticated router; assert branch, list, baseline and patch all
come from B. Repeat after a read-only command in A and a hook reporting A.
Verify A-to-B-to-C switches, stale snapshot rejection, removed B, clear, old
records, authorization/revision failures, restart/history, and unchanged bare
shell resolution. Existing live-binding preservation and marker-gate tests are
required. The [implementation plan](../plans/2026-10-25-session-worktree-location.md)
contains the delivery checks.
