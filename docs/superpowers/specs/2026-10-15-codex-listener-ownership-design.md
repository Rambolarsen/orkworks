---
type: Design
title: Codex listener ownership before capability delivery
description: Proposed connected-socket proof and native-client requirements before disclosing the private Codex capability.
tags: [orkworks, codex, security, runtime]
status: draft
---

# Codex listener ownership before capability delivery

Status: **owner-approved for dormant local macOS proof on 2026-10-16;
all-client integration remains blocked**. The owner approved the linked local
proof proposal in the active Codex session ("lets do it"). This does not approve
a new upstream transport, native-client identity mechanism, or production rollout.
Issue: [#763](https://github.com/Rambolarsen/orkworks/issues/763).
Parent outcome: [#690](https://github.com/Rambolarsen/orkworks/issues/690).
This completes a startup design investigation, not the Needs You fix or a
production verification gate. The [approved native design](2026-10-03-codex-native-approval-status-design.md)
and [ADR 0076](../../adr/0076-codex-owned-native-approval-observer.md) retain
all existing signal, configuration, metadata-writer, and rollout gates.

## Outcome and pass condition

Before any WebSocket upgrade can disclose the native capability, prove that
its already-connected TCP stream terminates in the retained owned Codex
server process. Apply this to the observer, native TUI, and every reconnect.
An accepting competitor must receive zero forwarded bytes even if it can
imitate initialization and the owned child remains alive.

## Evidence and alternatives

The source audit covers Codex 0.160.0 at
[a956835d020762cb2b570053af06f643a11c0ecc](https://github.com/openai/codex/tree/a956835d020762cb2b570053af06f643a11c0ecc)
and installed Codex 0.162.1 at
[092d3acd6bec3e3a14bdc7e7a2810ab628ab759d](https://github.com/openai/codex/tree/092d3acd6bec3e3a14bdc7e7a2810ab628ab759d).
The latter is the dereferenced upstream tag; matching version output alone
does not authenticate installed binary provenance or qualify its protocol.

- In the pinned 0.160.0 [TCP acceptor](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server-transport/src/transport/websocket.rs),
  the server binds its own TCP listener. Authentication occurs in the HTTP
  upgrade handler. The listener exposes health/readiness responses, but those
  responses do not prove process ownership. No inherited-listener contract is
  used by this proposal.
- The pinned [Unix acceptor](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server-transport/src/transport/unix_socket.rs)
  accepts upgrades without the TCP capability-token check. The same distinction
  exists in the audited 0.162.1 transport. Filesystem privacy alone would not
  retain the intended exclusion of same-user tool processes from native
  approval control. A direct Unix-socket substitution is rejected.
- The installed `codex-cli 0.162.1` help advertises TCP and Unix transports and
  remote-TUI authentication. Help was inspected without starting a Codex
  server, conversation, or second coding harness. The [official app-server
  documentation](https://learn.chatgpt.com/docs/app-server) still labels this
  upstream interface experimental; existing owner acceptance remains required.
- Checking only the observer connection leaves the TUI's independent upgrade
  and reconnects exposed. A retained relay covers both without changing native
  JSON-RPC or introducing an approval client.

On 2026-10-15, a disposable C/Python probe on macOS 26.6.2 (25G83) exercised
`proc_pidinfo(PROC_PIDLISTFDS)` and
`proc_pidfdinfo(PROC_PIDFDSOCKETINFO)` against fake child servers. It matched
an accepted IPv4 loopback TCP socket by its reversed local/remote address and
port tuple, TCP kind, and established state. The SDK declarations came from
`MacOSX.sdk/usr/include/{libproc.h,sys/proc_info.h}`. Compilation used
`clang -Wall -Wextra -Werror`.

The first fake child retained an accepted stream but released its listener.
A second child bound the same listening port and accepted a new stream. The
probe accepted the original child/stream pair, rejected the original child
for the replacement stream, and accepted the replacement only for its actual
owner. A conditional send to the rejected stream left the competitor with
zero application bytes. All four assertions passed and both children exited.
The shell sandbox initially denied loopback bind; the isolated probe then ran
through the approved escalation path. No native capability or session data was
used. These disposable probes establish local feasibility only; they are not
committed regression tests, an installed Codex diagnostic, relay verification,
or Linux/Windows evidence.

## Rejected relay and crash finding

A sidecar-owned retained TCP relay can check every upstream stream while the
sidecar lives. It is insufficient as the full listener-ownership solution.
If the sidecar crashes, its relay listener closes while the native TUI may
survive with the capability and endpoint. A competitor can bind that endpoint
before the TUI reconnects. The proxy no longer exists to enforce its proof.
Normal shutdown ordering cannot exclude this crash window.

This is a real upstream path, not a hypothetical retry assumption: the pinned
0.160.0 [TUI reconnect implementation](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/tui/src/app/reconnect.rs)
reinitializes a fresh client and retries with delays 0, 1, 2, 4, then 8 seconds
within a shared 120-second budget. The audited 0.162.1 source retains this
reconnection architecture. Do not introduce the relay or count live-sidecar
competitor tests as crash-safe TUI evidence.

Passing a listener lease to the TUI or relying on parent-death cleanup is not
an established upstream contract. Neither is assumed here. Private Unix paths
and a second relay token also fail to establish server identity to a surviving
native client. No runtime port, token, hook trust, or launch behavior changed
during this investigation.

## Proposed identity contract

Keep authenticated IPv4 loopback WebSocket and the memory-only 256-bit
capability. Require **each client itself** to prove its connected server peer
before disclosing that capability. The observer can implement this inside
OrkWorks; the unmodified audited native TUI cannot currently meet the contract.
No new native CLI flag or upstream API is claimed to exist.
The pinned [native remote client](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server-client/src/remote.rs)
builds its Authorization header before calling URL-based WebSocket connect;
that TCP path has no expected owned-process/birth-identity check.

1. Start the canonical owned Codex server with the existing private server-port
   attempt and capability digest. Its execution environment excludes the
   plaintext capability. Retain the unreaped child identity and existing
   executable and runtime-generation fences.
2. Each client opens an upstream TCP stream without sending application bytes.
   Obtain that retained stream's actual local and peer addresses and ports.
   Prove an exact reversed established socket in the owned server leader's
   descriptor table using the macOS boundary below. A listener, banner,
   successful upgrade, or RPC response is insufficient.
3. Recheck the owned process instance and executable identity before the first
   upgrade byte. Missing, denied, truncated, malformed, ambiguous, or stale
   proof closes the stream without sending the capability. A listening port
   that belongs to the child is insufficient; its accepted peer must match.
4. Perform the authenticated WebSocket upgrade on that **same retained stream**.
   Never verify one stream and then reconnect by URL. If the peer closes, the
   retained stream cannot migrate to a replacement listener on the same port.
5. Every native TUI and observer retry must repeat the proof on its new stream.
   The expected server identity is immutable for the conversation, never
   rediscovered from endpoint banners or a global daemon. The child PID must
   be paired with process-birth identity so PID reuse after owner death cannot
   qualify a different process. Sidecar-owned unreaped children retain PID
   identity while the sidecar lives; that alone does not fence TUI survival
   across sidecar death.
6. Loss of the expected server/process identity is terminal for that connection
   target. Keep native attention conservative; never replay a conversation or
   let a surviving TUI attach to a replacement server.

The proof trusts the identity-checked Codex process and the OS kernel. It does
not defend against a compromised server, kernel, or an attacker able to alter
client memory. Unrelated same-user listeners and PID reuse remain in scope.
Only the exact server leader qualifies; a descendant, shared daemon, or another
session's server does not. Keep upstream capability authentication to exclude
other clients even if they discover the listening port.

**Native client prerequisite:** an upstream-reviewed, exact-version native-TUI
connection contract must enforce steps 2–6 before each upgrade, including its
120-second reconnect path and sidecar-crash survival. The reviewed implementation
must establish how immutable server PID/birth identity reaches the TUI and how
it is checked without exposing the capability first. Existing `--remote` and
`--remote-auth-token-env` help do not establish that support. Until it exists,
production startup and the installed diagnostic stay unavailable. An owner
approval of this document cannot make the audited binary satisfy this contract.

## macOS proof boundary

Use `proc_pidinfo` to obtain a bounded, complete descriptor inventory for the
retained child PID, then `proc_pidfdinfo` for socket descriptors. Require full
structure sizes, `AF_INET`, `IPPROTO_TCP`, `SOCKINFO_TCP`,
`TSI_S_ESTABLISHED`, loopback addresses, and both reversed ports. Decode the
SDK's network-order port fields explicitly. A listening socket is insufficient.
Zero or multiple matches fail closed. Table growth/truncation, disappeared
FDs, process exit, or permission errors may be retried within the existing
startup budget; they never permit forwarding. FD-number reuse cannot substitute
an unrelated tuple. Retaining the client stream prevents an independent live
TCP connection from reusing that same four-tuple during the proof.

Inventory and structure buffers are bounded to 1 MiB, with at most three
inventory attempts. Run kernel inspection off the async reactor. An accepted
connection that has not yet appeared in the owned child's descriptor inventory
may be retried, without sending HTTP first. Uncertain results remain unavailable.
All limits, integer conversions, ABI layouts, and failure paths need behavioral
coverage against the actual supported SDK/OS; the disposable probe is not the
implementation.

Initial implementation eligibility is **macOS arm64 only**, still with an empty
shipping compatibility table. Linux and Windows return unavailable and retain
direct launch until their own connected-stream identity contract and evidence
are reviewed. Codex 0.162.1 does not inherit 0.160.0 protocol verification.

## Bounds, failure, and lifecycle

Keep at most three server attempts within the existing five-second startup
budget, each with a fresh capability. Initial connection and proof share that
deadline. Bound any later observer connection/proof to three seconds; native
client retries require their independently reviewed deadline and cancellation
contract. No bytes or capability leave a client before its proof passes.

Runtime-generation revocation cancels sidecar proofs. End and workspace
shutdown revoke observation and use existing bounded descendant cleanup.
Observer disconnect closes only its connection and cannot answer or dismiss a
native prompt. An established conversation is never replayed or silently
reattached. A failed proof before native TUI startup may select the unchanged
direct-launch path. A failure after startup keeps attention conservative.

## Delivery and required evidence

The [native-client proposal](2026-10-16-codex-client-identity-proposal.md)
refines the missing upstream handoff and reconnect requirements; its process-instance
backend remains an explicit prerequisite, not an approved implementation.
The [implementation plan](../plans/2026-10-15-codex-listener-ownership.md)
separates the locally feasible proof from the missing native-client contract.
Before removing the listener gate, require:

- Committed regressions for both startup and reconnect where an accepting
  competitor receives zero capability/upgrade bytes. A 401-only fixture is
  insufficient. Cover both the sidecar observer and actual native TUI.
- Positive ownership; wrong process, birth identity, tuple, and socket state;
  permission failure; descriptor growth/truncation; deadlines; cancellation;
  and child exit. A reused PID must fail the native client's identity check.
- Listener loss/rebinding between proof and upgrade: either the retained
  connection still reaches its owned peer or the upgrade fails. A replacement
  must never receive bytes from that stream.
- A sidecar-crash test with a surviving native TUI, an accepting replacement
  listener, and a real native reconnect attempt. Verify zero capability bytes
  and no attachment to the competitor, without relying on timely child cleanup.
- Installed exact-binary/OS/architecture evidence, including startup failure,
  bounded cancellation/descendant cleanup, and a held native approval that
  survives observer disconnect. Record build identity and token-free evidence.

Keep `VERIFIED` empty and native clearing disabled. Listener proof alone does
not satisfy [#788](https://github.com/Rambolarsen/orkworks/issues/788),
[#789](https://github.com/Rambolarsen/orkworks/issues/789), or #690's installed
manual, slow-auto, overlap, root/subagent, configuration, and lifecycle gates.

## Review and approval

Inline review disposition: **Investigate native client support; local proof
contract ready for owner review**. Review rejected the original relay proposal
because the audited TUI reconnect path survives loss of the relay owner. The
local macOS probe supports only observer-side feasibility. No full startup
implementation is ready under the current upstream contract.

Owner approval can authorize the dormant local proof implementation and the
native-client contract investigation. It does not waive the absent upstream
mechanism or authorize a relay, installed diagnostic, compatibility record, or
production rollout. Metadata work #788 remains an independent ready prerequisite
under its separately approved design.
