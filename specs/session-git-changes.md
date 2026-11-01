# Selected-session Git changes

Status: accepted by the owner on 2026-10-24; implementation in progress.
Tracking: [#96](https://github.com/Rambolarsen/orkworks/issues/96).

## Outcome and authority

Inspect the selected session's current worktree and committed branch changes
in the main window while its terminal keeps running. The owner requested both comparisons and an
obvious distinction between them on 2026-10-24. The owner approved the concrete design and linked scoped implementation plan
in the PR #1003 conversation.

This is read-only observation. It never stages, discards, edits, commits, fetches,
merges, approves work, acknowledges attention, or requests independent review.
Changes belong to a worktree/branch; they are not attributed to a session or
assignment. Sessions sharing a worktree see the same source changes. This
increment does not consume Taskmaster result/evaluation records.

## Entry and shared panel

Add a reachable **Review changes** button beside the Git facts in Details,
independent of attention tone, dirty state and changed-file count. It works for
live or historical sessions when their current registered directory is available.
Missing Git context shows a disabled button and an explanatory reason. Revalidate
on open: old metadata is not evidence that a directory still exists.

The owner approved the main-window redesign on 2026-10-31 and authorized
implementation under [#1207](https://github.com/Rambolarsen/orkworks/issues/1207).
[ADR 0101](../docs/adr/0101-main-window-git-changes.md) supersedes the original
inspector-only presentation. Changes uses the central utility host, retaining
the existing Sessions sidebar. No new terminal, shell tabs, movable panels or
persisted navigation fields. Selecting a session explicitly leaves Changes for
that session's Terminal. Opening Changes leaves document Review; Review remains
independently reachable through its existing entry.

The shared content header identifies the selected session and provides **Terminal**
to return from Changes; Changes has no separate terminal-return button.
Keep **Review changes** in Details and add a central toolbar Changes entry.
Closing or focus-scoped Escape returns focus to the selected Terminal without
changing attention or sending input. Opening focuses the Changes heading. A
later explicit open starts in Uncommitted. A compact comparison toolbar sits
above a filterable file list beside the diff; both reading regions scroll
independently. At narrow content widths the bounded file list stacks above the
diff. Controls wrap at zoom. Long paths wrap in file rows and remain available
in the selected-file header. Comparison details disclose full paths, source,
baseline and abbreviated OIDs; capture time, branch and partial/error status
remain visible without expanding details.

Unified and split diffs show line numbers, addition/deletion signs and colors,
and basic syntax coloring for common code tokens. Split mode aligns replacement
blocks and leaves empty cells for unmatched lines; hunk headers, patch metadata
and no-newline markers retain their meaning. Render code as escaped React text,
never HTML. Render an initial batch of 1,000 display rows for large patches,
disclose the displayed/total row count and let the user reveal the next 1,000.
This display batching does not alter captured file or comparison totals.
Unsupported/binary/conflict notices remain explicit.

Illustrative wide-window content (the terminal runtime remains alive):

```text
Shared header · selected session            Terminal
Changes
feature

Comparison: [Uncommitted changes v]
HEAD → working tree · staged, unstaged, new files
Updated 14:32:08                           Refresh
Comparison details (collapsed)
2 files · +12 −3              src/App.tsx          Unified / Split
Filter files                 @@ ... @@
  Modified  src/App.tsx       - old line
  New       src/changes.ts   + new line
```

Selecting Branch changes replaces the baseline line with, for example,
`merge-base(main, feature) → feature HEAD · committed only`, plus a Base branch
selector and abbreviated commit IDs. The comparison selector is a labeled native
select, not shell navigation. Use words, status names and +/- prefixes alongside
color. File buttons expose selection, full accessible names and rename paths;
retain visible focus, keyboard access, selectable code and screen-reader labels.
At 200% zoom and narrow widths, controls wrap and the shell provides its normal
explicit return path. No navigation keystroke reaches the PTY.

## Two distinct comparisons

| Choice | Baseline and target | Included |
| --- | --- | --- |
| Uncommitted changes (default) | HEAD tree → worktree with index | Combined net staged/unstaged changes, plus non-ignored untracked files |
| Branch changes | merge-base(selected base, HEAD) tree → HEAD tree | Committed net branch changes only; excludes index/worktree/untracked changes |

Uncommitted uses an empty tree before the first commit, matching the existing
[MVP Git totals](orkworks-mvp.md). Reverted staged changes can therefore produce
no net patch even when status reports a dirty index. Explain this empty state
as **No net uncommitted changes against HEAD**; do not force status counts to
match the net comparison. Branch changes says **No committed branch changes
since the merge base** for a genuinely empty result. No branch history exists
before the first commit; Branch changes is unavailable with that explanation.

Default the base to the locally recorded remote default branch (origin/HEAD),
then local main, then local master, choosing only an existing commit ref different
from HEAD's branch. Never default to the feature branch's tracking upstream.
With no eligible default, require selection rather than guessing. The selector
lists existing local and remote-tracking commit refs (bounded to 500, with an
explicit limit notice); no free-form revision expressions or network access.
An explicit choice may be the current branch, resulting in an honestly empty
comparison. Base selection is in-memory for the open panel only. Missing refs,
missing objects/shallow history, no common ancestor and multiple best merge
bases show an unavailable comparison with a reason, never an empty diff or a
fallback to two-tip comparison. Detached HEAD is labeled with its commit ID and
can still compare against a selected base. Local remote-tracking refs may be
old; say **Local refs; no fetch performed** beside the base selector.

Uncommitted comparisons honor stage-zero index modes when filesystem executable
bits or native symlinks are disabled. Intentionally unmaterialized skip-worktree
entries use index content, preserving staged differences. Built-in CRLF cleaning
honors text/eol attributes and core.autocrlf, including bounded large-file equality;
external filters remain disabled. Attribute and ignore evaluation uses private
bounded copies, never ordinary opens of source control files. Each source control
file is capped at 1 MiB and each control traversal/index copy set at 4 MiB.

Both views support additions, deletions, modifications, type/mode changes,
renames (old → new), binary notices and submodule commit-ID changes. Detect
renames before line totals; pure renames show zero added/deleted lines. Binary
files contribute file counts, no text line totals. Unmerged entries are labeled
**Conflict; patch unavailable** and are not silently presented as resolved.
Do not recurse into submodules; their own registered sessions can inspect them.
Symlinks show the link target text as Git content without following the link.
Non-UTF-8 path/content, unreadable files and oversized files have explicit
unsupported/unavailable notices rather than misleading text. Ignored untracked
files are excluded; tracked files remain eligible regardless of ignore rules.

## Retrieval and bounded authority

Keep privileged retrieval in Rust using the existing git2 dependency, with
thin handlers and the existing Electron-main → authenticated sidecar transport
pattern used for plan content. Credentials stay in Electron main, never in the
renderer. Extend the narrow preload API with Changes-specific methods and keep
independent corresponding Electron/renderer types. No arbitrary command,
filesystem path, blob-ID or arbitrary revision API is exposed.

Additive read API:

- `GET /sessions/:id/git-changes?comparison=uncommitted|branch&baseRef=...`
  resolves the registered session through the sidecar's current effective-CWD
  precedence, validates it in the current workspace, and creates a bounded
  immutable in-memory inspection snapshot. baseRef is an exact enumerated ref.
- `GET /sessions/:id/git-changes/:snapshotId/files/:fileId` returns only the
  already captured patch/notice for an opaque file ID in that snapshot.
- `GET /sessions/:id/git-change-bases` returns bounded eligible refs and the
  proposed default for the same resolved subject.

All routes require the same Electron-owned desktop credential as plan-content;
missing credentials fail closed. Bind every snapshot to workspace generation,
exact session ID, canonical worktree root, comparison, base ref/OID, HEAD OID,
capture time and a random opaque snapshot ID. The renderer sends session IDs,
comparison and eligible ref names, never a trusted filesystem root. Revalidate
current workspace/session/effective root on every request, including patch reads;
a removed session or changed root invalidates the snapshot. An expired/mismatched
snapshot returns a typed refresh-required error, never another file's contents.

Only include Git delta paths. Validate both rename sides; reject absolute,
traversing, administrative .git paths and filesystem escapes. Read worktree
regular files through a no-follow, containment-checked path walk so ancestor or
leaf symlink replacement cannot escape the root. Read committed blobs through
Git objects and symlinks as link text; never follow them. Do not read untracked
directories through symlinks, device files, FIFOs or sockets. Disable external
Git drivers, textconv and filters; no shell commands or external processes.
Revalidate root and HEAD/base IDs after capture; if they changed, discard the
capture and ask for Refresh. Worktree reads are best-effort over a capture
interval, not an atomic filesystem snapshot; make that limitation visible.

Capture limits: 500 displayed file entries, 10,000 visited candidates, 1 MiB
per text input, 256 KiB rendered patch per file, 4 MiB aggregate captured text,
64 KiB displayed line length, and 5 seconds cooperative capture budget. Unchanged large tracked files may
be checked by streaming Git-blob hashing in 64 KiB chunks, capped at 64 MiB
aggregate I/O within that same time budget; their text is never buffered for
display. Identical committed tree entries are skipped without reading blobs. Abort
traversal/output on limits and return an explicit partial/limit notice; report
unknown totals when the whole comparison was not inspected. Never say clean
for a partial result. Keep at most two snapshots and 8 MiB text per sidecar,
expire after five minutes and clear on workspace shutdown. A single blocking
worker admits one capture at a time; further requests get a retryable busy
state, with cooperative checks during enumeration/diff callbacks. The time
budget is not a claim of hard preemption inside a library call. Failure to
implement bounded, no-follow access is a blocker, not permission to weaken it.

## Identity, freshness and lifecycle

Use request generations in the renderer. Clear old results synchronously when
session/workspace/comparison/base changes, and ignore stale list, base or patch
responses including stale errors. A later explicit open resets
comparison/base/file state and loads fresh for the selected session.
Workspace switches close Changes and clear its data. Explicit session selection
closes Changes and returns to Terminal. Switching comparison/base
keeps focus on its selector; successful Refresh retains selected file and scroll
only if the file identity remains, otherwise focuses the file-list heading.

Load explicitly on open, comparison/base change and Refresh; do not continuously
replace a patch while someone reads it. Display capture time, worktree, exact
comparison and abbreviated OIDs. A successful capture describes that capture
interval; subsequent session-list Git updates may mark it **Refresh to update**
but cannot silently relabel/replace its baseline. Keep an old snapshot on failed
manual refresh only for the same exact subject/comparison/base and label it
**Refresh failed; showing capture from …**. Source invalidation clears it.
Loading, empty, partial, binary, unavailable, busy and retry states are distinct.

Opening/closing/replacing Changes or the inspector neither selects another session nor
changes attention, lifecycle, approval, unread state or terminal input. Preserve
PTY output draining, attachment/refit and replay through wide overlay, pin and
responsive page transitions. Retain no durable Changes navigation or snapshots;
existing layout preference schema and central Review restoration stay unchanged.

## Acceptance and implementation gate

The linked [implementation plan](../docs/superpowers/plans/2026-10-24-session-git-changes.md)
covers Git fixtures, authenticated bounded access, real entry/panel interactions,
stale-response fencing, accessibility and terminal continuity. The source viewer
implements the read-only comparisons and main-window presentation described here;
this does not establish availability in older published installers. Owner approval
of the concrete design and scoped plan was given on 2026-10-24, satisfying the #96
implementation gate. [ADR 0091](../docs/adr/0091-read-only-session-git-changes.md)
records the accepted read boundary.
The 2026-10-31 owner-approved redesign under #1207 changes presentation and
navigation only; it preserves those retrieval and security acceptance criteria.
Same-file refresh scroll restoration remains tracked separately in
[#1143](https://github.com/Rambolarsen/orkworks/issues/1143).

## Explicit task-worktree binding

The owner approved [explicit session worktree reporting](../docs/superpowers/specs/2026-10-25-session-worktree-location-design.md)
on 2026-10-25. A revisioned authenticated agent report binds the task worktree
without changing launch/resume CWD. Details and Changes prefer that durable
binding over hook/process/launch fallback, revalidate its repository membership,
and report unavailable if it disappears. Source labels disclose fallback use.
See [ADR 0094](../docs/adr/0094-explicit-session-worktree-binding.md).
