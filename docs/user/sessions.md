---
type: "User Guide"
title: "Follow your sessions"
description: "User guide to follow your sessions in OrkWorks."
tags: ["orkworks", "user-guide"]
---

# Follow your sessions

Start with one session, then add another when you have independent work to do.
The sessions list keeps their coding tools and attention states visible while
you work in one terminal at a time.

## Create a session

In current source builds, **New session** opens a composer in the main pane.
Enter an optional initial prompt, choose a coding tool and optional model, then
choose **Start** to open its terminal. Empty workspaces show the composer
automatically; no session starts until you choose **Start**. **Cancel** returns
to the previous view. Existing sessions keep running while you compose.
Published installers may still show the earlier creation popup.

## Switch without stopping the work

Select a session to return to its terminal. Switching away does not stop its
process: other sessions keep running while the backend remains alive. Recent
terminal history is replayed when you return. Closing the application is
different; live processes do not survive a backend restart.

## Resume an ended session

In current source builds, select an ended session and use **Resume session**
beside its title above the terminal. The button appears while connected when
the coding tool has an available resume option. Exact resumes start directly;
folder or repository fallbacks open Details so you can read which conversation
will resume before choosing. Resume support varies by coding tool. Published
installers may still place resume actions inside Details.

## Saved shell layout

Current source builds use a fixed arrangement of Sessions, Terminal and an
optional inspector. An upgrade notice explains that the earlier saved panel
arrangement is retained. Dismiss the notice to acknowledge it; the legacy
arrangement is not deleted.

If saved layout preferences cannot be read, the shell uses defaults and offers
**Reset Layout…**. Its native confirmation dialog asks before rebuilding those
preferences. Cancel keeps the current view and leaves the saved data untouched.
Published installers may still use the earlier panel layout.

## Read the signals

Peon is OrkWorks’ AI observer. With a provider configured, it reads recent
terminal output and produces summaries and attention signals. Some coding
tools can also report events through an explicitly installed integration.
For Codex and OpenCode, an active integration reports prompt-driven **Needs
You**; a status you set yourself can also show **Needs You**. Peon still
supplies summaries but does not infer **Needs You** from chat text for those
tools.

Use a signal such as waiting for input as a reason to inspect the session.
An inferred summary can be wrong; it is not proof that tests passed or work
is complete. Integration coverage varies by coding tool.

The session details keep its working directory and Git context near the
conversation. OrkWorks shows context; your existing tools still create
branches, manage worktrees, and merge changes.

In current source builds, the Details Git row shows live uncommitted changes,
for example **3 files · +124 −37**: changed files, added lines, and removed
lines. It includes staged edits, unstaged edits, and non-ignored new files.
Pure renames contribute no line changes. Binary files count toward files but
contribute no lines. Sessions sharing a
worktree show the same totals; the counter does not identify which agent made
each edit. Git results are reused for five seconds; committing or undoing
changes updates the counter on the first session refresh after that window.
Line totals are omitted when they cannot be read.

## Scroll a terminal

Use the mouse wheel or scrollbar to look back through terminal history. Some
coding tools run a full-screen interface that captures mouse input, so the
wheel scrolls that tool's view instead. Current source builds keep small
trackpad gestures responsive and combine continued movement into controlled
scrolling in these full-screen views. OpenCode is one: if the wheel does
nothing there, scroll with OpenCode's own keys — `PageUp` / `PageDown` by
page, `Ctrl+Alt+Y` / `Ctrl+Alt+E` line by line, and `Ctrl+Alt+U` /
`Ctrl+Alt+D` by half page.

## Read a plan

In current source builds, select a session with an associated readable plan or
specification, then choose **Review** in the content header or View menu, or
**Review plan** in Details. The document replaces the terminal in the main work
area. **Back to Terminal** returns to the same session's terminal or saved
history, including in narrow windows.

Use **Refresh plan** to read the current document again. If it has moved or is
unreadable, Review clears the old content and offers Retry. Opening, refreshing,
closing or restoring Review sends no prompt. For a live session, the separate
**Request independent review** button explicitly asks its coding tool to review
the plan, delegating when supported.

Published installers may still use the earlier Review tab; these instructions
describe current source behavior.

## When a signal looks wrong

Check the terminal first, then the selected provider and coding-tool
integration in Settings. A quiet terminal, an unavailable provider, and an
agent waiting for input are different situations. Report reproducible
problems through [GitHub issues](https://github.com/Rambolarsen/orkworks/issues).
