---
type: Validation Evidence
title: Harness workspace signal qualification
description: Live directory observations, startup-instruction checks and remaining harness coverage gaps.
tags: [harnesses, sessions, git, worktrees, validation]
status: draft
---

# Harness workspace signal qualification

Recorded 2026-11-03 for [#1347](https://github.com/Rambolarsen/orkworks/issues/1347)
and the [approved design](../superpowers/specs/2026-11-03-harness-workspace-signals-design.md).
The owner authorized continuing with disposable harness fixtures after the
initial evidence-tooling PR. No production hook configuration, native identity,
sandbox policy or another OrkWorks session was changed.

## Fixture and retention

A disposable Git repository had separate launch A, task B and other C checkouts.
The standalone probe classified observed directory fields against those roots.
Its wrapper retained at most 16 derived records and never stored raw hook
payloads, tool arguments, responses, native IDs or credentials. OrkWorks runtime
capabilities were removed from child harness environments. Model output was
reduced in memory to completion/tool-count flags, then discarded. Codex used
its ephemeral mode; Claude used no-session-persistence. These runs do not claim
that every harness has an ephemeral-storage option.

The fixture used installed CLI versions and POSIX hooks on macOS. It does not
qualify Windows live behavior, other versions, arbitrary plugins or async order.
The probe keeps `rootScope: unverified` even when IDs compare equal.

## Results

| Harness and version | Observation | Qualification limit |
| --- | --- | --- |
| Claude Code 2.1.309 | Launch `SessionStart.cwd` and first `PreToolUse.cwd` classified as A. After `cd` into B, `PostToolUse.cwd` classified as B and `CwdChanged` supplied A-to-B `old_cwd`/`new_cwd`. The next three tool pairs remained B, including an absolute-path read of a file in A. | Confirms these directory semantics in a sequential root fixture. It does not qualify async ordering or child exclusion; no automatic location source is enabled. |
| Claude Code 2.1.309, source reporter | A separate no-tool run received the modified reporter's narrow SessionStart instructions and returned the exact requested acknowledgement. The reporter used fake capabilities and an offline curl stub; no real metadata report was attempted. | Live context delivery, not proof that future agents comply or that installed helpers have updated. |
| Codex 0.166.0 | An ephemeral, read-only run completed exactly two shell calls with B then A as tool directories. Three item-completed and one turn-completed events were counted. No configured fixture hook records were received. | Hook execution/discovery/trust was not established. No trust bypass or persisted trust mutation was used. This result cannot qualify native directory semantics. Existing explicit helper reporting is independently observed in the owning session. |
| Copilot CLI 1.0.90 | The capped fixture exited before producing a hook record. A minimal capped no-tool run also failed; retained diagnostics identified a credit-related startup error without retaining raw messages. | Live behavior remains unverified. Automatic approval review rejected a retry without the original one-credit cap because that would introduce unbounded potential usage cost. No uncapped retry occurred. |
| OpenCode 2.2.5 | Installed version read successfully. | No live session/location run. Plugin-instance location remains insufficient under the native documentation. |
| Aider | Executable unavailable. | No live qualification. |
| Antigravity 1.1.22 | Version available. | No qualified hook/schema was identified; no live location run. |
| Generic shell / retired Gemini / custom | Existing fallback policy retained. | No new native source or universal hook claim. |

Primary contracts: [Claude hooks](https://code.claude.com/docs/en/hooks#cwdchanged),
[Codex hooks](https://learn.chatgpt.com/docs/hooks),
[Copilot hooks](https://docs.github.com/en/copilot/reference/hooks-reference), and
[OpenCode server plugins](https://opencode.ai/v2/docs/build/plugins/server).
These current documentation sources are separate from installed-version fixture
observations. Native directories do not establish task ownership.

## Delivered source behavior

The existing shared helper already accepts narrow authenticated worktree reports
from any owning harness context with the reporting capability. The source
reporters now deliver its worktree-only instructions through Codex, Claude and
Copilot startup hooks. Claude and Codex use the SessionStart `hookSpecificOutput`
envelope; Copilot uses root `additionalContext`. The two latter adapters do not
receive general metadata instructions even if a general metadata marker exists.
Missing capabilities and unrelated events emit no instruction context.

POSIX subprocess regressions cover the three formats, omitted capabilities and
unrelated events. A native Windows PowerShell fixture exercises the real helper
through a scoped Python launcher and passed native Windows CI on code head
`2b86b4375694dc13d38b662cfb4121d1cdc6f671`; it is skipped on macOS. Source code and synthetic fixtures do not establish live Copilot/Windows
qualification. Existing hook installation and trust/reconciliation rules apply.
Existing sessions receive new instructions only when the normal native startup
or resume event runs with the updated installed reporter; the app does not
restart or inject into an existing session automatically.

OpenCode, Aider and Antigravity do not gain an unqualified instruction channel.
Their agents may still invoke the same helper explicitly where their tool
context supports it. Process/launch fallback remains labeled. No event can
replace an explicit task binding, and no location update changes attention,
launch/resume directory or native identity.


## Verification of the delivery change

`bash scripts/verify-repo.sh` passed with pinned Node 22 in the owned worktree,
including Rust build/tests, OpenCode reporter tests, desktop type-check, native
ABI repair, desktop tests/build, docs build and close-out checks. The standalone
metadata helper suite passed 45 tests with its Windows-only fixture skipped on
macOS. Both new Workspace reporting CI jobs passed on Linux and Windows.

The required `/code-review low` approved code SHA
`2b86b4375694dc13d38b662cfb4121d1cdc6f671` against base `70d8e272` with no
findings. A later documentation-only verification update does not change that
reviewed code diff. The read-only native-app inspection timed out, so this record
does not claim live rendered Git-view verification.
