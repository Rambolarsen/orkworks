# Session Activity and Unread Result Redesign

Date: 2026-07-14
Status: approved
Issue: [#62](https://github.com/Rambolarsen/orkworks/issues/62)

## Goal

Make unread state mean that an inactive live session finished a working turn and
has a result the user has not seen. The session row should use one fixed status
slot: a spinner while work is in progress, a result-colored unread dot for an
unseen result, and the normal status icon after the row is selected.

## Transition semantics

Unread is renderer-local and remains separate from attention. Attention says
what state the session is in; unread says whether the user has seen the result
of its latest working turn.

An unread latch is created only when all of these conditions hold:

- the session is not active
- the session lifecycle is `alive`
- its previous normalized attention was `working`
- its current normalized attention is one of `idle`, `needs_you`, `blocked`,
  `failed`, or `capped`

The first poll and a session first appearing on a later poll establish baseline
state without creating unread. Changes between two non-working states do not
create unread. Raw activity timestamps do not participate in the transition.
Dead or remembered sessions do not create or display unread.

Once set, unread persists in memory across polling while the session remains
live. Selecting the row clears it immediately. An unchanged poll returns the
same unread-state object so React can bail out of a redundant render.

If an unread session unexpectedly returns to `working`, retain its unread latch
defensively but show the working spinner. Observer-only metadata updates should
not cause that transition; a separate stabilization issue tracks that invariant
without blocking genuine user-driven session resumption.

## Session-row signal slot

The separate unread gutter is removed. `StatusIndicator` owns the one 14px
signal slot and accepts an internal `variant?: "status" | "unread"` property.

- `working` and transitional lifecycle states render the existing spinner,
  regardless of unread state
- an unread non-working session renders a 7px circular dot with the accessible
  label `Unread: <status>`
- a read non-working session renders the existing status icon
- the unread dot uses the current attention tone: idle gray, needs-you blue,
  blocked/capped orange, and failed red

Unread rows retain their background tint. Unread no longer makes the session
label bold; actionable status text on the right remains unchanged. The active
session detail panel keeps its existing `StatusIndicator` rendering and does
not opt into the unread variant.

## Boundaries

- No backend, HTTP, metadata protocol, persistence, or public API changes.
- `trackUnread()` keeps its current signature.
- Unread resets when the desktop app restarts.
- Session switching remains the context-switch primitive; this change adds
  signal to the list and does not add parallel terminal rendering.

## Verification

- Unit coverage pins every canonical `working` result plus first/new/dead,
  active, persistence, clearing, non-working transitions, raw activity, and
  unchanged-poll identity behavior.
- Session-row contract coverage pins spinner/dot/icon precedence, one signal
  slot, accessible tone-colored unread dots, unread tint without bolding, and
  unchanged detail-panel rendering.
- Run the full desktop tests, TypeScript check, production build, documentation
  currency check, and lightweight code review before handoff.
