// TODO(#271): derive this from a backend-declared event-semantics field on
// the integration status instead of a per-harness special case here — Codex
// and OpenCode are the only integrations today whose hook doesn't mean
// "needs input" (both report a session ID only; see issue #110).
export function isAttentionSignal(harnessId: string): boolean {
  return harnessId !== "codex" && harnessId !== "opencode";
}
