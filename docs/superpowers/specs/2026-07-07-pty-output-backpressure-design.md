# PTY Output Backpressure Design

> **Date:** 2026-07-07
> **Scope:** Bound terminal-runtime memory when PTY output outpaces websocket and persistence consumers

## Goal

Prevent `orkworksd` from growing memory without bound when a session produces terminal output faster than the renderer or metadata persistence can drain it.

## Problem

`start_session_runtime(...)` currently uses two unbounded queues in the PTY output path:

- the blocking PTY reader thread sends raw chunks into the async driver with an unbounded `driver_tx`
- the async driver sends completed terminal lines into the metadata persistence task with an unbounded `persist_tx`

The websocket fanout path is already bounded by `broadcast::channel(256)`, but those two unbounded queues still allow memory growth under output floods such as `yes`, repeated build spam, or a runaway TUI redraw loop. The sidecar keeps accepting bytes even when downstream work is slower.

## Options Considered

### 1. Bounded PTY and persistence channels with real backpressure

Use bounded Tokio channels for both internal queues. When persistence falls behind, the async driver stops pulling more PTY chunks until persistence drains, which in turn stops the blocking PTY reader when the PTY-driver queue fills.

Pros:

- keeps memory bounded at the right boundary
- preserves terminal history fidelity
- uses normal PTY/kernel flow control rather than an app-level drop policy
- keeps `kill_rx` in the driver's `select!`, so kill remains independent of websocket drain

Cons:

- adds some runtime coordination and test coverage

Recommended.

### 2. Bounded PTY queue with drop or coalesce on persistence

Pros:

- smaller implementation

Cons:

- risks losing terminal-history lines during floods
- makes replay/debugging less trustworthy

Rejected.

### 3. Inline synchronous persistence in the driver loop

Pros:

- conceptually simple

Cons:

- couples kill responsiveness to filesystem latency
- makes the output path more fragile than necessary

Rejected.

## Decision

Implement bounded channels for both:

- PTY reader thread -> async runtime driver
- async runtime driver -> metadata persistence task

The driver will only receive a new PTY chunk when it has capacity to enqueue any resulting persisted lines. That creates real backpressure all the way back to the PTY read thread and, ultimately, the child process's PTY buffer.

## Design

### Channel boundaries

- Replace the unbounded `driver_tx` with a bounded channel sized for a small burst of PTY chunks.
- Replace the unbounded `persist_tx` with a bounded channel sized for a small burst of completed line batches.

The exact capacities do not need to be user-configurable in this fix. Small named constants near `start_session_runtime(...)` are sufficient.

### Driver behavior

For a PTY `DriverEvent::Output(data)`:

1. Update in-memory replay, scan, label, peon, and websocket fanout state exactly as today.
2. If the chunk produced completed lines for persistence, enqueue them on the bounded persistence channel.
3. If the persistence queue is full, await capacity before receiving another PTY chunk.

This keeps memory bounded without changing websocket semantics. Slow websocket consumers may still lag and drop broadcast messages per existing `broadcast` behavior, but they no longer cause raw-chunk accumulation inside the sidecar.

### Kill responsiveness

Kill must remain responsive while a session is flooding output.

The driver loop will continue to prioritize `kill_rx` and runtime `control_rx` in the `tokio::select!`. A bounded persistence enqueue may wait for persistence capacity, but it must not permanently block the runtime from observing kill once the current PTY chunk is handled.

### Persistence semantics

This fix preserves:

- existing line splitting behavior
- existing terminal-output append behavior
- existing final tail flush on exit/error

It only changes how much queued work may accumulate in memory at once.

## Testing

Add focused runtime tests that prove:

- internal PTY output buffering is bounded rather than unbounded
- persistence buffering is bounded rather than unbounded
- a kill signal still terminates a flooding runtime promptly enough to complete the test without hanging

The test does not need to assert process RSS directly. It only needs to pin the bounded-flow-control behavior that prevents unbounded accumulation.

## Out Of Scope

- changing websocket replay semantics
- changing terminal history format
- introducing lossy output dropping or coalescing
- making queue capacities user-configurable
