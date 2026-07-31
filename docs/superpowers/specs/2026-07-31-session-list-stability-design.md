# Session list stability

## Decision

Keep the existing visible order of active session rows across polling updates.
A session with newer activity or output is promoted to the top only when the
current top active row has been quiet for at least one minute. Dead sessions
remain below active sessions.

## Implementation

The renderer's existing session merge keeps the prior ordering and compares
the incoming session's newest activity timestamp with its previous value.
No order is changed for metadata-only updates. A focused unit test covers the
one-minute promotion threshold and stable updates below it.

## Scope

No setting, persistence, sidecar/API, or manual ordering is added.
