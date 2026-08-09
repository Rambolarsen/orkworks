# Terminal plan links

**Status:** proposed

## Context

The existing plan-review card depends on automatic association. For Codex and
other harnesses without an authoritative plan-path hook, that means a strict
terminal-text heuristic: a write verb and a workspace-relative path in one of
three approved directories. A session that writes a plan in a linked worktree
therefore has no review entry point even though the path is visible to the
user in its terminal.

## Decision

Terminal output is the primary discovery surface. When a session prints a
Markdown path below `docs/superpowers/plans/`, `docs/superpowers/specs/`, or
`specs/`, OrkWorks renders that path as a terminal link. Relative and absolute
forms are eligible.

Clicking a link is explicit user approval to associate and review that
artifact. OrkWorks validates the target, stores it as the session's selected
plan, opens the reusable Review tab, and then exposes the existing Details
review actions. Terminal-output heuristics and harness hooks may still create
the same association automatically, but their absence must not hide the user
path to review.

## Validation and trust boundary

The renderer's terminal link provider passes only the selected session ID and
the literal terminal path to a privileged Electron IPC handler. Electron calls
an authenticated sidecar endpoint. The sidecar canonicalizes the target and
accepts it only when it is a readable Markdown file beneath an allowed plan
root in either:

- the session workspace; or
- a Git worktree sharing the session repository's Git common directory.

The sidecar derives the worktree relationship itself; it does not trust a
renderer-provided root. The persisted association records the artifact's
worktree anchor plus its workspace-relative path, so it remains resolvable
after the terminal link has scrolled away. The existing path-only
`SessionMetadata.plan_path` representation is evolved accordingly, with
legacy same-workspace values read as an anchor to the session workspace.

The click does not open a `file:` URL, invoke the system shell, or widen the
existing web-link allowlist. Invalid, missing, non-Markdown, escaping, or
unrelated-worktree paths leave the session unchanged and surface a concise
error.

## User experience

Plan/spec paths printed in the terminal appear as links. A click opens the
single Review tab with that artifact; it never creates one tab per file. The
clicked artifact is then available in Details as **Plan available** or **Plan
ready for review**, including the existing explicit **Request independent
review** action for live sessions.

Existing sessions with an automatically associated same-workspace plan retain
their current Details-card behavior. No background watcher, repo-wide queue,
or generic filesystem browser is added.

## Tests and documentation

Tests cover terminal-path recognition and activation, IPC forwarding,
same-workspace and sibling-worktree validation, rejected unrelated paths,
legacy metadata compatibility, Review-tab opening, and the Details-card state
after a click. Update the session-plan-review spec, domain-entities reference,
and architecture documentation for the new session-scoped artifact reference.
