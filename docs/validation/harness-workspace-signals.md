---
type: Validation Evidence
title: Harness workspace signal qualification
description: Live directory observations, startup-instruction checks and remaining harness coverage gaps.
tags: [harnesses, sessions, git, worktrees, validation]
status: draft
---

# Harness workspace signal qualification

Recorded 2026-11-03 and updated 2026-11-04 for [#1347](https://github.com/Rambolarsen/orkworks/issues/1347)
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
| Copilot CLI 1.0.90 | Capped fixtures produced no hook record. The owner subsequently approved exactly one no-tool request without the credit cap, bounded to 90 seconds; it exited 1 after 8.7 seconds with derived `401` and `quota` diagnostics and no hook record or acknowledgement. | Live delivery remains unverified pending external authentication/usage resolution. The one approved uncapped invocation was consumed; no credential change or further retry occurred. |
| OpenCode 2.2.5 | A model-free private-server fixture read exact-session parent/location fields, moved root A→B and child A→C independently, and observed B/C after one restart. An instruction entry attached to the root after child creation was present only on the root. | Native session API evidence only. No hook event, model-context delivery, async ordering or automatic task-location signal is qualified. Plugin-instance location remains insufficient. |
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
reviewed code diff. The initial read-only native-app inspection timed out.
The subsequent rebuilt-app check below supplies rendered Git-view evidence.


## Rebuilt desktop verification

On 2026-11-04 the rebuilt running desktop had the updated installed reporter.
The owning Codex session explicitly reported a disposable linked worktree B
through the installed helper from its actual tool context; launch checkout A
remained unchanged. Native UI inspection verified:

- Details displayed B's branch, dirty state and reported-task-worktree source.
- Uncommitted changes displayed B's working edit; Branch changes displayed
  only B's committed addition against the local `origin/main` ref.
- Comparison details displayed B's absolute root and reported-task-worktree source.
- A temporary read-only command in A left Changes on B after refresh.
- Intentionally reporting A switched Changes to main and cleared B's old diff.

The disposable worktree and branch were removed; the primary checkout stayed
clean. This establishes rendered behavior for the owning Codex session, not
universal harness startup delivery or automatic task-location discovery.

## Source lifecycle qualification

The isolated tests in
`crates/orkworksd/src/session_location/lifecycle_tests.rs` reopen the persisted
workspace into a fresh application state with no live runtime or reported cwd.
They check the task branch and agent source, retained unavailable bindings,
and resume through the shared application workflow using a fake executable.
The fake process records its actual cwd so launch-directory preservation is
checked independently of Git inspection. After ending that runtime, the tests
reopen persisted state and inspect the same task binding again.

This is source-level lifecycle coverage for format-1 metadata and the shared
resume path. It does not launch an installed coding harness, test native
conversation restoration, restart the production daemon, or qualify format-2
storage activation. Native identity, ordering and remaining instruction channels
retain the limits in the results table and tracking issue.


## OpenCode exact-session API fixture

On 2026-11-04 the owner approved a disposable model-free OpenCode 2.2.5
fixture, bounded to 90 seconds and one private-server restart. The first
execution stopped during readiness; its client omitted required Basic
authentication. It produced no session records. After the owner separately
approved one corrected execution, the client used a fresh random password
held only in memory for its own loopback server and the fixture completed.
See [standalone server authentication](https://opencode.ai/v2/docs/cli/web#authentication).
Neither execution changed an existing server credential or security protection.

The completed fixture used an independent temporary Git repository with linked
A/launch, B/task and C/other worktrees. XDG configuration, data, cache and state
were isolated beneath the disposable directory. No inherited OrkWorks capability,
provider credential or proxy was forwarded. Warming was disabled; no prompt,
generation, shell or tool endpoint was called. Raw server output was discarded.
The runner retained nine derived records containing only step labels,
identity/parent equality booleans, instruction-entry and timestamp-presence flags,
and A/B/C classes.
Native IDs, payloads and passwords were not printed or retained outside the
removed native fixture storage. Cleanup left no matching fixture directories.

Observed through the documented [session API](https://opencode.ai/v2/docs/api):

- Exact root reads matched the requested identity, had no parent, and initially
  reported A through `Session.Info.location.directory`.
- The child had its own identity and matched the expected root through
  `parentID`; it initially reported A.
- Moving the root to B and the child to C produced independent locations. A
  subsequent root read still reported B.
- A fake instruction entry attached to the root after child creation appeared
  in the root's instruction-entry list and was absent from that existing child.
  This does not test whether instructions present before child creation inherit.
- After stopping and restarting the private server against the same disposable
  storage, exact reads retained root B, child C and the expected parent relation.
- All seven retained session observations exposed an updated-time field. Presence alone does
  not establish an ordering fence or distinguish delayed hook observations.

These facts narrow installed-version session schema and persistence uncertainty.
They do not qualify `ctx.location`, native root-hook correlation, model-request
context injection, agent compliance, native model conversation restoration,
Windows behavior, or async event ordering. The probe's authority remains
`diagnostic-only` with `rootScope: unverified`. No production adapter or new
location authority was enabled; OpenCode agents still use explicit helper
reporting where available and the existing fallback otherwise.
