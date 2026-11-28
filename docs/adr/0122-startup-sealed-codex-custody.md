---
type: Architecture Decision
title: Keep Codex capture custody in startup-sealed workers
description: Record owner acceptance of a bounded macOS qualification for startup-sealed channels and persistent custody workers.
tags: [orkworks, architecture, codex, custody]
status: stable
---

# Keep Codex capture custody in startup-sealed workers

- Status: accepted
- Deciders: project owner
- Date: 2026-11-27

## Context

[Issue #1981](https://github.com/Rambolarsen/orkworks/issues/1981) tracks descriptor isolation during concurrent macOS child launch. The earlier capture design expected a short-lived helper to return resource descriptors to the sidecar. The [launch audit](../validation/ordinary-review-codex-spawn-audit.md) found no common exclusion covering both descriptor receipt and existing application launch families. The channel setup itself also needs to occur before concurrent launches.

## Decision

The owner approved the [startup-sealed design](../superpowers/specs/2026-11-22-review-codex-custody-owner-design.md) from [PR #1992](https://github.com/Rambolarsen/orkworks/pull/1992) for bounded macOS qualification. Create and seal all worker channels during exclusive startup before any controlled child starts. Use at most four persistent workers that own capture resources and create no child processes. The sidecar sends resource descriptors to those workers and receives bounded bytes. An exact worker lease retains custody until close is confirmed.

Use one three-second publication budget for startup. Late or uncertain attempts remain charged to their slot until termination and reap are confirmed. A failed slot retires for that sidecar lifetime. The [qualification plan](../superpowers/plans/2026-11-22-review-codex-custody-owner-qualification.md) governs the proof and keeps production activation subject to separate approval.

## Consequences

Capture needs native evidence for sealed channels, exact worker binding, lease release, both concurrent launch orders, and cleanup. The qualification also measures startup, idle memory, and idle CPU costs. Production rollout remains a separate decision.

## Amendment — 2026-11-28

Native macOS 26.6.2 arm64 qualification hit the unexpected-ancillary-rights gate on a single bidirectional `AF_UNIX` `SOCK_STREAM` channel: when a peer sends `SCM_RIGHTS`, XNU invokes `unp_externalize()` and installs a new, inheritable (`FD_CLOEXEC == false`), leaked descriptor into the parent across plain `read()`, `recvmsg(msg_controllen = 0)`, and `recvmsg(msg_controllen = 1)` (which reports `MSG_CTRUNC` only after installation).

With owner approval, each of the four custody slots is amended to pre-seal **two** unidirectional channels during exclusive startup before any controlled child starts:

1. An `AF_UNIX` `SOCK_STREAM` request channel (`parent -> worker`), with the reverse direction shut down at both ends (`SHUT_RD` on the parent, `SHUT_WR` on the worker) so only parent-to-worker requests and resource descriptors cross it.
2. An OS `pipe()` response channel (`worker -> parent`, sealed `FD_CLOEXEC`), where `sendmsg` fails with `ENOTSOCK` so descriptor installation into the parent is kernel-impossible.

This uses 4 of the 16 reserved descriptors per slot (16 pre-sealed endpoints across 4 slots), leaving 12 root/resource descriptors per slot. Custody workers launch via macOS `posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT` and `posix_spawn_file_actions_addinherit_np` for only the two selected endpoints plus `/dev/null` standard streams (`0..=2`), closing ambient host descriptors in the child without modifying parent descriptor flags. The worker still enforces an exact `PROC_PIDLISTFDS` inherited-descriptor check and capability-free environment check (allowing only CoreFoundation's pre-`main` `__CF_USER_TEXT_ENCODING` triple on `orkworksd`).
