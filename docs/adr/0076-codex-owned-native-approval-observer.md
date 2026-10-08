# Codex approval status from a runtime-owned native server

- Status: accepted (implementation and version-specific rollout gated)
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

## Decision

For feature-probed, configuration-compatible Codex launches with an exact
version/platform entry backed by completed native protocol verification, own one private
app-server and remote native TUI within the session runtime. Use authenticated
loopback WebSocket with a fresh memory-only token; tool execution inherits the
session reporting environment, while only the TUI and passive observer receive
the native connection secret. The TUI remains the sole approval controller.
Unsupported configurations or unverified versions/platforms retain the existing
isolated direct launch. Help compatibility alone never authorizes native clears.

Pair validated PermissionRequest with a two-second grace and continuous,
bounded, exact-root native observation. Show Needs You after grace when pending
is confirmed, or conservatively when observation is unavailable or ambiguous.
Clear only an unambiguously owned wait with a fresh baseline and a newly
observed attributable pending edge, followed by a fresh pending-to-active transition
with no waiting flags and all generation, identity, input, and source fences
intact, including an attention-owner write token that identical competing writes
revoke. Native idle and generic completion cannot clear it. Keep overlap and
uncertain subagent flows conservative. Time alone never classifies approval.

Owner approval: on 2026-10-05 the repository owner approved the linked design
and its experimental upstream dependency with “lets go ahead”. This approval
permits implementation; it does not waive the production verification gates.

The full [approved design](../superpowers/specs/2026-10-03-codex-native-approval-status-design.md)
defines configuration eligibility, protocol bounds, lifecycle ownership,
attention transitions, evidence limits, and release gates. Written review is
complete; version-specific verification remains required before rollout. No compatibility entry is authorized by this approval alone.
The shipped mappings in ADR 0051 and startup in ADR 0072 remain unchanged until
a verified entry enables the implementation.

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

## Amendment — 2026-10-05 implementation audit

The decision stands; two implementation constraints operationalize its passive
observer and ownership requirements.

For the pinned 0.160.0 protocol, ordinary initialization names originate global
client identity and implicit gateway login. The observer must use the reviewed
non-originating initialization contract without adding capabilities or overriding
execution configuration. The exact source and read-only installed checks are
recorded in the [verification evidence](../superpowers/verification/2026-10-05-codex-native-approval-status.md).
Other protocols require their own reviewed initialization contract.

The MVP permits direct agent session-JSON writes. A sidecar revision and file
identity do not atomically fence an arbitrary direct writer during a final
check and rename. Production native clearing therefore remains disabled under
the design's unfenced-writer rule, independently of version eligibility.
[#761](https://github.com/Rambolarsen/orkworks/issues/761) tracks the required
written producer/ownership contract. This implementation does not demote that
producer or impose a new protocol lock. Cooperating-writer tests alone cannot
remove this gate.

## Amendment — 2026-10-06 listener ownership review

The authenticated owned-server decision stands. Review found that releasing
a reserved port before spawn and checking child liveness does not prove the
listener belongs to that child. A competing listener can receive the bearer
during upgrade, before response validation. The implementation must fail
closed before bearer delivery for production and the installed diagnostic.
Controlled test-only fake fixtures do not qualify an installed listener.

[#763](https://github.com/Rambolarsen/orkworks/issues/763) tracks the required
written and reviewed owned-listener handoff/identity contract, including
subsequent connections. No readiness-output, port-zero, inherited-socket or
additional RPC contract is assumed. The prepared operator procedure must
remain withheld until that prerequisite is implemented and verified.

## Proposed Amendment — 2026-10-08 sidecar-mediated metadata writes

Pending repository owner review and approval. Until accepted, the existing
accepted ownership contract and production-clear gate remain authoritative;
this proposal does not authorize producer-protocol or runtime changes.

The proposal keeps one canonical session record and requires active metadata
writes for native-enabled Codex sessions to pass through an authenticated,
versioned sidecar API. The launch adapter advertises the protocol only for a
migrated integration. Direct JSON reads remain available; direct JSON writes
are unsupported for those active sessions. The API uses a whole attention
tuple or an allowlisted work-metadata patch, rejects stale revisions, and
serializes the full read/check/modify/write operation with native clear under
the same per-session transaction boundary. Direct agent, harness-hook, and
native Codex signals use the same `agent` attention tier; persisted
`attentionSource`, `attentionConfidence`, and `attentionOrigin` distinguish
attention authority from general work-metadata provenance. Every accepted
attention write, including an identical-value write, advances the ownership
revision. A durable attention-specific update time preserves the existing
strict Peon staleness rule without letting unrelated metadata writes refresh
it.

The versioned agent operation uses the live session report token and session
ID. A stale revision returns a conflict for reread and retry; the endpoint
never falls back to direct file mutation. The token remains a bearer
capability, not proof of OS process identity. The single-record projection
keeps existing tuple and source-priority semantics, and metadata read/write
failures cannot be interpreted as permission to clear or lower the source.

This proposal changes the current direct-write contract for native-enabled
sessions and therefore requires explicit owner approval before implementation.
Until accepted, the existing JSON contract and production-clear gate remain
authoritative. The detailed API contract, compatibility limits, timestamp
behavior, race tests, and production-clear evidence gate are in the
[sidecar-mediated metadata design](../superpowers/specs/2026-10-08-codex-native-attention-layer-design.md).
This proposal does not enable production clearing; producer migration,
behavioral tests, #690 signal verification, and the independent #763 listener
gate remain required.
