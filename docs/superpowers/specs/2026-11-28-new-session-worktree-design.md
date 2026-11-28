---
type: Design
title: Opt-in worktree launch for new agent sessions
description: Proposed opt-in worktree provisioning, hook reconciliation, agent handoff, and safe archive-time cleanup for new sessions.
tags: [orkworks, sessions, git, worktrees, desktop]
status: stable
workflow_status: approved
---

# Opt-in worktree launch for new agent sessions

Status: owner approved on 2026-11-28; runtime implementation has not started and
remains gated on the linked
[implementation plan](../plans/2026-11-28-new-session-worktree.md), tracking
issue, an amendment to [ADR 0008](../../adr/0008-git-context-detection-not-control.md),
and a scope update to [the MVP specification](../../../specs/orkworks-mvp.md).

This design works alongside [Auto-archive, favorites, and staged session retention](2026-11-28-auto-archive-favorites-retention-design.md),
which introduces the shared `OrganizationStore` v2 record and archive-time
cleanup lifecycle. **Sequencing:** land the staged-retention `OrganizationStore`
v2 foundation first (or as Phase 1 of a joint rollout) so worktree launch builds
directly on `OrganizationStore` v2 without adding temporary compatibility hooks
to legacy `maxSessions` / `maxAgeDays` retention. Neither document changes
current product behavior until approved and implemented.

## Intent and product boundary

Users running multiple AI coding sessions against one repository want a
low-friction way to separate their work at launch so parallel agents do not
collide on the same checkout. Today OrkWorks warns after multiple sessions
share a dirty checkout and tracks agent-created worktrees through
[ADR 0094](../../adr/0094-explicit-session-worktree-binding.md), which works
well when a repository has a worktree skill such as
`skills/starting-work/SKILL.md`, but leaves gaps for
quick tasks, repositories without skills, and coding tools that do not run the
skill before editing files.

This design introduces a narrow, explicit **opt-in launch-time worktree
convenience** for ordinary user-created sessions:

- **Goals**
  - Let the user opt into launching a new session in a fresh, OrkWorks-managed
    Git worktree based on a chosen local or remote-tracking branch.
  - Keep the primary checkout untouched so concurrent sessions running there
    are never disturbed.
  - Preserve working hook-based attention and identity reporting inside the new
    worktree by reconciling the workspace's enabled OrkWorks integrations before
    launch (without breaking existing Git-safety rules).
  - Merge cleanly with agent-owned worktree workflows by signaling OrkWorks
    ownership to the session and updating
    `skills/starting-work/SKILL.md`.
  - Clean up clean OrkWorks-owned worktrees when a session is archived or
    deleted, while preserving uncommitted edits and flagging dirty worktrees so
    the user or an agent can revisit them.
- **Non-goals**
  - Switching branches in the primary checkout from the New Agent Session view.
  - Selecting an arbitrary existing worktree as a session launch directory
    (independent workspaces remain governed by
    [multi-workspace](../../../specs/multi-workspace.md)).
  - Running `git fetch`, `git commit`, `git merge`, `git rebase`, `git push`,
    `git stash`, or deleting Git branches.
  - Running repository-specific setup such as `pnpm install`, Cargo builds, or
    `apm install` inside OrkWorks.
  - Force-discarding uncommitted edits (`git worktree remove --force`) in the
    initial slice.
  - Claiming OS/filesystem confinement or changing the gated ordinary-child
    orchestration contract in [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md).
- **Anti-goals**
  - Silently losing uncommitted edits or unpushed commits.
  - Writing unignored hook files that `git add -A` would accidentally commit, or
    dirtying tracked repository files during worktree setup.
  - Colliding with user- or agent-created sibling worktrees (`../<repo>-*`).
  - Causing an agent to create a second nested/sibling worktree or delete its
    own shell's working directory.

## New agent session composer behavior

The New Agent Session composer
([NewSessionDialog.tsx](../../../apps/desktop/src/components/NewSessionDialog.tsx))
stays fast and uncluttered:

1. **Visibility:** Show the worktree controls only when the active workspace is
   a non-bare Git repository with at least one commit (resolvable `HEAD`). Hide
   them for non-Git folders and bare repositories. For an unborn repository
   (initialized Git repo with zero commits), hide the toggle or show it disabled
   with a short explanation that a first commit is needed.
2. **Isolation toggle:** Offer a compact two-choice control: **Current
   workspace** (default) and **New worktree**. Always reset to **Current
   workspace** when opening the composer—never persist it as sticky across
   launches so quick read-only or question sessions never spin up worktrees
   accidentally.
3. **Base branch selector:** Render only when **New worktree** is selected.
   - Populated from a read-only, strictly local sidecar endpoint listing local
     branches (`refs/heads/*`) and remote-tracking branches
     (`refs/remotes/*/*`, excluding symbolic `*/HEAD` aliases), plus detached
     `HEAD` if the workspace is detached.
   - Strictly local and instant: never run `git fetch` automatically, so launch
     never blocks on network access or fails offline.
   - Default selection precedence:
     1. Remote default branch if present (resolved from
        `refs/remotes/origin/HEAD`, then `origin/main`, then `origin/master`).
     2. Local default branch (`main`, then `master`).
     3. The workspace's currently checked-out branch, or detached `HEAD`.
   - If the selected base ref disappears or fails validation before launch, fail
     closed before spawning and show the error inside the composer without
     discarding the user's draft prompt or attached images.
4. **Automatic branch naming (no custom branch input in v1):**
   - OrkWorks generates the new branch name automatically so starting an
     isolated session takes one click.
   - Format: `orkworks/<slug>-<short-id>` where `<slug>` is a short sanitized
     kebab-case prefix from the initial prompt (when provided, capped at 32
     ASCII characters) and `<short-id>` is 6–8 lowercase hex characters from OS
     randomness; when the initial prompt is blank, use `orkworks/<short-id>`.
   - Validate with Git ref-format rules and retry on collision against existing
     `refs/heads/*`. Agents and users remain free to rename the branch later via
     `git branch -m`.

## Managed worktree storage and provisioning

### Managed directory layout

OrkWorks creates managed worktrees strictly under its global application data
directory rather than as siblings in the repository's parent folder:

```text
~/.orkworks/worktrees/<workspace-hash>/<worktree-id>/
```

- `<workspace-hash>` matches the owning workspace's existing metadata hash under
  `~/.orkworks/workspaces/<workspace-hash>/`.
- `<worktree-id>` is a compact random identifier (for example 12 lowercase hex
  characters) to keep paths short on Windows and avoid collisions.
- Every creation, validation, and removal step canonicalizes the target and
  verifies strict containment under `~/.orkworks/worktrees/<workspace-hash>/`
  with no symlink traversal outside that root.

Keeping OrkWorks-managed worktrees under `~/.orkworks/worktrees/` prevents
collisions with hand-made or agent-made `../<repo>-*` checkouts, works even when
the parent folder is read-only, and makes OrkWorks ownership unmistakable during
archive-time cleanup. **Changes** and **Open in Visual Studio Code** continue to
resolve through the session's bound worktree root so the checkout stays one
click away.

### Creation sequence, async spawn, and rollback

Keep OrkWorks' existing asynchronous `POST /sessions` spawn architecture while
validating and provisioning the worktree before returning `creating`:

1. **Authorization and admission:** Managed-worktree creation (`worktreeMode:
   "new"` and `baseRef: "<ref>"` alongside `expectedWorkspacePath`) requires
   Electron-main desktop authorization (`x-orkworks-open-plan-token` via
   preload/IPC with `runtime.runReady` generation checks—never callable by
   harness hooks or agent reporting tokens). Verify the expected canonical
   workspace path and active runtime generation match.
2. **Synchronous Git validation & provisioning (pre-`creating`):**
   - Confirm the workspace is a non-bare Git repository, resolve `baseRef`
     locally to an exact commit OID (`baseOid`), and pick a non-colliding branch
     name and `<worktree-id>`.
   - Create `~/.orkworks/worktrees/<workspace-hash>/` and run
     `git -C <workspace> worktree add -b <branch> <managed-path> <baseOid>`
     through OrkWorks' owned-subprocess helper with a bounded timeout. Never
     modify the primary checkout's `HEAD`, index, or working tree.
   - Any failure here returns an HTTP error directly to the composer without
     creating a session record or losing the user's draft prompt/images.
3. **Hook reconciliation (non-fatal compatibility degradation):**
   - Run the Git-safe hook reconciliation step below against `<managed-path>`.
   - Hard filesystem errors fail creation and trigger immediate pre-record
     rollback (remove `<managed-path>` and delete the unused branch ref at
     `baseOid`).
   - Non-fatal compatibility blocks (such as an unignored hook path or a foreign
     tracked hooks file on the chosen base branch) do **not** abort launch; they
     proceed in degraded mode with prompt-hook authority disabled (falling back
     to Peon/PTY observation) and store a short `hookNote` on `managedWorktree`.
4. **Durable ownership and `creating` session record:**
   - Persist `managedWorktree` in `OrganizationStore` v2 before spawn, create
     the `creating` session metadata with `cwd` set to `<managed-path>`, and
     pre-populate `worktreeBinding` ([ADR 0094](../../adr/0094-explicit-session-worktree-binding.md))
     with `{ root: <canonical managed-path>, commonDir: <canonical common dir> }`.
   - If record persistence fails, roll back `<managed-path>` and the unused
     branch ref synchronously and return an error to the composer.
   - Return `200 OK` with `status: "creating"` as today.
5. **Asynchronous PTY spawn and failure rollback:**
   - Spawn the PTY with `<managed-path>` as its working directory (applying
     existing Windows verbatim-path normalization in `windows_cwd.rs`),
     exporting the managed-worktree launch environment markers and suppressing
     worktree-reporting instruction injection (since the session is already
     bound).
   - Image attachments supplied in the same composer submission remain stored
     under the owning workspace metadata directory and are handed to the coding
     tool as absolute paths, unaffected by the worktree `cwd`.
   - If background PTY spawn fails and enters `publish_startup_failure`, run
     clean-only rollback of `<managed-path>` and delete the unused branch ref
     only if it still points to `baseOid`. Update the failed `error` session
     row's `managedWorktree.cleanupStatus` to `"removed"` (or `"removal_failed"`
     with diagnostic if rollback fails) so startup failures never silently leave
     orphaned worktrees behind.

### Durable ownership record

Store managed-worktree ownership in `OrganizationStore` v2
(`session-state/<id>.json`, shared with the
[staged retention design](2026-11-28-auto-archive-favorites-retention-design.md)):

```json
{
  "managedWorktree": {
    "root": "/Users/example/.orkworks/worktrees/<hash>/<id>",
    "commonDir": "/Users/example/workspace/repo/.git",
    "branch": "orkworks/fix-login-a1b2c3",
    "baseRef": "origin/main",
    "cleanupStatus": "active",
    "cleanupNote": null,
    "hookNote": null,
    "managedHookFiles": [
      {
        "relativePath": ".claude/settings.local.json",
        "sha256": "<hex>"
      }
    ]
  }
}
```

`cleanupStatus` is one of:

- `active`: worktree is owned by OrkWorks and expected on disk.
- `removed`: OrkWorks removed the worktree (or confirmed it was already absent
  and pruned Git's worktree registration).
- `retained_dirty`: cleanup ran on Archive, found uncommitted changes, left the
  worktree intact on disk, and flagged the session for revisit.
- `removal_failed`: cleanup attempted removal on a clean worktree and failed
  (for example due to an OS file lock or permission error); left flagged for
  retry.

Keeping `managedWorktree` in `OrganizationStore` guarantees that external agent
metadata writers cannot forge ownership of a user directory or clear a dirty
flag.

## Workspace hook reconciliation

OrkWorks installs workspace-local hook files inside the workspace root
(`.claude/settings.local.json`, `.codex/hooks.json`,
`.github/copilot/settings.local.json`,
`.opencode/plugins/orkworks-session-reporter.js`, `.agents/hooks.json`). Because
untracked/gitignored files are not copied by `git worktree add`, a fresh
worktree lacks them.

When provisioning a managed worktree for harness `H`:

1. **Preserve existing Git-safety rules (never relax them):**
   - Keep the existing `require_local_or_ignored_untracked` check for local-only
     files (`.claude/settings.local.json`, `.github/copilot/settings.local.json`,
     `.opencode/plugins/orkworks-session-reporter.js`). Never write an unignored
     hook file that an agent running `git add -A` could accidentally commit.
   - For shared files that may be tracked in Git (`.codex/hooks.json`,
     `.agents/hooks.json`): **never edit a tracked file** inside a managed
     worktree. If the checked-out commit's tracked file already contains
     OrkWorks' owned hook entry, reuse it as-is without writing anything. If
     the path is ignored and untracked, install the owned file normally. If it
     is tracked without OrkWorks' entry or unignored, do not modify it.
2. **Only reconcile already-enabled integrations:**
   - Run reconciliation only when `H`'s integration is currently **enabled** and
     **installed/owned** in the parent workspace (reusing the confirmation the
     user already granted for this workspace, without re-prompting).
   - Run the existing idempotent, canonical/no-follow installer targeting
     `<managed-path>`.
   - Record every untracked/ignored file written by OrkWorks in
     `managedWorktree.managedHookFiles` with its relative path and SHA-256
     digest.
3. **Per-session prompt-hook readiness & degraded fallback:**
   - Prompt-hook readiness and launch generation issuance for a managed-worktree
     session evaluate the **managed worktree** (`<managed-path>`), not the
     parent workspace—so a degraded worktree never claims prompt-hook authority.
   - When reconciliation skips installation due to a Git-safety/compatibility
     reason on the chosen base branch (unignored target or foreign tracked
     file), launch proceeds in degraded mode (Peon/PTY fallback) and records a
     concise `managedWorktree.hookNote` surfaced in Session Details.

## Coexistence with agent-owned workflows and `starting-work`

### Launch environment contract and pinned `worktreeBinding`

For managed-worktree launches (and exact resumes of a session whose managed
worktree is still active):

- Export two environment variables after stripping any inherited values of the
  same names case-insensitively on **all** launches (preventing leakage to
  non-managed sessions or nested children):
  - `ORKWORKS_MANAGED_WORKTREE=1`
  - `ORKWORKS_MANAGED_WORKTREE_ROOT=<canonical managed worktree path>`
- Suppress `worktree-instructions` injection (`worktree_instructions.rs`) on
  managed-worktree launches so agents are not instructed to report a different
  worktree.
- Pin `worktreeBinding` on sessions that have an active `managedWorktree`:
  `PATCH /sessions/:id/metadata` rejects attempts to overwrite or clear
  `worktreeBinding`, keeping `managedWorktree.root` and `worktreeBinding.root`
  consistent for Git facts, Changes, and recovery actions.

These markers are cooperative workflow guidance, not a security credential.

### Updating `skills/starting-work/SKILL.md`

Update `skills/starting-work/SKILL.md`
with a short preflight check before "Detect a shared checkout":

- If `ORKWORKS_MANAGED_WORKTREE=1` is set and `$(pwd -P)` matches (or sits
  inside) `ORKWORKS_MANAGED_WORKTREE_ROOT` in a linked worktree on a non-`main`
  branch:
  1. **Skip primary-checkout residue checks and worktree creation.** The session
     already owns an isolated worktree on its own branch; do not create a second
     worktree.
  2. **Keep repo-specific bootstrapping.** Run offline skill sync
     (`node scripts/sync-repo-skills.mjs`) and scoped dependency installs as
     needed for the task.
  3. **Skip worktree self-removal at close-out.** After the PR merges, do not
     run `scripts/finish-pr.sh` or `git worktree remove` against the managed
     worktree (which `finish-pr.sh` already refuses for the current worktree).
     OrkWorks owns removing the worktree when the session is archived.
  4. **Branch naming:** The agent may rename the auto-generated branch (e.g.
     `git branch -m <descriptive-slug>`) before pushing; OrkWorks tracks the
     worktree by path, not branch name.
- When `ORKWORKS_MANAGED_WORKTREE` is absent, the existing agent-owned workflow
  remains completely unchanged.

## Lifecycle, resume, and safe archive-time cleanup

### Removal preconditions (fail-closed)

OrkWorks may remove a worktree **only** when every check passes:

1. The session has a valid `OrganizationStore` v2 record matching `(sessionId,
   createdAt)` with `managedWorktree.cleanupStatus` in `{"active",
   "retained_dirty", "removal_failed"}`.
2. The session's live runtime is confirmed closed (zero owned live descendants
   and `session_pids` absent).
3. Canonicalizing `managedWorktree.root` confirms strict containment under
   `~/.orkworks/worktrees/<workspace-hash>/`.
4. No other non-terminal live session in the workspace has its launch `cwd`,
   reported `cwd`, or `worktreeBinding.root` pointing inside that managed
   worktree.
5. If the directory no longer exists on disk, run `git -C <workspace> worktree
   prune` and transition `cleanupStatus` to `removed`.
6. Otherwise verify it is a linked worktree sharing `managedWorktree.commonDir`
   (never the primary checkout), then run cleanliness inspection covering
   tracked, untracked, **and ignored** files:
   - First verify every recorded entry in `managedHookFiles` directly on disk:
     if the file still exists and is either a symlink or its SHA-256 no longer
     matches the recorded digest (meaning the user or agent edited the ignored
     hook file), immediately classify the worktree as **dirty**.
   - Run `git -C <root> status --porcelain=v1 --untracked-files=all --ignored=traditional`.
   - Any tracked change (`M`, `A`, `D`, `R`, `C`, `U`) or untracked entry (`??`)
     always classifies the worktree as **dirty** (except an exact-matching
     regular file recorded in `managedHookFiles`).
   - Any ignored entry (`!!`) also classifies the worktree as **dirty** (so
     ignored user files such as `.env` or `.env.local` are never silently
     destroyed by `git worktree remove`), **except**:
     1. Exact-matching regular files recorded in `managedHookFiles` (verified by
        SHA-256 above), and
     2. An explicit, narrow allowlist of regenerable dependency/build cache
        directories (`node_modules/`, `target/`, `.vitepress/cache/`,
        `.vitepress/dist/`, `dist/`, `.next/`, `__pycache__/`, `.venv/`,
        `.pytest_cache/`, `.mypy_cache/`). Any other ignored file or directory
        preserves the worktree as **dirty**.

### Behavior on Archive

When a session transitions to **Archived** (via manual Archive or auto-archive):

1. Complete the existing Close-first step if the session was Active.
2. Run the removal check above.
3. **Clean:** Delete exact-matching `managedHookFiles` if present, run
   `git -C <workspace> worktree remove <root>` (never `--force`), leave the Git
   branch and commits intact, and commit `cleanupStatus = "removed"` together
   with `state = "archived"`.
4. **Dirty:** Complete archiving (`state = "archived"`), leave the worktree
   untouched on disk, commit `cleanupStatus = "retained_dirty"` with a concise
   summary (such as changed-file count), and flag the session visually.
5. **Removal error (e.g. locked file on Windows):** Complete archiving, leave
   the worktree untouched, and commit `cleanupStatus = "removal_failed"` with
   the diagnostic message so the user can retry.

### Behavior on Manual Delete

Applies uniformly whether the session's `managedWorktree.cleanupStatus` is
`active`, `retained_dirty`, or `removal_failed`:

1. **Pre-delete confirmation warning:** When deleting a session that still has a
   non-removed managed worktree (`active`, `retained_dirty`, or
   `removal_failed`), the Delete confirmation warns that any uncommitted changes
   in the managed worktree will **not** be deleted and will remain on disk,
   showing its path.
2. **Post-Close execution:** After Close-first completes (if Active), run the
   same clean-only removal check.
   - **Clean:** Remove the worktree, then delete the session record and
     evidence.
   - **Dirty or removal failed** (including if an Active session wrote files
     during Close-first): Leave the worktree untouched on disk, still delete the
     session record and evidence, return a structured response from the Delete
     action containing `{ preservedWorktreePath, reason }`, and display a
     visible post-delete toast/notice naming the preserved directory path so the
     user can inspect or remove it manually.

### Flagged dirty worktrees and revisiting them

Archived (or restored) sessions with `cleanupStatus = "retained_dirty"` or
`"removal_failed"`:

- Display a visible **Dirty worktree** (or **Worktree cleanup failed**) indicator
  in the Sessions list and in Session Details.
- Are automatically **protected from auto-deletion** and from `workspace_gc`
  (see the companion [staged retention design](2026-11-28-auto-archive-favorites-retention-design.md)).
- Expose safe actions in Details (always targeting `managedWorktree.root`):
  - **Open in Visual Studio Code** and **Changes** so the user can inspect,
    commit, stash, or discard changes.
  - **Restore to Closed** (and Resume into the still-present worktree if the
    user wants an agent to finish or clean up the uncommitted edits).
  - **Remove worktree** (routed through Electron-main desktop authorization
    `x-orkworks-open-plan-token` + `sessionActionToken`; retries the exact
    clean-only removal check once uncommitted edits are resolved and clears the
    flag on success).
- Expose `managedWorktree` status in the session projection so agents or future
  Taskmaster recommendations can also recognize stranded dirty worktrees.

### Authorization boundaries

- **Electron-main desktop authorization (`x-orkworks-open-plan-token` via
  preload/IPC):** Required for managed-worktree creation (`POST /sessions` with
  `worktreeMode: "new"`) and explicit **Remove worktree** retry.
- **Existing `SessionAction` / sidecar retention path:** Clean-only removal
  triggered during `Archive`, `Delete`, or background auto-archive runs inside
  the sidecar under the existing `sessionActionToken` and strict containment,
  closed-runtime, and clean-only preconditions.

### Resume after worktree removal

- **Worktree still present (`active` / `retained_dirty` / `removal_failed`):**
  Exact resume launches in `managedWorktree.root` with
  `ORKWORKS_MANAGED_WORKTREE=1` restored and `worktree-instructions` suppressed.
- **Worktree removed (`removed`):** Session metadata and terminal replay remain
  readable, and the Git branch/commits remain intact in the repository.
  Worktree-backed surfaces (**Review** plan file, **Changes**, and **Resume**
  after Restore) report clearly that the managed worktree was removed on archive
  (noting the preserved Git branch) and refuse to silently fall back to the
  primary checkout.

## Decisions made by the designer (for owner review)

- **Sequencing:** Land the companion staged-retention `OrganizationStore` v2
  foundation first so managed worktrees build directly on v2 without modifying
  legacy retention.
- **Managed path under `~/.orkworks/worktrees/<workspace-hash>/<worktree-id>/`**
  rather than sibling folders (`../<repo>-*`) so ownership is unambiguous and
  never collides with manual/agent worktrees.
- **Local-only base branch listing and creation** (no automatic `git fetch`) and
  **auto-generated branch names without a launch-time text box** to keep the
  composer one-click.
- **Preserve Git-safety rules for hooks & degrade gracefully:** Never write
  unignored hook files or dirty tracked `.codex/hooks.json` / `.agents/hooks.json`;
  if a base branch cannot safely host the owned hook, launch in degraded
  Peon/PTY fallback with a per-session Details notice instead of aborting launch.
- **Keep `POST /sessions` async:** Run Git provisioning synchronously before
  returning `creating` (so Git/base-ref errors stay in the composer), and run
  clean-only worktree/branch rollback inside `publish_startup_failure` if
  background PTY spawn fails.
- **Pin `worktreeBinding` on managed-worktree sessions** and suppress injected
  `worktree-instructions` so `managedWorktree.root` and `worktreeBinding.root`
  never diverge.
- **No `git worktree remove --force` button in v1:** Users resolve uncommitted
  edits in VS Code or a resumed session, then click **Remove worktree**.
- **Authorization split:** Desktop token for worktree creation and explicit
  **Remove worktree**; sidecar-enforced clean-only removal on Archive/Delete.

## Left to the builder

- Exact CSS/layout placement of the **Current workspace / New worktree** switch
  and base-branch picker inside `NewSessionDialog.tsx` consistent with existing
  composer typography and keyboard navigation.
- Exact HTTP route shape for listing local/remote branches (e.g.
  `GET /workspace/git/branches`) and field names on `CreateSessionRequest`,
  preserving strict `deny_unknown_fields` / validation conventions.
- Exact slugification helper and retry loop for generated branch names.
- Exact wording of composer/Details notices, post-delete toast, and icons (using
  existing Lucide icons and design tokens).

## Compatibility, recovery, and verification

- **Compatibility:** Additive `OrganizationStore` v2 and `SessionInfo` fields.
  Older clients ignore new projection fields; v1 `OrganizationStore` readers
  treat unknown v2 records as unreadable and fail closed (protected from
  deletion). Existing workspaces without managed worktrees behave identically.
- **Recovery:** Startup reconciliation does not delete worktrees automatically.
  If a managed worktree directory was deleted externally while the session is
  `Closed`, opening Details/Changes fails closed per ADR 0094 and archiving
  safely prunes the Git registration and marks `cleanupStatus = "removed"`.
- **Approval and implementation prerequisites:**
  1. Owner approval of this written design.
  2. New ADR (or amendment to [ADR 0008](../../adr/0008-git-context-detection-not-control.md))
     and update to [specs/orkworks-mvp.md](../../../specs/orkworks-mvp.md)
     recording opt-in launch-time worktree provisioning and clean-only archive
     removal.
  3. GitHub issue tracking implementation.
- **Required verification before PR handoff:**
  - Rust unit and integration tests using real temporary Git repositories:
    creation from local and remote-tracking refs, unborn/non-Git/bare refusal,
    branch collision handling, synchronous pre-record rollback and async
    `publish_startup_failure` rollback, Git-safe hook reconciliation (ignored
    install, tracked shared-file no-op vs degraded fallback, explicit on-disk
    digest verification, modified-ignored-hook-counts-as-dirty), ignored-file
    protection (`.env` marks worktree dirty while allowlisted `node_modules/` /
    `target/` remain clean), pinned `worktreeBinding` rejection, dirty-worktree
    archive retention + flag, Delete with clean vs dirty worktree returning
    `preservedWorktreePath`, concurrent-session refusal, external deletion
    pruning, resume refusal after removal, and env-var stripping on non-managed
    launches.
  - Desktop tests for composer visibility, non-sticky default reset, base-branch
    selection, error preservation of draft, Details dirty-worktree flag /
    retry action / hook degradation note, and Delete confirmation + post-delete
    preserved-path notice.
  - Full `bash scripts/verify-repo.sh`, `bash scripts/doc-check.sh`, and
    current-diff `/code-review low`.
