---
type: "Implementation Plan"
title: "Session-creation Git scan implementation plan"
description: "Implementation plan: Session-creation Git scan implementation plan."
tags: ["orkworks", "plans"]
---

# Session-creation Git scan implementation plan

**Goal:** Implement issue #729: isolate creation Git work from async workers and preserve admission, cancellation, cwd, response, and metadata behavior.

**Spec:** `specs/orkworks-mvp.md`, Git and Worktree Context.

**Architecture:** Run read-only detection on Tokio blocking workers before registration. Reuse the launch-cwd context for persistence when its workspace path matches. If workspace replacement changes the persistence path, detect that path outside the lock, then revalidate it before synchronous registration/persistence. Keep detached runtime startup and the creating response.

**Tech stack:** Rust, Tokio, git2, Axum, existing reqwest test fixtures; no added dependencies.

## Constraints and blind-spot checkpoint

- Git stats remain API-only; the durable metadata schema is unchanged.
- Least confidence: cancellation or workspace replacement across the new await. Investigated the workflow: registration, topic queues, persistence, and startup currently form a synchronous tail after integration checks. All new awaits must precede those side effects; scanning holds no workspace guard.
- Project blind spot: a second full scan occurs under the workspace lock. Moving the first scan alone leaves async blocking. Reuse it normally; scan any changed persistence path before acquiring the admission lock.
- HTTP measurements use a temporary repo and nonexistent executable; they never start a coding harness or write production recommendation files.

## Steps

- [x] Add gated real-detection regressions in `session_application.rs`: HTTP health progress while a scan is held, cancellation before admission, and literal dirty-worktree field parity across creation/listing/persistence. Watch responsiveness fail on the synchronous baseline.
- [x] Record baseline creation latency and concurrent real HTTP health latency using the ignored `measure_creation_git_scan_http_latency` fixture (24 dirty tracked files, 4096 untracked files, one async worker, five requests).
- [x] Replace synchronous detection with `tokio::task::spawn_blocking(move || detector(&path)).await`; map task failure to `SessionError::Internal` before side effects. Revalidate the current persistence path, reuse or scan its context outside the lock, then register/persist without further awaits.
- [x] Verify task-failure and changed-workspace behavior with focused regressions. Repeat the identical HTTP measurement and record both results in the PR.
- [x] Run focused tests, Rust build/full tests/format, the repository verification helper, and a fresh independent diff-scoped `/code-review medium` (concurrency/cancellation change).
- [ ] Check live Taskmaster recommendations for a matching verified improvement; tie off only an applicable record through its API. Open one PR with `Closes #729`, follow the babysitting workflow, wait for required checks/review, squash-merge, and clean up the owned worktree.

## Recorded measurement and review

Debug build, one async worker, real HTTP requests, same temporary fixture and five creation requests per run. Fixture preparation is excluded from request timing.

| Measurement | Synchronous baseline | Blocking scan, shared snapshot |
| --- | --- | --- |
| Median creation latency | 7,650.44 ms | 3,858.67 ms |
| Maximum concurrent `/health` latency | 7,653.19 ms | 2.45 ms |
| Health samples during five creations | 7 | 4,901 |

The responsiveness regression failed on the synchronous baseline with `health request stalled until the Git scan ended` and passed with the blocking scan. The canonicalized-path assertion in the workspace replacement test accounts for macOS `/var` aliases; it does not change production behavior.

Independent `/code-review medium`: no actionable findings. Reviewed cancellation, workspace→sessions lock order, path revalidation, topic queue ordering, and response semantics. Live Taskmaster inventory contained 217 active recommendations; no `proposedImprovement` matched this verified Git-scan change, so there is no recommendation to complete.

Full `scripts/verify-repo.sh` passed: Rust build/format, 1,609 unit tests and four reporter integration tests; nine OpenCode reporter tests; desktop type-check, 1,110 passing tests (one skipped), and build; docs build; diff and currency checks. Reviewed the flagged `docs/user/sessions.md` Git totals and homepage claims: they still describe the unchanged implemented API/UI contract, so no user-guide correction is needed. The two merged worktrees flagged by the fleet check belong to other tasks and remain untouched.
