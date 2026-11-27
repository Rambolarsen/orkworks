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
earlier pairs. The codec covered both directions, changed individual fields,
every short length, and trailing bytes. Native worker binding and concurrent
application launches still require qualification.

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
