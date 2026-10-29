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
Choose a coding tool and optional model in the friendly introduction, enter an
optional initial prompt, then
choose **Start session** to open its terminal. Empty workspaces show the composer
automatically; no session starts until you choose **Start session**. **Cancel** returns
to the previous view. Existing sessions keep running while you compose.
Published installers may still show the earlier creation popup.

Current source builds recheck the chosen coding tool when you press **Start
session**. If the check fails or the tool cannot be found, the composer explains
the problem and keeps your tool, model, prompt and attachments. Check the tool
settings or installation, then press **Start session** to retry.

Current source builds can include images when starting a verified Codex version
(currently Codex 0.163.0 on macOS arm64). Paste a screenshot, drop image files
into the composer, or choose **Attach images**. Review the thumbnails and use
**Remove** before starting. PNG, JPEG, and WebP are supported, up to eight images,
5 MiB each and 20 MiB total. An image-only start asks the agent to describe them.
Unsupported coding tools retain the draft and explain why Start is disabled.
Published installers may not include image attachments yet.


With an explicit Codex model selected, **Reasoning effort** offers the values
reported by that installed coding tool. **Tool default effort** keeps Codex's
configured default. Changing the tool or model clears the effort selection.
This applies to the new interactive session, including image attachments, and
is separate from background analysis settings. Other coding tools keep their
own effort defaults. This describes current source builds; an older published
installer may not include the control.

## Switch without stopping the work

Select a session to return to its terminal. Switching away does not stop its
process: other sessions keep running while the backend remains alive. Recent
terminal history is replayed when you return. Closing the application is
different; live processes do not survive a backend restart.

## Close, archive or delete a session

Current source builds have three session states: **Active**, **Closed** and
**Archived**. Use the Sessions **State** filter to show All unarchived (the
default), Active, Closed or Archived. Filtering preserves your selected terminal
or saved history, even when its row is hidden.

Right-click a row or choose its **More actions** button. With the Sessions list
focused, use the context-menu key or **Shift+F10** for the selected row. Arrow
keys move through the menu; Escape returns focus.

**Close** stops an Active session. **Archive (close first)** stops it and waits
for saved ending/history to finish before archiving. **Delete… (close first)**
asks for confirmation before stopping anything, then permanently deletes the
record, events and saved terminal scrollback only after Close finishes. If
Close fails or is still finishing, the second step does not run; refresh and
try again. Closed sessions can be archived or deleted directly.

Archived sessions retain history and are protected from automatic age/count
cleanup. Choose **Restore to Closed** before resuming one. Restoring does not
start a process, and normal retention settings apply to it again. Archive and
Restore preserve activity timestamps. Only deleting the selected session record clears the selection. If later
evidence cleanup fails after record removal, the missing row is reconciled while
the cleanup error remains visible.

Archive is organization in OrkWorks; it does not archive the conversation in
the coding tool or accept its work. Older installers may lack these actions.
Downgrading to a binary without archive-aware retention removes the automatic
cleanup protection, so keep a compatible build when relying on that protection.

## Resume an ended session

In current source builds, select an ended session and use the Resume split
button beside its title or in Details. The main button resumes the preferred
available option; the arrow opens other choices and explains unavailable ones.
Exact resume starts directly. A folder or repository fallback may open a
different conversation, so it first shows a warning with **Cancel** and a
confirmation naming the chosen scope. Cancel leaves the session unchanged.
Resume support varies by coding tool. An option that becomes unavailable fails
with an explanation rather than switching to another scope. Published installers
may still use the earlier resume controls.

## Open a session in its desktop app

In current source builds, the top-bar **Open in** control names the desktop app
as soon as the selected session’s coding tool is known. It checks availability
in advance and refreshes when the native conversation ID or session lifecycle
changes. Built-in Codex sessions can open the same conversation in **ChatGPT**.
Built-in Claude Code sessions can open in **Claude** after you explicitly
**Close** the session in OrkWorks. Opening never closes a running session for you.

The exact native conversation ID must have been captured. The menu explains
missing IDs, unsupported tools or custom configurations, and missing desktop
apps. Claude also requires a supported CLI version, platform and subscription
login. If opening fails, the error stays in the menu. OrkWorks never substitutes
the most recent conversation or starts a new one. Published installers may not
yet include this action.

## Find the next thing to inspect

In current source builds, **Actions** shows session attention followed by
workspace recommendations. It opens in the right inspector in a wide window and
as a temporary page in a narrower window. Session attention and recommendation
counts stay separate; loading, partial and unavailable information is labeled.
Session usage-limit status remains in the session list and Details. The
unfinished standalone Capacity destination has been removed from current source
builds; its dashboard is deferred.

**Show next action** reveals and focuses the first current item. Inspecting it
keeps your selected session and unread attention unchanged. Choose **Open
terminal/history** to switch sessions or **Open recommendation** to use the
existing recommendation controls. Close Actions to return to the previous
central view. Changed or missing items require a new inspection before their
details become current. Published installers may not include this overview yet.

## Saved shell layout

Current source builds use a fixed arrangement of Sessions, Terminal and an
optional inspector. An upgrade notice explains that the earlier saved panel
arrangement is retained. Dismiss the notice to acknowledge it; the legacy
arrangement is not deleted.

In current source builds, choose **Collapse sidebar** at the top of the
sidebar, beside the workspace controls, to leave a narrow icon rail. Hover an icon
for its label; New session, Recommendations and Settings remain
available. **Expand sidebar** restores the session list at its saved width.
The header's **Workspace** menu keeps workspace switching available. Narrow
windows use the temporary Sessions page instead of the rail.

The right inspector overlays the main view with a subtly translucent background
by default. Choose **Pin inspector** to make it opaque and reserve space beside
the main view; **Unpin inspector** restores the overlay. The pin preference is
saved across restarts, and Reset Layout returns it to unpinned. With keyboard
focus inside the inspector, **Escape** closes it and restores focus to the
control that opened it. Clicking outside an unpinned Details
or Actions overlay hides it without interrupting the clicked control.
Pinned inspectors stay open. Narrow windows keep the temporary inspector page.
This describes current source builds; published installers may differ.

If saved layout preferences cannot be read, the shell uses defaults and offers
**Reset Layout…**. Its native confirmation dialog asks before rebuilding those
preferences. Cancel keeps the current view and leaves the saved data untouched.
Published installers may still use the earlier panel layout.

## Read the signals

Peon is OrkWorks’ AI observer. With a provider configured, it reads recent
terminal output and produces summaries and attention signals. Some coding
tools can also report events through an explicitly installed integration.
For OpenCode, an active integration reports prompt-driven **Needs You**.
Current source builds also read Codex's own terminal status: manual prompts
show **Needs You**, while automatic approval review stays **Working**.
This works without reinstalling hooks. Codex versions that do not emit this
status retain the earlier hook behavior, which can show a false wait during
automatic review. Published installers may have earlier behavior.

Codex terminal detection is best effort and can miss an uncommon prompt.
A status you set yourself, or one explicitly reported by an agent, remains
protected. Peon still supplies summaries but does not infer **Needs You** from
chat text for Codex and OpenCode.

In current source builds, the GitHub Copilot CLI integration recognizes
permission and clarification prompts. Once it has reported a prompt, submitting
your next prompt clears **Needs You** and shows working. Background-agent
completion does not clear a prompt or establish that the main session is idle.
Update an existing Copilot integration through Settings to add the submission
hook; published installers may have earlier behavior.

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
Line totals are omitted when they cannot be read. A stalled Git scan is stopped after
five seconds, leaving Git facts unavailable instead of displaying clean or zero
counts. Session refreshes can recover after the filesystem problem is fixed.
Git status errors also leave these facts unavailable. When only line totals
cannot be read, the dirty state and file count remain visible.

## Inspect Git changes

In current source builds, select a session, open **Details**, and choose
**Review changes**. The existing right panel shows a file list and read-only
unified diff; **Back to Details** returns to the session facts. The entry is
available even for clean worktrees, which may still contain branch commits.

Use **Comparison** to distinguish two views:

- **Uncommitted changes** compares HEAD with the working tree, combining staged,
  unstaged, and non-ignored new files into one net diff.
- **Branch changes** shows committed changes from the merge base with the
  selected **Base branch** to the current HEAD. It excludes uncommitted edits.

Current source builds support an explicit task-worktree report from the agent.
When a coding agent starts in one checkout but works in another, its reported
task worktree takes precedence over the harness, process and launch directories.
Details and Changes label the directory source. Without a report, the fallback
is labeled; a missing reported worktree shows unavailable rather than silently
switching back to the launch checkout. Temporary commands elsewhere do not
change the binding. This requires the matching source build and updated Codex
integration; it does not establish availability in older installers.

File/symlink conversions appear as one changed file with removed and replacement
content and line totals. Symlinks display their target text.

The panel identifies the worktree, baseline and capture time. Base choices use
local refs; the viewer never fetches. Choose **Refresh** to capture newer changes.
Sessions sharing a worktree show the same changes; this does not identify which
session made an edit. Limits and unavailable/binary content are labeled, and an
incomplete comparison is not reported as clean. Reading changes never stages,
discards, edits, commits, approves or submits work to a coding tool.

The panel keeps the selected terminal alongside it at wide widths and uses the
normal temporary page at narrower widths. These instructions describe source
behavior; an older published installer may not include the viewer.

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
history, including in narrow windows. Escape also returns when focus is in the
Review document or its navigation controls. Focus returns to the opening
control when it is still available, or to the terminal when Review was restored
or opened from terminal input through the View menu. Cancelling workspace
selection or failing to open a remembered path preserves the current plan;
Review actions pause while a workspace selection is pending.

Use **Refresh plan** to read the current document again. If it has moved or is
unreadable, Review clears the old content and offers Retry, including when the
first read fails. If a saved Review cannot be revalidated when opening a
workspace, OrkWorks stays on Terminal and explains why. Opening, refreshing,
closing or restoring Review sends no prompt. For a live session, the separate
**Request independent review** button explicitly asks its coding tool to review
the plan, delegating when supported.

If saved navigation cannot be read, the shell explains the problem and offers
**Rebuild navigation…** for a corrupt or unsupported record. Rebuilding requires
native confirmation; cancelling keeps the saved record intact.

Published installers may still use the earlier Review tab; these instructions
describe current source behavior.

## When a signal looks wrong

Check the terminal first, then the selected provider and coding-tool
integration in Settings. A quiet terminal, an unavailable provider, and an
agent waiting for input are different situations. Report reproducible
problems through [GitHub issues](https://github.com/Rambolarsen/orkworks/issues).
