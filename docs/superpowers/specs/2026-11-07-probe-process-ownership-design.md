---
type: Design
title: Owned help and version probe processes
description: Proposed shared ownership and bounded cleanup contract for short coding-tool discovery probes.
tags: [harnesses, probes, process-ownership, safety]
status: draft
workflow_status: approved
---

# Owned help and version probe processes

Proposed contract for [#694](https://github.com/Rambolarsen/orkworks/issues/694).
Owner approved this concrete contract on 2026-11-07 (“go ahead” after the
#1511 approval request). Approval authorizes dependent implementation and its
synthetic fixtures; it does not establish implemented behavior, qualified native
cleanup or authorization for installed coding-tool/model probes.

## Outcome and scope

Help/version wrappers can fork without `exec`. Current Tokio `kill_on_drop`
terminates the direct child, leaving a descendant that inherited its pipes or
continues running. Give short discovery probes an invocation-owned native
process group or Job and perform cleanup on every outcome, including success.
Reuse the existing provider ownership mechanisms without changing provider
transport, trust, suspended-child assignment or cleanup budgets.

Initial consumers are the common harness version/help probes in
`harness/detect.rs`, Codex isolation help in `runtime/codex_launch.rs`, and the
OpenCode exact-resume path-selector probe in `opencode_session_store.rs`.
Image capability checks and worktree instruction version checks use the common
entry points and inherit their ownership. This is a complete migration of these
short piped discovery probes, not a new universal command-execution API.
Interactive app-server/model discovery, PTYs, reporters, native approval servers,
background inference and user commands retain their separate owners.

Keep exact argv, cwd, environment selection, output interpretation, cache identity,
generation revalidation, resume identity and safe startup diagnostics. Do not
stop an existing coding-tool daemon, enumerate processes by executable name,
read user conversation content, change global settings or supply launcher tokens.
No orchestration eligibility or same-user confinement is inferred.

## Practical native boundary

| Platform | Owned set and cleanup action | Explicit residual limit |
| --- | --- | --- |
| Linux/macOS | A fresh child-led process group established before executing configured code; send SIGKILL to that group while its direct-child identity is still reserved. | A descendant can leave the group using native session/group APIs, or ask an external service to launch work. Such processes are outside this owned set; do not chase them. |
| Windows | An unnamed non-inheritable kill-on-close Job, with neither breakaway option enabled. Create the child suspended, assign it and resume only after assignment. Terminate the Job and close its final handle during cleanup. | Brokered launches and same-user external services need not join the Job. Assignment/resume failures reject the probe; no unsupervised fallback. |

These are ordinary coding-tool guardrails, consistent with the accepted
[native role assurance](2026-11-02-native-role-guardrails-design.md).
Double-forking alone does not remove inherited group membership; a successful
`setsid`/group change does. This proposal documents that distinction rather than
claiming all daemonizing descendants are confined. Tests must expose the residual
escape without using it to authorize a broader process sweep.

Primary platform references: [Linux setsid](https://man7.org/linux/man-pages/man2/setsid.2.html),
[Apple setsid](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setsid.2.html),
[Linux group signaling](https://man7.org/linux/man-pages/man2/kill.2.html), and
[Windows Jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
A successful termination request is not proof of immediate physical exit on an
unresponsive kernel. The assurance is native ownership, correctly targeted
termination and bounded caller waiting, with fixture evidence for ordinary
owned descendants. Sidecar crash recovery/adoption and durable process identity
remain outside this change.

## Shared ownership seam

Add a private short-probe runner plus a small native ownership primitive usable
by it and the current provider runner. Move or parameterize the existing
`providers/windows_process.rs` Job mechanics; preserve its suspended assignment,
initial-thread resume checks and failure cleanup. Providers retain their existing
public interfaces and behavior. Unix probe-specific unreaped-leader ordering
belongs to the probe owner and does not silently rewrite provider semantics.

The probe runner receives an already selected executable, exact argv/cwd/env,
one monotonic deadline, per-stream bounds and the caller's exit/output policy.
It owns null stdin, piped stdout/stderr and all child/native ownership handles.
It returns bounded bytes/status or a typed private failure, never a configured
cleanup command. The caller keeps executable resolution, parsing, capability
acceptance and safe user-facing messages.

Establish an owned `std::process::Child` immediately after successful OS spawn,
before any fallible pipe conversion or other post-spawn work. Convert its pipe
handles to Tokio asynchronous readers only after the cleanup guard exists.
This avoids a child escaping cleanup on pipe-registration failure. A single owner
controls status observation/reaping; no independent wait task races group cleanup.
Windows assignment uses the owned process handle/PID, keeping those handles alive
through the existing attach/resume checks.

On Unix, retain the live or unreaped direct child until the last group signal.
For success-required probes, observe exit without consuming it using
`waitid(P_PID, WEXITED | WNOHANG | WNOWAIT)`; never call a reaping wait/try_wait first.
Signal the owned group before collecting the final child status. This prevents
cleanup from targeting a reused numeric process-group ID after reaping the
leader. If identity/status observation fails (including unexpected ECHILD),
report cleanup unconfirmed and do not issue a delayed numeric group signal.
No process-name lookup, parent-PID traversal or reconstructed PID authority is
allowed. The builder must preserve the sidecar's normal SIGCHLD disposition and
ensure no other component reaps this child.

[waitid WNOWAIT](https://man7.org/linux/man-pages/man2/waitid.2.html) supplies the
non-consuming exit observation; the pinned libc source exposes it on Linux and
Apple platforms. Implementation still needs native fixture verification on each
supported platform; source inspection is not that verification.

## Deadline, capture and cleanup rules

Retain the existing three-second operation deadline, starting before spawn.
Cleanup settlement waits at most one second **within the remaining operation
budget**, never an additional second. If the deadline is reached, issue the
owned termination request, close the runner's pipes and return the timeout
without awaiting another cleanup window. OS process-creation/termination syscalls
are not preemptible; this is the existing cooperative deadline, not a new hard
real-time guarantee under kernel or filesystem stalls.

| Consumer | Capture and exit policy retained |
| --- | --- |
| Common version probe | Up to 64 KiB per stream, bounded prefix accepted; nonzero or still-running exit is not by itself version failure. Do not wait for voluntary exit before cleanup. Preserve stdout+stderr concatenation and lossy UTF-8. |
| Common help probe | Up to 64 KiB per stream with current bounded-prefix behavior; require the observed original child exit to be successful. |
| Codex isolation help | At most 64 KiB per stream; read at most one extra byte to reject overflow. Require successful original exit, preserve separator/ANSI parsing and exact `--no-daemon` insertion. |
| OpenCode path selector | At most 8 KiB per stream plus one overflow byte; successful original exit and current absolute-path/UTF-8 checks. Native database read-only/schema/ID checks are unchanged. |

Terminate the owned group/Job after complete capture, cap completion, successful
or unsuccessful exit, overflow rejection, I/O failure, timeout, explicit
cancellation and caller-future drop. Successful output does not authorize a
background helper to survive. Preserve a captured pre-cleanup successful exit
status; the cleanup signal must not turn it into a failed-help result. Version
prefix capture can still succeed after intentionally stopping its noisy writer.

A cancellation/drop path synchronously requests native termination and transfers
the already-owned child/native handles to a sidecar-owned asynchronous reaper;
it does not block the UI or require the cancelled future to be polled again.
There are no blocking pipe-reader threads to retain. The reaper only collects
that exact direct child after the termination request; it never sends a later
signal by a bare remembered Unix PGID.

Bound active plus pending-reap owners to **64 per sidecar**, without a waiting
queue. Acquire one permit before spawn; the original owner retains it through
reaping. Saturation returns private `Busy` immediately, mapped to existing
unavailable/probe-failed behavior, without spawning, retrying or provider
fallback. The limit is an author-proposed operational bound, not the version
cache's existing 64-entry limit or a hard native memory ceiling. A single bounded
reaper service uses nonblocking child-status checks; it does not start an
unbounded task/thread per stalled child. A child that cannot be reaped retains
its permit until disposition or sidecar exit. Ordinary PTY input and stop do
not depend on this permit or reaper.

A successful probe result requires the tree-termination request to have been
accepted (or the still-anchored owned set already absent), the original direct
child to be reaped within budget, and capture/exit policy to pass. This does not
claim all Unix group members have physically disappeared at that instant.
Termination/status failures or exhausted cleanup time return private
`CleanupUnconfirmed`; retain owned cleanup state, log a fixed stage/code and do
not accept capability evidence. On sidecar shutdown, close probe admission,
request termination for every owned invocation and allow at most one second
for reaping. Do not delay shutdown indefinitely or recover/kill by saved PID on
restart. Abrupt sidecar loss retains ordinary Unix escape/cleanup limitations;
Windows final Job-handle close retains its native kill-on-close behavior.

## Authority and caller compatibility

Keep existing environment filtering independently per caller. Codex isolation
and OpenCode path selection retain the terminal filter and removal of every
case-insensitive `ORKWORKS_` key; OpenCode still keeps its path selectors.
The common detect probes retain their current ambient environment policy;
changing that policy is a separate compatibility decision, not an incidental
consequence of extracting the runner. Never copy a session/approval token into
new request fields, logs or diagnostic records.

Map private runner failures back to existing APIs: common detect returns None;
OpenCode returns its existing unverifiable diagnostic; Codex keeps the existing
safe failed/timeout/oversized startup codes, with ownership/setup/cleanup failures
using `codex_help_failed`. Cache TTLs and generation/identity rechecks stay intact.
No new public HTTP shape, status vocabulary, retry, configuration setting or
provider fallback is introduced. Native ownership setup failure must not run
configured code outside the owned boundary, including on unsupported platforms.

## Delivery and finite verification endpoint

After approval, implement the shared primitive/runner and migrate the three
owning modules in one feature PR. Update ADR 0072 and the scoped runtime/harness
contracts to describe probe ownership; preserve ADR 0055's provider safeguards.
Update user troubleshooting for a probe that cannot establish/settle ownership.
Use TDD and the repository's current-code review and verification gates.

Synthetic fixtures only; no hosted inference or installed coding-tool probe is
needed. Unix and Windows fixtures must include a non-exec wrapper and child
holding pipes on timeout, output overflow/cap, read failure and cancellation;
a successful wrapper that closes pipes while a child keeps running; nonzero and
silent version output; and ownership setup/assignment/resume failure. Verify
captured marker files/process handles identify only fixture descendants, that
ordinary owned descendants stop, and an unrelated sentinel remains alive.

Add race/ownership cases for leader exit before cleanup (without reaping), drop
before/after spawn and during pipe conversion, deadline during cleanup, saturated
64-owner admission, unconfirmed cleanup retaining its permit, safe shutdown,
and normal provider behavior/exit-code/timeout regression. Linux/macOS escaped
session fixtures document the residual limit and use an independently retained
fixture-only process handle/identity to clean themselves up; production never
adopts that escape. Do not mark escape termination as a passed guardrail.

Endpoint: all migrated caller policies, bounded deadline/capture behavior and
owned-descendant cleanup fixtures pass on their supported platforms; no unrelated
process is targeted; provider regressions, full repository checks and current-head
review pass. #694 closes after implementation and that evidence, without requiring
an exhaustive proof against all same-user bypasses. This design-only PR leaves
the issue open and grants no probe or implementation budget.

## Baseline and author choices

Source baseline: `30c7aca09c7cc694dd76bf4c21106a8e0765c10f`.
Inspected owners: common detect output capture; Codex isolation help; OpenCode
path selection; provider Unix group setup/termination and Windows ProcessJob.
Pinned Tokio exposes pipe conversion from std handles; pinned libc exposes
Apple WNOWAIT/waitid. These are feasibility/source facts, not native test results.

Author choices for owner approval: the practical group/Job boundary and escape
limits; cleanup after success; unreaped-leader signal ordering; retaining the
three-second deadline with cleanup inside it; the 64-owner fail-fast admission
and bounded reaper; and preserving existing per-caller capture/environment
semantics. No choice above is recorded as already owner-approved. A fresh read-only
builder check on 2026-11-07 returned “nothing important left.” It does not
substitute for owner acceptance or platform fixture evidence.

## Amendment — 2026-11-10: settling Darwin exit confirmation

The owner approved the [bounded confirmation plan](../plans/2026-11-10-probe-exit-confirmation.md)
after #957 reproduced EPERM before the original child's exit became observable.
Only on Darwin, a single rejected group signal followed by a still-pending
original-child exit can enter private pending confirmation. Pending is not
accepted cleanup. Observe exit asynchronously, without holding the owner mutex
across awaits, then require the existing bounded group-membership proof.
Confirmation and reaping share one endpoint: min(operation deadline, cleanup
start + one second). No additional signal, retry, ownership reconstruction,
quota increase or additional cleanup window is allowed.

Retain the original Child and permit; the reaper must not consume its exit
while confirmation is pending. Drop, timeout and shutdown finalize pending
confirmation as unconfirmed, permitting only original-child reaping afterward.
No late success is published, and non-reapable owners retain their permits.
All existing exit/capture policies, native proof requirements and non-Darwin
behavior remain. Approval is not implementation or platform qualification.
