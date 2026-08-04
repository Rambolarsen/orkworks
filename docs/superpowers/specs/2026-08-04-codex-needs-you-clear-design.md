# Codex needs-you clear

## Problem

After accepted Codex terminal input, the process transition sets a session to `working`, but the asynchronous input-label inference can subsequently merge a full Peon inference and restore `needs_you`.

## Decision

Input-label inference may update the session label only. It must not merge status, attention, summary, or other Peon metadata. Output-triggered inference remains unchanged.

## Verification

Add a regression test that simulates an accepted input followed by label inference returning `waiting_for_input`; the session must remain `working` while its label updates.
