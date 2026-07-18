# Harness-Verified Working State Design

## Goal

Prevent terminal typing and local terminal echo from marking a session as
**Working**. A session becomes Working only when the harness explicitly reports
active model work, or—where that reporting is unavailable—when the fallback
observes output after a completed submission that is not attributable to the
terminal echo.

## Scope

This changes only the transition into the normalized `working` attention state.
It preserves existing handling for `idle`, `needs_you`, `blocked`, `failed`,
and `done`, as well as input-derived labels and last-user-input metadata.

## Design

### Capability-based hook authority

An attention-hook registration declares whether it can report active model
work. Active model signals include the harness's equivalents of `working`,
`thinking`, and other model-processing states. These are normalized to the
existing OrkWorks `working` attention state.

When an active-work-capable hook is registered for the session's harness,
active hook reports are the only way that session may enter `working`.
Terminal output remains available for persistence, Peon inference, and
metadata, but cannot independently promote the state. A registered capable
hook that is unavailable or emits malformed data fails closed: it does not
enable the terminal fallback.

Existing hook reports for non-working states retain their current meaning and
priority.

### Fallback for unsupported harnesses

When the harness has no registered active-work-capable hook, a non-empty input
line terminated by `\r` or `\n` arms a 10-second fallback window using
`tokio::time::Instant`. Merely editing a prompt does not arm it. A later output
event may promote the session to `working` only when it contains a visible
character that remains after stripping ANSI control sequences and consuming
the submitted-line echo.

A **single printable keystroke** received while the session is in `needs_you`
set by an agent hook report also arms the fallback window, using the
in-progress input-line buffer as the echo prefix and re-arming on each
subsequent printable keystroke. This variant exists because Claude Code's
prompts are predominantly single-keystroke (yes/no, choice lists, Esc-to-cancel)
and never produce an Enter-terminated line; without it, the session sticks on
`needs_you` indefinitely after such an answer. The single-key path is gated
on `metadata_source == "agent"` so that Peon-detected `needs_you` on shell-mode
sessions (where the terminal echoes each keystroke) is unaffected — only
hook-sourced `needs_you` arms on a single key. See
`docs/superpowers/specs/2026-07-17-single-key-work-signal-design.md` for the
full design, gates, and edge cases.

Note: the Claude Code attention hook falls under this fallback path. Its hook
is **not** active-work-capable — it only POSTs `waiting_for_input`, never
`working`/`thinking`. Future readers should not reach for the capable-hook
path (above) when reasoning about Claude Code.

The fallback tracks the submitted line's printable characters as an echo
prefix across output chunks. It consumes matching characters, including a
single leading carriage return or newline. ANSI-only output, empty output, and
output that consists entirely of this echo prefix do not promote the state. If
the first visible output differs from the remaining echo prefix, or if visible
characters follow a fully consumed echo prefix, that output qualifies. This
deliberately excludes only control-only redraws and the exact submitted echo;
the PTY protocol cannot reliably identify every visually similar redraw.

The fallback state is consumed after the first qualifying output and expires at
10 seconds without changing attention if no qualifying output arrives. This
prevents an old submission from making unrelated later output appear to be
model work.

### Input and output treatment

The terminal continues collecting submitted input for labels and session
metadata. It does not clear observed attention, schedule Peon classification,
or otherwise promote `working` solely because the user typed.

Output classification keeps all existing output persistence and Peon scheduling
behavior. Its state transition additionally requires either a supported active
hook report or an armed fallback with output that is not local terminal echo.

## Data flow

```text
terminal keystrokes ────────────────> label / last-user-input only
submitted input, no active hook ────> arm fallback
terminal echo / control-only redraw ─> do not promote
qualifying post-submit output ───────> working (fallback harnesses only)
active hook event (working/thinking) ─> working (capable harnesses)
```

## Error Handling

An unsupported or unregistered hook uses the fallback path. A registered hook
that advertises active-work capability but is unavailable or malformed is
treated as unavailable, leaving the session's current attention unchanged; it
does not fall back to PTY heuristics. No new background process, network
request, or persistent migration is required.

## Tests

- Typing and terminal echo leave an idle session idle.
- A supported active-work hook reporting `working` or `thinking` yields
  normalized `working`.
- A capable hook prevents terminal output alone from promoting `working`.
- A harness without active-work hook capability promotes only after a complete
  submission and qualifying post-submit output within 10 seconds.
- ANSI-only redraws, full submitted-input echoes, and split input echoes leave
  an idle session idle.
- A registered capable hook that is unavailable or malformed does not enable
  the fallback path.
- Non-working hook reports retain their existing behavior.

## Alternatives considered

1. **Hook-only** — most precise, but leaves harnesses without an active-work
   hook permanently idle.
2. **Output-only heuristic** — broad compatibility, but conflates local echo
   and redraws with real model work.
3. **Capability-based hook plus guarded fallback (chosen)** — preserves precise
   hook data where available and gives unsupported harnesses a conservative
   fallback.
