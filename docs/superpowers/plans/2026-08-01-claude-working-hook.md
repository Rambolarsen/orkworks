# Claude Code working hook

## Harness adapter note

- Harness ID / adapter: `claude-code` / Claude integration
- Launch: `claude`; resume: `claude --resume <session-id>`; latest fallback is unchanged.
- Native ID: Claude hook JSON `session_id`, source `claude_hook` (0.98 confidence).
- New signal: owned `PreToolUse` reports `working`; existing `Notification` continues to report `waiting_for_input`.
- Capacity: terminal limit patterns remain the only cap setter. A timestamped `working` event clears Claude's shared stale latch only when no newer capacity latch exists.
- No user approval is needed for the existing integration install action; it retains its normal config-ownership checks.
- Tests: Claude integration merge/probe/remove; attention handler latch clearing and stale-event protection.

## Steps

1. Install and reconcile both owned Claude hook groups.
2. Pass the hook status through the shared POSIX and PowerShell reporters.
3. Clear stale Claude capacity latches from a trustworthy, ordered working signal without allowing list snapshots to overwrite the reset.
4. Run the sidecar suite and integration review.
