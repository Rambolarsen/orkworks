# Taskmaster macOS Child-Launch Boundary Feasibility Plan

> **For agentic workers:** This is a research-only record. It does not authorize production child launches or changes to the runner implementation.

**Goal:** Determine whether a narrow macOS process boundary can contain a Taskmaster child launch and identify a supportable next design if the least intrusive prototype is unsuitable.

**Architecture:** Probe disposable native helpers under temporary Seatbelt profiles. Measure filesystem access, socket behavior, descendant policy behavior, and process ownership independently. Treat `sandbox-exec` as feasibility-only because Apple marks it deprecated; a passing fixture would not establish a production-ready interface.

**Tech Stack:** macOS ARM64, C fixture, Seatbelt profiles, native process and filesystem observations.

**Design reference:** [Taskmaster native child-launch boundary design](../specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md). That accepted design specifies Linux. This investigation does not qualify macOS or alter the accepted design.

## Execution gate

The user approved a disposable macOS feasibility prototype. All fixtures ran under a unique `/private/tmp` directory, used no credentials or real coding harness, and were removed after checking that no fixture process remained. No OrkWorks runtime code was changed.

A result may guide a later spec/design review; it cannot set `native_boundary_proven` or `runner_eligible`.

## Spike result — 2026-09-26

**Outcome: no-go for using `sandbox-exec` as the child-launch boundary.**

The strict `deny default` profile aborted the helper with signal 6 before its first flushed output. Adding broad `file-read-metadata` access did not fix it. Adding broad `file-read*` access did make the helper run, but then the parent successfully read the outside-worktree sentinel. That control profile is not an acceptable filesystem boundary.

With the broad-read profile, the parent could create an IPv4 socket; its loopback `connect()` failed with `EPERM`. The prior fixture conflated socket and connect errors, so it did not show that socket creation was denied.

An exec'ed descendant that called `setsid()` behaved differently from its parent under that profile: the descendant's outside-sentinel read and socket creation both failed with `EPERM`, while the parent could read the sentinel and create a socket. The cause of this difference is unresolved, so it is not counted as proof of a reliable policy contract.

The sandbox launcher returned while the exec'ed descendant was still running as a reparented session leader (PPID 1). Sending `SIGTERM` to the exact recorded fixture PID stopped it; a follow-up process check confirmed it was gone. `sandbox-exec` itself did not own or clean up that detached descendant.

Apple's developer forum states that `sandbox-exec` is deprecated and its sandbox APIs are no longer supported ([Apple Developer Forums](https://developer.apple.com/forums/thread/661939)). Together with the failed strict profile, parent boundary failures, inconsistent parent/descendant observations, and missing process-tree ownership, this rules out continuing this prototype toward production.

## Decision gate

Do not invest further in repairing this `sandbox-exec` profile. A production proposal needs both a supported enforcement mechanism and an owner that can prove complete descendant cleanup.

The next design review should compare:

1. **App Sandbox with an inherited helper and scoped worktree access.** This uses Apple's supported app model, but may require app-wide signing/entitlements and careful proof that an external coding CLI receives only the intended worktree access.
2. **A per-child virtual machine.** This gives a clearer OS boundary but adds packaging, startup, resource, and worktree-sharing costs.
3. **A Windows candidate.** Reconsider only as an explicit architecture choice; do not silently retarget the accepted Linux spec or assume existing Job Object use supplies filesystem/network isolation.

The least intrusive next investigation is to verify whether App Sandbox can confine OrkWorks' actual helper/CLI process chain while granting a single selected worktree. If it cannot, compare the VM and Windows alternatives before writing a replacement child-launch spec. No production design is selected by this plan.
