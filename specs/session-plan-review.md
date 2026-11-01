# Session Plan Review

Status: accepted
Date: 2026-08-01

## Purpose

OrkWorks should let a user read and explicitly request review of a plan or specification produced by the selected session, without becoming a repo-wide task inbox or a general terminal-input controller.

## Design

- A session may have one associated workspace-relative Markdown artifact. A harness-reported `planPath` remains authoritative; as a fallback, OrkWorks recognizes a valid path printed in terminal output below `docs/superpowers/plans/`, `docs/superpowers/specs/`, or `specs/` — covering both the `writing-plans` skill's plan output and the `brainstorming` skill's design-doc checkpoint. The fallback only matches when the printed path sits within a few tokens of a write-signal verb at the start of the line (e.g. `Wrote <path>`, `Created <path>`, `Plan written to <path>`, `Spec written and committed: <path>`) or immediately after a write-signal verb anywhere on the line (verb preceding path, e.g. `A model writes <path>`) — so an incidental mention, a reference to an existing path, a verb that *follows* the path (`<path> written`), or prose that happens to quote a write verb far from the path is not mistaken for authorship; this intentionally favors missing the card over misattributing an unrelated document (see [issue #278](https://github.com/Rambolarsen/orkworks/issues/278) for replacing the fallback with a hook-reported signal instead of a wider vocabulary list). Every path is canonicalized, contained within the workspace, Markdown-only, and rejected when it contains a control character.
- A printed eligible plan/spec path is also a clickable terminal link. Clicking is explicit user approval to associate the artifact; it takes precedence over hook and fallback associations. Relative links resolve against the session launch worktree, and when the launch root has no matching copy, they may fall back to a linked worktree or the main checkout sharing the session repository's Git common directory, with the launch-root copy keeping priority — this covers a path printed from inside a linked worktree while the session launched elsewhere in the same repository. Absolute links may resolve only in that worktree or a linked worktree with the same Git common directory. Every candidate is revalidated as an existing supported Markdown file inside that same Git worktree family, so resolution can never leave the session repository; the residual ambiguity (a same-named document in a sibling worktree is chosen only when the launch root has no copy) is accepted in exchange for opening the artifact the agent actually wrote. Every later read and review handoff revalidates the stored anchored reference.
- Claude Code's `PostToolUse` `Write|Edit` hook (ADR 0038) is now the authoritative `planPath` delivery: the owned integration installs a synchronous hook whose shared `report-harness-event.sh`/`.ps1` reporter invocation passes `--report-plan-path`, forwarding the hook payload's `tool_input.file_path` to `POST /sessions/:id/plan-path` and skipping the generic attention + harness-session POSTs. A successful hook report overwrites any prior terminal-fallback association, since `report_session_plan_path` stores the path unconditionally (the terminal fallback's `is_none()` guard stays in place as the only protection against the terminal fallback itself clobbering a path that a real hook has already established — the fallback is intentionally lower-confidence than the hook). Other harnesses without a canonical file-path hook event (Codex, Aider, ...) keep the terminal fallback.
- The selected session's Details inspector shows a card whenever the associated artifact remains readable. It says **Plan ready for review** when the session needs the user and **Plan available** otherwise.
- **Review plan** opens the single central Review surface for that exact selected session and renders the artifact. It records a bounded return destination to Terminal or the originating Workflow; it never creates a document tab per file.
- **Request independent review** is available only for a live selected session. The user click is the explicit approval: Electron main calls a sidecar endpoint authenticated with its per-sidecar secret; the sidecar revalidates the stored path and writes one fixed review prompt plus Enter to that session's PTY.
- For review requests, the renderer provides only a session ID and cannot supply a path, command, or arbitrary terminal text. The terminal-link selection IPC is the narrow exception: it may pass the exact clicked path text with its session ID, and the sidecar remains responsible for all resolution and validation. The sidecar appends an event recording each user-approved handoff.

## Prompt

The sidecar constructs one fixed prompt using the validated workspace-relative path. It asks the live session's agent to delegate the review to a subagent when possible, rather than reviewing its own plan — the same author reviewing their own spec defeats the point of an independent check — but falls back to letting the agent review it directly. This stays within the existing same-session PTY handoff rather than starting a second OrkWorks session, and stops short of mandating an independent reviewer (see Non-goals):

`Please review the plan or specification at <path>. Delegate this review to a subagent if you can; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.`

## Non-goals

- A repo-wide review queue, task list, digest Peon, or background artifact watcher.
- Arbitrary terminal text injection, keyboard automation, command approval, or sending prompts without a user click.
- Automatically starting a second session or requiring an independent reviewer.

## Acceptance criteria

- [ ] A readable session plan/spec shows a Details card regardless of session attention state.
- [ ] Review plan opens the associated artifact in the single reusable central Review surface with an explicit return destination.
- [ ] A user can explicitly send the fixed review prompt to a live session; the prompt is submitted exactly once.
- [ ] Missing, absolute, non-Markdown, and workspace-escaping paths are rejected before any PTY input.
- [ ] The renderer never receives an artifact path or sends arbitrary terminal text.
- [ ] The user-approved handoff is recorded in the session event log.

## Branch document extension

Status: accepted, 2026-11-01. The owner approved the shared branch behavior,
two-step UI, latest ordering and additive storage/protocol design in the
originating conversation under
[#1230](https://github.com/Rambolarsen/orkworks/issues/1230). This extension
supersedes the single-session, single-document association/display rules above;
the existing reporting transports and user-approved review authority remain.

### Outcome and display

Replace the single session artifact with two independently available documents:
**1. Design / specification**, then **2. Implementation plan**. The Details
inspector keeps both steps visible, shows readable document titles and
available/missing state, and offers expandable source information. Selecting a
step reads it in the existing single central Review surface with its existing
Terminal/Workflow return destination. A step number indicates order, never
approval, completion or permission to proceed. The underlying coding workflow
owns all approvals and gates. Retain the explicit fixed independent-review
action for the selected document and live session.

Default to the latest document of each kind on the branch. A branch may be
reused for several work items; OrkWorks does not infer a work-item hierarchy or
try to pair documents semantically. A new spec can temporarily appear with the
previous plan. Older/other documents remain accessible through Changes.

### Identity and durable association

Within one workspace owner, a document pair belongs to the canonical Git common
directory and full local branch ref, not to a session, branch display name or
worktree path alone. Sessions resolving to that key share selections and source
information. Use the validated explicit worktree binding before existing
harness/process/launch directory fallbacks. An unavailable explicit binding is
unavailable; never silently inspect a different root.

Resolve the branch from the current validated worktree on each refresh/read.
Switching branches uses the new branch's pair without copying the old pair.
Detached HEAD has no shared branch key: retain a revalidated session-local
explicit/reported artifact, but do not invent a branch or share by commit ID.
Unborn branches may discover uncommitted documents. Independent workspace
instances retain their existing isolation; this change creates no cross-instance
writer, registry or global ownership service.

Store a version-1 additive branch record under the owning workspace's
`branch-documents/<sha256-key>.json`. The key hashes an unambiguous encoding of
the canonical common directory and full branch ref; the record also stores and
checks those identity fields. Each optional slot retains the anchored reference,
source, observation time and whether a user explicitly pinned it. Use atomic
replacement and a retained per-key lock under the workspace's single-writer
lease. Reject malformed/unknown-version records without overwriting them.
Preserve existing `SessionMetadata.planPath`; no destructive migration.

Retain a selected automatic document's establishing HEAD/object evidence. A
commit or advancing base does not erase a valid established branch association
when there is no newer candidate. Deletion, repository mismatch or rewritten
history that no longer supports that evidence invalidates the automatic
association; rediscover without copying it to another branch. Explicit pins
remain deliberate choices and are revalidated for readability and containment.

Explicit user selection pins the corresponding branch slot and survives restart
and later automatic discoveries. Missing pinned files display unavailable;
automatic discovery must not silently substitute a different document. A narrow
**Use latest** action clears that slot's pin. A terminal link remains an explicit
selection; agent/hook reports and terminal heuristics are automatic evidence,
not implicit pins. Share them only after binding to the originating repository
and branch at acceptance time; old session metadata cannot be reattributed to
whatever branch happens to be checked out later.

### Discovery and latest ordering

Supported Markdown roots remain `docs/superpowers/specs/`,
`docs/superpowers/plans/`, and `specs/`. Classify the first and third as
design/specification, the second as implementation plan. Exclude directory
indexes (`index.md`, `README.md`) from automatic discovery. Do not classify by
prose semantics, search the entire repository or create documents.

Agent/hook paths and existing conservative terminal signals remain discovery
inputs. Git additionally discovers eligible uncommitted regular files and
committed net changes against the branch's default base. Reuse Changes' local
base preference: origin/HEAD, then local main, then local master, excluding the
current branch. No fetch. No eligible base, ambiguous/missing merge base or
shallow/incomplete history makes committed discovery unavailable; it does not
authorize scanning all inherited documents. Uncommitted discovery and validated
explicit/reported paths may still work independently.

For automatic defaults, rank current uncommitted changes before committed
documents. Rank dirty/untracked candidates by filesystem modification time,
then deterministic relative-path order for ties. Among committed net branch
candidates, walk HEAD's first-parent history back to the unique merge base;
the nearest commit changing a candidate wins. Use history order rather than
author/committer clocks. A reverted/deleted path is not an openable candidate.
Agent reports do not permanently freeze an older automatic default; when the
same chosen path has stronger reporting evidence, retain that provenance.

Inspection is read-only and bounded: at most 10,000 visited file/tree/commit
entries, 500 eligible documents, a five-second cooperative scan budget, 1 MiB
per document read and 4 MiB aggregate document reads per scan. Report limits or
unavailable inspection explicitly; an incomplete scan cannot claim a latest
winner or overwrite an established association. Failed scans still revalidate
saved selections independently, so a readable pin remains accessible. Exclude
nested repositories and Gitlinks from discovery and access; preserve Git's
literal-symlink checkout semantics. Cache results for at most five
seconds per worktree/branch/HEAD and invalidate on accepted reports/selections,
root/branch changes and workspace adoption. Discovery runs outside the global
terminal-input fence; revalidate workspace ownership and session location before
publishing prepared scan results, then revalidate before returning the projection.
A scan captured before a newer
publication cannot overwrite it. Automatic associations retain normalized
content-hash evidence as well as ancestry, so reverting a branch change cannot
keep an inherited document attached. Never hold runtime/workspace locks
through discovery. Reuse bounded Git-object and no-follow regular-file access;
never read symlinks, FIFOs, devices, source Git control files through ordinary
libgit2 filesystem readers, or execute external filters/drivers.

### Access, compatibility and recovery

Expose sanitized ordered slot summaries and shared-branch session context in
session projection. Titles come from bounded document content or a sanitized
filename fallback. Do not return trusted filesystem paths to the renderer.
Keep `hasOpenablePlan` as the aggregate of readable slots for older consumers.

Extend authenticated content/review access with an optional closed
`design | implementation` kind. Electron validates it and retains the desktop
credential and workspace-generation fence; renderer supplies session ID, kind
and an optional opaque selection ID from the projected slot. This ID identifies
the owning repository/branch, displayed document and content without exposing
paths or granting authority. Availability changes invalidate cached reader bytes
and show an unavailable state, including when the last document disappears.
Content, review and pin-reset requests reject a changed selection rather than
silently acting on a newer document. Existing bodyless requests preserve their legacy
selection, otherwise default to implementation then design. Pin reset accepts
only session ID, kind and optional selection ID through that desktop boundary.
Revalidate workspace, effective root, branch, anchored repository containment,
regular Markdown type and read bounds for each read and review handoff.
Terminal-link selection returns its validated kind and opaque selection ID;
retain these for the subsequent content/read handoff. Its own same-branch
publication cannot cancel the explicit open, while session, workspace and branch
switches still invalidate it. Legacy clients may ignore the additive response.
For an unbound session's permitted sibling-worktree selection, resolve the exact
validated explicit selection by its opaque ID and use the document's branch for
admission while preserving the session's own location/runtime fences. Explicit
task-worktree bindings remain authoritative and reject sibling selections.
Capture the runtime before document inspection; at input admission revalidate
that runtime, workspace and selected document. A resumed runtime cannot receive
a click made for its predecessor. Audit only acknowledged delivery.

Keep legacy string/anchored `planPath` records readable. A legacy reference
without recorded branch identity remains session-local until fresh explicit or
agent evidence establishes that identity. It may remain the compatibility
fallback, but must not seed another branch's shared record merely because a
session is now inspecting that branch. Preserve existing explicit-clear behavior
for legacy fallback. Failed storage publication leaves the prior branch record
intact. Dedicated document-report operations report that failure. A revisioned
metadata patch already committed must still acknowledge success and update live
bookkeeping; supplementary branch-publication failure is logged, while its
session-local reference remains available. Downgrading ignores additive branch records and retains
the original session metadata; no downgrade deletes user selections.

### Acceptance additions

- [ ] Both ordered steps remain visible, including either independently missing.
- [ ] Same repository/branch sessions share documents, while equal branch names
  in different repositories and deliberate branch switches remain isolated.
- [ ] Committed documents appear after missed reports and after restart; inherited
  base documents are excluded from automatic discovery.
- [ ] Latest rules cover dirty/untracked files, committed history, branch reuse,
  reverted/deleted files, incomplete scans and clock-skewed commit timestamps.
- [ ] Explicit pins survive commit/restart and automatic discoveries; Use latest
  restores automatic choice for that slot only.
- [ ] Legacy metadata/requests remain supported without false cross-branch
  migration; unavailable bound roots and malformed storage fail honestly.
- [ ] Authenticated reads and fixed review requests support either kind, reject
  invalid kinds/paths, and fence stale session/workspace/document responses.
- [ ] No approval/completion state, Git mutation, workflow gate or work-item
  hierarchy is introduced.
