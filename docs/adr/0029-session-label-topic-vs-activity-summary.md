# Session label is a one-shot Peon-authored topic, decoupled from the turn-by-turn summary

- Status: superseded by [ADR 0042](./0042-workflow-observations-replace-summary-checkpoints.md)
- Deciders: Rambolarsen
- Date: 2026-07-27

## Context

`SessionMetadata.label` / `SessionInfo.label` is the session's display name (there is no separate `title` field). Two write paths compete for it today, and both treat it as "whatever just happened" rather than a stable topic:

- Every completed, descriptive terminal input line overwrites `label` unconditionally (`record_terminal_input_impl`), for the life of the session.
- Every Peon inference cycle that produces a `summary` (the "what's happening right now" field, see ADR 0024) also copies that exact text into `label` (`merge_peon_inference`).

Whichever fires most recently wins. Since keystrokes are far more frequent than Peon's idle-triggered inference cycle, `label` in practice tracks the last thing typed, not what the session is about — and whenever Peon does write it, `label` and `summary` become identical, so there is no field left that represents a stable topic.

A dormant scaffold already exists for a better source: `InferenceMode::InputLabel` in `peon_runtime.rs` runs a one-shot, non-debounced LLM pass over `"[User input]: <hint>"` and treats the result as the label, gated by `label_hint`/`label_pending` on `PeonState`. Nothing in production populates those maps, and the path doesn't persist its result to disk — only a unit test exercises it.

`label` was never part of the `metadata_source`/`metadata_confidence` precedence system in ADR 0005 (no `label_source`/`label_confidence` field exists). This ADR makes explicit that this is intentional, not an oversight.

## Decision

- `label` represents a stable, one-shot topic for the session: what the session is about, not what's currently happening.
- `summary` (and its durable checkpoint history per ADR 0024) remains the sole place for "what's happening right now." `merge_peon_inference` no longer writes `label` as a side effect of writing `summary`.
- `label` is seeded synchronously from the first descriptive user input (typed, or the New Session dialog's initial prompt) as an immediate, cheap fallback — so the title is never blank while an LLM pass runs — and is refined once by Peon's `InputLabel` inference mode shortly after. Once seeded, further keystrokes no longer touch `label`; it is not continuously overwritten.
- `label` remains outside the ADR 0005 precedence system. It is a one-shot-then-frozen value, not a field with ongoing source/confidence contention. There is no rename endpoint in this change; this ADR does not add one.
- The `InputLabel` inference path is made production-live (wired from real call sites instead of only test code) and fixed to persist its result to `SessionMetadata` on disk, not just the in-memory `SessionInfo`.

## Consequences

- Session titles stop churning on every keystroke and stop chasing the latest Peon summary; they read as a topic instead of a live activity feed.
- `summary` and its checkpoint log become the single, unambiguous home for "current task detail," which the frontend can now surface (via the existing `GET /sessions/:id/summary-log` endpoint) without it being conflated with the title.
- A session can still end up with a placeholder (`"Session <id>"`) label if no descriptive input is ever seen and Peon's inference produces nothing usable — this is accepted as an edge case, matching today's behavior for non-descriptive sessions.
- Because `label` is not part of the ADR 0005 precedence system, there is no user-authoritative override yet if the Peon-authored topic is wrong; adding a rename affordance is separate future work.
