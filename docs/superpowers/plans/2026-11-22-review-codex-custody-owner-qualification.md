---
type: Qualification Plan
title: Qualify a startup-sealed Codex custody owner
description: Proposed bounded source and native qualification sequence for non-spawning workers and presealed private channels.
tags: [orkworks, review, codex, custody, qualification]
status: stable
workflow_status: active
---

# Qualify a startup-sealed Codex custody owner

Outcome: resolve #1981 with a concrete sound boundary before any real Codex
observer is implemented. Read the [accepted owner architecture](../specs/2026-11-22-review-codex-custody-owner-design.md),
its accepted parent contracts, and the [source/native evidence](../../validation/ordinary-review-codex-spawn-audit.md).
The owner approved the process lifetime and startup changes for bounded macOS
qualification. The plan keeps the pool dormant during qualification.

## Completed investigation and remaining decision

- [x] Inspect actual production std/probe/inference/PTY/private-helper boundaries,
  identify which guards exist and distinguish test-only Tokio uses. Trace
  portable-pty 0.8.1's best-effort child cleanup and its std conversion.
- [x] Inspect supported SDK and pinned primary Rust/Apple sources; reject claiming
  post-receive fcntl or a fresh socket/piped stdio as atomic on this host.
- [x] Execute four independent-literal temporary-file cases: local/isolated
  receiver, receive-first/spawn-first, with ordinary Rust parent/child and exact
  object metadata. Preserve positive control and finite native proof limits.
- [x] Propose complete exclusion versus startup-sealed persistent owners,
  including startup/channel sealing, process lifetime, caps, failure/drain,
  lease reuse, expiry/currentness, compatibility and rollout boundaries.
- [x] Fresh builder and independent contract reviews completed in one round.
  Correct the evidence's failure-cleanup overclaim; explicitly charge parent,
  worker and transient descriptor ownership. Native source boundary stays open.
- [x] Verify docs metadata/indexes/regressions/site and prepare the concrete
  proposal package for draft-PR owner review.
- [x] Owner accepted the architecture and startup/process cost on 2026-11-27 for bounded macOS qualification. [ADR 0122](../../adr/0122-startup-sealed-codex-custody.md) records the decision.

## First native check

Verify the empty channels independently before introducing worker framing or
resource transfer. In a dedicated macOS fixture entry process, create exactly
four Unix stream pairs before the first child launch. Verify close-on-exec on
every endpoint before publication. Treat a reported failure as all-or-nothing:
drop all created pairs and start no child. Verify the same registered endpoints
are inaccessible in an ordinary exec child after sealing. The fixture uses no
user paths, resource descriptors, production entry point or worker mode. It
supplies initial channel evidence for step 1. The actual executable worker,
private framing, delayed startup, and native lifecycle measurements follow
under that step.

Plan review for this check: **Ready.** The fixture has one native channel
owner and a local rollback. It adds no persistent state. Existing accepted
bootstrap and process bounds define the checks. Actual worker and channel
binding remain the next risk-bearing qualification boundary.

Pin the bootstrap bytes separately from lease operations. Each bootstrap
message is exactly 54 bytes in this order.

| Bytes | Field |
| --- | --- |
| 0–3 | ASCII `OWCO` |
| 4 | Protocol version `1` |
| 5 | Direction tag `1` for parent challenge or `2` for worker ready |
| 6–21 | The 16-byte nonce selected for that attempt |
| 22–53 | The 32-byte source-owned build identity expected by that attempt |

Validate against the expected direction, nonce and build identity as a
complete byte slice. Reject truncated, extra and mismatched bytes. The future
stream owner must enforce this exact message boundary and its deadline and
bind the exchange to the retained child. The pure check exercises fixed wire
vectors and rejects changed fields, wrong direction, truncation and trailing
data. Worker binding and acquisition of the trusted build identity remain
source-bound qualification work. Lease operation framing follows under step 2.

Native component evidence on macOS 26.6.2 arm64. The command
`cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --test codex_custody_bootstrap`
passed the native checks, including explicit confirmation from the exec child. The fixture
confirmed all eight endpoints were closed in the exec child and remained
sealed in the parent. A synthetic third-pair allocation failure closed both
earlier pairs. An empty child test selection ran after the positive check and
was rejected. The codec covered both directions, changed individual fields,
every short length, and trailing bytes. Native worker binding and concurrent
application launches still require qualification.

## Synthetic worker startup check

Extend the macOS fixture with a serial worker check using the same four
presealed pairs. Each attempt retains its exact child handle and sends the
challenge over its selected channel. The child checks the selected channel,
closure of the other seven registered endpoints, null standard streams, and
the fixture-only environment. It opens a fresh close-on-exec duplicate for
its Rust-owned stream. The parent generates a fresh nonce for each attempt. The child returns it
in the ready frame with the fixture build tag.

Use half-closed stream directions to validate one exactly sized message in each
direction. This closes the synthetic exchange at EOF. The persistent worker
will need message-boundary handling that preserves the channel for later lease
operations. Share one three-second deadline across the four attempts. Require
both a valid response and confirmation from the owned child. Check rejected
nonce and missing response paths without publishing readiness. Kill and reap
the exact child when the response times out. Check the common deadline
again when each spawn returns. Owned cleanup may finish after expiry.

Plan review for this check: **Ready.** It adds a test-only same-binary exec
path, a fixed close boundary for the synthetic exchange, and exact local child
cleanup. The child environment carries only fixture data. A fixed test build
tag exercises matching. Build identity for the real sidecar executable, full
inherited descriptor inventory, persistent transport, delayed native spawn,
and actual application launch measurements remain in step 1.

On macOS 26.6.2 arm64, the fixture launched the four attempts serially
within the shared deadline. Two matching responses confirmed readiness. The
third exec completed its checks and its response was rejected for a different
expected nonce. The fourth received no challenge. The controller timed out,
sent SIGKILL to the retained child, and reaped its signaled exit. Successful
exec checks verified the seven other registered descriptors were closed,
standard streams were null, and the worker environment held only fixture
data. The parent verified all eight close-on-exec flags after the attempts.
Exact-length, truncated, and trailing-data stream cases also passed. A closed
selected descriptor was rejected before constructing an owned stream.

## Sidecar executable entry check

Set `ORKWORKS_CUSTODY_QUALIFICATION=1` at compile time for the macOS arm64
debug qualification build. The private worker dispatch runs before the Git helper and normal sidecar
startup. The parent launch supplies an empty environment. The actual macOS
executable can acquire the platform's numeric `__CF_USER_TEXT_ENCODING` tuple
before Rust runs. Entry accepts only that tuple if present, plus exactly
three null standard streams and one connected Unix stream. Inspect the worker's complete descriptor inventory before creating a
Rust-owned close-on-exec duplicate and releases the inherited original. The
fixture execs the produced sidecar
binary with only the intended endpoint made inheritable in the child.

The challenge and ready messages retain the fixed 54-byte format. This
entry reads exactly one challenge and returns one ready frame while keeping
the stream open. A worker with the wrong startup tag, malformed invocation,
or additional input already queued behind the challenge refuses startup. After
readiness the worker waits for controller EOF. Additional input at that point
exits with a protocol failure during this qualification. The fixture
checks that the exact child stays alive after readiness, then closes the
controller and confirms reap. A source-derived tag covers this startup entry
and its frame contract. Exact artifact provenance and full lease framing
remain separate qualification checks.

Plan review for this check: **Ready.** Release builds omit the entry, and
default debug builds leave the dispatch disabled. The normal startup owner
has no code that selects or starts the worker. The test
uses only the private channel, a bounded startup exchange, and exact child
cleanup. Process cost, resource transfer, and concurrent application launch
qualification remain open.

On macOS 26.6.2 arm64, the opt-in sidecar binary returned readiness on the
still-open channel and reaped successfully after controller EOF. Its startup
check required exactly four kernel-listed descriptors, and rejected a
different startup tag and surplus bytes queued before readiness. A byte
sent after readiness caused a protocol failure and reap. The parent launched
with an empty environment. The observed platform locale tuple was the single
allowlisted entry. Independent refusal cases supply a valid challenge with
an extra blank fixture socket or an unexpected fixture environment entry.
Both cases exit before sending readiness.

Run the executable fixture with
`ORKWORKS_CUSTODY_QUALIFICATION=1 cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --test codex_custody_entry -- --ignored`.
Run the synthetic fixture with
`cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --test codex_custody_bootstrap`.

## Overlapping sidecar worker lifetimes

The opt-in executable fixture seals all four empty channel pairs before
launching any worker. It starts all four workers before sending the first challenge. Each controller
checks readiness with a fresh nonce under one shared three-second deadline.
The fixture then requires all retained child processes to be alive at once.
It closes the controller streams one by one, waits for the corresponding
child to exit successfully, and checks that the remaining children stay
alive. The same child ownership guard handles cleanup on assertion failure.

Both opt-in tests passed on macOS 26.6.2 arm64. The four-worker case also
passed six additional runs of the same test executable. Concurrent
application-launch and resource-lease checks remain open.

## Failed slot accounting

Exercise the controller's account of a four-slot startup in the private
qualification fixtures. Reserve a slot before beginning its owned attempt.
Use the existing total startup deadline. Closing startup moves unfinished
attempts to draining and preserves workers already ready. A failure after
readiness removes only that worker from the ready set.

The controller polls the retained draining owner and keeps its charge through
a pending or failed poll. Only that owner's confirmed exit releases the charge
and permanently retires the slot.
No fifth attempt or replacement can be admitted. Deterministic tests exercise
pending, error, confirmation, late readiness, and admission after close. The
opt-in native test obtains confirmation from the retained child process.

On macOS 26.6.2 arm64, four policy tests and all three opt-in native tests
passed. The new native case passed six more runs. It retains a pending
draining child while the other three slots stay ready, confirms reap of the
failed child, and closes and reaps the peers independently. Actual startup
supervision, leases, and ordinary application launch isolation remain open.

## Persistent sidecar worker startup check (Step 1)

Native macOS 26.6.2 arm64 qualification first tested unexpected `SCM_RIGHTS`
sent over an `AF_UNIX` `SOCK_STREAM` channel and found that plain `read()`,
`recvmsg(msg_controllen = 0)`, and `recvmsg(msg_controllen = 1)` all invoke
XNU `unp_externalize()` and install a new, inheritable (`FD_CLOEXEC == false`),
leaked descriptor into the parent process. Under the 2026-11-28 amendment to
[ADR 0122](../../adr/0122-startup-sealed-codex-custody.md), each slot uses two
pre-sealed unidirectional channels (16 total endpoints across 4 slots, 4
controller / 12 resource descriptors per slot):

- Request channel (`parent -> worker`): `AF_UNIX` `SOCK_STREAM` with reverse
  direction closed at both ends (`SHUT_RD` on parent, `SHUT_WR` on worker) and
  `SO_NOSIGPIPE`.
- Response channel (`worker -> parent`): OS `pipe()` sealed `FD_CLOEXEC` with
  macOS `F_SETNOSIGPIPE` (`73`), where `sendmsg` fails in the kernel with
  `ENOTSOCK`.

The private worker dispatches at the start of `crates/orkworksd/src/main.rs`
(`--codex-custody-worker <request_rx_fd> <response_tx_fd>`) before
`git::run_scan_worker()` or Tokio/runtime initialization. Normal sidecar
startup does not construct a custody pool. Custody workers launch via macOS
`posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT` and
`posix_spawn_file_actions_addinherit_np` for only the two selected endpoints
plus `/dev/null` on `0..=2`, closing ambient host descriptors without touching
parent flags.

The 32-byte source-owned build identity (`codex_capture_owner::build_identity`)
is derived via `sha2::Sha256` over package/target/profile metadata and
`include_bytes!` of the `src/taskmaster/codex_capture_owner/` module sources
without modifying frozen `crates/orkworksd/build.rs`. After the fixed
54-byte bootstrap exchange (`Challenge` on `request_tx` → `Ready` on
`response_rx`, without half-closing the active directions), persistent control
frames share one 10-byte length-delimited envelope capped at 1 MiB:

| Bytes | Field |
| --- | --- |
| 0–3 | ASCII `OWCO` |
| 4 | Protocol version `1` |
| 5 | Frame kind (`1` Ping, `2` Pong, `3` Shutdown, `4` ShutdownAck; lease operations reserve kinds `>= 10`) |
| 6–9 | Big-endian `u32` payload length (`16` for Ping/Pong, `0` for Shutdown/ShutdownAck, max `1_048_576`) |
| 10.. | Exact payload bytes |

Clean EOF at a frame boundary exits the idle worker cleanly; mid-frame EOF
returns `UnexpectedEof`; bad magic, version, kind, length, oversized payload,
or unsolicited trailing bytes (`poll(POLLIN, 0)` + `FIONREAD`) fail closed and
retire the slot permanently without replacement.

Native evidence on macOS 26.6.2 arm64 (debug `orkworksd` executable,
`cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --test codex_custody_bootstrap --test codex_custody_worker`):

- All 16 split-channel endpoints verified `FD_CLOEXEC` before spawn; injected
  third-slot failure rolled back and closed all 8 prior descriptors (`EBADF`).
- Reverse `sendmsg(SCM_RIGHTS)` on `request_rx` failed in the kernel
  (`EINVAL`/`EPIPE`) and `sendmsg(SCM_RIGHTS)` on `response_tx` failed with
  `ENOTSOCK`; zero descriptors matching the test file appeared in the parent.
- Real `orkworksd` workers passed full `PROC_PIDLISTFDS` inventory
  (`{0, 1, 2, request_rx, response_tx}`), `/dev/null` stdio, endpoint-type
  checks, capability-free env check (allowing only macOS CoreFoundation's
  pre-`main` `__CF_USER_TEXT_ENCODING` triple), build/nonce handshake, three
  rounds of persistent `Ping`/`Pong` per slot without EOF, and clean `Shutdown`
  / `ShutdownAck` exit for all 4 children.
- Negative controls against the real `orkworksd` binary rejected missing args
  without starting HTTP (`ORKWORKSD_PORT=` absent), unexpected env variables
  (exit 65), an extra inherited descriptor via `pre_exec` (exit 66), a Unix
  socket passed as the response pipe (exit 68), and a mismatched 32-byte build
  identity (all 4 slots retired cleanly).
- Deadline and lifecycle checks verified zero workers started on expired
  pre-seal budget, partial readiness (2 ready slots kept usable while a stalled
  third slot timed out, was killed/reaped, and prevented slot 4 from starting),
  unconfirmed-cleanup slot retention until final reap on shutdown, and
  permanent single-slot retirement after unexpected `SIGKILL` without affecting
  surviving slots.
- Measured process cost across 4 persistent debug `orkworksd` workers (15
  consecutive runs on macOS 26.6.2 arm64): warm 4-worker startup 29–80 ms
  (cold first launch ~1.4 s), resident memory ~9.5 MiB per worker (~38.0 MiB
  total, `PROC_PIDTASKINFO.pti_resident_size`), and `0 µs` idle CPU delta over
  a 300 ms idle window (`pti_total_user + pti_total_system`).

Step 2 (reusable 4-slot lease accounting and kernel-bound root transfer) and
Step 3 (actual concurrent application-launch race proof) remain open under
#1981.

## Bounded executable qualification after approval

Before code, record the accepted amendment through ADR 0026's lifecycle and pin
the private byte/descriptor framing (version, exact request/response shape,
field/order/length limits, EOF/extra data handling and endpoint ownership). A
response ID, weak IPC path or self-reported PID may not substitute for an actual
owned process/channel. Protocol choices that change this architecture return
for approval; ordinary framing details may be settled under this decision.

1. **Bootstrap and channel ownership.** Work beside the integration source in a
   private `codex_capture_owner` module, with a minimal early private-worker
   dispatch seam in main. No normal production bootstrap selection. Use native
   synthetic roots/qualification fixtures to prove every channel endpoint is
   close-on-exec before any controlled child can start, no recursion into the
   normal runtime, child-only endpoint inheritance, exact build/Child/nonce
   binding, rejection of unknown inherited handles and zero capabilities in the
   worker environment. Validate the actual controller byte-reader behavior on
   unexpected ancillary rights. Exercise failure before/after each bootstrap
   publication, measured startup/idle memory and CPU cost, total three-second
   admission, partial readiness, late startup
   quarantine and cleanup-unconfirmed ownership. No extra creation after seal.
2. **Reusable private worker lease.** Implement four-slot accounting across all
   booting/idle/bound/busy/draining/unknown actors, per-slot 16-descriptor
   reservation and global 256 cap; nonblocking acquisition with no queue. Pin
   BindAndObserve/Revalidate/Release sequencing, exact live-worker references,
   checked counter exhaustion, complete close-before-ACK and stale-ACK rejection.
   Native temporary roots must prove kernel-bound root transfer only to the
   worker and no resource rights in the parent. Use a bounded synthetic resource
   operation to qualify deadlines/cancel/expiry and hung-reader termination/reap;
   do not mistake it for the complete five-file observer or source owner.
3. **Actual boundary race evidence.** At the implemented source boundary,
   coordinate receive/open and unrelated launches in both orders through real
   std/probe/PTY paths as applicable to the audited build. Match the actual
   object identity, not an assumed FD number; prove the positive control detects
   inheritance. Show parent never installs resource rights and no worker forks
   after receipt. Prove sealed-channel non-inheritance during startup, not only
   post-start flag inspection. Native primitive cases alone cannot check off
   #1981's final acceptance criterion. Include independent cleanup and process
   lifetime evidence, expired lease refusal, sidecar replacement and lost reply.
4. **Gate the next observer slice.** Only after those checks pass, prepare a
   bounded plan for the fixed five-resource physical observer under the accepted
   no-follow/absence/path/source-byte/budget policy. Shared epoch enrollment,
   catalog/context/native evidence, producer publication, physical admission and
   prepared Start are still separate. No fallback into an ordinary launcher or
   renewal of the original preview may make a failed qualification available.

Use TDD for product code. Each source slice runs focused actual-boundary tests,
`bash scripts/verify-repo.sh`, required current-code review (medium if the process/
unsafe/security diff meets the recorded escalation conditions), normal CI and
PR lifecycle. Keep the source-only qualifications dormant and make no user-file
writes/native coding-tool probes. Rollback of dormant code removes only that
code; activation/upgrade/downgrade recovery remains independently gated.

## Risk assessment and review routing

| Dimension | Rating | Evidence and mitigation |
| --- | ---: | --- |
| Dependencies | 4 | Bootstrap, kernel channels, process lifecycle and existing source owners; qualify one boundary before fixed-file observation. |
| Blast radius | 3 | Early main seam, but no ordinary launch mutex; no production bootstrap selection in qualification. |
| State changes | 1 | This proposal changes no runtime/persisted state. |
| Reversibility | 1 | Documentation/dormant-code revert; no user-file migration. |
| Uncertainty | 4 | Actual startup seal, late/uncertain process cleanup, lease reset and native source race proof are outstanding. |

Total 13/25 for this proposal slice. Self-review maps each owner/cap/budget and
recovery requirement to an explicit test stage and preserves the raw-leaf/context/
source/native/admission limits. Disposition: **Fresh reviews complete with the required evidence correction applied;
the owner accepted bounded qualification; actual source boundary Investigate.**

One root owns the artifacts. Dispatch exactly two read-only fresh reviewers in
one round (current lower-cost tier first): builder checks consequential planning
ambiguity; contract reviewer checks the source/native claims, unchanged parent
contracts and bootstrap/lifetime/resource implications. No parallel writers or
implementation agents. Root reconciles both into this one proposal before handoff.

## Documentation verification — 2026-11-23

All 2,566 documentation tests, metadata/index checks and site rendering passed
(build 29.24 seconds). Native ordered outcomes ran as recorded in the evidence;
source/worker lifecycle and failure descendant cleanup are not inferred from them.
README requires no change because this proposal changes no shipped behavior,
installer or production availability. Existing chunk warnings are outside the
diff. Generated pnpm executable lock metadata was inspected/restored. Foreign
shared-checkout edits/worktrees remain untouched. Owner approval and the actual
#1981 application-boundary qualification remain pending.

## Pending launch after startup closes

Reserve the slot before starting a launch. The qualification owner retains
the result receiver and polls it without waiting. After the shared deadline,
the slot stays charged while the launch is pending. If a child arrives, the
draining owner closes its controller channel and polls that exact child until
reap is confirmed. A failed or disconnected launch report leaves cleanup
unconfirmed and keeps the charge.

Exercise the pending, failed-report, and confirmed-reap states with
deterministic controls. On macOS arm64, hold the native launch behind a test
gate across the same three-second budget that starts before channel sealing.
Keep a healthy worker ready throughout. Release the test gate after the
startup phase closes and require reap of the late child and the healthy peer.
The launch delay is injected before the native spawn call.

On macOS 26.6.2 arm64, all seven policy tests and four native opt-in
tests passed. The late-launch case passed three additional runs. Startup
closed while the launch was waiting for its gate. After the gate opened,
the exact child reaped and the healthy peer exited normally. The source
qualification uses the existing empty worker channels.

## Split-channel pool deadline handoff

Run the pool startup against a launch hook held beyond the shared deadline
while one native worker is ready. Startup must return with the pending
attempt charged and the ready worker usable. Start later slots only while
the budget remains and the preceding launch has reported back.

Transfer the exact late child and the sealed channel owners to cleanup.
Shutdown must report an unresolved launch while it is pending and allow
later reconciliation. If the pool is dropped before handoff, the launch
task owns cleanup of any child it obtains. The test uses empty channels and
a gate immediately before the native spawn call.

On macOS 26.6.2 arm64, the deadline regression failed on the previous
synchronous implementation and passed with supervised launch. All 17
tests in the native worker target passed. One test skipped a task first
scheduled at the deadline. A second test confirmed the pending launch
appeared in the shutdown report and reconciled after release. A separate
test dropped the pool before release and observed reap of the resulting
child. Moving cleanup ownership after a child has already been handed to
the pool remains a separate shutdown boundary.

## Cleanup ownership after pool drop

Create a cleanup task before any launch and keep its handoff endpoint in the
dormant pool. If task creation fails, no child starts. Pool drop closes the
channels and attempts bounded termination and reap. Transfer the unresolved
owned children together, retaining a maximum of four handles and polling
each independently until its exit is confirmed. An `ECHILD` report permanently
ends operations on the old PID and retains an unconfirmed outcome.

The native fixture holds two already-owned children behind separate reap
gates. Shutdown reports both as unconfirmed and pool drop returns while both
gates are held. Non-consuming observations in an isolated process verify the
second reaps while the first is still waitable. Releasing the first then
confirms its exact reap.

The original implementation failed the single-child regression. The updated
macOS 26.6.2 arm64 target passed all 19 cases, and the two-child regression
passed six additional runs. The fixture gates observation of reap. An actual
kernel delay and application startup or shutdown integration stay open.
