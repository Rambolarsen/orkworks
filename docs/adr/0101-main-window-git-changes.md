---
type: Architecture Decision
title: Main-window Git changes
description: Give read-only Git comparison a central file browser and diff while preserving the selected terminal runtime.
tags: [orkworks, desktop, git, navigation]
status: stable
---

# Main-window Git changes

- Status: accepted
- Deciders: user
- Date: 2026-10-31

## Context

The inspector-only viewer leaves too little room for code and gives comparison
metadata more prominence than the changes. The owner approved an interactive
main-window mockup and authorized implementation in the task conversation.
Tracked in [#1207](https://github.com/Rambolarsen/orkworks/issues/1207).

## Decision

Partially supersede the inspector-host decision in
[ADR 0091](0091-read-only-session-git-changes.md). Its authenticated bounded
read-only retrieval decision remains accepted. Extend the central utility
presentation used by Recommendations and Skills with transient Changes; reuse
the existing destination state without changing persisted navigation schemas.

Changes replaces the central presentation and keeps the Sessions sidebar.
Selecting a session explicitly returns to its terminal. Back to terminal and
focus-scoped Escape return without selecting, acknowledging or sending input.
The terminal runtime keeps draining output under ADR 0022. Opening Changes
leaves document Review; the existing Review entry remains available.

Show a filterable file list beside a unified or split diff, with independent
scrolling, line numbers, addition/deletion markers and basic syntax coloring.
Use compact comparison controls and expandable comparison details. Keep
partial-result and error states visible. On narrow content widths stack the
bounded file list above the diff; controls wrap at zoom. No new shell tabs,
movable panels, Git mutations, backend API or persisted preference fields.

## Consequences

Code has the main reading area. Changes and the terminal are alternate central
presentations. Comparison snapshots and diff preferences are in memory only.
Rollback restores the inspector presentation without data migration. Existing
capture validation, limits, unavailable states and authorship caveats continue
to apply. Source implementation does not imply availability in old installers.
