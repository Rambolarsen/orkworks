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

## Switch without stopping the work

Select a session to return to its terminal. Switching away does not stop its
process: other sessions keep running while the backend remains alive. Recent
terminal history is replayed when you return. Closing the application is
different; live processes do not survive a backend restart.

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

The source shell currently defers the Review destination and its plan-opening
controls to [issue #780](https://github.com/Rambolarsen/orkworks/issues/780).
Plan metadata remains available to the session; use your coding tool to read it
until the destination is implemented. Published installers may have the earlier
Review tab.

## When a signal looks wrong

Check the terminal first, then the selected provider and coding-tool
integration in Settings. A quiet terminal, an unavailable provider, and an
agent waiting for input are different situations. Report reproducible
problems through [GitHub issues](https://github.com/Rambolarsen/orkworks/issues).
