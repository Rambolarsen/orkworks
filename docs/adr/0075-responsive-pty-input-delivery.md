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
queued input is rejected after stopping. Apply this to initial, startup-buffered,
and live input alike. Keep the existing bounded control queue.

The blocking write owns its delivery acknowledgement until the OS operation
returns. Driver cancellation or child exit signals cancellation but cannot
pretend that a running blocking write was aborted. Check cancellation between
writes and before acknowledgement. A frontend timeout cannot release a
recommendation reservation or authorize duplicate input. The existing delivery
failure rollback releases it only after the writer has reached a final result.
No automatic retry, session creation, or recommendation completion is added.

## Consequences

ADR 0022's runtime ownership and ADR 0048's explicit approval remain unchanged.
The driver may finalize child exit while a blocking write is still unwinding;
its acknowledgement remains owned by that write, preventing premature retry.
Portable blocking I/O has no universal cancellation primitive: a platform or
surviving slave holder that never returns from `write` retains an uncertain
handoff reservation. Never trade that limitation for duplicate delivery.
Real Unix PTY tests cover the observed backpressure cycle and stalled delivery
followed by stop, failure recovery, and one explicit retry. Windows uses the
same driver logic; native ConPTY stall verification remains a platform limit.
