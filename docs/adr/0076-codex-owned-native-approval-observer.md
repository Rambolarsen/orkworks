# Codex approval status from a runtime-owned native server

- Status: proposed
- Deciders: repository owner, Codex
- Date: 2026-10-04

## Context

Codex PermissionRequest also fires for automatic review. The ADR 0051 redacted
serial capture gate passed for real manual and automatic sequences, but the
payload does not identify the reviewer. PostToolUse occurs after execution;
a hook-only grace period still shows false Needs You during a long approved
tool. An isolated native experiment observed a manual approval's
waitingOnApproval flag disappear while execution continued.

The current independent direct launch (ADR 0072) does not provide an owned
passive status connection. Introducing that connection must retain per-session
execution credentials and native approval control, not borrow a shared daemon.

## Decision (proposed)

For feature-probed, configuration-compatible Codex launches, own one private
app-server and remote native TUI within the session runtime. Use authenticated
loopback WebSocket with a fresh memory-only token; tool execution inherits the
session reporting environment, while only the TUI and passive observer receive
the native connection secret. The TUI remains the sole approval controller.
Unsupported configurations retain the existing isolated direct launch.

Pair validated PermissionRequest with a two-second grace and continuous,
bounded, exact-root native observation. Show Needs You after grace when pending
is confirmed, or conservatively when observation is unavailable or ambiguous.
Clear only an unambiguously owned wait with a fresh baseline and a newly
observed attributable pending edge, followed by a fresh pending-to-active transition
with no waiting flags and all generation, identity, input, and source fences
intact, including an attention-owner write token that identical competing writes
revoke. Native idle and generic completion cannot clear it. Keep overlap and
uncertain subagent flows conservative. Time alone never classifies approval.

The full [proposed design](../superpowers/specs/2026-10-03-codex-native-approval-status-design.md)
defines configuration eligibility, protocol bounds, lifecycle ownership,
attention transitions, evidence limits, and release gates. Its written review
and version-specific verification are required before rollout. This ADR does
not yet change the accepted mappings in ADR 0051 or startup in ADR 0072.

## Consequences

The approach can resolve approval waits before long tool completion without
answering a native request. It adds a server process, authenticated client,
configuration mapping, and platform-specific ownership checks per session.
Unsupported or ambiguous flows may still show false Needs You.

Upstream labels app-server and WebSocket experimental and unsupported for
production workloads; owner acceptance must explicitly include that dependency
risk. Slow automatic-review and root/subagent semantics remain blocking signal
verification gates. The existing successful serial hook and native experiments
are evidence for the proposal, not proof that the production fix is complete.
