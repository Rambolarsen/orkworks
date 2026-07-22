# Input-generation staleness guard

## Context

Accepted terminal input clears a session's prior attention state and marks it
`working`. That transition races with two writers that can be based on terminal
state observed before the input:

- a harness attention report can persist before the input transition, then be
  overwritten by its later persistence;
- an in-flight Peon inference can reapply an obsolete question or
  `waiting_for_input` state after the input transition lowers the metadata
  source to `process`.

The first race can also leave the persisted session metadata different from the
in-memory projection. Issue #193 requires both writers to distinguish a newer
observation from an observation made before accepted input.

## Decision

Maintain an in-memory, per-session input generation plus the accepted-input
timestamp. Increment and timestamp it only after a terminal input write and
flush succeeds.

The installed Claude reporter sends `observedAt`, an optional RFC 3339 UTC
timestamp with exactly microsecond precision, captured at the start of hook
invocation before it performs either the harness-session or attention POST. It
uses the script's existing Python 3 dependency to emit this format portably.
The sidecar parses a present value strictly and returns 400
for malformed or non-UTC values. It rejects a report whose time is less than
or equal to the latest accepted-input timestamp: if both events fall in the
same microsecond, user input wins conservatively. Reports without `observedAt`
retain existing arrival-order behavior for compatibility with custom reporters;
only the managed reporter receives the stronger guarantee.

Peon stores each retained terminal line with a monotonically increasing Peon
output revision. Accepted input records the current revision as a minimum
boundary. A state-bearing Peon inference receives only lines with a revision
greater than that boundary and may apply only when its input generation still
matches. This prevents a newly started inference from treating the retained
pre-input ring buffer as fresh evidence. Scheduler candidates carry an explicit
`Output` or `InputLabel` mode. `InputLabel` inference uses the descriptive user
input only and commits only the live label; it never calls the metadata merge,
persists provider context, or changes status, attention, questions, options,
metadata source, or inference bookkeeping.

The input, hook, and Peon merge paths use the existing `workspace` then
`sessions` lock order when they simultaneously hold guarded-commit locks.
Within that critical section, the input path increments the generation, records
the timestamp, writes metadata, then updates the in-memory projection. Hook
and Peon paths validate their stale boundary, write metadata, then update the
projection before releasing the locks. This order applies only to these newly
guarded critical sections; unrelated multi-step paths are not part of this
change.

## Data flow

1. Terminal input is delivered to the PTY and acknowledged after write plus
   flush.
2. While holding the guarded commit locks, the input path advances the
   generation, records the acceptance timestamp, persists cleared `working`
   metadata, and updates the in-memory projection.
3. The managed hook captures and sends `observedAt` at hook start, before any
   network I/O. The hook path
   rejects a report at or before the accepted-input timestamp before it alters
   the Peon input buffer or session metadata; an absent timestamp preserves
   arrival-order compatibility and a malformed present timestamp returns 400.
4. Peon captures generation and output revision with its terminal snapshot.
   Immediately before merge, it checks both the generation and the post-input
   output boundary while holding the guarded commit locks; a failed check drops
   the state-bearing inference before metadata, provider context, labels, or
   scheduling state are changed.
5. A non-stale hook or Peon update then follows the existing metadata priority
   rules, persists, and updates memory in the same critical section.

The generation is an in-memory concurrency guard, not durable protocol state.
The accepted-input timestamp is retained in the session handle for the current
process lifetime. A restart has no in-flight Peon operation; delayed external
hook reports fall back to arrival order until the next accepted input records a
new timestamp.

## Alternatives

- Hold locks while hooks or Peon inference run. This would serialize slow or
  external work and increase contention.
- Compare timestamps. Clock precision and event ordering leave the exact race
  ambiguous.
- Use an input generation. This explicitly represents the invalidation
  boundary, is deterministic, and supports direct regression tests.

## Error handling

If terminal write or flush fails, input is not acknowledged, the generation and
timestamp are not changed, and no status transition occurs. If metadata
persistence fails, no in-memory-only state is published. A rejected stale hook
does not clear the Peon input buffer. A rejected stale Peon inference does not
persist provider context, update labels, alter inference bookkeeping, or change
retry scheduling.

The input transition clears `needs_user_input`, `detected_question`, and
`suggested_options` in both the durable metadata and the live session
projection, alongside observed status and attention.

## Tests

Regression tests will simulate both interleavings:

- a managed hook observed before accepted input arriving afterward and being
  rejected by `observedAt`, including the conservative same-microsecond tie;
- a complete hook update followed by accepted input, with input overwriting
  both disk and memory consistently;
- malformed and absent `observedAt` values, proving strict managed timestamp
  validation and compatible custom-reporter arrival ordering respectively;
- a Peon inference based on generation N completing after input advances the
  session to N+1;
- a new Peon candidate that sees only retained pre-input output at generation
  N+1 and is not eligible to restore an old prompt;
- input-label inference after input, proving that it can update only the label
  and cannot restore attention-bearing fields;
- terminal write/flush failure and metadata-write failure, each proving no
  generation/timestamp advance and no projection update;
- an observation begun after accepted input, proving it can still apply.

Each test verifies that the stale update is rejected and that persisted metadata
and the in-memory projection remain `working` with the old attention/question
data cleared. The idle-timer path is out of scope because it only marks an
otherwise working session idle after the configured post-output timeout; it
does not reapply prompt/question state. Its timer deadline is reset by new PTY
output as today.

## Scope

The change is limited to session runtime, terminal input transition, the
managed Claude reporter and attention request, attention report merging, Peon
merge gating, and their tests. It does not add a metadata-file field or alter
metadata priority ordering for non-stale observations. The attention request's
new timestamp field is optional for reporter compatibility.
