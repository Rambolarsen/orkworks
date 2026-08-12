# Session Plan Review

Status: proposed
Date: 2026-08-01

## Purpose

OrkWorks should let a user read and explicitly request review of a plan or specification produced by the selected session, without becoming a repo-wide task inbox or a general terminal-input controller.

## Design

- A session may have one associated workspace-relative Markdown artifact. A harness-reported `planPath` remains authoritative; as a fallback, OrkWorks recognizes a valid path printed in terminal output below `docs/superpowers/plans/`, `docs/superpowers/specs/`, or `specs/` — covering both the `writing-plans` skill's plan output and the `brainstorming` skill's design-doc checkpoint. The fallback only matches on a line that also reports having written the file (a small set of write verbs and inflections), so an incidental mention or reference to an existing path is not mistaken for authorship; this intentionally favors missing the card over misattributing an unrelated document (see [issue #278](https://github.com/Rambolarsen/orkworks/issues/278) for replacing the fallback with a hook-reported signal instead of a wider vocabulary list). Every path is canonicalized, contained within the workspace, Markdown-only, and rejected when it contains a control character.
- A printed eligible plan/spec path is also a clickable terminal link. Clicking is explicit user approval to associate the artifact; it takes precedence over hook and fallback associations. Relative links resolve against the session launch worktree; absolute links may resolve only in that worktree or a linked worktree with the same Git common directory. Every later read and review handoff revalidates the stored anchored reference.
- Claude Code's `PostToolUse` `Write|Edit` hook (ADR 0038) is now the authoritative `planPath` delivery: the owned integration installs a synchronous hook whose shared `report-harness-event.sh`/`.ps1` reporter invocation passes `--report-plan-path`, forwarding the hook payload's `tool_input.file_path` to `POST /sessions/:id/plan-path` and skipping the generic attention + harness-session POSTs. A successful hook report overwrites any prior terminal-fallback association, since `report_session_plan_path` stores the path unconditionally (the terminal fallback's `is_none()` guard stays in place as the only protection against the terminal fallback itself clobbering a path that a real hook has already established — the fallback is intentionally lower-confidence than the hook). Other harnesses without a canonical file-path hook event (Codex, Aider, ...) keep the terminal fallback.
- The selected session's Details panel shows a card whenever the associated artifact remains readable. It says **Plan ready for review** when the session needs the user and **Plan available** otherwise.
- **Review plan** selects a single reusable Review tab beside Terminal and renders that artifact. It never creates a document tab per file.
- **Request independent review** is available only for a live selected session. The user click is the explicit approval: Electron main calls a sidecar endpoint authenticated with its per-sidecar secret; the sidecar revalidates the stored path and writes one fixed review prompt plus Enter to that session's PTY.
- For review requests, the renderer provides only a session ID and cannot supply a path, command, or arbitrary terminal text. The terminal-link selection IPC is the narrow exception: it may pass the exact clicked path text with its session ID, and the sidecar remains responsible for all resolution and validation. The sidecar appends an event recording each user-approved handoff.

## Prompt

The sidecar constructs one fixed prompt using the validated workspace-relative path. It asks the live session's agent to prefer delegating to a separate review subagent over reviewing its own plan — the same author reviewing their own spec defeats the point of an independent check — but falls back to letting the agent review it directly when its tooling can't spawn a subagent. This stays within the existing same-session PTY handoff rather than starting a second OrkWorks session, and stops short of mandating an independent reviewer (see Non-goals):

`Please review the plan or specification at <path>. If your tooling can spawn a separate review subagent, delegate the review to it instead of reviewing your own work; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.`

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
