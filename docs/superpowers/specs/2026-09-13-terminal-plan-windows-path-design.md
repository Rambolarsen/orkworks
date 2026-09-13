# Windows terminal plan-path links

Date: 2026-09-13
Status: approved for implementation planning

## Problem

The desktop terminal already detects the representative path and routes a
user click through the existing terminal-plan selection IPC. On Windows,
OrkWorks can receive a drive-qualified path in the `/C:/...` display form.
The sidecar's printed-path resolver currently gives that leading slash to
Windows path parsing, so the click fails before the existing canonicalization
and worktree validation. The result is an unusable link even though the
Markdown artifact is a valid plan/spec in a linked worktree.

## Goals

- Make the existing `/C:/.../specs/*.md` and
  `/C:/.../docs/superpowers/{plans,specs}/*.md` links usable in live and
  historical terminals on Windows.
- Open the existing reusable Review tab after the user clicks the link.
- Preserve the sidecar as the authority for canonicalization, worktree-family
  checks, containment, Markdown extension checks, and file readability.
- Keep existing renderer matching, POSIX, `C:\...`, home-relative, and
  relative path behavior unchanged.

## Non-goals

- Making arbitrary absolute Markdown or local-file paths clickable.
- Opening the file in an external editor or Explorer.
- Adding a generic renderer filesystem or file-opening capability.
- Changing plan discovery, hook reporting, or Review-tab layout.

## Design

The renderer already detects the representative path through its existing
POSIX-absolute plan-path shape; no renderer matcher change is needed. The
sidecar will add one Windows-only input alias at the printed-path boundary:

| Accepted terminal spelling | Sidecar interpretation |
| --- | --- |
| `/C:/.../specs/file.md` | Forward-slash Windows drive path with one display prefix slash |
| `/C:\\...\\specs\\file.md` | Backslash Windows drive path with one display prefix slash |

The drive letter is one ASCII letter; the character after the colon must be a
path separator. On Windows, the resolver strips exactly that one display slash
before constructing `Path`; `//C:/`, `/1:/`, `/C:relative`, and other malformed
forms are not normalized and are rejected before metadata mutation. Existing
path-root, token-boundary, punctuation, quoted, space-containing, and
wrapped-terminal behavior remains governed by the current matcher rules. The
representative matcher regression case is:

`/C:/Users/froma/source/repos/orkworks-multi-workspace-design/specs/multi-workspace.md`

The link provider passes the exact clicked text and session ID through the
existing terminal-plan selection IPC. It does not normalize the path or gain
filesystem access. The sidecar normalizes only the leading-slash drive aliases
(`/X:/...` → `X:/...` and `/X:\\...` → `X:\\...`) at the Windows path
boundary, then runs the same resolution and validation used for all printed
paths. The production normalizer is compile-time gated: Windows strips the
alias, while non-Windows leaves the input unchanged and retains existing POSIX
behavior.

Validation completes before the sidecar writes the selected plan reference.
The renderer dispatches the existing `terminal-plan-selected` event only after
selection succeeds, so a failed or stale click causes no association, Review
tab creation/focus, or persisted-state mutation.

The sidecar canonicalizes an absolute target independently, derives its actual
worktree root, canonicalizes the session launch root, compares Git common
directories, and computes the target-root-relative path. It then enforces the
existing supported-root allowlist (`docs/superpowers/plans`,
`docs/superpowers/specs`, or `specs`), regular-file status, and Markdown
extension. A linked worktree is accepted only when it belongs to the same Git
common-directory family. It stores the validated worktree root plus
workspace-relative Markdown path, as it does today; the existing plan-content
resolver consumes both fields, so Review reads the clicked linked-worktree
artifact rather than reopening the relative path under the launch root.

Selection holds the existing workspace mutex from session lookup through
validation and metadata/event write, which serializes concurrent selections and
is the transaction boundary for this operation. A missing/stale session fails
before mutation. Both live `Terminal` and historical `HistoricalTerminal`
instances use the same session ID and IPC callback. The existing App flow then
selects that session and reuses the singleton Review panel; repeated clicks do
not create another tab.

## Verification

- Existing renderer terminal-link tests remain green, including a regression
  assertion that the literal representative `/C:/...` text is detected and
  forwarded unchanged by the provider. The matcher test uses the literal
  display path; sidecar tests derive paths from temporary Windows fixtures and
  never depend on a particular user name or checkout location.
- Sidecar tests cover the compile-time normalizer contract: Windows strips one
  leading slash from a drive-qualified path, while non-Windows leaves it
  unchanged. A Windows validation test resolves a derived `/X:/...` path and
  confirms the stored worktree-relative reference matches the equivalent
  native path, including a real linked worktree.
- Direct selection tests confirm the sidecar allowlist still rejects a readable
  `README.md`, malformed aliases (`//C:/`, `/1:/`, `/C:relative`, and bad
  separators), parent traversal, non-Markdown/unreadable files, control
  characters, and symlink/junction escape attempts, with no metadata mutation
  on every rejection.
- Existing selection tests continue to cover validation-before-write, missing
  sessions, and serialized workspace updates; existing App/Dockview tests
  cover session targeting and one Review panel. No renderer or App production
  code changes are required.
- The existing selection ordering continues to validate before metadata write;
  live and historical terminals continue to use the same session callback, and
  App-level behavior continues to reuse one Review panel. No renderer or App
  production code changes are required.
- Run the focused renderer terminal-link test, focused Rust plan-handoff tests,
  `pnpm --dir apps/desktop exec tsc --noEmit`, `cargo fmt --check`, and the
  repository's required diff checks. The Windows-specific resolver assertion
  must run on the repository's Windows CI/native validation job, not only on a
  POSIX host where Rust `Path` has different drive semantics.

## Acceptance criteria

- A Windows terminal line containing
  `/C:/Users/froma/source/repos/orkworks-multi-workspace-design/specs/multi-workspace.md`
  exposes a usable clickable link.
- Clicking it opens the existing Review tab for that session.
- Invalid, escaped, unrelated, or unreadable paths remain rejected before any
  plan association or terminal handoff.
- No existing supported path form regresses.
