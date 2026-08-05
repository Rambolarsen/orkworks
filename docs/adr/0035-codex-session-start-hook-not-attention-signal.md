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

`.codex/hooks.json` nests every event under a top-level `hooks` object —
`{"hooks": {"SessionStart": [...], ...}}` — confirmed against this repo's own
committed file (APM's `ponytail` skill installs `Stop`/`sessionStart`/
`userPromptSubmitted` hooks there). An initial implementation read and wrote
a root-level `SessionStart` key instead; Codex would have silently ignored
it while OrkWorks reported it as installed. Caught during PR review (both
GitHub Copilot's and ChatGPT's automated reviewers independently flagged it
against the real file) before merge, not after.

That same real file also surfaces an unresolved tension: it's git-tracked,
not gitignored, because APM's shared hooks legitimately need to be
versioned for the whole team. Codex has no separate local-only config file
the way Claude Code's `settings.local.json` is local by convention — so the
`require_local_or_ignored_untracked` safety rule this framework applies to
every JSON-hook target correctly *refuses* to install in a repo shaped like
this one, rather than writing a machine-specific reporter-script path into
a file every teammate shares. That refusal is safe (no corruption), but it
also means the integration is a no-op in exactly the repo it was built and
tested in. Not resolved here — see Consequences.

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
  extracting the `orkworks:harness-integration:` marker value embedded, as
  a whole `--marker '<value>'`/`-Marker '<value>'` flag argument, in the
  hook's single shell-interpreted `command` string, then comparing it
  exactly — the same "different marker → `Ambiguous`, not `Drifted`" safety
  invariant `claude.rs`/`gemini.rs` already enforce, adapted to a
  structural-substring extraction rather than a discrete-field read.
  Requiring the flag structure (not just the marker text anywhere in the
  string) keeps an unrelated command that merely mentions the marker from
  being misidentified as ours. The exactness check also verifies the
  group's outer `matcher` is absent — `merge()` never sets one, so a group
  edited to add one (narrowing which sources fire) reads as `Drifted`, not
  `Installed`.
- Grant Codex the `NativeSessionId` capability in the harness capability
  registry, now that a deterministic (not Peon-fuzzy) capture source exists.
- Codex is the first `JsonHookHandler` whose confirmation copy must not
  claim the generic "OrkWorks reports when this tool waits for input"
  warning — `base_status`'s `executable_code_warning`/coverage-summary
  fields, previously hardcoded `true`/`"Limited harness notifications"` for
  every handler, are now conditioned on `harness_id != "codex"` (same
  marker-string-special-case pattern as the reporter script, same
  issue #271 follow-up). The Settings UI (`HarnessIntegrationSection.tsx`)
  mirrors this with its own `isAttentionSignal` check, and separately
  surfaces `activation === "needs_trust"` as a distinct "installed but not
  yet approved" state instead of the same success copy every other
  integration's `installed` registration gets.

## Consequences

- Codex sessions get exact resume once a user approves the hook inside
  Codex once — same reliability tier as Claude Code, not Peon's 0.50-capped
  guess.
- **Resolved by ADR 0036**: project-level `.codex/hooks.json` being tracked
  rather than local-only is handled by writing a portable, `$HOME`-relative
  reporter command instead of an absolute one, and relaxing the tracked-file
  safety check specifically for that portable-safe case. See
  [ADR 0036](0036-codex-hooks-portable-reporter-path.md).
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
