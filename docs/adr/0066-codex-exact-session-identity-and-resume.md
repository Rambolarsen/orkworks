# Codex exact session identity and resume

- Status: superseded by [ADR 0067](./0067-codex-subagents-share-owning-session-identity.md)
- Deciders: OrkWorks maintainers
- Date: 2026-09-26

## Context

OrkWorks passes its session ID and authenticated reporting capability to the
launched coding tool. Child processes inherit them. A nested Codex process can
therefore run the same project hook and report a different native Codex thread
ID against the parent OrkWorks session. Equal-confidence reports currently
replace one another, so a later exact resume may reference the nested thread
instead of the conversation OrkWorks launched.

Codex also reports its native thread ID before its local rollout is necessarily
available. A thread ID alone does not prove that Codex can resume the thread.
Resuming the latest thread instead would risk switching conversations.

Codex CLI subagents run inside their owning Codex session. They are not
independent OrkWorks sessions and do not receive separate OrkWorks session
IDs. Only the owning CLI's root `SessionStart` reports native identity.

## Decision

For sessions owned by the Codex harness, retain the first accepted Codex native
session ID. Ignore later differing Codex IDs unless an authenticated Codex
`SessionStart` report identifies `source: clear` and OrkWorks has recorded the
explicit session reset for that prior ID, and the report's process ID matches
the live Codex process bound to that OrkWorks session. The reporter finds the
nearest Codex ancestor through a bounded process-parent walk. Missing process
identity, a different process, or a nested Codex process cannot authorize the
replacement. This ownership binding is in-memory and is cleared when the
OrkWorks session ends or is forgotten; an authenticated resume binds the newly
launched Codex process. Pass the `SessionStart.source` and process ID through
the existing reporter; other events cannot authorize replacement.

Codex resume is exact-ID-only. Before launching `codex resume <id>`, verify in
read-only mode that the exact ID exists in the supported local
`state_5.sqlite` `threads` table and that its recorded rollout file exists. If
the ID is absent or the local saved state is unavailable, do not launch Codex.
Never use `codex resume --last` as a fallback.

## Consequences

Nested Codex sessions cannot silently displace the parent conversation's
identity. Explicit Codex `/clear` or `/new` may establish the replacement ID
only when the owning process can be verified; unsupported or unavailable
process inspection keeps the old ID and exact-resume target intact.
An ID whose rollout has not yet been saved, or whose local state is missing,
cannot be resumed through OrkWorks until Codex saves it again. A missing ID
never resumes another conversation.
