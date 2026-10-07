# Codex native approval status verification

Tracked by [#690](https://github.com/Rambolarsen/orkworks/issues/690).
Contract: [approved design](../specs/2026-10-03-codex-native-approval-status-design.md).

## Disposition

Implementation approved by the repository owner on 2026-10-05, including the
experimental upstream dependency. Native startup and installed diagnostic execution are blocked pending the
owned-listener contract in [#763](https://github.com/Rambolarsen/orkworks/issues/763).
Do not run the future operator procedure below on this branch.
This record does **not** qualify a shipping
compatibility entry. Live signal/configuration/lifecycle gates remain open.

## Exact source audit

Candidate executable: Codex CLI **0.160.0**, macOS arm64.
Pinned upstream source: `a956835d020762cb2b570053af06f643a11c0ecc`
(the dereferenced `rust-v0.160.0` tag).

- [Thread status](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server/src/thread_status.rs#L438)
  derives waitingOnApproval from pending user-facing permission requests.
  [Approval routing](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/core/src/tools/approvals.rs#L505)
  invokes Guardian review first and sends a user-facing request only on fallback.
  Source supports distinguishing automatic review from a pending native prompt;
  fresh independently correlated slow-auto behavior is still required.
- [Loaded threads](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/core/src/thread_manager.rs#L1620)
  exclude internal registrations, including Guardian reviewer sessions.
  Ordinary child threads remain relevant to conservative eligibility.
- [Root session construction](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/core/src/session/session.rs#L904)
  makes the root session ID equal its thread ID, while descendants inherit the
  agent-control session identity. [Hook construction](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/core/src/hook_runtime.rs#L155)
  reports that session identity. The observer must require exact accepted hook
  ID == thread.id == thread.sessionId, absent/null parentThreadId, and root
  source `cli`/`mcp` for this protocol. Tree-shared sessionId alone is insufficient.
- The installed CLI's generated [ThreadReadResponse schema](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server-protocol/schema/json/v2/ThreadReadResponse.json)
  matches the pinned schema, including optional nullable parentThreadId.
  Loaded-list completeness still requires explicit terminal nextCursor=null,
  within the 64-ID bound, both before and after exact-root status read.

A fresh-context adversarial source review separately checked the root identity
chain and internal reviewer exclusion. It did not replace live approval tests.

## Remote capability environment audit

Pinned native TUI requests do not automatically copy its process environment
into thread/start, resume or turn/start. The remote token is used for the
WebSocket handshake; execution configuration overrides are selected config
fields. The separate command/exec environment map is explicit; inspected
production callers add named Git/GitHub overrides, not inherited variables.
Native server environment construction must still exclude the TUI capability,
and live effective-config/isolation tests remain required.

The upstream remote endpoint/connection-args types derive raw Debug containing
the token. No direct production formatting or persistence callsite was found
in the focused source audit. This is a latent diagnostic exposure risk; it is
not proof of an active leak or a whole-program logging guarantee. The OrkWorks
owner/client must omit secret Debug and mark the Authorization header sensitive.

## Authenticated loopback preflight

A temporary private server used an OS-selected IPv4 loopback port and fresh
256-bit memory-only capability. It received only a SHA-256 digest in arguments.
The test made initialize/initialized and thread/loaded/list requests only; it
started no conversation and loaded no transcript.

| Check | Observed result |
| --- | --- |
| Missing bearer capability | HTTP 401 |
| Wrong bearer capability | HTTP 401 |
| Correct capability initialize | Accepted |
| Loaded IDs | Empty |
| nextCursor key | Present and null |
| Owned server after cleanup | Stopped |

No plaintext capability, native ID, prompt, tool input, or raw frame is retained
in this record. This verifies basic transport authentication/readiness only.
It does not verify port-race retry, effective config, or approval behavior.

## Passive initialization contract

The installed 0.160.0 response has exactly `userAgent`, `codexHome`,
`platformFamily` and `platformOs`. An initial custom observer name did not
produce the fixture-assumed CLI version prefix. The pinned
[initialize processor](https://github.com/openai/codex/blob/a956835d020762cb2b570053af06f643a11c0ecc/codex-rs/app-server/src/request_processors/initialize_processor.rs#L19)
explains why: ordinary names mutate global originator/user-agent metadata and
may enable implicit gateway login before the TUI initializes. Its explicitly
non-originating `codex_app_server_daemon` name avoids those originating-client
branches for this pinned protocol. No additional capabilities are requested.

A fresh read-only preflight using that name accepted authentication, returned
`codex_cli_rs/0.160.0` with the exact key set, absolute home and matching
unix/macos platform fields, and returned an empty explicit-null loaded page.
Missing/wrong capabilities still returned 401; the owned server stopped and
no conversation started. This validates the initialization expectation only;
it does not qualify a compatibility entry or establish full configuration parity.

The raw-source download path was blocked by the shell domain allowlist.
The authorized `gh api` path successfully retrieved the exact pinned public
source. This was a transport restriction, not an authentication failure.

## Baseline checks

`rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd codex`
passed before implementation: **112 passed, 0 failed, 1 ignored**.

## Reducer verification

Commit `550637cf` added the pure correlation reducer. Its implementation report
records **30 passed, 0 failed** for the focused `codex_approval` test filter,
including deadline, overlap, stale/disconnected observations and exhaustion
cases. Fresh task review approved the reducer within that scope.

The integration review identified required owner acknowledgements for failed
show/clear persistence. Backend commit `4239598f` adds those acknowledgements,
cooperating-writer ownership, the private authenticated callback and owned
runtime integration. Failed writes retain the original candidate/deadline and
resolution proof; fresh application checks run before persistence commit.
Fresh backend review found a timer that omitted metadata-read time; correction
`75365b95` now reads the actual monotonic clock after loading and before rename,
and scoped re-review confirmed the correction. The later transport checkpoint
below wires the relay/reporters; these checks do not establish the user-visible fix.

## Backend verification checkpoint

For corrected backend commit `75365b95`, the full sidecar test run passed
**1,773 unit tests and four reporter integration tests**, with four ignored tests. Build, formatting
and Clippy invocations exited successfully; Clippy reported repository warnings
and dead-code warnings for the callback awaiting transport wiring. This is not
a warning-free result.

Focused tests passed for 34 reducer and eleven application cases, 31 native
protocol/launch cases, and an owned lifecycle matrix covering exact resume,
TUI-first exit, server-first loss, renderer detachment and workspace shutdown.
The failed-readiness fixture checks acknowledged cleanup before returning.
The commit-time persistence fixture stages a record, rejects expired freshness
before rename, preserves the prior tuple/token and removes its temporary file.
A FIFO regression also delays the actual metadata read by 200 ms after an
observation is already 200 ms old; the corrected implementation rejects the
clear before rename and preserves the live wait. These use fake native
server/TUI fixtures, not a real Codex model conversation.

## Private transport verification checkpoint

Commit `8c75ca44` wires closed authenticated approval envelopes through the
existing local mailbox. Reporters route PermissionRequest privately only when
the owned runtime marker is present and atomic publication succeeds; otherwise
they preserve immediate legacy HTTP attention. Stop/UserPromptSubmit retain
immediate HTTP transitions and supply matching private boundary evidence.
Direct Pre/Post remain capture-only. Direct launches strip the native marker.

The final local run passed **1,781 unit tests and four reporter integration
tests**, with four ignored tests. The POSIX reporter suite and nine OpenCode
reporter tests passed. Build, formatting, Clippy and whitespace checks exited
successfully, with existing repository warnings. Fresh transport review found
that private correlation timestamps could suppress both copies of an older
trusted boundary. Correction `3841ba93` separates correlation from attention
ordering and records validated boundary floors before rejecting a stale
transition. New actual handler/relay permutations cover Pre/Post, both
boundaries and delivery orders, preservation of newer accepted attention,
and rejection of invalid or input-obsolete boundary proof. The corrected full
run passed **1,784 unit tests and four reporter integration tests**, with four
ignored tests; 53 focused approval tests passed. Build, formatting and Clippy
exited successfully with existing warnings. Scoped fresh re-review confirmed
the finding addressed and identified no new Critical/Important breakage.

Behavioral regressions cover fixed first receipt across activation retries,
changed retry bytes revoking correlation without overwriting a live wait,
retained overflow files preserving Permission delivery and fair identity
admission, bounded scanning, and old Permission reports after accepted trusted
HTTP/private boundaries. Overflow conservatively disables correlation for the
remaining relay generation rather than renewing a grace deadline.

An actual Windows PowerShell subprocess fixture and exact CI discover/run step
were added. PowerShell is unavailable locally, so execution remains a CI gate.
No Windows native launch support or installed approval behavior is claimed.
The compatibility table remains empty and the independent clear gate remains
false.

## Unfenced supported metadata producer

The implementation audit found that the MVP explicitly permits agent-written
session JSON. `MetadataStore::read_session` accepts those records without a
cooperative writer lock. An opaque per-store revision fences sidecar writes;
a file identity detects a completed external replacement, including identical
values. Neither makes a final check and rename atomic with an arbitrary direct
JSON writer.

The approved design requires native clearing to remain disabled if any accepted
attention/source writer is unfenced. Production clear therefore needs a separate
gate from version eligibility. The implementation preserves existing direct-JSON
authority and does not silently impose a lock or migrate the protocol. A written,
reviewed producer/ownership contract is required before enabling clearing;
[#761](https://github.com/Rambolarsen/orkworks/issues/761) tracks that prerequisite.

## Listener ownership review

The complete-branch review found that the released reservation allows an
accepting competitor to receive the bearer before initialize validation.
Owned-child liveness proves neither bind success nor endpoint identity.
The earlier missing/wrong-token and 401-competitor checks do not establish
client capability confidentiality. Native startup and the installed
diagnostic therefore require a separate closed gate before bearer delivery.
[#763](https://github.com/Rambolarsen/orkworks/issues/763) tracks the written
owned-listener contract. This record assumes no upstream ready-output,
port-zero, inherited listener or additional RPC solution.

## Final corrective verification

The accepting-competitor regression first reproduced bearer receipt with
boolean-only evidence and a shape-correct response while the fake owned child
remained alive. The guard now rejects before port reservation, capability
creation, argument augmentation or native process spawn. Ordinary plans have
no listener proof; production cannot mint the private cfg(test) marker used
only by controlled launch fixtures. The installed diagnostic constructor
returns Unavailable before probes or plan construction.

The regression passes after rejection and independently checks that the
retained competitor still answers an unauthenticated initialize request.
No bearer was received, no native/TUI readiness accepted, and no owned
process spawned. Focused checks passed for 33 native, two owned-lifecycle and
27 broader diagnostic tests. The serial full run passed **1,794 unit tests
and four reporter integration tests**, with five ignored. Build, formatting
and whitespace checks passed. Clippy and scoped final re-review are recorded
in the final PR validation; this record does not assert a warning-free run.

No installed conversation, login or native approval gate ran. Closed startup
is the disposition of the review finding; listener ownership remains an
unimplemented prerequisite, not verified by those fake results.

## Open production gates

- Effective model/approval/sandbox/config parity between direct and native paths,
  including ordered shared options and unchanged selected-model metadata behavior.
- Manual approval held beyond two seconds, one approval click, working before
  long execution completes, and prompt usability during observer disconnect.
- Independently correlated actual automatic Pre/Permission/Post invocation,
  no approval click, review exceeding two seconds, long execution without false wait.
- Root/subagent and overlapping prompts proving no independent pending wait clears.
- Owned process descendants, startup/cancellation/failure in either order,
  detached renderer and simultaneous session isolation on each enabled platform.

Unsupported versions, platforms and configurations retain direct launch. No
shipping record is permitted while its required gates remain open.

## Prepared operator diagnostic

Local diagnostic checkpoint `1f87ad2` adds an explicitly ignored test using the
actual owned runtime, generated reporter, private relay and approved-input
bridge. The final serial run passed **1,792 unit tests and four reporter
integration tests**, with five ignored tests; 14 focused diagnostic tests
passed. Build, formatting and Clippy exited successfully with existing
warnings. The installed test was listed, compiled, and **not executed**.
Fresh task review approved the local diagnostic subset with no findings.
Installed behavior remains unexecuted. The checkpoint commit IDs in this record
refer to the local implementation before rebasing onto current main; the PR
records the final reviewed head.

Fake tests cover actual passive-client closure while TUI input remains usable,
ordinary disconnection revocation, input-driven working with zero native
clears, raw terminal restoration, EOF/deadline/input/error exits, current
reporter installation and final persistence/capability cleanup. A test-only
final-drain acknowledgement supplements Ended, which precedes that drain.
The exact-correlation counter reuses existing reducer identities and counts
only a first explicit matching Pre/Permission/Post chain on an eligible,
unlocked candidate; missing IDs, overlap, disconnect and repeats do not count.
Field-presence counts are separate and do not prove invocation identity.

### Withheld operator procedure

**Blocked by #763. Do not run these setup/login/diagnostic commands yet.**
They document the future exercise after an approved listener-ownership
implementation and review. The current installed path rejects before
bearer delivery; no fixture-only bypass may be used with an installed binary.

### Future setup in an operator terminal

These future steps are for the operator after #763 is implemented and
reviewed, from the fix checkout. They
start no automatic prompt or approval answer. The candidate is the canonical
Codex **0.160.0 on macOS arm64**. Other platforms/configurations remain unverified.
Create dedicated homes, without copying or symlinking normal credentials,
configuration or rollouts:

```bash
ORK690_LOGIN_HOME="$(rtk proxy mktemp -d /private/tmp/ork690-login.XXXXXX)"
ORK690_AUTH_HOME="$(rtk proxy mktemp -d /private/tmp/ork690-auth.XXXXXX)"
ORK690_BINARY=/opt/homebrew/Caskroom/codex/0.160.0/bin/codex
```

Write and review `config.toml` in `ORK690_AUTH_HOME` with the intended persistent
auth store, model/provider, reviewer, approval policy, sandbox and hooks
settings. Ephemeral auth cannot be assumed to survive a separate login process;
Keyring/Auto depends on canonical CODEX_HOME. Apply any intended
[hook network settings](../../user/coding-tools.md#codex-hook-reports) to this
dedicated home. A minimal diagnostic configuration and clean shell environment
are not proof of ordinary configuration parity.

Log in using the exact binary; keep login output in the operator terminal.
Login-status output is discarded because it can include partial key text:

```bash
rtk proxy env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$ORK690_LOGIN_HOME" CODEX_HOME="$ORK690_AUTH_HOME" SHELL=/bin/sh ENV=/dev/null BASH_ENV=/dev/null ZDOTDIR="$ORK690_LOGIN_HOME" "$ORK690_BINARY" login
rtk proxy env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$ORK690_LOGIN_HOME" CODEX_HOME="$ORK690_AUTH_HOME" SHELL=/bin/sh ENV=/dev/null BASH_ENV=/dev/null ZDOTDIR="$ORK690_LOGIN_HOME" "$ORK690_BINARY" login status >/dev/null 2>&1
```

Record only the status exit result. Build under the normal development
HOME/toolchain, then select the executable printed by Cargo:

```bash
rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd runtime::session_runtime::native_diagnostic::installed_codex_native_approval_diagnostic --no-run
```

The verified local artifact currently has suffix `orkworksd-c7bd17e6c11e21a6`.
Use the newly printed path if rebuilding changes it. In the same interactive
terminal, run only the ignored test; do not tee, redirect or record its TUI:

```bash
ORK690_TEST_EXECUTABLE="$PWD/crates/orkworksd/target/debug/deps/orkworksd-c7bd17e6c11e21a6"
rtk proxy env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$HOME" TERM=xterm-256color LANG=C.UTF-8 ORK690_DIAGNOSTIC_BINARY="$ORK690_BINARY" ORK690_DIAGNOSTIC_CODEX_HOME="$ORK690_AUTH_HOME" "$ORK690_TEST_EXECUTABLE" --exact runtime::session_runtime::native_diagnostic::installed_codex_native_approval_diagnostic --ignored --nocapture --test-threads=1
```

The test supplies a disposable Git workspace, fake HOME, actual hook assets,
new reporting capabilities and clean shell choices. It never deletes the
authenticated home. System/managed policy and login-shell startup remain
separate authorities; their parity remains unverified.

### Exercise and evidence

Trust the displayed fixture workspace and all six current entries in native
`/hooks`, then use `/new` through the bridge to obtain a fresh SessionStart.
Require `root_bound=true`; submitted diagnostic input must also establish
`hook_active=true` before interpreting approval results. Trust after startup
is not a retroactive SessionStart. The operator supplies every conversation
input and approval action. Do not save an approval rule.

- Hold a manual approval beyond two seconds and observe native status/Needs You.
  Approve once during a long tool execution. Input-driven working is counted
  separately and cannot establish native clearing.
- Ctrl-\ toggles passive-client pause. Require `observer_paused=true`, check
  the held native prompt remains usable, then resume conservative observation.
- In a separately chosen automatic-review configuration, check review beyond
  two seconds, no approval click, exact correlated Post count and no independent
  input clear. Counts alone do not establish review duration or reviewer type.
- Ctrl-] exits the diagnostic. Ctrl-C is forwarded to the native TUI. The
  15-minute monotonic deadline is a failed gate, followed by bounded owned
  cancellation/cleanup checks; stalled delivery plus cleanup may add 20 seconds.

Transcribe only allowlisted counts, enums, booleans, durations and operator
outcomes. Hook rows are Pre, Permission, Post, Stop and UserPromptSubmit;
columns are Accepted/Retry/Rejected. Effect rows are ShowWait/ClearWait;
columns are Applied/PersistFailed/Rejected. Never export raw histories,
frames, IDs, capabilities, prompts or tool input.

Success removes only the disposable fixture after ended lifecycle, credential
revocation, final drain and terminal restoration. Failures retain private
artifacts; a cleanup timeout retains fake HOME through owned task destruction.
Retained histories are not evidence. Their later removal requires proof that
this exact test subprocess and its owned children ended. Login/auth homes
remain operator-owned.

This prepares a currently blocked signal/usability exercise. Native clearing still cannot pass
its production gate while #761 remains unresolved. Effective direct/owned
configuration equality, exact resume, installed root/subagent/overlap and OS
lifecycle proof remain open; the RPC allowlist was not expanded. No installed
gate is marked complete by compilation or fake results.
