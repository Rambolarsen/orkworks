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
`specs/`, OrkWorks renders that path as a terminal link. The custom xterm link
provider is installed for both live and historical terminals while retaining
the current HTTP(S) link behavior. It recognizes quoted paths and strips only
unambiguous trailing punctuation; it does not treat arbitrary Markdown files,
wrapped fragments, or OSC-8 labels as plan links.

Relative links are deliberately narrow: their path must start with one of the
three approved roots and resolves against the session's immutable launch Git
worktree root, never the sidecar process CWD or a later terminal `cd`.
Absolute links may target that worktree or a linked worktree of the same Git
repository. This makes resolution deterministic: a relative `specs/plan.md`
cannot silently select the copy in another worktree.

Clicking a link is explicit user approval to associate and review that
artifact. OrkWorks validates the target, stores it as the session's selected
plan, refreshes the session's plan availability, then opens the reusable
Review tab. The existing Details review actions become available only after
that successful transition. A failed click does not change the selected
artifact, event log, or currently displayed Review tab.

Associations have explicit provenance and authority:

| Incoming association | Replaces | Cannot replace |
| --- | --- | --- |
| User-selected terminal link | hook or fallback | — |
| Hook-reported path | fallback or none | user selection or user clear |
| Terminal fallback | none | hook, user selection, or user clear |

User clear records a clear tombstone and suppresses later automatic fallback
and hook associations until a new user selection. Every successful transition
records a provenance-specific session event. Terminal-output heuristics and
harness hooks remain conveniences, never the only route to review.

## Validation and trust boundary

The renderer's terminal link provider passes the selected session ID and the
literal matched path to a narrowly scoped, privileged Electron IPC handler.
This intentionally supersedes ADR 0025's session-ID-only preload rule and
requires a new ADR before implementation. Electron validates the ID and path
types, then calls an authenticated sidecar endpoint; the renderer never gets a
filesystem path from the sidecar and no generic file-open or terminal-write
API is introduced.

The sidecar canonicalizes the target and accepts it only when it is a readable,
regular Markdown file beneath an allowed plan root in either:

- the session workspace; or
- a Git worktree sharing the session repository's Git common directory.

The sidecar derives launch-worktree identity and the Git common-directory
relationship itself; it does not trust a renderer-provided root. Selection,
Details-card availability, Review-content reads, and review-prompt writes all
use one shared resolver that revalidates the stored anchor, containment, file
type, extension, and common-directory relationship immediately before use.

The persisted association is an untagged `PlanReference`: legacy strings read
as `{ anchor: SessionWorkspace, relativePath: <string>, source: legacy }`;
new values are objects containing the canonical worktree anchor,
worktree-relative path, and provenance (`user_selected`, `hook_reported`, or
`terminal_fallback`). New hook input remains the existing string `planPath`
wire format and is normalized server-side. New writes use the object form.
Malformed mixed forms are rejected without mutating the existing association.

The click does not open a `file:` URL, invoke the system shell, or widen the
existing web-link allowlist. Invalid, missing, non-Markdown, escaping, stale,
or unrelated-worktree paths leave the session unchanged and surface a concise
error. The fixed review prompt uses the validated anchored artifact reference
rather than assuming the live PTY and artifact share a directory.

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

Tests cover terminal-path grammar and activation in both terminal modes, HTTP
link preservation, IPC forwarding, same-workspace and sibling-worktree
validation, repeated relative names across worktrees, post-print `cd` changes,
and rejected unrelated paths. They also cover legacy/malformed metadata,
restart resolution, precedence transitions, revalidation before content and
PTY input, review-tab success and failure races, and no duplicate prompt on
repeated activation. Update the session-plan-review spec, domain-entities
reference, architecture documentation, and a new ADR for the revised
privileged contract and session-scoped artifact reference.
