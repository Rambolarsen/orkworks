---
type: Design Proposal
title: Codex client identity before authenticated local connection
description: Proposed upstream client contract that binds every native connection and reconnect to one owned server instance before sending credentials.
tags: [orkworks, codex, security, runtime, upstream]
status: draft
---

# Codex client identity before authenticated local connection

Status: **proposed; implementation and upstream submission not approved**.
Tracks [#763](https://github.com/Rambolarsen/orkworks/issues/763), prerequisite
of the long-running automatic-approval attention fix
[#690](https://github.com/Rambolarsen/orkworks/issues/690).
The owner requested this proposal after the dormant local proof merged in
[PR #900](https://github.com/Rambolarsen/orkworks/pull/900).
It refines the missing native-client contract in the
[listener ownership design](2026-10-15-codex-listener-ownership-design.md),
without replacing its approved local scope or
[ADR 0076](../../adr/0076-codex-owned-native-approval-observer.md).

## Problem and required outcome

An application owns one Codex app-server, then launches the native TUI and a
passive observer against its private IPv4 loopback WebSocket. A process can
win the bind race or replace a listener after its owner crashes. The bearer
must not reach that replacement, even if it accepts the connection and imitates
initialization. The TUI must enforce this independently of its launcher.

Every client must bind its actual connected stream to the same immutable,
expected server process instance before sending any HTTP upgrade bytes.
Reconnection to that instance is allowed within the existing native lifecycle;
attachment to a replacement instance or conversation replay is forbidden.
The TUI remains the only approval controller. This proposal does not add an
approval response API or change attention-clearing rules.

## Source evidence and limits

Audit date: 2026-10-16. The latest release API returned `rust-v0.163.0`, published
2026-10-15T21:30:38Z. Annotated tag
`129b8ec7ee6f4511557a4856279f529025c25af4` resolves to source commit
`83086188ae0614bd882818ff1aa23f3194962012`.

- The pinned [remote TCP connector](https://github.com/openai/codex/blob/83086188ae0614bd882818ff1aa23f3194962012/codex-rs/app-server-client/src/remote.rs#L718-L779)
  inserts Authorization and calls URL-based `connect_async_with_config`.
  That function accepts no expected process instance or proved stream.
- The pinned [TUI connection dispatcher](https://github.com/openai/codex/blob/83086188ae0614bd882818ff1aa23f3194962012/codex-rs/tui/src/app_server_connection.rs)
  passes a cloned remote endpoint into the remote connector. Its separate
  Windows local-daemon branch checks that daemon's socket policy, which does
  not establish ownership of this application-launched TCP server.
- The pinned [native reconnect loop](https://github.com/openai/codex/blob/83086188ae0614bd882818ff1aa23f3194962012/codex-rs/tui/src/app/reconnect.rs#L30-L122)
  retains a target, creates fresh connections, and retries within 120 seconds
  with delays 0, 1, 2, 4, then 8 seconds. Its current catch-all error handling
  also needs a terminal identity-failure distinction.
- [Official transport documentation](https://learn.chatgpt.com/docs/app-server#transports)
  describes bearer authentication during the WebSocket handshake and calls
  WebSocket transport experimental and unsupported. Authentication of a client
  to a server does not establish the reverse process-identity guarantee.

These are source observations, not qualification of a distributed binary.
No Codex server, native TUI, conversation, or installed diagnostic was started.
No existing flag or public upstream identity API is claimed to implement the
proposal. Exact API spelling below is conceptual and subject to upstream review.

## Recommended contract

Add an opt-in owned-local-server policy in the shared upstream remote client,
and carry it unchanged through native TUI initial connection and reconnect.
Ordinary remote and local-daemon modes retain their current policy; they cannot
be selected as fallback inside an owned-server conversation.

The launcher supplies a versioned connection policy through the private TUI
launch boundary. It contains the literal `ws://127.0.0.1:PORT` endpoint, a
positive server leader PID, an OS-backed process-instance identity, executable
identity constraints, and a policy version. It does not contain the bearer.
Capture the identity from the retained spawned child before launching any
client. Never derive the expected identity from the listener, an initialization
response, a discovered daemon, or the first process found at that PID.

The TUI validates and freezes this policy before opening a connection. It
keeps it in the connection target for the lifetime of the conversation,
including every clone, bootstrap, background retry, and resumed transport.
An external `resume` invocation starts a new launch/qualification decision; it
cannot reuse a stale identity bundle. Missing or unsupported policy fields
are errors, not instructions to downgrade to ordinary remote mode.

For the initial macOS scope, propose a one-shot private inherited read descriptor
whose number is supplied by a new, explicitly opt-in TUI launch argument.
The launcher writes one versioned policy message, bounded to 4 KiB, closes its
write end, and transfers only the read end to the intended TUI. The client
reads within the remaining startup deadline, requires EOF, rejects duplicate,
unknown or missing fields, freezes the validated policy, and closes the descriptor
before any tool spawn. Expected identity and executable fields use a tagged
platform encoding fixed by the reviewed backend; arbitrary strings cannot
qualify it. The argument name and serialization are proposed upstream API work,
not existing Codex support. Windows is unavailable under this initial handoff.

The identity message contains no bearer. Keep the current private bearer
delivery separate, remove private launch values before spawning tool children,
and never persist credentials in config or thread logs. The descriptor conveys
the trusted launcher's expectation; it does not itself prove the server peer.
Neither a user-configurable option nor a self-reported server identity may
activate OrkWorks eligibility. Upstream review must confirm descriptor ownership,
inheritance and early parsing before this handoff can be implemented.

Conceptual shared-client boundary:

```text
OwnedServerPolicy -> connect TCP without application writes
                  -> validate expected process instance and executable
                  -> prove reversed accepted tuple in that leader
                  -> revalidate instance, executable, deadline and cancellation
                  -> authenticated upgrade on that exact retained stream
```

The proof consumes or exclusively owns the stream until upgrade. It may not
return a URL for another connector to open. Reject hostnames, proxies,
redirects, alternate addresses, TLS mode changes, IPv6, and daemon discovery
for this initial IPv4-only policy. TCP control packets are permitted; no health
probe, HTTP header, authentication byte, or JSON-RPC message precedes proof.

## Process identity and the remaining platform prerequisite

The local OrkWorks helper borrows an unreaped owned leader, which prevents PID
reuse while its owner remains alive. A surviving native TUI does not own that
child and cannot assume this protection after the sidecar dies.

The native backend must retain an OS-backed instance reference or demonstrate
an equivalent kernel guarantee that distinguishes process reuse across every
inspection and the upgrade boundary. A numeric PID, executable path, matching
UID, or a freshly read birth value alone is insufficient. Expected birth
identity must originate at trusted spawn and remain immutable. Any kernel
reference transferred to the TUI must be restricted, validated, and absent
from tool-child inheritance.

The public macOS SDK inspected locally exposes process start seconds and
microseconds in `proc_bsdinfo`, queried by `PROC_PIDTBSDINFO`. Header presence
does not demonstrate a unique, non-reusable instance reference, atomic binding
to socket inspection, or survival semantics across owner death. This proposal
therefore **does not select timestamps as a completed macOS backend**. Upstream
and platform review must establish those guarantees, including executable
replacement and sockets inherited or duplicated into another process. If a
supported guarantee cannot be established, owned-local-server mode remains
unavailable on that platform; do not weaken the invariant to ship it.

Only an exact accepted socket in the expected leader qualifies. Require the
reversed four-tuple, IPv4 loopback, TCP stream kind, established state, complete
bounded descriptor results, and exactly one match. Reuse the local helper's
bounded SDK inspection lessons, not its owner-lifetime assumption. Linux and
Windows require independent backends and evidence; they inherit no macOS claim.

## Bounds and failure semantics

Initial TCP connect, proof, and upgrade share the launcher's remaining startup
deadline, capped by the existing five-second startup budget. No more than three
server attempts are permitted; each gets a fresh capability. The native client
must receive or derive a remaining budget without resetting it at each phase.

For reconnect, one connect/proof/upgrade attempt is capped at three seconds
and clipped to the existing 120-second overall reconnect deadline. Backoff
uses that same deadline. SDK inspection stays off the async reactor, has bounded
buffers and retries, and checks cancellation before each query and before
returning a usable stream. Cancellation closes the stream; background work
cannot later upgrade it. No detached worker receives the bearer.

| Result | Required behavior |
| --- | --- |
| Connection refused or accepted socket not yet visible | Retry without writes, within the shared budget and bounded inventory attempts. |
| Ambiguous, malformed, denied or truncated identity inspection | Close without writes; report unavailable and stop this owned connection target. |
| Expected instance missing, changed, exited, or executable mismatched | Terminal identity loss; no further reconnect, rediscovery, or downgrade. |
| Timeout or cancellation | Close without writes; release proof resources and preserve bounded shutdown. |
| Peer disappears after proof | Fail on the retained stream; never open an unproved replacement. |
| Same original instance reconnects successfully | Perform native bootstrap only after proof and authenticated upgrade. |

Typed identity-loss/unavailable errors must escape the current reconnect loop's
catch-all retry handling. Render a credential-free connection failure and keep
input and attention conservative. Once the TUI has started, a failed proof
cannot launch a second conversation. Before TUI startup, OrkWorks may use its
existing direct-launch fallback only after owned-attempt cleanup completes.

A sidecar crash does not authorize replacement discovery. If the original
server survives, a TUI may reconnect only after proving that same instance.
If it exits or cannot be proved, the target is terminal. Parent-death cleanup
is useful resource hygiene but is not a security prerequisite for this rule.

## Alternatives considered

| Approach | Disposition |
| --- | --- |
| Shared native-client owned-instance policy | Recommended: enforces the existing exact-process contract at every upgrade, including after launcher death. Platform identity proof remains a prerequisite. |
| Sidecar relay, readiness check, or inherited listener alone | Rejected: a surviving TUI can send its bearer to a replacement relay; bind ownership alone does not prove every future connection. |
| Per-instance cryptographic server authentication | Possible separate design: would require supported key delivery, possession proof and reconnect pinning. It changes the identity/transport assumptions and is not authorized by the local-proof approval. |
| Unix socket replacement | Rejected under the existing audit: it does not preserve the TCP capability authentication boundary merely by using a private path. |

## Acceptance and delivery

1. Upstream review accepts or revises the proposed bounded inherited-descriptor
   handoff and shared initial/reconnect connector; it identifies every native
   client path that can send credentials. Confirm CLI/API spelling and platform
   policy encoding before implementation.
2. Platform review establishes instance lifetime, executable identity, descriptor
   sharing and post-owner-death behavior. Record exact supported OS/SDK/architecture
   and fail closed everywhere else. This is a blocker, not deferred hardening.
3. Regression tests inject an accepting competitor before initial bind, after
   sidecar crash, and during a real TUI reconnect. Competitors receive zero
   application bytes; no successful initialize response can override identity.
4. Test PID reuse with the old expected birth identity, owner exit, wrong
   executable, wrong leader/descendant, transferred/duplicated socket, ambiguous
   tuples, accept visibility lag, denied/truncated inspection, cancellation,
   shared deadlines and replacement between proof and upgrade. A retained
   original stream either reaches its expected peer or fails.
5. Native TUI tests prove policy retention and terminal-error handling through
   the actual reconnect path. Both observer and TUI must enforce the contract;
   a fake client or local helper test alone cannot qualify either installed path.
6. Qualify the exact upstream artifact and configuration with token-free evidence,
   including a surviving TUI after launcher death and a held approval that the
   observer cannot answer. Only then consider OrkWorks integration under a
   reviewed ADR amendment and implementation plan.

This document can be reviewed and stored as a proposal without approving
implementation. The next decision is whether to pursue this upstream client
contract and its platform feasibility review. Sending an upstream issue or PR
requires separate authorization. No such message has been sent.

`VERIFIED` stays empty. Production native startup, installed diagnostics, launch
markers and native attention clearing stay disabled. Metadata handshake
[#789](https://github.com/Rambolarsen/orkworks/issues/789) and #690's manual,
slow-auto, overlap, root/subagent and lifecycle evidence remain independent.
There is no timer-based fix for long-running tools in this proposal.

## Inline design review

Checked the proposal against the existing listener design and ADR 0076:
initial and reconnect streams share the same invariant; sidecar lifetime is
not used as a TUI guarantee; uncertain platform support fails closed; no new
flag, transport or installed capability is asserted. The material unresolved
questions are the upstream handoff and a qualifying native process-instance
backend. They are explicit prerequisites to implementation, not missing
permissions to bypass the existing gate. No production behavior changes.
