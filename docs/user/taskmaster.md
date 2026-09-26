# Turn workflow friction into a next step

Taskmaster looks across recorded workflow observations and suggests
improvements to the way you work. For example, repeated missing context may
lead to a suggestion to improve repository instructions.

## Inspect before acting

A workflow-improvement card includes the proposed change, its target, and
supporting observations. Read that evidence before deciding whether the
suggestion is useful. Observations can come from coding agents or Peon;
their confidence is evidence to weigh, not a guarantee.

- **Dismiss** declines the suggestion.
- **Fix with AI** sends a scoped fix prompt into your currently active
  session. It requires an active session and your explicit action.
- **Analyze now** asks the Brain to look for an improvement on demand. If one
  is already proposed or being implemented, Taskmaster asks you to handle that
  recommendation first. It works even when automatic Background discovery is
  off, while still respecting the daily analysis limit.

This action does not start a new session. The coding agent in your selected
session carries out the work under your repository’s normal instructions.
Taskmaster does not merge changes or accept work on your behalf.

## Current scope and future direction

The current workflow-improvement surface is one part of the broader
[Taskmaster design](/specs/taskmaster). That specification also describes
review and verification chains and approved session transitions. Treat it
as design scope, not a checklist of features available in an installer.
