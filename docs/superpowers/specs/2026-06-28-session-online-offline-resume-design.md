# Session Online/Offline Resume Design

## Summary

Redesign session state so the primary user-facing distinction is whether a session is currently reachable as a live process.

- `online`: the session has a live process and may expose runtime attention states such as `working`, `idle`, or `waiting_for_input`
- `offline`: the session no longer has a live process and is presented uniformly in the sessions list, regardless of how it exited

Resume behavior becomes a first-class capability model owned by the detail panel, not an implication of lifecycle status labels.

## Motivation

The current model mixes several different ideas into `status` and `observedStatus`:

- process liveness: is there a live PTY-backed session
- attention state: does the user need to act, is the agent working, is the session idle
- terminal outcome: did the process end normally, get killed, or fail
- resume capability: whether the session can be resumed and how

This leads to confusing states in the sessions list and pushes too much meaning into status badges. A session that is no longer running should not continue to present itself as `waiting_for_input`, `done`, `error`, or `killed` in the main list. Once the process is gone, the primary question becomes: can it be resumed, and in which ways?

## Goals

- Make the sessions list answer a simple question first: `online` or `offline`
- Keep live attention states available only for online sessions
- Preserve exit outcome and diagnostic history without using them as primary list state
- Move resume choice and resume explanations into the detail panel
- Show every session in the list, regardless of whether it is resumable
- Sort sessions by last activity instead of status buckets

## Non-goals

- Do not redesign the resume transport or harness adapters in this change
- Do not remove diagnostic history for `ended`, `killed`, or `error`
- Do not change the set of live attention states unless existing tests require small adjustments
- Do not add list-level resume buttons
- Do not hide non-resumable offline sessions

## User-Facing Model

### Primary Presentation State

Every session has one primary presentation state:

- `online`
- `offline`

This state is what the sessions list uses for its top-level treatment.

### Live Attention State

Only online sessions may expose a live attention state:

- `working`
- `idle`
- `waiting_for_input`
- `blocked`
- `failed`
- `done`
- `stale`

These remain useful for sorting, labels, and attention styling while the session is online.

Offline sessions do not expose a live attention state to the list UI. Their previous live state may still be preserved internally or in diagnostics, but it does not control list badges, sorting, or tone.

### Offline Diagnostic Outcome

Offline sessions may keep a secondary internal outcome for history and debugging:

- `ended`
- `killed`
- `error`

This outcome is not the primary state shown in the list. It may appear in a lower-emphasis area in the detail panel or event history.

### Resume Capabilities

Offline sessions expose a list of resume methods. Each method includes:

- strategy id
- user-facing label
- whether the strategy is currently available
- a reason when unavailable
- whether it is the preferred option

The detail panel is the only place where resume methods are shown.

## Proposed Data Model

### API DTO Shape

Extend the session DTO with explicit presentation and resume fields.

Example shape:

```ts
type SessionConnectivity = "online" | "offline";

type ResumeOption = {
  strategy: "exact" | "latest_cwd" | "latest_repo" | "none";
  label: string;
  available: boolean;
  reason?: string;
  preferred: boolean;
};

type SessionInfo = {
  id: string;
  status: string;
  observedStatus?: string;
  connectivity: SessionConnectivity;
  terminalOutcome?: "ended" | "killed" | "error";
  resumeOptions: ResumeOption[];
  lastActivityAt: string;
};
```

Notes:

- `status` may remain during migration so existing backend logic and tests do not break all at once
- `connectivity` becomes the primary field for UI rendering
- `observedStatus` is only meaningful when `connectivity === "online"`
- `terminalOutcome` is optional secondary metadata for offline sessions
- `resumeOptions` supersedes the current single preferred-resume representation in the detail UI

### Domain Model

Keep the existing domain/session lifecycle outcome if needed, but add a separate concept for presentation liveness:

- connectivity: `Online | Offline`
- attention state: present only while online
- terminal outcome: present only after going offline
- resume options: derived capability set, not inferred from status labels

This keeps liveness, attention, diagnostics, and resumability on separate axes.

## Backend Behavior

### Online/Offline Rules

- A newly created session starts as `online`
- A session with a live harness process remains `online`
- When the PTY-backed process exits or is killed, the session becomes `offline`
- Once offline, the session no longer emits live attention state to the frontend list

### Transition Rules

On transition from online to offline:

1. Mark `connectivity = "offline"`
2. Preserve internal terminal outcome as `ended`, `killed`, or `error`
3. Stop treating `observedStatus` as active list state
4. Compute or refresh resume options for the detail panel
5. Update last-activity timestamp

The old issue-26 idea of frontend-visible lifecycle phases is no longer the right center of gravity for this design. If we still need an internal transition helper for atomic cleanup, it should support the online/offline model rather than become the user-facing state contract.

### Resume Option Derivation

Resume options should be computed from the same sources the backend already uses for resumability decisions:

- harness capabilities
- stored resume memory
- stored harness session id
- cwd/repo fallback support
- whether a concrete strategy is currently executable

Each strategy should be returned even when unavailable if the product wants to explain why it cannot be used.

Unavailable reasons should be explicit and stable enough for UI display, for example:

- exact resume not available because no harness session id was captured
- latest repo resume not available because this harness does not support repo-scoped resume
- latest cwd resume not available because no compatible remembered session exists

## Frontend Behavior

### Sessions List

The sessions list should:

- render all sessions
- use `connectivity` as the top-level presentation state
- show live attention styling only for online sessions
- show a uniform offline treatment for offline sessions
- sort by last activity, newest first

Offline sessions should not be specially bucketed below online sessions beyond the natural result of last-activity sorting.

### Detail Panel

For online sessions, the detail panel continues to show live context such as attention label, current summary, and provider state.

For offline sessions, the detail panel should emphasize:

- offline status
- list of resume methods
- disabled styling for unavailable methods
- explanation text for unavailable methods

Secondary history may include:

- last diagnostic outcome
- last observed attention state
- last activity time

But that history should not compete visually with resume actions.

## Sorting

Sorting should be based on last activity timestamp rather than attention-priority buckets alone.

Recommended order:

1. Newest `lastActivityAt`
2. Tie-break by label

If the current UI still needs a small amount of attention ordering among online sessions with identical timestamps, that can be applied as a secondary tie-break only.

## Error Handling

- If resume options cannot be computed, return a disabled option set with an explanation rather than omitting the section entirely
- If terminal outcome is unknown, still render the session as offline
- If legacy metadata lacks enough information for a specific strategy, expose it as unavailable with reason instead of guessing

## Migration Strategy

Implement incrementally:

1. Add `connectivity`, `terminalOutcome`, `resumeOptions`, and a canonical last-activity field to the backend DTO
2. Keep existing `status` and `observedStatus` during migration
3. Update the frontend list to key off `connectivity`
4. Update the detail panel to render resume options for offline sessions
5. Demote `status` and terminal outcome to secondary detail/debug information
6. Remove old UI assumptions that offline sessions can still be sorted or labeled by live attention state

## Testing

### Backend

- session creation yields `connectivity = "online"`
- PTY exit transitions session to `offline` with `terminalOutcome = "ended"`
- kill transitions session to `offline` with `terminalOutcome = "killed"`
- startup or runtime failure transitions session to `offline` with `terminalOutcome = "error"`
- offline sessions return resume options with correct availability and reasons
- online sessions do not leak stale offline terminal outcomes into primary presentation

### Frontend

- sessions list renders online and offline states distinctly
- offline sessions do not display live attention labels
- sessions sort by last activity rather than status priority
- detail panel renders all resume methods
- unavailable resume methods are visibly disabled and include reason text
- online detail panel behavior remains intact

## Open Issue Alignment

This design partially supersedes the framing of Issue #26.

What remains useful from #26:

- preserving atomic transition behavior when a live session exits
- ensuring live attention state does not outlive the live session
- keeping history without misleading active-state presentation

What should change in #26:

- stop treating explicit frontend lifecycle phases as the primary solution
- reframe the work around `online/offline` presentation state plus secondary terminal outcome and resume options

## Recommendation

Adopt the two-layer model:

- primary presentation state: `online | offline`
- secondary live attention state for online sessions only
- secondary terminal outcome for offline diagnostics
- explicit resume options in the detail panel

This is simpler for users, cleaner in the domain model, and a better fit for the repo’s existing resume architecture than expanding the current lifecycle status matrix.
