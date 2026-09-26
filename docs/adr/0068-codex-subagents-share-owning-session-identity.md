# Codex subagents share the owning OrkWorks session identity

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-09-26
- Supersedes: process-ownership requirements in [ADR 0067](./0067-codex-exact-session-identity-and-resume.md)

## Context

Codex CLI subagents run inside the owning Codex session and use its native
thread identity. They are not independent OrkWorks sessions and must not
receive a separate OrkWorks session ID. OrkWorks' Codex hook bundle reports
identity only for the root `SessionStart`; other events, including a
`SubagentStart` event, cannot submit native identity.

The reporter's PID is request data. The OrkWorks report token authenticates the
session but is inherited by child processes, so it cannot prove which process
sent a report. Comparing that PID to an in-memory PID does not provide process
authentication.

## Decision

Do not create OrkWorks sessions or independent OrkWorks IDs for Codex CLI
subagents. Keep the first accepted native Codex thread ID. A different ID may
replace it only after OrkWorks records the explicit reset for the previous ID
and receives an authenticated root `SessionStart` report with `source: clear`.
Do not send or trust a reporter-supplied process ID as reset authority.

Resume remains exact-ID-only. Before launching `codex resume <id>`, verify in
read-only mode that the exact ID exists in the supported local
`state_5.sqlite` `threads` table and that its recorded rollout file exists. If
the ID is absent or local saved state is unavailable, do not launch Codex.
Never use `codex resume --last` as a fallback.

## Consequences

Internal Codex subagents stay associated with their parent's OrkWorks session
and cannot submit a separate native ID through the hook reporter. Explicit
Codex `/clear` or `/new` can establish a replacement only after the user has
recorded the matching reset in OrkWorks and the authenticated root hook reports
`source: clear`.

The reporting capability proves the OrkWorks session, not the operating-system
process. Since child processes inherit it, this protocol does not distinguish
an internal subagent from another process holding the capability. The product
does not model nested standalone Codex CLIs as subagents or give them a new
OrkWorks session ID.

An ID whose rollout has not yet been saved, or whose local state is missing,
cannot be resumed through OrkWorks until Codex saves it. A missing ID never
resumes another conversation.
