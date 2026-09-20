# Pinned and enumerable installation-scoped workspace history

- Status: accepted
- Deciders: owner
- Date: 2026-09-19

## Context

ADR 0060 introduced installation-scoped workspace history
(`recentWorkspacePaths`, a single recency-ordered, 20-entry-capped list) as a
startup hint for `lastWorkspacePath`. Nothing in the UI reads this list —
there is no way for a user to see, switch to, or deliberately retain a
workspace shortcut without re-browsing the filesystem through the native
folder picker every time.

## Decision

Extend the on-disk workspace-memory schema from v1 to v2 by adding a second,
separate list, `pinnedWorkspacePaths`, alongside the existing
`recentWorkspacePaths`:

```json
{
  "version": 2,
  "revision": 43,
  "lastWorkspacePath": "<canonical path or null>",
  "recentWorkspacePaths": ["<canonical path>"],
  "pinnedWorkspacePaths": ["<canonical path>"]
}
```

- A path lives in at most one of the two lists at a time.
- `recentWorkspacePaths` keeps its existing 20-entry cap; both lists share
  the file's existing 64 KiB total serialized-size cap.
- `pinnedWorkspacePaths` is exempt from recency-based eviction but gets its
  own 50-entry cap, enforced as a distinct `pin_limit_reached` diagnostic
  (a count overflow) alongside the existing byte-size diagnostic that the
  generic write path already enforces for either list.
- `lastWorkspacePath`'s validity invariant relaxes from "must be present in
  `recentWorkspacePaths`" to "must be present in `recentWorkspacePaths` OR
  `pinnedWorkspacePaths`," since opening a pinned workspace updates only
  `lastWorkspacePath`, not either list's membership.
- v1 files (no `pinnedWorkspacePaths` key) migrate at read time to v2 with an
  empty pinned list, following the same read-time migration pattern already
  used for the pre-#569 legacy format (three-tier reader: legacy → v1 → v2).
- The renderer gains a `WorkspaceHistoryList` surface (behind the titlebar
  switcher and the no-workspace state) that reads this history over new IPC
  channels and lets a user open, pin, unpin, or remove an entry.

Full design: [`docs/superpowers/specs/2026-09-19-recent-workspace-switcher-design.md`](../superpowers/specs/2026-09-19-recent-workspace-switcher-design.md).

## Consequences

Users can pin workspaces they return to often and switch to any remembered
workspace directly, instead of only ever seeing a single silent
`lastWorkspacePath` startup hint. The on-disk format gains a version; no
cross-version compatibility window applies beyond the one-directional
read-time migration, since this is a single-writer, installation-local file.
The existing lock/revision/atomic-replace write mechanism, byte-size bound,
and corrupt-file-preserved-on-failure behavior are unchanged and apply
uniformly to both lists.
