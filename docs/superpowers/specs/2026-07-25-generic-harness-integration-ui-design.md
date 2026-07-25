# Generic harness integration UI (Gemini, Copilot)

## Context

Issue #217: `apps/desktop/src/components/SettingsModal.tsx` only ever renders
integration-status UI (Notification hook install/uninstall, the Detected
badge, and the custom-path override added by the harness-tool-detection
work) for `"claude-code"`. Everything is hardcoded to that one harness: a
single `claudeIntegration`/`claudeIntegrationBusy` state pair, a `useEffect`
that fetches `getHarnessIntegrationStatus("claude-code")` only, and four
Claude-specific handlers (`installClaudeIntegrationHandler`,
`uninstallClaudeIntegrationHandler`, `saveCustomPathHandler`,
`clearCustomPathHandler`), all closed over the literal string `"claude-code"`
and the literal display string `"Claude Code"` in three places (checking-status
text, the hook-install confirmation copy, and the custom-path warning copy).

The backend has no equivalent gap. `crates/orkworksd/src/harness/integrations/gemini.rs`
and `copilot.rs` both use the same `JsonHookHandler` as `claude.rs` — identical
`IntegrationStatus` shape, identical `confirmation` semantics, identical
`tool_detected` population via the PATH-detection probe shipped in the
harness-tool-detection work, identical `IntegrationCoverage::Limited` and
probe-merge-remove wiring. (One inert difference: `gemini.rs` sets
`activation: IntegrationActivation::Unknown` where `claude.rs`/`copilot.rs`
both set `Active` — doesn't affect this UI, since `SettingsModal.tsx` never
reads `.activation`, only `toolDetected`/`registration`/`confirmation`/
`diagnostics`, all of which are genuinely identical across the three.) The
HTTP routes
(`/workspace/integrations/:harness_id/{status,install,uninstall}`) and the
Electron IPC bridge (`getHarnessIntegrationStatus`/`installHarnessIntegration`/
`uninstallHarnessIntegration`/`setHarnessCommandOverride`/
`clearHarnessCommandOverride`, all already `harnessId`-parameterized) are
already generic. This is purely a frontend gap.

## Goals

Add the same integration UI — Detected badge, Notification-hook
install/uninstall, custom-path override — for Gemini CLI and GitHub Copilot
CLI, reusing one component instead of copy-pasting the Claude block twice
more.

## Non-goals

- Codex, OpenCode, Aider. Codex/OpenCode hardcode `NotApplicable`/`Unknown`
  activation by design (no Notification-hook support); Aider has its own
  bespoke status logic. None fit this same UI meaningfully — out of scope
  for #217 and for this change.
- Any backend change. Confirmed unnecessary: `gemini.rs`/`copilot.rs` already
  implement the full `IntegrationHandler` contract.
- Generalizing to *every* harness with an `integration` capability via a
  loop. The three IDs are enumerated explicitly.

## Design

### New file: `apps/desktop/src/components/HarnessIntegrationSection.tsx`

A self-contained component:

```typescript
interface HarnessIntegrationSectionProps {
  harnessId: string;
  harnessName: string;
  harness: HarnessConfig | undefined;
}
```

It owns all the state and handlers currently inlined in `SettingsModal.tsx`
for Claude, renamed from the `claude*`/`custom*` prefixes to be
harness-agnostic, and parameterized by `harnessId`/`harnessName` wherever the
current code hardcodes `"claude-code"` or `"Claude Code"`:

- `integration`/`integrationBusy` state (was `claudeIntegration`/`claudeIntegrationBusy`)
- `customPathDraft`/`customPathActive`/`customPathBusy`/`customPathError` state (unchanged names — already generic)
- the fetch `useEffect`, keyed on `harnessId` in its dependency array instead of a `hasClaudeCodeHarness` boolean
- `installIntegrationHandler`/`uninstallIntegrationHandler`/`saveCustomPathHandler`/`clearCustomPathHandler`, each calling `window.orkworks.*("...", harnessId, ...)` instead of the literal `"claude-code"`
- the `looksAbsolute()` helper (moves here from `SettingsModal.tsx` module scope — it's only used by this component's custom-path logic)
- the JSX currently at `SettingsModal.tsx:436-532` (the whole `{h.id === "claude-code" && activeDraft.includes(h.id) && (...)}` block), with the three hardcoded "Claude Code" copy strings replaced by `{harnessName}`

`harness: HarnessConfig | undefined` is passed in (rather than having the
component look it up from a `harnesses` array prop) so `SettingsModal.tsx`
keeps sole ownership of harness data, matching how it already passes
`harnesses` down elsewhere — this component only ever needs its one harness's
`launch.command` for the custom-path prefill/heuristic.

**Accepted behavior change:** today, Claude's integration state lives in
`SettingsModal`'s own hooks, so it's session-persistent — merely JSX-hidden,
never unmounted, regardless of the "Active" checkbox. Moving it into
`HarnessIntegrationSection` makes it mount-scoped: unchecking a harness's
"Active" checkbox now unmounts its section, discarding any unsaved
`customPathDraft`/`customPathError`, and re-checking it remounts (refetching
status fresh). This is intentional, not an oversight — deactivating a harness
discarding an in-progress, unsaved edit to *that harness's* settings is
reasonable, and refetching on reactivation is arguably more correct than
serving a possibly-stale cached status. Not treated as a regression to guard
against.

### `SettingsModal.tsx` changes

- Delete everything enumerated above as "what moves in."
- Add a constant `const INTEGRATION_HARNESS_IDS = ["claude-code", "gemini", "copilot"];` near the top of the file.
- In the `harnesses.map(...)` row rendering, replace the `{h.id === "claude-code" && activeDraft.includes(h.id) && (...)}` block with:
  ```tsx
  {INTEGRATION_HARNESS_IDS.includes(h.id) && activeDraft.includes(h.id) && (
    <HarnessIntegrationSection harnessId={h.id} harnessName={h.name} harness={h} />
  )}
  ```
- Remove the now-unused `hasClaudeCodeHarness`/`claudeHarness`/`claudeLaunchCommand`/`claudeHasCustomPath` derived values (their equivalents move into the new component, derived from the `harness` prop instead of a `harnesses.find(...)`).

### Testing

Same story as the original detection feature: no existing React
component-render test setup in this repo. Verified via `tsc --noEmit` and a
manual browser check exercising all three harnesses (Claude Code, Gemini
CLI, GitHub Copilot CLI) — install/uninstall, Detected badge for both a
found and not-found case, and the custom-path override round-trip for at
least one non-Claude harness.
