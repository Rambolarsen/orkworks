# Unread notification dot design

## Purpose

Make a completed off-screen turn noticeable without treating an idle session as
unimportant. The unread marker means the user should check in on a result; its
color adds the current urgency of that completed result.

## Scope

This changes only the unread marker in the sessions list. It does not change
the attention model, session lifecycle, unread derivation, sorting, or normal
read-state status indicators.

## Interaction contract

- A live session becomes unread when an off-screen working turn resolves to a
  result, using the existing unread derivation.
- Selecting the session clears its unread marker.
- Unread state remains in memory only and resets when the app restarts.
- Ended sessions do not show an unread marker.
- When shown, the marker reflects the session's latest normalized result tone.
  If an unread idle session becomes blocked before it is viewed, the marker
  changes from blue to amber.
- If an unread result is followed by a new working turn before the user views
  it, hide the unread marker and show the normal working spinner. The dot is a
  completed-turn check-in signal, not a persistent unread latch.

## Color mapping

| Normalized attention tone | Raw statuses covered | Unread dot color |
| --- | --- |
| `idle` | `idle`, `stale` | Blue |
| `needs-you` | `needs_you`, `waiting_for_input` | Blue |
| `blocked` | `blocked`, `checking_capacity`, `capped` | Amber |
| `failed` | `failed` | Red |

The ordinary read-state icon continues to use the existing attention colors;
in particular, idle remains gray there. Blue is therefore the unread-specific
color for idle, not a replacement for the idle status color.

There is no `done` mapping. Completion is represented by lifecycle, not by an
attention state.

## Accessibility

The unread marker retains an accessible label that combines unreadness and the
plain-language completed attention state (for example, `Unread: Idle`). Color
is secondary to that label and the session-row status text/icon.

## Verification

Add or update focused tests covering:

1. an off-screen working turn resolving to idle, needs-you, or a legacy alias
   with the same normalized tone produces a blue unread dot;
2. capped/blocked and failed unread sessions use amber and red respectively;
3. an unread dot updates to the latest state before the session is selected,
   and hides if the session begins a new working turn;
4. each unread dot retains its `Unread: <state>` accessible label;
5. selecting a session clears the unread marker; and
6. ended sessions do not display an unread marker.
