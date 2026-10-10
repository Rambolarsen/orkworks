---
type: "Architecture Decision"
title: "PTY writes keep the session driver responsive"
description: "Architecture decision record: PTY writes keep the session driver responsive."
tags: ["orkworks", "architecture"]
---

# PTY writes keep the session driver responsive

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-10-03

## Context

Issue #728 reproduces a circular wait on macOS: the session driver blocks in
`write_all`, the PTY reader blocks on the full driver-event queue, and Codex
blocks flushing stdout. The same driver cannot process the session stop signal.
The user approved fixing delivery, stopping, and safe retry in this session.

## Decision

Run at most one PTY write at a time on the blocking pool. The session driver
continues draining output, persisting history, and observing its independent
stop channel while the write runs. Input and resize commands remain ordered;
queued input is rejected after stopping. Every blocking writer checks the shared
stop watch at write admission, even if the async driver has not observed the
notification yet. Admission linearizes at this check; a native call already
admitted may finish after stop. Do not hold the watch read lock during native
I/O. Publish stop with `send_replace` so it survives startup without subscribers. Apply this to initial, startup-buffered,
and live input alike. Keep the existing bounded control queue.

The blocking write owns its delivery acknowledgement until the OS operation
returns. Driver cancellation or child exit signals cancellation but cannot
pretend that a running blocking write was aborted. Check cancellation between
writes while bytes remain. The native writers are unbuffered: once the final
write succeeds, acknowledge delivery even if stop, child exit, or a flush error
follows. Those events cannot prove that the complete prompt was not submitted. A frontend timeout cannot release a
recommendation reservation or authorize duplicate input. The existing delivery
failure rollback releases it only after the writer has reached a final result.
Capture the selected runtime identity and control sender during the approval
reservation. Dispatch through that sender and record live input effects only
while that generation still owns the session. Hold the existing projection gate
across identity validation and synchronous input bookkeeping; resume admission
and rollback take the same gate before replacing a runtime. Release it before
awaiting PTY delivery. A resumed runtime must never
receive an old approval.

Bind finalization to the original workspace path and a weak reference to its
advisory lease. Same-workspace reopens preserve the lease and remain eligible
for finalization, even though their observation store instance changes. A
replacement lease or different workspace rejects stale finalization. The weak
reference must not keep a switched-away workspace locked. Unleased test stores
use their instance identity as a fallback.

No automatic retry, session creation, or recommendation completion is added.

## Consequences

ADR 0022's runtime ownership and ADR 0048's explicit approval remain unchanged.
The driver may finalize child exit while a blocking write is still unwinding;
its acknowledgement remains owned by that write, preventing premature retry.
Portable blocking I/O has no universal cancellation primitive: a platform or
surviving slave holder that never returns from `write` retains an uncertain
handoff reservation. Never trade that limitation for duplicate delivery.
Real Unix PTY tests cover the observed backpressure cycle. On macOS, a stalled
write returns failure after stop, permitting one explicit retry; its real-PTY
regression covers that verified path. Local Linux reproduction processed stop
and ended the child, but the native input write remained blocked after the last
slave closed. Its unresolved delivery must retain the reservation; stop does
not promise immediate retry on Linux. Deterministic writer tests cover partial
cancellation and full-write acknowledgement on every platform. Windows uses the
same driver logic; native ConPTY stall verification remains a platform limit.
