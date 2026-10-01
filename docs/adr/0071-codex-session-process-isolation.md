# Codex sessions use independent processes

- Status: accepted
- Deciders: repository owner, Codex
- Date: 2026-10-01

## Context

Codex CLI 0.159.2 defaults to a shared app-server daemon. A live investigation
found three OrkWorks-launched conversations served by one daemon process.
Their shell snapshots all carried the first launch's `ORKWORKS_SESSION_ID`
and `ORKWORKS_CODEX_SESSION_REPORT_DIR`. Native IDs from later conversations
were consequently reported to the older OrkWorks session and rejected by its
identity replacement guard. Per-PTY environment injection alone cannot scope
capabilities when the coding tool delegates execution to a shared process.

The installed CLI's `codex --help` advertises `--no-daemon` as running without
the shared background server, even when one is already running. Older Codex
versions supported by OrkWorks may not recognize that argument.

## Decision

At the common Codex runtime startup boundary, probe the actual configured
executable with `--help` before spawning the PTY child. Bound the probe to
three seconds and 64 KiB per output stream. Add `--no-daemon` before existing
arguments only when the help output advertises that exact option, without
adding a duplicate. This applies to new sessions and exact resume. An
unsuccessful or truncated probe aborts startup with a readable retry error;
a successful probe without the flag preserves the older CLI's arguments.
Other coding tools do not run this probe.

Retain the existing capability generation, report mailbox, hook trust,
identity replacement rules, and exact-resume checks. Do not edit global Codex
configuration, stop its shared daemon, or change already-running sessions.

## Consequences

Each compatible OrkWorks Codex launch owns its execution environment instead
of borrowing an earlier launch's capability. It also pays one bounded help
probe at startup. Existing sessions must be ended and relaunched to obtain
isolated environments; their native identities are never guessed or rewritten.
A separate process may use more resources than Codex's shared daemon.

This decision constrains Codex launch preparation; ADR 0068 remains the
identity contract and ADR 0069 remains the transport contract.
