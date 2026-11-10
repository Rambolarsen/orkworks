---
type: "Architecture Decision"
title: "Codex sessions use independent processes"
description: "Architecture decision record: Codex sessions use independent processes."
tags: ["orkworks", "architecture"]
---

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
unsuccessful or truncated probe aborts startup and logs a retry diagnostic;
a successful probe without the flag preserves the older CLI's arguments.
The probe uses the PTY environment filter and excludes all `ORKWORKS_`
variables, including UI approval authority and any inherited session reporting
capability. Other coding tools do not run this probe.

Retain the existing capability generation, report mailbox, hook trust,
identity replacement rules, and exact-resume checks. Do not edit global Codex
configuration, stop its shared daemon, or change already-running sessions.

## Consequences

Each compatible OrkWorks Codex launch owns its execution environment instead
of borrowing an earlier launch's capability. It also pays one bounded help
probe at startup. Startup failures retain the existing generic UI error;
the detailed probe failure is available in sidecar logs. Existing sessions
must be ended and relaunched to obtain
isolated environments; their native identities are never guessed or rewritten.
A separate process may use more resources than Codex's shared daemon.

This decision constrains Codex launch preparation; ADR 0068 remains the
identity contract and ADR 0069 remains the transport contract.

## Proposed extension 2026-10-03

[ADR 0076](./0076-codex-owned-native-approval-observer.md) proposes a runtime-owned
private app-server plus remote native terminal for compatible configurations,
retaining independent execution environments and this ADR's direct launch as
fallback. It requires written review and lifecycle/configuration verification;
the accepted startup decision above remains in effect.

## Amendment — 2026-11-07: own short discovery probe processes

The owner approved the [shared probe ownership contract](../superpowers/specs/2026-11-07-probe-process-ownership-design.md)
on 2026-11-07. Short version/help and OpenCode path-selection probes own a Unix
process group or a suspended-child Windows Job, clean up after every outcome
including successful output, and retain their current per-caller policies.
Unix signaling precedes direct-child reaping; escaped sessions and brokered
launches remain disclosed limits. Native termination requests do not promise
immediate physical exit or same-user confinement. Provider safeguards from
ADR 0055 are preserved.

The existing three-second operation budget includes cleanup waiting; cleanup
settlement is at most one second within that budget. Admission is bounded to
64 active/pending-reap owners per sidecar, with no queue or retry at saturation.
Shutdown closes admission and requests cleanup before a bounded reaping drain.
This amendment does not authorize a hosted model probe, native activation,
storage cutover, process adoption or termination of a user's shared daemon.

## Amendment — 2026-11-10: confirm a settling Darwin probe leader

The owner approved the [#957 correction](../superpowers/plans/2026-11-10-probe-exit-confirmation.md).
After one Darwin EPERM group-signal result, an original-child exit that is not
yet observable may be confirmed asynchronously within the existing shared
cleanup/reaping endpoint. Acceptance still requires original-child exit, the
existing bounded native membership proof and timely reaping. Retain original
ownership until confirmation is finalized; no later numeric signal is issued.
Cancellation/shutdown finalize pending confirmation as unconfirmed and leave
only exact-child reaping. This operationalizes the existing ownership decision
and does not supersede its practical boundary, deadline or failure policy.
