---
type: Design
title: Ordered completion hook delivery
description: Define bounded sender-ordered hook publication and acknowledgement without qualifying a production launch.
tags: [codex, hooks, runtime, protocol]
status: stable
---

# Ordered completion hook delivery

Owner-approved protocol extension, 2026-10-24: the owner answered “keep going”
to the concrete design approval request. Implements the delivery prerequisite
of the owner-approved 2026-10-23 completion mitigation. Tracks #690 and #789.
This proposal does not activate completion clearing or qualify a launch.

## Established evidence

The installed explicit-proxy metadata helper now matches merge 1d780581 and
passes authenticated GET, validation-only PATCH and follow-up GET inside the
actual restricted sandbox. The current session record stayed byte-identical.
The current launch remains legacy: there is no API-only marker or live upgrade.

The existing hook relay sorts identity UUID filenames and retries bounded
approval-slot records; neither is a complete ordered hook stream. Approval
records lack tool_name. The completion reducer requires sender-established
contiguous sequence, root/turn/tool/invocation scalars and strict observed order.
Receive-time counters or filename sorting cannot supply that evidence.

Official hooks documentation says command hooks are synchronous by default,
background hooks can finish out of order, and a failed PreToolUse can permit
the tool to continue. PermissionRequest has no documented invocation ID.
Source: https://learn.chatgpt.com/docs/hooks . These facts constrain the design;
a local queue cannot prove that the harness invoked every callback.

## Proposed wire boundary

Add an additive, separately versioned completion stream inside the existing
private per-runtime hook relay directory. Preserve existing identity/approval
mailboxes and legacy attention paths. Native approval traffic never grants
completion eligibility. Start with macOS/POSIX implementation and no Windows
qualification; unsupported platforms keep current behavior.

Use one bounded queue-state JSON file plus a retained process-shared POSIX
advisory lock file. The queue header contains version, runtime stream identity,
next sender sequence and acknowledged sequence. Pending records carry their
assigned sequence. Producers and the consumer hold the same retained lock across
read/check/modify/atomic replacement; the lock inode is never rotated or deleted
while this runtime is live. A source publication assigns its next sequence and
adds the record in one atomic file replacement, with flush/fsync before release.
Durability requires fsync of the new file and its containing directory after
atomic rename. Marker creation/removal and poison publication likewise require
directory sync. A platform/filesystem lacking the required durable operations
cannot qualify. There is no independently advanced counter file and no
receive-time assignment.
A failed publication poisons assistance rather than retrying an ambiguous append.

The consumer acknowledges only events already processed or conservatively
invalidated by the application. It removes their pending records while retaining
the monotonic header; sender sequence is never reused within a runtime. A lost
acknowledgement cannot replay an already processed effect: the runtime retains
its accepted sequence fence, and existing completion proofs remain one-use.
Acknowledgement/replacement failure disables assistance. The accepted fence lives in the owning sidecar/runtime object and survives
renderer detach/reconnect there. Losing or reconstructing that consumer state
irrevocably invalidates completion assistance for the old stream; it cannot
restart from queued records. Sidecar restart requires a fresh runtime generation
and private directory. Old runtime files and delayed old writers are never
adopted by a replacement runtime; no durable resume/adoption protocol is added.

Each v1 record contains only version, sender sequence, source observed time,
hook fingerprint, root ID, optional turn ID, event, tool name and invocation ID.
Reuse existing bounded scalar validation; no commands, inputs, outputs, tokens,
transcript paths or arbitrary payload keys. PermissionRequest may omit invocation
ID; its association still requires the existing unambiguous serial reducer.
Maximum record size is 4 KiB. Bound pending storage to 256 records / 1 MiB.
The consumer drains acknowledged records, so long-lived sessions do not exhaust
a lifetime event budget. Overflow, counter exhaustion or corrupt header state
permanently disables assistance for this runtime; never evict unacknowledged
records or reset the sequence to regain authority.

Before parsing/publishing a hook, each source wrapper creates its own unique
in-flight marker. Writers never share or remove another writer’s marker. The
consumer cannot grant completion authority while any marker exists. Bound the
marker count to 256; excess markers poison assistance. A writer removes only
its own marker after durable queue replacement. Validation, lock, publication
or acknowledgement failures attempt an independent fixed poison marker; a
retained in-flight marker, unreadable queue, corruption or known failed
publication invalidates completion assistance. No timeout clears attention.
Poison is never removed by a later hook. Source lock operations have a bounded
wait and must not hold the queue lock during sidecar HTTP calls.

The sidecar uses the existing private-directory/no-follow ownership rules and
bounded reads. It validates the queue header and pending records, version and exact contiguous
sequence before forwarding events to the dormant completion application. It
rejects duplicates, rewrites, truncation, gaps, unsupported scalars and stale
runtime/fingerprint/root identity. Journal/in-flight/poison failure cannot be
converted into an empty clean stream. A read made while an uncommitted producer is in flight
cannot grant a clear. Keep ordinary immediate PermissionRequest behavior.

## Qualification boundary and limits

This proves ordering only for records durably published by our source wrapper.
It cannot certify a callback that the harness never started. The exact installed
version/configuration still needs audited and observed synchronous callback,
error/timeout/cancellation and overlap behavior. If missing callbacks or surviving
timed-out writers can escape detection, no qualification row may ship. Do not
weaken this gate to justify the queue implementation.

A future qualified launch must also prove internal subagents are disabled under
its effective configuration, all supported metadata writers are API-only, and
trusted proxy selection reaches the actual child context. The existing helper
option is explicit invocation support; this proposal does not silently adopt
ambient proxies or add automatic launch selection. No source capability,
marker, native startup, current-session upgrade or compatibility row is added.

## Implementation and acceptance plan

1. Save this approved wire contract and a concise plan in an owned worktree.
2. Add failing producer/consumer tests before code: actual subprocess writers,
   parallel writers, reversed directory order, interrupted replacement, malformed
   input, missing/duplicate sequence, writer timeout, retained in-flight state,
   poison, bounded overflow and legacy coexistence. Prove a sender failure
   cannot be misread as clean completion evidence.
3. Add the gated writer and parser using the existing relay lifetime and
   source validation. Do not connect an unqualified production launch to the
   completion application. Tests use the real store/relay boundaries.
4. Run required full repository checks and current-diff medium review; open
   and complete the PR. Then qualify the installed reporter in a human-owned
   diagnostic launch with trusted hooks. No agent launches another coding
   harness or approves native hook trust as a shortcut.

## Approval boundary

This introduces a new persistent hook-message format and cross-process ordering
contract. Root AGENTS.md requires a written protocol/security design and owner
approval before dependent implementation. Approval covers this bounded relay
extension only. Production activation still requires the evidence gates above.

## Builder review disposition

The independent review identified three protocol gaps. They are resolved above:
per-writer bounded markers prevent one writer hiding another; directory/file
sync define durable replacement and marker state; loss of the runtime consumer
fence invalidates the stream, with no restart/adoption of old queued effects.
The reviewer found no other material gaps. These clarifications do not qualify
installed callback completeness or authorize production activation.

## Wire v1 implementation contract

Use a dedicated private child directory of the existing runtime relay. Names:
`queue.json`, retained `queue.lock`, fixed terminal `poison`, per-producer
`inflight-<32 lowercase hex>` and atomic-replacement temporary
`pending-<32 lowercase hex>`. No other files are valid. Directory and regular
files are owned by the current user, private to that user, and not symlinks;
regular files have one link. Open through retained directory descriptors.
The sidecar alone creates a fresh directory/header/lock, with no adoption API.

The header has exactly `version: 1`, `streamId` (32 lowercase hex),
`generation` (u64), `hookFingerprint` (64 lowercase hex), `nextSequence` (u64),
`acknowledgedSequence` (u64), and `records` (array). Initially next is 1 and ack
is 0. Require ack < next and records to cover exactly ack+1 through next-1.
No duplicate or unknown JSON keys, floating integers, or bool-as-integer values.

Each record has exactly `version: 1`, `sequence` (u64), `observedMicros`
(positive i64 source wall time), `hookFingerprint`, `rootId`, `turnId`, `event`,
`toolName`, and `toolUseId`. All optional scalar fields are present as null
when absent. IDs are ASCII alphanumeric/underscore/hyphen, 1..128 bytes;
non-null tool names are printable ASCII without spaces/controls, 1..128 bytes.
Events are UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, Stop.
The reducer remains responsible for event-specific missing-scalar semantics.

The standalone POSIX publisher is `codex-completion-report.py DIRECTORY EVENT
FINGERPRINT`, with one bounded native hook JSON object on stdin (maximum 1 MiB).
Require payload hook_event_name to match EVENT; retain only the allowlisted
scalars, never input/output. It cannot create a stream. Its lock wait is bounded
to two seconds. A failure exits 2 with a fixed sanitized diagnostic, leaves its
own marker and attempts terminal poison. No retry or HTTP/file-metadata fallback.

The Rust queue API creates a fresh stream inside `CodexHookReportRelay`, exposes
its private path to a future qualified caller, and consumes records via a
synchronous application callback. It holds the retained stream lock across
read/validation/callback/ack replacement. Caller lock order is workspace →
sessions → queue → metadata store. Marker creation and removal also hold the queue lock. A producer first locks,
creates/fsyncs its unique marker and directory, unlocks to parse stdin, then
locks again to publish and remove its marker durably. Thus no marker can appear
between the consumer’s in-flight check and its callback/ack transaction.
The publisher never takes application locks
or calls HTTP while holding its lock. A live in-flight marker yields no events
and no clear; a retained marker cannot disappear through consumer cleanup.
No consumer reconstruction is supported. Failure latches assistance off, and
losing the accepted fence requires abandoning that stream.

Implementation remains dormant: no normal reporter invocation, installed-asset
list, source capability, launch marker or completion application caller is added.
A standalone source publisher and real subprocess tests prepare this contract
without feeding unqualified legacy reports into the completion application.
