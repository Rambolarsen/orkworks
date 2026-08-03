# Session Plan Review

Status: proposed
Date: 2026-08-01

## Purpose

OrkWorks should let a user read and explicitly request review of a plan or specification produced by the selected session, without becoming a repo-wide task inbox or a general terminal-input controller.

## Design

- A session may have one associated workspace-relative Markdown artifact. A harness-reported `planPath` remains authoritative; as a fallback, OrkWorks recognizes a valid path printed in terminal output below `docs/superpowers/plans/`, `docs/superpowers/specs/`, or `specs/` — covering both the `writing-plans` skill's plan output and the `brainstorming` skill's design-doc checkpoint. The fallback only matches on a line that also reports having written the file (a small set of write verbs and inflections), so an incidental mention or reference to an existing path is not mistaken for authorship; this intentionally favors missing the card over misattributing an unrelated document (see [issue #278](https://github.com/Rambolarsen/orkworks/issues/278) for replacing the fallback with a hook-reported signal instead of a wider vocabulary list). Every path is canonicalized, contained within the workspace, Markdown-only, and rejected when it contains a control character.
- The selected session's Details panel shows a card whenever the associated artifact remains readable. It says **Plan ready for review** when the session needs the user and **Plan available** otherwise.
- **Review plan** selects a single reusable Review tab beside Terminal and renders that artifact. It never creates a document tab per file.
- **Ask this agent to review** is available only for a live selected session. The user click is the explicit approval: Electron main calls a sidecar endpoint authenticated with its per-sidecar secret; the sidecar revalidates the stored path and writes one fixed review prompt plus Enter to that session's PTY.
- The renderer provides only a session ID. It cannot provide a path, command, or arbitrary terminal text. The sidecar appends an event recording the user-approved review handoff.

## Prompt

The sidecar constructs one fixed prompt using the validated workspace-relative path. It asks the live session's agent to delegate to a separate review subagent rather than reviewing its own plan — the same author reviewing their own spec defeats the point of an independent check, and this stays within the existing same-session PTY handoff rather than starting a second OrkWorks session (see Non-goals):

`Please spawn a separate review subagent — not the agent that wrote it — to review the plan or specification at <path>. Have it check for missing requirements, risky assumptions, and unclear steps, then report its findings.`

## Non-goals

- A repo-wide review queue, task list, digest Peon, or background artifact watcher.
- Arbitrary terminal text injection, keyboard automation, command approval, or sending prompts without a user click.
- Automatically starting a second session or requiring an independent reviewer.

## Acceptance criteria

- [ ] A readable session plan/spec shows a Details card regardless of session attention state.
- [ ] Review plan opens the associated artifact in the reusable Review tab beside Terminal.
- [ ] A user can explicitly send the fixed review prompt to a live session; the prompt is submitted exactly once.
- [ ] Missing, absolute, non-Markdown, and workspace-escaping paths are rejected before any PTY input.
- [ ] The renderer never receives an artifact path or sends arbitrary terminal text.
- [ ] The user-approved handoff is recorded in the session event log.
