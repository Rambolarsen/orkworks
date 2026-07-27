# Unread notification dot design

## Purpose

Make a completed off-screen turn noticeable without treating an idle session as
unimportant. The unread marker means the user has not seen a relevant result;
its color adds the current urgency of that result.

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
- The marker always reflects the session's latest attention state. If an
  unread idle session becomes blocked before it is viewed, the marker changes
  from blue to amber.

## Color mapping

| Current attention state | Unread dot color |
| --- | --- |
| `idle` | Blue |
| `needs_you` | Blue |
| `blocked` | Amber |
| `failed` | Red |

The ordinary read-state icon continues to use the existing attention colors;
in particular, idle remains gray there. Blue is therefore specific to ordinary
unread activity, not a replacement for the idle status color.

There is no `done` mapping. Completion is represented by lifecycle, not by an
attention state.

## Accessibility

The unread marker retains an accessible label that combines unreadness and the
plain-language current attention state (for example, `Unread: Idle`). Color is
secondary to that label and the session-row status text/icon.

## Verification

Add or update focused tests covering:

1. an off-screen working turn resolving to idle or needs-you produces a blue
   unread dot;
2. blocked and failed unread sessions use amber and red respectively;
3. an unread dot updates to the latest state before the session is selected;
4. selecting a session clears the unread marker; and
5. ended sessions do not display an unread marker.
