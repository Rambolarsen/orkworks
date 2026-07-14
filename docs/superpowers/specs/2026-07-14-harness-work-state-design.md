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

When this capability is present, active hook reports are the only way that
session may enter `working`. Terminal output remains available for persistence,
Peon inference, and metadata, but cannot independently promote the state.

Existing hook reports for non-working states retain their current meaning and
priority.

### Fallback for unsupported harnesses

When the harness has no registered active-work hook capability, submitted
input arms a short-lived fallback transition. Merely editing a prompt does not
arm it. A subsequent output event may promote the session to `working` only
when it contains visible content that is not the echoed submitted input or a
terminal redraw.

The fallback state is consumed after the first qualifying output and expires
without changing attention if no qualifying output arrives. This prevents an
old submission from making unrelated later output appear to be model work.

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
terminal echo / redraw ─────────────> do not promote
qualifying post-submit output ───────> working (fallback harnesses only)
active hook event (working/thinking) ─> working (capable harnesses)
```

## Error Handling

An unavailable, malformed, or unsupported hook is treated as lacking the
active-work capability, so the session uses the fallback path. No new
background process, network request, or persistent migration is required.

## Tests

- Typing and terminal echo leave an idle session idle.
- A supported active-work hook reporting `working` or `thinking` yields
  normalized `working`.
- A capable hook prevents terminal output alone from promoting `working`.
- A harness without active-work hook capability promotes only after a complete
  submission and qualifying post-submit output.
- Non-working hook reports retain their existing behavior.

## Alternatives considered

1. **Hook-only** — most precise, but leaves harnesses without an active-work
   hook permanently idle.
2. **Output-only heuristic** — broad compatibility, but conflates local echo
   and redraws with real model work.
3. **Capability-based hook plus guarded fallback (chosen)** — preserves precise
   hook data where available and gives unsupported harnesses a conservative
   fallback.
