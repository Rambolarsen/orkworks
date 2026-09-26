# Codex exact session identity and resume

- Status: accepted
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

## Decision

For sessions owned by the Codex harness, retain the first accepted Codex native
session ID. Ignore later differing Codex IDs unless an authenticated Codex
`SessionStart` report identifies `source: clear` and OrkWorks has recorded the
explicit session reset for that prior ID. Pass the `SessionStart.source` value
through the existing reporter; other events cannot authorize replacement.

Codex resume is exact-ID-only. Before launching `codex resume <id>`, verify in
read-only mode that the exact ID exists in the supported local
`state_5.sqlite` `threads` table and that its recorded rollout file exists. If
the ID is absent or the local saved state is unavailable, do not launch Codex.
Never use `codex resume --last` as a fallback.

## Consequences

Nested Codex sessions cannot silently displace the parent conversation's
identity. Explicit Codex `/clear` or `/new` may establish the replacement ID.
An ID whose rollout has not yet been saved, or whose local state is missing,
cannot be resumed through OrkWorks until Codex saves it again. A missing ID
never resumes another conversation.
