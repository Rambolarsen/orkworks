---
type: Design
title: Keep Codex capture custody in a startup-sealed owner pool
description: Proposed resolution of received-descriptor inheritance using presealed channels and a bounded pool of non-spawning resource owners.
tags: [orkworks, review, codex, custody, processes]
status: stable
---

# Keep Codex capture custody in a startup-sealed owner pool

**The project owner accepted this architecture for bounded macOS qualification on 2026-11-27.** See [ADR 0122](../../adr/0122-startup-sealed-codex-custody.md).
Track [#1981](https://github.com/Rambolarsen/orkworks/issues/1981), under
#68/#66/#1899 and #738/#741. Source base: `5320f358`.
Reuse the accepted [capture inventory/custody architecture](2026-11-22-review-codex-capture-design.md),
[configuration leaves](2026-11-20-review-configuration-leaves-design.md),
[isolation](2026-11-20-review-codex-isolation-design.md) and
[exact projector](2026-11-22-review-codex-fragment-design.md).
The [source audit and native witness](../../validation/ordinary-review-codex-spawn-audit.md)
explain the gap and their finite limits. The [bounded qualification plan](../plans/2026-11-22-review-codex-custody-owner-qualification.md)
adds neither the shared mutation epoch nor a producer/Start consumer.

## Problem and decision

SCM_RIGHTS receipt into the multithreaded sidecar can create an inheritable file
handle before a later close-on-exec call. Existing launch families have no common
receive/spawn exclusion. Merely moving handles to a helper is insufficient if
its newly created controller pipes/socket endpoints have the same bootstrap gap.

Use **up to four persistent, non-spawning custody workers, with all controller
channels sealed before concurrent controlled sidecar launches are possible**.
Resources move only from the sidecar into a worker. No resource descriptor returns
to the multithreaded sidecar. A preview retains the exact live worker lease,
not a parent-side file receipt. Workers retain actual no-follow root/file custody,
perform bounded reads/rechecks themselves and never fork/exec or invoke a tool.

This replaces the accepted proposal's short-lived observer returning descriptors
to its parent. The fixed five-resource inventory, typed joins, leaf formats/digest
rules, source-only first profile, original preview expiry and all native/admission
gates remain unchanged. My proposed changes are process lifetime, channel bootstrap,
lease reuse and quota interpretation; earlier approval does not approve these.

| Alternative | Concrete requirement / reason not selected |
| --- | --- |
| One global receive/spawn mutex | All std/model/probe/custom/native/Git/PTY and relevant dependency launch paths must enroll, including channel creation. Guarding Command::spawn can hold exclusion through a stalled child pre_exec; this would serialize ordinary launches and need a broader lock-order/compatibility proof. |
| POSIX_SPAWN_CLOEXEC_DEFAULT on every launch | Per-child policy, not receipt atomicity. Requires comprehensive launch changes including alternate/fork paths; an attribute on the new helper alone leaves other children uncovered. |
| Receive then fcntl / duplicate with F_DUPFD_CLOEXEC | Leaves the original received descriptor exposed until changed/closed. Reject as a complete solution. |
| Fresh isolated worker/channel per capture | Resource separation helps, but newly created parent IPC and worker-start ownership remain unqualified on the tested host. |
| Startup-sealed persistent pool | Selected for review. Keeps the custody change out of ordinary launch paths; costs bounded startup work and four idle processes. |

No claim that all possible macOS APIs lack an atomic route is needed. An independently
qualified supported atomic mechanism could motivate a later simpler profile.
SDK declarations for macOS 27 are not evidence of availability on this 26.6.2 host.

## Bootstrap before capture authority exists

The eventual normal-main bootstrap occurs after private helper-mode dispatch and
before `run_sidecar_runtime` constructs Tokio workers or publishes AppState,
provider discovery, PTY or other controlled launch authority. Private custody
workers dispatch to their loop before either normal runtime or Git helper startup;
they cannot recursively construct a pool. Current production roles remain empty,
so no production bootstrap is added by this proposal or its qualification slice.

1. Reserve the entire bounded bootstrap owner and create all four Unix stream
   channel pairs in the exclusive startup phase. Set/verify close-on-exec on
   **every endpoint before the first custody child or other controlled child**.
   The phase must have no concurrent controlled fork/exec. Audit the actual
   supported executable startup, not merely a convenient constructor call.
2. Start workers serially through the same running sidecar executable, with an
   empty environment, null stdio and the already-created private channel. Do not
   ask Command to create new data pipes. Make only the selected channel inheritable
   in the worker child, leaving all parent descriptor flags unchanged. Any
   pre_exec operation must be async-signal-safe; no allocation, logging or locks.
3. Each worker validates the inherited channel capability, closed bootstrap
   shape, source-owned build/protocol identity and boot nonce. Parent binds the
   response to its exact owned Child/attempt, not a self-reported PID or label.
   Reject unexpected inherited descriptors at worker entry; never grant roots
   during incomplete bootstrap. A private selector/argv alone grants no file access.
4. Bootstrap has a **three-second total publication budget**, not three seconds
   per child. Only fully owned/handshaken workers completed before the deadline
   can be admitted. Partial success gives fewer usable slots. A late/uncertain
   startup never becomes ready; its owned attempt remains charged until actual
   terminate/reap confirmation. No additional bootstrap attempt starts after
   the phase closes, although an already-started blocked attempt may drain later.
   If channel sealing itself misses the deadline, retire the whole bootstrap
   outcome: any late empty-channel cleanup can never be admitted, reused or receive
   roots/data. No worker starts from that failed sealing outcome.
5. Hand the pool owner to the normal runtime; seal the creation phase permanently.
   No replacement worker, socket pair or bootstrap retry during this sidecar's
   normal operation. Worker death retires that slot until sidecar restart.
   Ordinary startup/launch behavior continues when this optional pool is unavailable.

The three-second limit bounds admission/wait, not an impossible promise that an
OS spawn or kill always settles on time. A supervised pending startup must not
publish roots or block ordinary startup after its budget. Serial startup and
owned-attempt accounting need actual source/OS tests; no physical proof is claimed.

## Private custody lease and message boundary

Each ready worker has at most one lease/operation in flight. Acquire a slot
nonblocking; busy/saturated returns unavailable without a queue. A lease binds
that exact worker/boot owner, a fresh UUID, a positive checked u64 sequence and
all existing parent configuration/workspace/evidence owners. Worker replacement,
exit, protocol error or observed owner mismatch permanently invalidates the preview.
A PID, response digest or deserialized lease ID alone cannot create custody.

Use a private versioned bounded channel with fixed operations: BindAndObserve,
Revalidate and Release. Bind transfers exactly the code-selected workspace/home/
packaged-root descriptors **to the worker only**, associated with their fixed
roles. No caller pathname list, arbitrary resource inventory, native command or
expected-template argument. The worker constructs the accepted five-item read set
and standard projector internally. Parent retains the original live root/configuration
owners and validates the kernel transfer/child binding, not just path/stat text.

Parent responses use a plain bounded byte reader, never an ancillary-FD receiver.
The exact socket/read behavior for unexpected ancillary rights must be qualified
before use; no descriptor installation in the parent may be assumed. Response
frames have version, lease/sequence, fixed category and bounded resource data only.
Exact framing, truncation/extra-frame rejection and limits must be pinned in the
bounded executable plan before code, without exposing a public API/worker route.

Each request has the original **three-second** observation/validation deadline,
256 KiB per file, 2 MiB raw read charge and 1 MiB channel-result cap. Canonical
128/512/640 KiB leaf caps and 16 KiB projection cap independently apply. There is
one request/response per operation; no hidden retry, probe, repair or install.
Raw carrier stays only transiently in the worker; reconstruct the owned fragment
before response. Raw native configuration, diagnostic paths and parser exceptions
never cross the channel. Parent leaf data is ephemeral and dropped after hashing.
Worker retains only live custody plus bounded receipt commitments needed for
rechecks, not raw carrier/serialized-leaf copies or a second preview cache.

Release closes every lease-owned root/resource and drops transient state before
an ACK for the exact sequence. Only that qualified ACK permits another lease on
the same process; no response from an earlier lease can release a later one.
Sequence overflow retires the slot. Parent invalidates preview usability first
on expiry/cancel/change, then releases asynchronously. Busy/stalled/uncertain
release quarantines and terminates that exact worker; no reuse or replacement
until a new sidecar instance. Process ownership/permit survives unconfirmed cleanup.
A local worker timer is defense in depth, never renewal of the parent's original
30-second expiry. After expiry, any outstanding cleanup is draining, not a usable lease.

## Process, descriptor and compatibility bounds

- Four **total** custody workers/attempts per sidecar, counting starting, idle,
  bound, busy, draining and cleanup-unconfirmed states together. No extra process
  may substitute for a failed slot. A ready idle worker may be reused only after
  confirmed release; no startup cost per preview.
- Reserve 16 owned descriptors per slot, including both controller channel ends,
  leaving at most 14 root/resource descriptors. Charge parent-retained root
  duplicates, worker copies and transient transfer/traversal handles wherever
  owned. A complete-layout qualification must prove fit, not count only the five
  final files; reject excess traversal instead of borrowing capacity. The
  existing global 256-descriptor ceiling remains a separate bound, including
  controller and draining charges. Ambient stdio is not a claim about total OS FDs.
- Original preview entry/byte caps still apply. The process bound limits this
  profile to four live/draining custody leases even if the logical cache has room.
  Saturation denies without eviction, alternate process or automatic retry.
- No observation, hashing, IPC wait, process wait or cleanup under terminal-input,
  workspace, catalog or shared configuration admission guards. Final publication
  compares the exact completed lease/reference and epoch under short existing
  fences. Revalidate freshly through the owning worker outside those fences.
- There is no new ordinary-launch mutex, descriptor-closing policy, user-facing
  dialog, persisted setting, installed asset change or permission. Future enabled
  availability costs up to three seconds of startup preparation and up to four
  idle helpers. Resident memory/CPU cost is unmeasured and must be recorded during
  qualification; no small-footprint claim is made. Native profile/activation
  approval remains separate.

No production pool exists today. Qualification uses synthetic temporary roots
and the actual supported macOS process/descriptor boundary. The implemented
observer still needs complete no-follow/absence/currentness/source-byte proof,
context/native evidence, shared mutation coverage and physical admission/Start.
An idle process or successful protocol exchange cannot stand in for those joins.

## Recovery, rollback and owner decision

Sidecar shutdown invalidates all leases before draining the exact owned children.
Restart creates new identities; no preview/worker/lease restoration or native
process adoption. Counter exhaustion, uncertain startup/release and lost channel
fail closed; successful path readback does not resurrect an invalid preview.
Qualification code must remain dormant with no production role or bootstrap
selection until later gates. Reverting that dormant slice rewrites no user file.
Activation/downgrade and user-visible availability policy still require their
own qualified rollout; rollback of a document is not a runtime recovery proof.

The owner accepted the startup-sealed pool and its lease, quota, and startup-failure rules for bounded qualification. That decision changes the process lifetime and bootstrap from #1972. The actual pool, bootstrap, and lifecycle boundary still requires native proof.

## Fresh review — 2026-11-23

A fresh builder check found no consequential missing planning decision. An
independent contract review confirmed the source-family/main claims and preserved
bootstrap, exact lease, reuse, invalidation, quota and separate native/Start gates.
Its required evidence-cleanup wording correction and optional descriptor-ledger
clarification were applied. Actual source/bootstrap/cleanup qualification remains
Investigate. The review preceded the 2026-11-27 owner decision.
