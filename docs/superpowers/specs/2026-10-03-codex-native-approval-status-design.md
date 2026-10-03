# Codex approval attention from an owned native runtime

Status: **proposed; written-spec review required before implementation planning**.
Tracked by [#690](https://github.com/Rambolarsen/orkworks/issues/690).
Architecture decision: [ADR 0076](../../adr/0076-codex-owned-native-approval-observer.md).
This document describes intended behavior; the shipped hook mapping is unchanged.

## Outcome and scope

For a supported, unambiguous Codex approval, leave attention unchanged for a
fixed two-second grace period. After that, show Needs You when fresh native
status confirms a pending approval. Clear that owned wait when native status
shows approval waiting has ended and the same conversation is actively running.
Continue observing throughout tool execution, including long-running tools.
Time is a debounce, not a classifier of human versus automatic review.

The native terminal retains approval control. OrkWorks neither answers requests
nor starts, resumes, or submits turns through its observer. Preserve exact
resume, hook trust, root identity, reporting capabilities, configured model,
working directory, environment isolation, and detached-terminal lifetime.
No renderer, public HTTP schema, shared daemon, or persisted native-status
registry is introduced. Queued questions remain outside this issue.

## Evidence and limits

The [investigation record](https://github.com/Rambolarsen/orkworks/issues/690#issuecomment-5973628408)
records the real captures and excluded attempts. The ADR 0051 serial capture
gate passed: both manual and automatic routes produced ordered PreToolUse,
PermissionRequest, PostToolUse sequences; pre/post shared tool_use_id, while
PermissionRequest lacked that identifier. permission_mode was default in both.
This establishes serial correlation, not parallel correlation or a reviewer
classifier. PR #702 remains capture-only.

A private Codex 0.160.0 server with a single native terminal and verified user
reviewer produced idle → active → active[waitingOnApproval] → active → idle.
The user confirmed approving once. Pending lasted 3,069 ms; active execution
continued another 9,281 ms. An isolated auto_review comparison completed
without a click and showed active without waiting flags for 20,333 ms. Its
individual command's escalation parameters were not independently captured;
it supplements the separate automatic hook capture rather than proving a
fresh automatic permission request by itself. A separate held native prompt
survived an eight-second observer disconnect. The later second-terminal
approval attempt did not show resolution and is excluded from clear evidence.
These observations do not establish hook/native ordering latency.

## Approaches

1. **Owned native runtime and passive status observation (recommended).**
   Supplies a mid-execution signal and preserves native approval control.
   Requires process ownership, config parity, protocol bounds, and live gates.
2. **Hook-only grace plus exact PostToolUse.** Smaller change, but PostToolUse
   follows execution. An automatically approved long tool would still show
   false Needs You while it runs. The preserved tracker prototype is not a fix.
3. **Elapsed time or terminal text as reviewer detection.** Human and automatic
   durations overlap; terminal inference cannot override hook authority.

## Native runtime and launch contract

Extend the sidecar's common Codex startup boundary to own one private app-server
and one remote native TUI per runtime generation. The server executes tools and
hooks; the TUI owns the user's conversation and approval interaction. A narrow
runtime owner manages both processes and a separate passive status client.
Existing direct launch remains the compatibility path under ADR 0072.

Use an authenticated loopback WebSocket endpoint on macOS, Linux, and Windows.
Create a fresh 256-bit token in memory. Pass only its SHA-256 digest to the
server with --ws-auth capability-token and --ws-token-sha256. Pass the token to
the TUI through --remote-auth-token-env in a dedicated per-runtime variable;
the server and tool-execution environment must not contain that variable.
The observer uses the token only as an authorization header. Never put the
plaintext token in arguments, files, logs, metadata, or model context.

Reserve an OS-selected IPv4 loopback port, release it immediately before
starting the server, and establish authenticated readiness within five seconds,
with at most three port attempts inside that deadline. Check the owned server
is alive before and after readiness. Each attempt has a new token. A bind
collision or failure tears down that attempt; it never attaches to an existing
service. No non-loopback listener, global daemon mutation, or cross-session
reuse is permitted. Authentication and port-race behavior need live tests;
the successful Unix-socket diagnostics did not verify this transport.

Eligibility is feature-probed on the actual configured executable with bounded
help output and deadlines, not assumed from the harness ID or version string.
Initially support canonical built-in launch and exact resume, including the
existing selected-model augmentation and shared -c/--config and --enable/
--disable options. Convert the selected model to an equivalent server config
value with a proper TOML string encoder. Preserve option order and precedence.
The TUI receives its remote connection options and exact resume ID; the server
receives execution configuration. Test the resulting effective config, not
only the generated argument strings.

The accepted argument parser is an explicit allowlist, not a test for unknown
spelling. Its only initial routes are: canonical empty new-session arguments
and the exact resume subcommand/validated ID go to the remote TUI; selected
-m/--model becomes the equivalent server model config; -c/--config entries
and --enable/--disable retain their original order on both server and TUI.
The model conversion participates in that same precedence order. Native remote
and authentication arguments are generated solely by this runtime owner.
Every supplied argument must have an explicit tested destination and equivalent
semantics. Reject partial parsing, extra positional arguments, missing values,
and argument terminators with unmapped trailing content. Recognized but unmapped
options, including --ask-for-approval/-a and --sandbox/-s, take the unchanged
direct launch, just like unknown options. They are not silently ignored or
translated by guesswork; default/config-file approval and sandbox values must
still match under the effective-config tests.

Any unmapped option, custom wrapper/program change, explicit remote endpoint,
profile whose execution config cannot be transferred exactly, or unsupported
protocol feature selects the current isolated direct launch before spawning.
Do not drop, reinterpret, or silently substitute user configuration. Retain
CODEX_HOME, cwd, supported inherited environment, hooks, approval policy,
approvals_reviewer, sandbox policy, and session mailbox/report credentials in
the execution server. Construct those credentials once for the runtime
before either child starts. The observer's launch probes carry no ORKWORKS_
capabilities. Startup failure after any child starts is an error with bounded
cleanup; it never launches a second conversation as a fallback.

The upstream [app-server documentation](https://learn.chatgpt.com/docs/app-server)
labels the command and WebSocket transport experimental and unsupported for
production workloads. Adopting this dependency requires explicit owner review
of this proposal and version-specific verification; it is not a supported
upstream guarantee. CLI connection options are described in the
[developer commands reference](https://learn.chatgpt.com/docs/developer-commands).

## Identity, observer, and bounded protocol

Hook validation remains the activation gate. Preserve the existing whole-tuple
activation/demotion clear and user-source protection rules. The grace delays
only the new permission wait; it does not delay authority activation or retain
a non-user tuple that the activation contract requires clearing. Observe only the exact native ID
already accepted for this runtime through the existing authenticated identity
path. The observer never discovers or binds an identity from loaded threads.
thread.id identifies a conversation; sessionId can identify a session-tree
root. Do not assume those IDs are interchangeable. If the installed schema
cannot prove the queried conversation is the accepted root, use conservative
hook behavior. Recorded native reset clears tracker/observer state and fences
all old responses before following a newly accepted root ID. Exact resume
continues to require the supported local database row and rollout path.

The client permits only initialize/initialized, thread/loaded/list, and
thread/read(includeTurns=false). It does not load or subscribe to a thread,
read turns or transcript contents, send turn/start or thread/resume, or answer
server approval requests. Unsupported server requests close this observer;
there is no default reply that could approve work. Drain unrelated notifications
within bounded buffers without treating them as authoritative attention.

Poll at 100 ms with one in-flight observation cycle, two-second RPC timeout,
1 MiB message limit, and at most 64 loaded IDs. A cycle includes loaded-root
checks around the exact-thread status read. Reject a cycle older than 300 ms
from its first request through effect commit. A fresh singleton loaded-root
result is necessary but not sufficient for safe correlation: concurrent hooks,
ambiguous pre/post ordering, or an unresolved additional request revoke serial
eligibility. Multiple loaded threads lock the current approval batch into
conservative behavior until a trusted turn boundary. Do not restore clear
eligibility merely because another loaded thread disappears.

Start each launch, resume, identity change, and observer reconnect with no
native clear authority. A pending flag in the first observation is pre-existing
or unattributable: it must not be claimed by a later permission candidate.
For serial native clearing, require a fresh nonpending active baseline in this
generation before the candidate, a matching unambiguous pre/permission chain,
and a newly observed nonpending-to-pending edge after that candidate, with no
intervening identity, hook, input, or eligibility revision. A candidate first
seen while pending may still conservatively display Needs You after grace, but
only exact serial completion or an existing trusted transition may clear it.
A reconnect never reconstructs a missed edge from two disconnected snapshots.
Pending state that precedes binding/resume has no candidate and cannot activate
hook authority or be claimed for native clearing. Test resume into a held
prompt and a new candidate arriving while that old prompt remains open.

The non-atomic reads do not themselves prove subagent approval aggregation.
Before shipping native clears, version-specific source or live evidence must
establish the relevant root/child semantics. If that proof fails, child-capable
or ambiguous flows retain conservative hook behavior. The observer may reconnect
with bounded backoff (250 ms up to two seconds) while its owned server lives;
connection loss never clears a wait. Logs contain bounded reason codes and
counts, not tokens, raw frames, native IDs, paths, prompts, or tool input.

## Approval state and effects

A runtime-local reducer consumes validated hooks, eligible fresh observations,
monotonic time, committed input, and lifecycle transitions. It owns correlation
and emits effects; session mutation remains in the existing attention owner.
Bound correlation records and retired-ID tombstones to 64 per turn; overflow
locks that turn into conservative behavior instead of evicting identity fences.
Track runtime generation, accepted native identity revision, hook/turn revision,
input revision, correlation identity, original permission receipt time, and
whether this reducer actually owns a displayed attention tuple. Duplicate
PermissionRequest does not renew a deadline; cleared identities cannot reopen
from late duplicates. Missing IDs or reorder/overlap revoke serial clear rights.
Do not reuse the unfinished prototype's unsafe assumptions as accepted behavior.

| Input or state | Effect |
| --- | --- |
| Accepted PermissionRequest with eligible native observation | Record a candidate and fixed two-second monotonic deadline; delay the new wait, after any required authority-activation tuple clear. |
| Candidate before deadline | Observe continuously; never infer reviewer type from elapsed time. |
| After deadline, fresh active[waitingOnApproval] | Show Needs You for the owned candidate. |
| After deadline, fresh active without waiting flags, no pending flag seen | Keep candidate hidden and keep observing until a matching completion or trusted turn boundary. A later pending flag can still show Needs You. |
| Native assistance unavailable, stale, ambiguous, or disconnected | After the grace deadline, conservatively show or retain Needs You for the outstanding validated permission. Missing-hook sessions retain their existing fallback. |
| Attributable newly observed pending edge transitions to fresh active with no waiting flags | Retire the candidate; clear only its displayed wait to working under all commit fences. |
| Fresh active[waitingOnUserInput] | Never clear to working; retain conservative wait ownership. This issue adds no independent question detector. |
| Native idle, notLoaded, systemError, malformed data, or timeout | No native clear. Existing trusted Stop, committed input, and lifecycle handling remain responsible. |
| Exact PostToolUse in an unambiguous serial pre/permission/post chain | Retire only that invocation's candidate and owned wait; generic or ambiguous completion never clears a wait. |
| Stop, UserPromptSubmit, accepted committed input, reset, revocation, or runtime end | Apply the existing authorized transition and invalidate old reducer effects. |

Hooks remain the reason an approval candidate exists; native status supplies
additional confirmation/resolution. Effects preserve codex_hook provenance only
when derived from that validated candidate. An unrelated native status never
creates attention authority. Apply an effect only if generation, native ID
revision, hook/turn revision, input revision, wait identity, and authority still
match. Clearing additionally requires an opaque attention-write ownership token
returned by the attention owner when the reducer's set succeeds, plus matching
whole-tuple/source values. Value equality alone never proves ownership. The
central attention mutation boundary advances an internal revision and revokes
the old token on every accepted write to the attention tuple or its record-wide
source, including identical-value writes from another producer. All hook,
Peon, user, input, and lifecycle paths participate; there is no unfenced writer.
Set and conditional clear allocate/check ownership atomically under the same
session lock. A duplicate reducer event with no accepted write changes no token;
a competing identical write invalidates it. Revision overflow invalidates all
ownership instead of wrapping. Tokens are runtime-local, never serialized, and
cannot survive replacement or process restart. If a write path cannot meet
this contract, disable native clears rather than rely on tuple comparison.
Never erase newer user, question, another approval, or another runtime's attention.

Native waitingOnApproval is thread-level, not an invocation ID. Parallel calls
or multiple outstanding permissions form an ambiguous batch: after the grace,
retain conservative Needs You until the existing trusted clear. One resolved
approval never clears another. This intentionally retains some false positives.
A slow automatic review that exposes an indistinguishable pending flag would
also defeat the proposed classifier; the gate below blocks claiming this fixes
that case, rather than assuming the two-second grace solves it.

## Lifetime and implementation boundaries

Terminal attachment remains detachable under ADR 0022. Renderer disconnect does
not stop the server, TUI, drain, or observer. TUI exit or session close cancels
observation and ends the owned server; unexpected server exit ends the TUI and
finalizes the runtime once. Startup cancellation, session deletion, reset, and
workspace shutdown use existing generation checks. Stop observation before
revoking credentials and use bounded process cleanup (two seconds graceful,
then force only owned processes). Preserve terminal history and mailbox rules.

Reuse a focused ownership abstraction: Unix process groups and Windows suspended
child assignment to a kill-on-close Job before resume. Do not reach through
provider-private internals or refactor unrelated transports. Prove descendant
cleanup with platform fixtures; do not promise crash-surviving adoption or
cleanup beyond existing ownership guarantees. Never kill a shared daemon or
user-owned diagnostic terminal.

Expected Rust boundaries are launch eligibility/config preparation, owned
server lifetime, a bounded read-only native client, a pure approval reducer,
and generation-fenced attention application. Keep startup orchestration in
session_runtime and attention writes in the current session application path.
Only the minimal allowlisted correlation scalars may enter hook reports; no
tool payload or transcript is transported. Reporter and adapter tests must
cover both POSIX and Windows forms. Hook fingerprint changes still require
native /hooks trust; OrkWorks does not grant that trust.

## Verification and release gates

The serial payload gate is complete; the production behavior gate is not.
Before claiming #690 fixed, require all of the following:

- Deterministic reducer tests for deadline boundaries, duplicate/late/missing
  events, reorder/overlap, long tool execution, hook/native ordering, late flags,
  questions, denied/cancelled/error/idle paths, stale RPCs, reset, revocation,
  source-priority races, competing identical-value writes, ownership-token
  invalidation, and observer loss/reconnect.
- Bounded fake-protocol tests for oversized frames, malformed/unknown replies,
  timeout, authentication, root-versus-child IDs, loaded-list changes, and the
  strict observer method allowlist. Prove it cannot answer approvals.
- Lifecycle tests for server/TUI failure in either order, cancellation during
  startup, detached terminal, workspace shutdown, two simultaneous sessions
  with distinct capabilities/mailboxes/cwd/model, and exact resume. Compare
  effective approval/sandbox/model/config settings with direct launch.
- Native macOS, Linux, and Windows launch/auth/isolation/owned-process cleanup
  checks. Unsupported configurations must keep the direct launch unchanged.
- Fresh one-terminal manual approval: hold past two seconds, observe Needs You,
  approve once without saving a rule, and observe working before a long tool
  finishes. Observer disconnect must leave the native prompt usable.
- Fresh automatic approval with independent PreToolUse/PermissionRequest/
  PostToolUse evidence for that actual invocation, no approval click, and a
  long execution with no false Needs You. Include automatic review lasting
  beyond two seconds and establish whether its native flag differs. If it
  cannot be distinguished, revise the signal design before rollout.
- Root/subagent and overlapping request coverage establishing that no
  independent pending prompt is cleared; otherwise those cases stay explicitly
  conservative and excluded from assisted resolution.

Use normal Rust/sidecar checks and the explicit /code-review gate on the future
implementation PR. Keep the old hook-only prototype separate until these
contracts determine which pieces can be reused. This docs change does not close
#690, complete a Taskmaster recommendation, or enable the new runtime.
