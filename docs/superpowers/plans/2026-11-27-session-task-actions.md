---
type: Implementation Plan
title: Deliver explicit task actions for existing sessions
description: Qualification, runtime delivery, title ownership, desktop wiring, and verification for issue 2140.
tags: [orkworks, desktop, sessions, terminal]
status: draft
workflow_status: proposed
---

# Deliver explicit task actions for existing sessions

Deliver **Start new task** and **Keep going** for a qualified live session.
Keep the existing **Resume** behavior available for ended sessions.
See [#2140](https://github.com/Rambolarsen/orkworks/issues/2140) and the
[proposed design](../specs/2026-11-27-session-task-actions-design.md).

1. **Approve and qualify.** After owner approval, record the authority
   decision in an ADR and update the MVP action and title rules. Inspect
   the existing Codex turn signals and runtime input owner. Retain
   representative ready, draft, busy, permission, and disconnect cases.
   Confirm readiness at dispatch, with ambiguity disabling the action.
   Verify one real root-conversation submission for each action, including
   short, long, and Unicode instructions. Stop before activation if evidence
   is insufficient.
2. **Bind and deliver actions.** Connect a narrow desktop-authorized action
   to the existing bounded runtime input owner. Test stale and foreign
   targets, invalid prompts, unqualified states, simultaneous actions,
   prompt changes between enqueue and dispatch, requester cancellation,
   and lost or partial acknowledgements. Verify separate paste and Enter
   ordering if needed, including recovery of a partial draft. In a controlled
   PTY, each eligible action generates one intended submission. Rejection
   before the first write produces zero input.
3. **Commit the explicit title.** Apply the reviewed title through the
   existing title transaction after acknowledged delivery. Test failed and
   ambiguous writes, a manual rename, a reset or runtime replacement before
   title settlement, pending native and Peon updates, exact Resume, fallback
   Resume and rollback, older records, and desktop publication. Assert that
   Keep going preserves placeholder and native titles and that action text
   bypasses generic metadata, inference and diagnostics. A post-delivery
   title failure preserves the delivery outcome and never repeats the prompt.
4. **Wire the desktop.** Bind the composer and actions to the selected
   Terminal using narrow preload and main-process methods. Verify focus,
   keyboard navigation, compact layout, stale selection, duplicate clicks,
   actionable errors, and the existing ended-session Resume flow in the
   desktop fixture. Run the scoped desktop and sidecar checks and repository
   verification. Use the applicable review gate for the current diff.

## Plan review

**Disposition** — Ready for design review. Implementation needs owner approval
and production activation needs prompt-readiness evidence.

- **Dependencies 3** — Desktop, sidecar and Codex behavior must agree.
  Existing PTY acknowledgement is available. An empty-editor signal has
  yet to be qualified.
- **Blast radius 3** — The additions share the terminal input owner.
  Scope the new authority to desktop actions, the selected session and a
  qualified owned runtime.
- **State changes 2** — One backward-compatible title marker and temporary
  delivery ownership. Earlier metadata has the existing behavior.
- **Reversibility 2** — Disable the actions and restore the previous naming
  policy by reverting the feature. Prompts already delivered remain in
  their conversations.
- **Uncertainty 4** — The existing input gate and writer acknowledgement
  cover local delivery ordering. Live Codex evidence is still required
  for empty-editor and modal discrimination, including native input that
  did not originate in an OrkWorks button.
