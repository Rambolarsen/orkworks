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
| OpenCode 2.2.5 | A model-free private-server fixture observed independent root B / child C locations and persistence. A separate fixture observed a root-only marker pass from the native context hook to the serialized request, with no child marker and zero transport attempts. | Context serialization is established only for an explicitly fixture-pinned root. Production root capture, native child-tool spawning, async ordering, model comprehension and automatic task-location authority remain unqualified. Plugin-instance location remains insufficient. |
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
approved one corrected execution, the runner generated a fresh random password
in memory for its private loopback server/client and the fixture completed.
The runner did not print or persist that password; all native storage was
disposable and removed afterward.
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


## OpenCode context serialization fixture

On 2026-11-04 the owner separately approved one native OpenCode 2.2.5 context
fixture, bounded to 90 seconds, one private-server start and two fixture-only
prompt admissions. The execution passed with four derived records and zero
requests reaching its owned rejecting HTTP endpoint. All matching disposable
fixture directories were absent after cleanup.

The fixture reused isolated XDG storage, temporary Basic authentication and the
strict inherited-environment allowlist from the session API fixture. It selected
only a fake custom provider/model explicitly for both sessions. Its endpoint was
an owned loopback HTTP stub, and its provider key was a fixed fake value. No
inherited provider credential, reporting capability, account change or paid
inference was involved. Warming was disabled, title generation was supplied
locally, retries were vetoed, outgoing tools were removed and WebSocket dispatch
was refused. Raw native output and serialized bodies were not retained.

A disposable plugin used the documented
[context and HTTP hooks](https://opencode.ai/v2/docs/build/plugins/server):

- The native `context` hook read the exact session. Only the fixture-pinned
  root with matching read-back identity and no parent received a harmless marker.
  The root observation matched B/task; the child matched C/other and the expected
  parent relation, and was ineligible for marker injection.
- A separate native `http.request` hook examined the assembled primary request
  before transport. The root request contained the marker; the child request
  did not. Both targeted the owned loopback endpoint.
- The HTTP hook intentionally threw before dispatch. No request reached even
  the local stub, and no model response or successful model turn is claimed.
- The retained records contain only stage/scope/location labels and booleans for
  identity, parent relation, eligibility, request kind/target and marker
  presence. Native IDs, prompts, serialized bodies and passwords were not printed.

Offline checks separately exercised root/child/peer discrimination, mismatched
read-back identity and parent refusal, serialized marker detection, dispatch
stopping, wrong-target refusal, privacy and retry veto. Those
checks are not substitutes for the native observations above.

This establishes a version-specific text-injection path through OpenCode's
request assembly for a root selected explicitly by the fixture. It does not
establish actual agent comprehension, helper-command compliance, native
child-tool spawning, pre-creation instruction inheritance, event ordering,
Windows behavior, or production root-ID capture/admission. The fixture's scope
is `fixture-pinned-only`, with `diagnostic-only` authority; it never selected a
production session or enabled an adapter.

The built-in source launch/resume templates already use a private server; see
[OpenCode server isolation](../agents/harness-integration-contracts.md#opencode-server-isolation).
Custom command arrays and existing sessions retain the limits recorded there.
Using the production reporter's captured identity to select a recipient would
require separate correlation and child-exclusion evidence. The independent
launch-selector proposal below avoids that dependency. No automatic directory
update is authorized by this run.

## OpenCode CLI-selected root and capture audit

On 2026-11-04 the owner approved one further 90-second qualification with the
normal interactive OpenCode 2.2.5 CLI, a fresh isolated native store, one private
server start and one fixture-only startup prompt. Before launch the runner
selected a fresh `ses_` plus 32 hex digits and supplied literal `--session <id>`.
No API-created native session or preexisting conversation was substituted.

The native context ID equaled that candidate. Exact-session read-back matched,
parent was absent, location was task B, and the primary serialized request
contained the fixture marker at the owned local endpoint. Two derived records
were retained; no request reached the rejecting stub. Dispatch was deliberately
stopped, with no real model response/inference. Raw PTY output was discarded.
Cleanup left no matching native fixture directories. This proves the selected
ID creation path in this version/platform, not an OrkWorks production adapter.

A separate deterministic replay executed the unchanged production reporter with
in-memory intercepted fetch and synthetic events; no native harness or sidecar
mutation was involved. Its SHA256 was
`8945a37e648fcd92c8fce6e5d62e74c8d388ba389fde192383c1ca8af5b07f20`,
also matching fetched main `f11e325f6c442f4bf17b1b7e35c76a14d6587d74`:

| Replay | Captured scopes | Parent API lookups |
| --- | --- | --- |
| Child step before root step | child | 0 |
| Root step, then child `session.created` with parentID | root, child | 0 |
| Root step, then peer `session.created` | root, peer | 0 |
| Root step, then child step | root | 0 |

These are source counterexamples to treating capture as proof of the intended
root, not a claim that every event order was observed natively. Sidecar source
inspection found the replacement refusal is Codex-only; equal-confidence OpenCode
reports have no equivalent immutable-root fence. The current exact-resume native
store check establishes ID existence, not parentage. Existing identity behavior
is not qualified or repaired by the successful CLI/context fixtures.

The proposed [instruction-only scope design](../superpowers/specs/2026-11-04-launch-scoped-worktree-instructions-design.md)
uses independent launch intent for context delivery while preserving those
existing identity/attention contracts. It requires owner review before runtime
implementation. No source adapter or automatic directory authority is enabled.

## Launch-scoped source preparation, 2026-11-05

The owner approved the [instruction-only design](../superpowers/specs/2026-11-04-launch-scoped-worktree-instructions-design.md)
and implementation plan. Source now carries explicit new/exact intent through
one runtime generation, qualifies source-owned commands and current confined
assets, and applies the CLI ID and selector env together before spawn. Inherited
selector names are filtered for every harness. Normal OpenCode reconciliation
also installs the shared helper; foreign plugin ownership remains a refusal.

The real reporter has an independent bounded context hook. Its subprocess suite
verifies actual narrow helper output for the selected root, child/peer exclusion,
read-back mismatch/parent refusal, capture-order independence, cleanup, one
unresolved lookup, helper drift/output/timeout failures and hook-registration
failure without disabling legacy reporting. Private Rust tests cover argv/env
coherence, exact resume, explicit models, profile/version/platform refusal, asset
ownership/drift and randomness failure. Runtime-startup tests additionally hold
preparation pending, cancel or replace the generation, and assert no spawn
boundary is reached; success and drift exercise actual pre-spawn argv/env selection.
Their non-executable fixture stops before any child launch.

Production preparation remains disabled pending actual-source native
qualification. An ignored diagnostic can export the actual prepared argv/env
for a disposable native fixture; normal tests do not select it. No live run of
this changed reporter/helper is claimed here. Existing sessions and published
installations retain the previously recorded limits.

## Actual-source preparation refusal, 2026-11-05

The owner authorized one further 90-second actual-source fixture after #1401.
Its frozen Rust exporter exited101 during preparation, before the interactive
CLI start or any fixture prompt. No native context/serialized records were
produced. The temporary fixture directory was removed; cleanup inspection found
zero matching native fixture directories. The exception is consumed; production
preparation remains disabled and no native retry has been performed.

The runner discarded exporter stderr, so this result alone does not identify
which precondition refused. Offline inspection found a concrete version-format
mismatch: the adapter required bare `2.2.5`, while the pinned CLI uses Effect
4.0.6's default `formatVersion`, which emits `opencode v2.2.5` with optional SGR.
The installed binary contains the same formatter and `opencode` command name.
Primary sources: [OpenCode v2.2.5 CLI runtime](https://github.com/anomalyco/opencode/blob/v2.2.5/packages/cli/src/framework/runtime.ts),
[command definition](https://github.com/anomalyco/opencode/blob/v2.2.5/packages/cli/src/commands/commands.ts),
and the published [Effect4.0.6 source artifact](https://registry.npmjs.org/effect/-/effect-4.0.6.tgz),
`src/cli/GlobalFlag.ts` and `src/cli/CliOutput.ts`. This is source evidence,
not a retained raw output from the failed probe.

The corrective regression now admits the exact pinned plain/SGR banner while
continuing to refuse prereleases, build suffixes, other versions, unrelated
command names and additional log text.

The corrected fixture retains at most 4 KiB of derived checkpoints: preflight
qualification booleans, version availability/match/counts and a final scope
readiness boolean. Its strict offline parser rejects unknown fields, incorrect
types, counts and ordering. No raw native output or credentials are retained.

These facts distinguish preparation phases, not every rejection reason. A final
`scopePrepared: false` does not separate post-probe asset and executable drift;
an error return can stop before the final record. The last checkpoint is progress
evidence, not a unique diagnosis or a successful native qualification.
Actual native qualification and activation remain pending a corrected, reviewed
fixture and a new bounded exception.
