---
type: Architecture Decision
title: Explicit session task-worktree binding
description: Resolve session Git context from an authenticated, durable task-worktree report.
tags: [architecture, sessions, git, worktrees]
---

# Explicit session task-worktree binding

- Status: accepted
- Date: 2026-10-25
- Deciders: OrkWorks owner

The owner approved the [design](../superpowers/specs/2026-10-25-session-worktree-location-design.md)
and [implementation plan](../superpowers/plans/2026-10-25-session-worktree-location.md)
on 2026-10-25 for [#1056](https://github.com/Rambolarsen/orkworks/issues/1056).

ADR 0032's harness/process/launch CWD chain cannot identify a task worktree when
the coding agent uses a tool-specific working directory. Add an optional durable
`worktreeBinding` resolved by the sidecar from a revisioned, authenticated
`agentMetadata.worktreePath` report. Same-repository bindings outrank that chain;
an unavailable binding fails closed instead of reverting to the launch checkout.
Launch/resume CWD and native identity remain unchanged. Git facts and Changes
share this source and disclose fallback origin. Temporary commands elsewhere do
not replace the task binding.

The narrow helper command and Codex SessionStart instructions work independently
of the gated general metadata helper commands. This does not enable the general
API-only launch marker, native observations or completion gates. Reporting is
cooperative, not automatic directory discovery. Older records retain labeled
fallback behavior; mixed-version writers are unsupported. See the linked design
for lifecycle, recovery, validation and verification contracts.

## Amendment — managed worktree reporting transport, 2026-10-31

The owner approved automatic selection of the verified Codex-managed relay for
worktree reporting in [#1193](https://github.com/Rambolarsen/orkworks/issues/1193).
The [transport design amendment](../superpowers/specs/2026-10-24-sandbox-metadata-proxy-design.md#amendment--automatic-worktree-transport-2026-10-31)
defines the trusted runtime marker/URL pair, strict validation and failure rules.
This operationalizes cooperative reporting inside the managed sandbox; it does
not discover directories, change CWD, or enable broader metadata commands.

## Amendment — launch-scoped instruction delivery, 2026-11-05

The owner approved the [launch-scoped instruction design](../superpowers/specs/2026-11-04-launch-scoped-worktree-instructions-design.md) and its implementation plan on 2026-11-05.
A private run-local selector may target the shared helper text to a qualified
OpenCode root independently of legacy native identity capture. New launches use
an OS-random CLI-selected ID; exact resume selects its requested ID; native
read-back must match with no parent before delivery. Children and peers receive
none. Only current owned assets and the qualified source-owned private-server
profile may participate. Asset checks are cooperative compatibility checks, not
an atomic native-loader or malicious local replacement fence.

The bounded context hook renders only `worktree-instructions`; the agent still
reports from its own tool context through the shared authenticated transport.
This adds no persisted identity, attention authority, Git resolver or automatic
location discovery. Separately owner-authorized qualification of the actual
changed reporter/helper and prepared launch scope passed on 2026-11-05 for
OpenCode 2.2.5/macOS arm64; source preparation is enabled for that profile.
The [evidence record](../validation/harness-workspace-signals.md#actual-source-native-qualification-2026-11-05)
does not establish installer deployment or agent compliance with the delivered text.
