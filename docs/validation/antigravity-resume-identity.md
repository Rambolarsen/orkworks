---
type: Validation Record
title: Antigravity resume identity evidence
description: Bounded 1.5.18 resume hook captures, reporter replay, and the remaining native attention qualification limits.
tags: [orkworks, antigravity, resume, hooks, validation]
status: draft
---

# Antigravity resume identity evidence

Supports [#1996](https://github.com/Rambolarsen/orkworks/issues/1996) and the
[Antigravity integration contract](../agents/harness-integration-contracts.md).
These are agent-operated macOS arm64 CLI 1.5.18 fixtures, captured on 2026-11-27
against source `f9391966`. They do not promote the integration beyond limited
coverage or qualify resumed desktop attention.

## Scope and isolation

The owner directed verification of resumed-session identity and native attention.
A new disposable Git workspace held only three silent diagnostic handlers:
SessionStart, PreInvocation and Stop. Existing workspace/global hooks were not
edited. The pinned installed-artifact copy matched SHA-256
`89d0b97a9ebcefcaebbdfe4ffeb2de877ef92ee4504de188e35a38489136be13` and reported
exactly 1.5.18 through a successful bounded version probe. The runner excluded
inherited reporting capabilities and model/proxy credentials, retained normal
native saved login/configuration, and suppressed updates only for this test.
No OrkWorks sidecar or Peon participated in these probes.

The initial budget was three native starts, at most one prompt per start,
75 seconds per headless start and 180 seconds for the interactive probe. After
the interactive exact-resume start stopped at startup permission, one additional
headless exact-resume start was explicitly bounded to the same 75-second cap.
Four starts and three no-tool prompts were used in total; no command or trust
approval was sent. Model-call accounting was not measured.

The [resume documentation](https://antigravity.google/docs/cli/commands/resume/)
describes `--continue` as workspace-cache selection and `--conversation` as
explicit-ID selection. Before continue, the runner checked only the new
workspace's entry in the native cache and required it to equal the freshly
captured ID. The exact target was that same privately retained fresh ID. No
foreign or old fixture session was selected. These documentation claims and the
[hook reference](https://antigravity.google/docs/hooks.md) are current discovery
sources; the observations below bind only to the pinned artifact and modes.

## Captured lifecycle

| Trial | Mode | Hook sequence | Identity result | Root outcome |
| --- | --- | --- | --- | --- |
| Fresh | `--print`, one no-tool prompt | SessionStart, PreInvocation, Stop | Established the private fixture ID | Exit 0 in about 7.4 seconds |
| Continue | `--continue --print`, one no-tool prompt | PreInvocation, Stop | Both IDs matched fresh; cache checked before launch | Exit 0 in about 17.3 seconds |
| Exact, interactive | `--conversation=<fixture-id>`, no prompt sent | No lifecycle capture before startup block | Resume identity not established by this trial | Collector interrupted; root exit code not retained |
| Exact, headless | `--conversation=<fixture-id> --print`, one no-tool prompt | PreInvocation, Stop | Both IDs matched fresh | Exit 0 in about 7.5 seconds |

Neither completed resume emitted SessionStart. Each first PreInvocation carried
`invocationNum: 0`, and each Stop carried `fullyIdle: true` with
`terminationReason: NO_TOOL_CALL`. All retained lifecycle payloads carried the
same native conversation ID and fixture cwd, and workspacePaths included the
fixture. The [derived record](antigravity-resume-1518-2026-11-27.json) replaces
identities and paths with fixed fixture values and omits arbitrary field values.
It retains observed key names and narrow scalar fields, not raw hook input.

The result refines the issue's original request for resumed SessionStart
payloads: in these headless 1.5.18 trials, resume identity arrived through the
first PreInvocation instead. It does not prove that all interactive resumes or
other versions omit SessionStart.

## Reporter verification

The current POSIX reporter was replayed against all seven sanitized observed
payload shapes with synthetic session credentials and a local curl stub. Fresh
produced four expected reports: SessionStart identity, first-PreInvocation
identity plus Working, and Stop Idle. Each completed resume produced three:
first-PreInvocation identity plus Working, and Stop Idle. The identity source
was `antigravity_hook`, confidence 0.98; report cwd matched the fixture. All
seven invocations exited successfully with empty stdout and stderr. This tests
the existing `invocationNum: 0` identity fallback without changing code.

The existing `report-harness-event.test.sh` suite also passed. This is reporter
replay with intercepted HTTP, not delivery to an authenticated live sidecar,
metadata identity retention, or proof that installed hooks executed.

## Native attention and cleanup limits

The interactive exact-resume probe requested brief native OSC 7501 emission.
It captured root `blocked/auth` and `blocked/permission` frames during startup,
then stopped without sending a prompt or granting trust. It did not capture a
resumed question, permission rejection or Working/Idle clearing transition.
Those resumed native attention paths remain unqualified. The existing
[fresh-session UI evidence](antigravity-native-status.md) retains its scope.

All three headless root exits were collected as zero. The interrupted interactive
collector did not retain a root exit code; its recorded elapsed value is only
the last saved snapshot. Follow-up executable-path inventories found no process
using the unique native task path. The pinned hash remained unchanged, and the
temporary binary/private ID file were removed. Complete descendant cleanup is
not proven. Native history/cache created by the normal CLI remain under native
ownership; no global history or saved permissions were edited by the operator.

Remaining #1996 work includes Windows reporter execution and installed-hook
live delivery. A resumed native attention check still needs a fixture that can
reach a turn without expanding trust, with the target resolved explicitly before
launch. Published installers and human-operated confirmation remain unverified.


## Later trusted-workspace native check

The [exact-resume native status trial](antigravity-native-status.md#trusted-workspace-exact-resume-native-status-2026-11-27)
reached a structured question in the previously trusted disposable workspace,
using a new fixture ID. Native Working, question Blocked, Working and Done frames
were captured around Escape, and both roots exited zero. Interactive exact
resume again used first PreInvocation identity rather than SessionStart.
This adds native protocol evidence; it does not qualify resumed desktop display,
metadata identity/origin or authenticated installed-hook delivery. The earlier
new-workspace startup block and its limits remain recorded above.


## Later exact-resume desktop check

A separate [source-built desktop check](antigravity-native-status.md#source-built-exact-resume-desktop-question-check-2026-11-27)
created a new seed session, closed its runtime, then used its exact-session
Resume control. The resumed native history retained the seed context. A
structured question coincided with Needs You and Attention 1; Escape/Skip was
followed by Working, then Idle, with Attention 0. Peon was disabled. This adds
one resumed UI cancellation path without retaining independent native-ID equality,
metadata-origin or reporter-transport receipts. Earlier probe sections retain
their original limits; resumed answer/command rejection and Windows remain
unqualified.
