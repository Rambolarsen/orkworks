# Codex `SessionStart` hook captures session ID, not attention

- Status: accepted
- Deciders: Lars-Erik, Claude Sonnet 5
- Date: 2026-08-02

## Context

Codex CLI now documents a stable `hooks.json` schema (`~/.codex/hooks.json` or
`<repo>/.codex/hooks.json`), superseding the earlier evidence-register finding
that no stable schema was retrievable (see
[`docs/agents/harness-integration-contracts.md`](../agents/harness-integration-contracts.md)).
Its `SessionStart` hook fires on `startup`/`resume`/`clear`/`compact` and
carries `session_id` on stdin — the deterministic, high-confidence signal
OrkWorks needs to grant Codex sessions exact resume (`codex resume
{harnessSessionId}`), replacing the only prior source (Peon's fuzzy
terminal-output LLM inference, capped at 0.50 confidence).

The obvious path — reuse the existing `JsonHookHandler` framework (ADR 0026)
and the shared `report-harness-event.sh`/`.ps1` reporter script already
wired for Claude Code, Gemini, Copilot, and Aider — runs into a real
semantic mismatch. Every existing integration hooks a genuine "needs input"
event (Claude/Gemini's `Notification`, Copilot's equivalent, Aider's
`--notifications-command`), so the shared script unconditionally posts a
`waiting_for_input` attention update whenever it's invoked. `SessionStart` is
not that kind of event — it fires at session start, resume, context clear,
and compaction, none of which mean "the agent needs input." Wiring Codex's
hook through the shared script unmodified would mislabel every freshly
launched Codex session as waiting for input.

Codex also requires a one-time `/hooks` approval inside the tool (hash-pinned
trust) before an installed hook definition actually executes — installing
the config file is not the same as it being active.

## Decision

- Add `crates/orkworksd/src/harness/integrations/codex.rs` as a real
  `JsonHookHandler`, installing one `SessionStart` hook group into
  `.codex/hooks.json` (project-level, matching every other JSON-hook
  integration's per-repo ownership model — not the global
  `~/.codex/hooks.json`).
- Set the handler's `activation` to `IntegrationActivation::NeedsTrust`
  (an existing enum value), reflecting that "installed" and "active" are
  different states for Codex specifically.
- Extend the shared reporter script with a `codex_hook` branch that
  captures `session_id` and posts it to `/harness-session`
  (`confidence: 0.98`, matching Claude's `claude_hook` tier), and make the
  generic attention POST conditional on the event actually being an
  attention-style signal — for now, a `session_source != "codex_hook"`
  check derived from the same marker match that identifies the harness.
- Because Codex's hook schema has no dedicated marker/name field (unlike
  Claude's `args` array or Gemini's `name` key), ownership is recognized by
  extracting the `orkworks:harness-integration:` marker value embedded in
  the hook's single shell-interpreted `command` string, then comparing it
  exactly — the same "different marker → `Ambiguous`, not `Drifted`" safety
  invariant `claude.rs`/`gemini.rs` already enforce, adapted to a
  substring-extraction rather than a discrete-field read.
- Grant Codex the `NativeSessionId` capability in the harness capability
  registry, now that a deterministic (not Peon-fuzzy) capture source exists.

## Consequences

- Codex sessions get exact resume once a user approves the hook inside
  Codex once — same reliability tier as Claude Code, not Peon's 0.50-capped
  guess.
- The "is this event an attention signal" distinction now lives as a
  string match on the marker inside two script files, not as a declared
  property on `ToolHookContract` or anywhere in the capability model. This
  is a known simplification, not a design endorsement: the next harness
  that hooks a non-attention event needs another string-matched branch in
  both scripts rather than a one-line contract field. Tracked as
  [issue #271](https://github.com/Rambolarsen/orkworks/issues/271) to
  generalize `ToolHookContract` with an explicit attention-signal flag once
  a second such harness exists — not done speculatively for a hypothetical
  one.
- `codex.rs`'s probe/merge/remove functions duplicate `gemini.rs`'s
  algorithm shape (single-array marker scan with ambiguity detection) for
  the third time (after `claude.rs`'s own variant). Left unextracted for
  now rather than refactoring already-shipped, tested handlers as a side
  effect of adding Codex; tracked as the same follow-up (issue #271).
