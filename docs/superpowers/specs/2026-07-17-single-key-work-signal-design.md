# Single-Key Work Signal Design

## Goal

Restore the `needs_you → working` attention transition for Claude Code sessions
where the user answers the prompt with a **single keystroke** (e.g. `y`/`n`,
`1`/`2`/`3`, or other choice keys) rather than an Enter-terminated line. Today the
session sticks on `needs_you` indefinitely after such an answer, because the
only path that clears it back to `working` requires a submitted input line
terminated by `\r` or `\n`, and Claude Code's prompts don't take Enter.

This is a targeted fix for issue #179, a regression introduced by #177.

## Scope

In scope: a new arming site for `pending_work_signal` that fires on a single
printable keystroke, gated so it only applies when the current `needs_you` was
set by an agent hook report (i.e. the Claude Code Notification hook path).

Out of scope:

- Wiring a true `working`/`thinking` signal from Claude Code's Notification
  hook. That is #71's broader per-harness attention port design and stays out
  of this fix.
- Recovering Peon-detected `needs_you` on TUI harnesses (e.g. Aider's TUI
  showing a yes/no prompt that Peon scrapes). Those sessions have
  `metadata_source == "peon"` and the narrow-scope gate intentionally excludes
  them. They keep sticking on `needs_you` until either the user hits Enter or
  the session ends. Filing a follow-up is appropriate but not part of #179.
- Acting on the cleared attention state. Observers stay read-only.
- Any frontend change. The `working`/`needs_you` labels already exist
  (`apps/desktop/src/labels.ts`); the existing 2-second `/sessions` poll
  surfaces the transition.

## Context

The Claude Code attention hook
(`crates/orkworksd/scripts/report-claude-session-from-hook.sh`) only ever POSTs
`{"status":"waiting_for_input"}` to `/sessions/<id>/attention`. There is no
`working`/`thinking` signal from this hook — Claude Code's Notification event
fires when the agent is *waiting*, not when it *starts* working. The hook POST
is consumed by `report_attention` (`http/session_handlers.rs:434`), which writes
`attention = "needs_you"` and `metadata_source = "agent"` via
`merge_agent_attention_signal` (`metadata.rs:992`).

The only paths that can promote a Claude Code session back to `working` after
#177 ("gate Working state on harness activity", commit `a886114`) are:

1. **Capable-hook path** — requires `active_work_hook = true` on the harness
   config. Claude Code uses `HarnessAttentionCapabilities::default()`, whose
   `active_work_hook` defaults to `false`, so this path is closed.
2. **Fallback path** — armed only on an Enter-terminated submitted input line.
   See `terminal_runtime.rs:261` (`let line = collected_line?;`) where
   `collect_input_line` only returns `Some(line)` on `\r`/`\n`, and
   `terminal_runtime.rs:292` which arms `pending_work_signal` only inside that
   branch. `should_infer_working` (`session_runtime.rs:91`) then requires
   `has_qualifying_work_signal` plus qualifying post-submit output to set
   `attention = "working"`.

Claude Code's prompts are predominantly single-keystroke. No Enter means
`collect_input_line` returns `None`, `record_terminal_input` early-returns at
the `?` on line 263, `pending_work_signal` is never armed, and subsequent
model output cannot promote to `working`. The session sticks on `needs_you`
until the next prompt arrives or the session ends.

The fix lives inside the existing fallback path — it does not change the
capable-hook path or the `should_infer_working` gate. It only widens the set
of inputs that arm the work signal.

## Design

### Mechanism

Add a single-key arming path inside `record_terminal_input`
(`crates/orkworksd/src/runtime/terminal_runtime.rs:249`), parallel to the
existing Enter-terminated arming at `:293`. When a printable keystroke arrives,
capture the in-progress input-line buffer (the same `state.peon.input_buf`
that `collect_input_line` mutates) and arm `pending_work_signal` with it as the
echo prefix. Re-arm on each subsequent printable keystroke so the prefix tracks
the typed-so-far line. The existing Enter path stays unchanged — Enter still
re-arms with the final committed line.

Refactor the input-buf lock to return both the collected line and the
in-progress buffer snapshot:

```rust
let (collected_line, in_progress_buf) = {
    let mut bufs = state.peon.input_buf.write().unwrap();
    let buf = bufs.entry(id.to_string()).or_default();
    let line = collect_input_line(buf, data);
    (line, buf.clone())
};
```

Then the new arming block, run **before** the `let line = collected_line?;`
early-return so single-key strokes without Enter reach it:

```rust
let has_printable = data.chars().any(|c| !c.is_whitespace() && !c.is_control());
if has_printable && !in_progress_buf.is_empty() {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(handle) = sessions.get_mut(id) {
        if !handle.active_work_hook
            && handle.info.attention.as_deref() == Some("needs_you")
            && handle.info.metadata_source.as_deref() == Some("agent")
        {
            handle.pending_work_signal = Some(arm_pending_work_signal(
                &in_progress_buf,
                tokio::time::Instant::now(),
            ));
        }
    }
}
```

The `has_printable` predicate duplicates the private `has_visible_character`
helper in `session_runtime.rs:43`. The helper is private to `session_runtime`,
so the inline is structurally necessary here; if a future refactor promotes
the helper to a shared scope, both sites should call it.

### Gates

The arming fires only when all of the following hold:

- **`has_printable && !in_progress_buf.is_empty()`** — pure control sequences
  (arrow keys, Esc, Ctrl-C) and empty data don't arm. `in_progress_buf` is
  empty only after Enter (cleared by `collect_input_line`), so the single-key
  path can't fire on the same Enter that the existing path will handle — no
  double-arm.
- **`!handle.active_work_hook`** — for capable-hook harnesses, only the hook
  drives `working` per the parent design (lines 26–32 of
  `2026-07-14-harness-work-state-design.md`). Prevents this fix from
  accidentally re-enabling the terminal fallback on a future harness that does
  declare active-work capability.
- **`attention == "needs_you"`** — only arm while the session is actually
  waiting. During `working`, keystrokes don't touch the signal slot (no noise
  on the working state — acceptance criterion 3). During
  `idle`/`blocked`/`failed`/`capped`, also no arming — those states have
  their own transition rules.
- **`metadata_source == "agent"`** — the narrow-scope gate. `report_attention`
  sets `metadata_source = "agent"` when it writes `needs_you` from a hook POST
  (`session_handlers.rs:492`). Peon-driven `needs_you` has
  `metadata_source = "peon"`, process-driven has `"process"` — neither arms.
  Sidesteps the shell-mode echo false-positive entirely, since shell sessions
  don't fire hook reports.

After the user's keystroke arms the signal but before any output promotes, the
gate's conditions still hold: `attention` is still `needs_you`,
`metadata_source` is still `agent` (keystrokes don't change either). So the
next keystroke re-arms cleanly. Once output promotes, in-memory `attention`
becomes `working` and the persisted session metadata source becomes `process`,
so the gate stops firing.

### Echo-gating

`pending_work_signal` is consumed by `consume_pending_work_signal`
(`session_runtime.rs:52`). The signal's `remaining_echo` is matched against
stripped output; only output with a visible character beyond the echo prefix
qualifies. No change to that logic — it is already echo-aware.

Two properties hold for the single-key prefix:

- **TUI harness (Claude Code):** terminal echo is off. The keystroke the user
  types does not appear as PTY output — the harness reads it from stdin and
  redraws its own UI via ANSI sequences. `peon::strip_ansi` on those redraws
  returns empty/whitespace, `has_visible_character` returns false, the signal
  stays armed. When the model starts producing visible output, it doesn't match
  the typed prefix, and `consume_pending_work_signal` returns true → promotion
  to `working`.
- **Prefix growth:** re-arming on each printable keystroke keeps
  `remaining_echo` matched to the current typed-so-far line. For single-key
  prompts (the primary #179 scope) re-arm is a no-op — there's only one
  keystroke. It earns its keep on Claude Code's *multi-char freeform* prompts
  (clarification questions, freeform text replies without a fixed choice list),
  where a longer reply typed over more than 10 seconds would otherwise see the
  first keystroke's window expire before the model produces output. Re-arm
  refreshes that window on each keystroke. Once the user stops typing and the
  model starts generating, the prefix is the full typed line. If the model's
  first visible line happens to start with the same characters as the user's
  input (rare for prompts like `y`/`1`),
  `consume_pending_work_signal` strips that prefix before checking for visible
  characters beyond it — so the matching prefix is consumed and whatever
  follows still qualifies.

The existing 10-second expiry is unchanged. If the user types one key and the
model never produces output (the user abandoned the session at a prompt), the
signal expires and the session stays at `needs_you` — correct, because nothing
has happened.

One degenerate case worth tracing explicitly: if the model's first visible
output exactly equals the echo prefix (e.g. prefix `"y"`, output `"y"` with
nothing else), `consume_pending_work_signal` returns `false` because the
post-strip output is empty (no visible character beyond the prefix). However,
`signal.remaining_echo.clear()` at `session_runtime.rs:82` still runs, so the
signal stays armed with an empty prefix. The next visible output chunk
qualifies normally (`has_visible_character` is true, no echo prefix to strip).
If no further output arrives, the signal decays on its 10s timer without
producing a false positive. The fix does not need to handle this case
specially — recovery is automatic.

## Data flow

```text
hook POST waiting_for_input ─────> attention=needs_you, metadata_source=agent
single printable keystroke ───────> arm work signal (echo prefix = in_progress_buf)
subsequent printable keystrokes ───> re-arm with grown prefix
Enter (terminated submission) ────> existing path re-arms with final line
ANSI redraw between keystrokes ────> stripped to empty, signal stays armed
model's first visible output ──────> consume_pending_work_signal → working
                                       in-memory attention=working;
                                       persisted metadata_source=process
```

## Edge cases

- **Keystroke before any hook report fired:** the gate's
  `attention == "needs_you" && metadata_source == "agent"` check fails, nothing
  arms. The previous behavior (no work signal) is preserved — correct, since
  without a hook report there's no known `needs_you` to recover from.
- **Peon-detected `needs_you` on a TUI harness:** `metadata_source == "peon"`,
  gate fails, no arming. Same as today. This is the documented limitation of
  the narrow-scope choice — captured here as out-of-scope for #179, deferred to
  a broader fix later (could be #71's territory).
- **User answers with Enter for once (e.g. pastes a longer reply):** the
  existing Enter-terminated path fires normally at `:293`. The new single-key
  path doesn't double-arm because `collect_input_line` clears the buffer on
  Enter, so `in_progress_buf` is empty when the new path's
  `!in_progress_buf.is_empty()` check runs (acceptance criterion 2 — multi-char
  submissions continue to work).
- **Two consecutive hook reports with no keystroke between them:** the user
  wasn't actually the actor (the hook fired twice, perhaps a retry), so no
  arming. The session stays at `needs_you`. Correct — only human input is
  evidence of resumption, per the parent design's line 38 ("non-empty input
  line").
- **Keystroke that gets dropped by `PendingActionQueue` for exceeding the byte
  cap:** `record_terminal_input` is only called for accepted input (per its
  docstring at `:241`), so dropped keystrokes never reach the arming block. No
  spurious arming.
- **User types into a non-Claude hookless harness (OpenCode env-session-reporter
  style):** that harness's hook never POSTs attention at all (its reporter
  script only sends a `harness-session` report), so it never sets
  `metadata_source == "agent"` on a `needs_you` — the gate doesn't fire. No
  change for OpenCode. Codex similar. Only Claude Code's hook currently produces
  the exact combo that lights up the new path.
- **Capable-hook harness that POSTs `waiting_for_input` (hypothetical future):**
  `active_work_hook == true`, gate fails. Hookless-fallback path is honored as
  the parent spec requires.
- **Backspace mid-typing (`\x7f`):** `collect_input_line` calls `buf.pop()` at
  `terminal_runtime.rs:187`, shrinking the in-progress line. Backspace is a
  control char so the new arming block's `has_printable` check is false — no
  re-arm on the backspace itself. The armed `pending_work_signal.remaining_echo`
  therefore holds the pre-backspace prefix until the next printable keystroke
  re-arms with the shorter prefix. The stale-prefix window is bounded by the
  10-second expiry and only matters if model output arrives during it (which
  requires the model to start generating while the user is mid-editing at a
  `needs_you` prompt — not a real Claude Code scenario, since the model waits
  for input at that point). Acceptable; recovery is automatic on the next
  printable keystroke or expiry.
- **Ctrl-C / Ctrl-D (`\x03`/`\x04`):** `collect_input_line` calls `buf.clear()`
  at `terminal_runtime.rs:188`, emptying the in-progress line. The new arming
  block's `!in_progress_buf.is_empty()` check is now false, so no new arm — but
  an *already-armed* signal is not disarmed by this path. The 10-second window
  stays live; any visible PTY output during those 10s (e.g. shell cancellation
  echo) could spuriously promote to `working`. This is pre-existing behavior
  for the Enter-terminated arming path as well (it doesn't disarm on Ctrl-C
  either), so the new path maintains parity rather than introducing a new
  defect. A future hardening pass could disarm on `\x03`/`\x04` for both paths;
  out of scope for #179.
- **`is_sensitive` (password prompt) ordering:** the existing
  `record_terminal_input` runs the `is_sensitive` check (which suppresses label
  and `last_user_input` metadata writes) at `terminal_runtime.rs:266–286`,
  **after** the new arming block. If the gate's
  `metadata_source == "agent"` condition holds while the user is at a password
  prompt, the password's first char would become `pending_work_signal.remaining_echo`
  in memory (the signal is in-memory only; not persisted to disk). In practice
  this is unreachable for the targeted case: shell password prompts (sudo, ssh)
  happen inside a subprocess whose parent session is in `working` state with
  `metadata_source == "process"`, not `needs_you` with
  `metadata_source == "agent"`. The gate's `metadata_source == "agent"` check
  is therefore sufficient to exclude password prompts. No additional
  `is_sensitive` check is needed in the new block; documenting for completeness.
- **Second hook report arrives while the signal is armed:** `report_attention`
  does not touch `pending_work_signal` (`session_handlers.rs:471–494`), so an
  already-armed signal persists across a fresh `needs_you` re-write from the
  hook. Behavior is benign: the signal still fires on the next visible output
  chunk, which would promote to `working` (correct if the user has now
  answered). If the user hasn't answered (a stale/retry hook report), the
  10-second window bounds the armed signal and it expires. No special handling
  needed.
- **Terminal re-attach / detach:** `record_terminal_input` is only called from
  `record_peon_input_side_effects` for *accepted* WebSocket input (see the
  dispatch sites in the WS read loop). While detached, no keystrokes arrive, so
  an armed signal decays on its 10-second timer unaffected. On re-attach, the
  replay path does not re-arm the signal; only new keystrokes do. The
  detach→10s-expire-while-detached→re-attach sequence is therefore
  indistinguishable from "user didn't answer for 10s" — correct behavior.
- **Session with `metadata_source == None`:** a newly-created session has
  `info.metadata_source: None` until its first metadata write. The gate's
  `Some("agent")` check correctly excludes `None`, so no arming during the
  create-before-output window. The doc previously listed only `peon` and
  `process` as the non-arming non-agent sources; `None` is also non-arming.

## Tests

Rust tests, in `crates/orkworksd/src/runtime/terminal_runtime.rs` test module
or a sibling. Map 1:1 to issue #179's acceptance criteria:

1. **Single-key acceptance promotes to working (criterion 1).** Set up a
   session with `active_work_hook=false`, `attention="needs_you"`,
   `metadata_source="agent"` (simulating a hook report). Call
   `record_terminal_input(state, id, "y")` — no Enter. Drive an output chunk
   through the runtime's output path containing visible content. Assert
   `handle.info.attention == Some("working")` and
   `handle.info.metadata_source == Some("process")`. This test fails before
   the fix.
2. **Multi-char + Enter path still works (criterion 2).** Same setup. Call
   `record_terminal_input(state, id, "fix")` then
   `record_terminal_input(state, id, "\r")`. Drive output. Assert `working`.
   Behavior should match today's — confirms no regression on the
   Enter-terminated fallback.
3. **No noise on working (criterion 3).** Session with
   `attention="working"`, `metadata_source="process"`. Call
   `record_terminal_input(state, id, "y")`. Assert the `pending_work_signal`
   slot is unchanged (still `None` or unchanged from a pre-set baseline). No
   re-arm during working.
4. **Peon-sourced needs_you doesn't arm (narrow scope).** Session with
   `attention="needs_you"`, `metadata_source="peon"`. Call
   `record_terminal_input(state, id, "y")`. Assert `pending_work_signal`
   stays `None`.
5. **Capable-hook session doesn't arm via single key.**
   `active_work_hook=true`, `attention="needs_you"`,
   `metadata_source="agent"`. Call with `"y"`. Assert `pending_work_signal`
   stays `None`.

The tests live alongside the existing
`record_terminal_input`/`collect_input_line` tests in `terminal_runtime.rs`.
They use the same `test_app_state_with_workspace` test scaffold already used
by the surrounding tests.

Tests #1 and #2 require driving output through the runtime's output path
(`start_session_runtime` with a `sleep; printf` PTY command, mirroring the
existing `output_within_startup_grace_is_replayed_without_marking_attention_working`
test pattern in `session_runtime.rs`), so they're `#[tokio::test]` with
timeouts. Tests #3, #4, #5 only assert that `pending_work_signal` *doesn't*
arm after a `record_terminal_input` call, so they're synchronous unit tests
constructing a `SessionHandle` directly via the existing
`test_state_with_runtime_session` helper (or equivalent). The asymmetry
between the live-runtime tests and the sync arming-no-op tests is intentional
— #1/#2 pin end-to-end behavior (arming → promotion), #3/#4/#5 pin just the
gate.

## Spec update

The parent design doc `docs/superpowers/specs/2026-07-14-harness-work-state-design.md`
line 38 reads:

> When the harness has no registered active-work-capable hook, a non-empty
> input line terminated by `\r` or `\n` arms a 10-second fallback window

That wording is what #177 implemented and is too narrow for Claude Code's
single-key prompts. The parent doc is updated in the same PR to soften that
wording to "a non-empty input line terminated by `\r` or `\n`, **or** a
single printable keystroke received while the session is in `needs_you` set by
an agent hook report, arms a 10-second fallback window" — and to point at this
doc for the single-key variant. The parent doc also gains an explicit note
that the Claude Code hook falls under the fallback path (its hook is not
active-work-capable), so future readers don't reach for the capable-hook path
when reasoning about Claude Code.

## Error handling

No new background process, network request, or persistent migration is
introduced. The new arming site uses the same locks (`state.peon.input_buf`,
`state.sessions`) already taken by `record_terminal_input`, taken in the same
order. The sessions-lock acquisition is conditional on a printable keystroke
being present and the in-progress buffer being non-empty, so pure-control
input (arrows, Esc) takes no extra lock beyond the existing `input_buf` write
lock. A session whose handle has been removed (the session ended between the
`input_buf` lock release and the sessions-lock acquisition) is handled by the
existing `if let Some(handle) = sessions.get_mut(id)` guard — silently no-ops,
same as the existing Enter-terminated arming.

## Alternatives considered

1. **Arm at the hook turn boundary.** When `report_attention` receives a
   `needs_you` for a session already in `needs_you`, treat the in-between user
   response as a submission and arm the work signal there. **Rejected:** the
   arm fires at the *next* prompt's arrival, which is too late — the working
   interval already happened between the user's answer and the new prompt, so
   the user would still see `needs_you` during the model's actual generation.
2. **Broad scope (any `needs_you`, regardless of `metadata_source`).** Apply
   the single-key arming whenever `active_work_hook=false` and
   `attention == "needs_you"`, regardless of whether the `needs_you` was set by
   the hook or by Peon. **Rejected for #179:** simpler, but carries a documented
   false-positive on shell-mode sessions where the terminal echoes each
   keystroke (the echo wouldn't match the growing prefix and would qualify as
   visible model output). The narrow `metadata_source == "agent"` gate
   sidesteps this entirely since shell sessions don't fire hook reports. Broad
   scope remains a viable follow-up if Peon-detected `needs_you` on TUI
   harnesses becomes a real complaint.
3. **Re-enable PTY-output-driven promotion for the hookless path.** Re-introduce
   a constrained version of the pre-#177 `is_terminal_observed_status` gate,
   letting PTY output alone promote to `working` after `needs_you`. **Rejected:**
   undoes the intent of #177 (preventing terminal echo and redraws from
   falsely promoting to working) and reopens the class of false positives #177
   closed. The single-key arming is more surgical — it still requires evidence
   of human input before promotion.
4. **Wire Claude Code's hook to also send `working`.** Have the hook script
   POST `working` on some Claude Code event. **Rejected for #179:** out of
   scope — Claude Code's Notification event only fires on waiting-for-input,
   not on work start. A true `working` signal requires a different hook event
   source or a different observation mechanism, which is #71's design space.
5. **Arm with an empty echo prefix.** Since Claude Code's TUI runs in raw mode
   with terminal echo off (per the Echo-gating section), the user's typed
   keystroke never appears as PTY output — the echo prefix exists only as
   defensive trimming against the rare case where the model's first visible
   output starts with the same character as the user's input. An empty-prefix
   arm (`remaining_echo = ""`) would qualify on the first visible model output
   regardless of prefix, simplifying the design and avoiding the degenerate
   "output exactly equals prefix" sub-case traced in Echo-gating. **Rejected:**
   the prefix is cheap defensive coverage and the degenerate sub-case recovers
   automatically on the next chunk (per Echo-gating). More importantly, the
   empty-prefix variant would qualify on ANY visible output arriving during the
   10-second window — including stray redraw artifacts that survive
   `peon::strip_ansi` (e.g. a printable box-drawing character from a TUI
   redraw). The typed-prefix variant only qualifies on output that has visible
   content *beyond* what the user typed, which is a tighter (if still
   conservative) signal of genuine model output. Keeping the prefix.

## Parent design reference

This design modifies the fallback path described in
`docs/superpowers/specs/2026-07-14-harness-work-state-design.md` (lines 38–57)
without changing the capable-hook path (lines 26–32) or the input/output
treatment section (lines 60–67). The parent doc is updated to point at this
one for the single-key variant.
