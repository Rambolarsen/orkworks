# Terminal Markdown Document Tabs

**Date:** 2026-07-23
**Status:** Approved in brainstorming

## Goal

Make Markdown paths printed in an OrkWorks terminal useful as navigation.
Clicking an existing workspace Markdown path should open a safe, readable,
read-only document tab beside the terminal without stopping the session or
leaving OrkWorks.

## Problem

Coding tools frequently finish planning or design work by printing a path such
as:

```text
docs/superpowers/specs/2026-07-23-example-design.md
```

The terminal may color that path differently, but OrkWorks does not currently
register a link provider for plain file paths. The user must copy the path,
switch to another application, locate the file, and then return to the session.
That breaks the fast review loop precisely when the coding tool is waiting for
feedback.

OrkWorks already has a secure **Open plan** handoff for a session-reported plan
path, but that action opens the operating system's configured file handler. It
does not make arbitrary Markdown paths printed in terminal output clickable or
render documents inside OrkWorks.

## Scope

In scope:

- Detect plain Markdown paths in live and historical terminal output through a
  custom xterm link provider.
- Support any existing regular `.md` file contained by the active workspace,
  including specs, plans, ADRs, READMEs, and user documentation.
- Open a read-only GitHub-flavored Markdown document as a Dockview tab in the
  same group as the Terminal panel.
- Keep one tab per canonical workspace-relative path; clicking the same file
  again focuses its existing tab.
- Keep multiple document tabs open while the user changes active sessions.
- Refresh open documents when their files change while preserving the reading
  position.
- Support same-document anchors, links to other workspace Markdown files, and
  external web links.
- Provide **Copy path** and **Open externally** actions.

Out of scope:

- Editing Markdown inside OrkWorks.
- General source-code or binary file viewing.
- Searching across documents or adding a document outline.
- Automatically discovering specs or plans that were not printed in a
  terminal.
- A review queue, read/dismissed state, digests, or review history.
- Changing how the existing session-detail **Open plan** action behaves.
- Opening paths outside the active workspace.

This design supersedes the earlier session-plan handoff design's blanket
Markdown-viewer non-goal only for workspace Markdown paths explicitly selected
from a terminal. The session-reported **Open plan** flow remains unchanged.

## Decision

Register a custom xterm `ILinkProvider` for Markdown path candidates. The
provider finds syntactic candidates, then asks Electron main to validate
bounded batches against trusted workspace and session context. Only eligible
files receive link affordances. Activation sends the session ID plus the
displayed path candidate through a narrow preload method and repeats
validation before reading a bounded UTF-8 snapshot. Electron main returns only
a safe document descriptor to the renderer.

The renderer opens the descriptor as a dynamic Dockview panel positioned
`within` the Terminal panel's group. Document panels are workspace-scoped rather
than session-owned. They coexist with the Terminal panel as tabs, preserving
OrkWorks' single-active-context principle: several contexts may be available in
the tab strip, but only one terminal or document is visible at a time.

Use `react-markdown` with `remark-gfm` for rendering. Do not enable raw HTML.

## User experience

### Link presentation

An eligible terminal path receives normal link affordances:

- pointer cursor;
- underline on hover;
- the existing link color remains usable;
- activation on a normal primary-button click;
- keyboard activation when xterm exposes the link through its accessible
  buffer.

Terminal input and selection keep their normal behavior. Text that resembles a
path but does not resolve to an eligible file remains ordinary terminal text.
Failure to open a path produces a toast and does not type into, detach, resize,
or otherwise disturb the terminal.

### Document tabs

Selecting a link creates and activates a document panel in the Terminal panel's
Dockview group:

```text
[ Terminal ] [ example-design.md × ] [ another-spec.md × ]
```

Rules:

- The tab title is the filename; its tooltip contains the workspace-relative
  path.
- Document panels use a closeable tab component. Existing structural panels
  retain their current hidden-close behavior.
- The document header shows the workspace-relative path and actions for
  **Copy path** and **Open externally**.
- The same canonical file has only one tab. Repeated activation focuses it.
- Different files receive separate tabs.
- Changing the active session focuses that session's terminal but does not
  close document tabs.
- Ending, forgetting, or closing the originating session does not close its
  documents.
- Closing a document tab releases its file watcher.
- Changing workspaces closes all old document tabs and watchers because their
  validation authority belongs to the previous workspace.
- Dockview layout persistence must not restore stale document contents across
  app restarts. A restored document panel must revalidate and reread its path,
  or be omitted from the persisted layout if dynamic-panel restoration cannot
  preserve that guarantee simply.

The live terminal process remains attached and continues draining output while
a document tab is active.

### Markdown rendering

The reader supports:

- headings and same-document anchors;
- paragraphs, emphasis, lists, and block quotes;
- fenced and inline code;
- tables, task lists, and strikethrough through GitHub-flavored Markdown;
- workspace-relative links to other Markdown files;
- `http` and `https` links.

Raw HTML nodes are not rendered and are never interpreted. The renderer does
not use `rehype-raw` or an equivalent HTML-enabling plugin. Images,
non-Markdown local links, custom URI schemes, embedded scripts, and inline
event handlers are not loaded in the first slice.

Internal Markdown links use the same validated document-open flow. A
same-document `#fragment` scrolls within the current panel. Web links are sent
to Electron main and opened with the system browser after scheme validation.
Other local file types remain inert.

## Path detection

### Candidate forms

The link provider recognizes case-insensitive `.md` paths in these forms:

```text
specs/example.md
./docs/plan.md
../workspace/docs/plan.md
/absolute/workspace/docs/plan.md
C:\workspace\docs\plan.md
`docs/spec with spaces.md`
"docs/spec with spaces.md"
docs/plan.md:42
docs/plan.md:42:7
docs/plan.md#decision
```

Unquoted spaces terminate a candidate. The matcher trims common surrounding
punctuation without consuming meaningful filename characters. Optional
line/column suffixes and fragments are not part of the filesystem path.
Fragments navigate to a same-document anchor. Line and column suffixes are
accepted and stripped for file recognition but do not change the initial
reading position in v1.

The provider must reconstruct candidates that wrap across adjacent terminal
buffer rows. Matching should be implemented as a pure function over buffer text
and cell coordinates so punctuation, wrapping, Windows separators, and
activation ranges can be table-tested without a live terminal.

Candidate existence and containment cannot be decided from terminal text. The
provider sends at most 16 candidates per eligibility request, with each
candidate limited to 1,024 UTF-8 bytes. Positive and negative results are
cached for two seconds by workspace instance, session ID, and candidate.
Activation always revalidates; eligibility is a presentation optimization, not
an authorization.

### Resolution order

Electron main, not the renderer, resolves the candidate:

1. Obtain the active workspace root and the trusted session working directory
   for the supplied session ID.
2. For a relative candidate, try the session directory first when it is inside
   the active workspace.
3. If that candidate does not exist, try the workspace root.
4. For an absolute candidate, use it directly only as input to canonical
   containment validation.
5. Select only an existing regular Markdown file whose canonical path remains
   inside the canonical active workspace.

If both relative locations contain a file, session-directory semantics win
because the link came from that terminal. The renderer-provided display text,
session directory, or workspace path is never trusted as validation authority.

## Security boundary

Reading a file into the renderer is a privileged operation even though the
path was visible in terminal output. The capability is intentionally bounded:

- Preload exposes bounded eligibility and document-open methods rather than a
  general file-read API.
- Electron main obtains the current workspace and session directory from
  trusted application/sidecar state.
- Main canonicalizes the workspace and candidate before containment checks.
- Symlinks, junctions, reparse points, and `..` segments cannot escape the
  workspace.
- The candidate must be a regular file with a case-insensitive `.md`
  extension.
- The file must be valid UTF-8 and no larger than 2 MiB.
- Main repeats file type and containment checks immediately before every read,
  refresh, or external open.
- The reader API never returns a trusted or canonical absolute path. Absolute
  path candidates already printed by a coding tool remain visible as
  untrusted terminal text. A successful read returns only a
  workspace-relative display path, UTF-8 source, version, optional fragment,
  and a stable document key.
- The document key is derived from the canonical workspace-relative path and
  is scoped to the current workspace instance. Electron main retains the
  canonical association only while the document is open, allowing internal
  relative links to keep working if the originating session later disappears.
  The key does not authorize arbitrary later paths.
- Web opening accepts only `http:` and `https:` URLs.
- Markdown raw HTML execution remains disabled.

A compromised renderer could request other Markdown candidates visible or
guessable within the workspace through the narrow preload method. The security
boundary therefore treats read-only access to regular, bounded workspace
Markdown as the explicit maximum authority granted to this UI feature. It does
not grant general filesystem reads.

## Data contracts

The Electron and renderer copies of these contracts remain independent because
`apps/desktop/electron/` and `apps/desktop/src/` cannot import from each other:

```ts
interface WorkspaceMarkdownEligibilityRequest {
  sessionId: string;
  candidates: Array<{
    candidateId: string;
    candidate: string;
  }>;
}

interface WorkspaceMarkdownEligibilityResponse {
  eligibleCandidateIds: string[];
}

type WorkspaceMarkdownRequest =
  | {
      source: "terminal";
      sessionId: string;
      candidate: string;
    }
  | {
      source: "document";
      documentKey: string;
      candidate: string;
    };

interface WorkspaceMarkdownDocument {
  documentKey: string;
  relativePath: string;
  title: string;
  source: string;
  version: string;
  fragment: string | null;
}

type WorkspaceMarkdownEvent =
  | {
      type: "updated";
      documentKey: string;
      source: string;
      version: string;
    }
  | {
      type: "unavailable";
      documentKey: string;
      reason: "missing" | "unsafe" | "too_large" | "invalid_encoding" | "read_failed";
    };
```

`version` is an opaque value based on the validated file snapshot. Callers may
compare it for equality but must not parse it. A document-source request
resolves the candidate relative to the already validated source document's
parent directory. Its document key remains valid only while that tab is open
in the current workspace instance.

## Components and responsibilities

### Terminal Markdown link provider

A renderer-only module:

- parses Markdown path candidates and activation ranges;
- batches syntactic candidates through the bounded eligibility method before
  exposing links;
- caches positive and negative eligibility for two seconds;
- registers one provider per xterm instance;
- asks the app-level document-tab controller to open a selected candidate;
- disposes with the terminal;
- never reads files or resolves absolute paths.

Live and historical terminal components both register the provider. Historical
terminal links use the historical session ID, whose stored working directory
still supplies resolution context.

### Electron workspace Markdown reader

An Electron-main module:

- validates request shape, request bounds, and source authority;
- validates terminal requests against trusted session context;
- validates document requests against the retained canonical source-document
  association;
- resolves and canonicalizes candidates;
- answers bounded eligibility batches without returning resolved paths;
- performs bounded UTF-8 reads;
- returns sanitized descriptors;
- watches parent directories for open documents;
- revalidates and rereads on changes;
- opens validated web links and validated Markdown files externally;
- reference-counts watchers by document key so duplicate subscriptions do not
  create duplicate native watchers.

Watch the parent directory rather than only the file so editors that save by
atomic replacement still produce refreshes. Debounce events for 150 ms.

### Document-tab controller

An app-level renderer module:

- maps document keys to Dockview panel IDs;
- adds new panels `within` the Terminal panel's group;
- activates an existing panel for duplicate opens;
- retains tabs across session changes;
- closes all document panels on workspace change;
- subscribes/unsubscribes to Electron document events;
- releases the retained main-process document association when the last panel
  reference closes;
- prevents stale events from a previous workspace instance from updating new
  tabs.

### Markdown document panel

A focused renderer component:

- renders safe GitHub-flavored Markdown;
- handles anchor, internal Markdown, and external web links;
- displays relative path and actions;
- owns reading position for its panel;
- shows loading, unavailable, and updated states;
- remains read-only.

## Live refresh

On a validated directory-watch event, Electron debounces, revalidates, and
rereads the document before emitting `updated`. The renderer accepts only an
event for the current workspace instance and a different version.

Before replacing the rendered source, the panel records:

1. the nearest visible heading anchor, when one exists; and
2. the fractional scroll position as fallback.

After rendering, it restores the heading offset or fractional position and
shows a brief **Updated** indicator. Refresh does not activate a background tab.

If the file is deleted, becomes unsafe, exceeds the size limit, or cannot be
decoded, the panel retains the last readable content and adds an
**Unavailable** banner. The directory watcher remains active; if an eligible
file reappears at the same path, the panel refreshes and clears the banner.

## Failure handling

- A candidate that no longer exists on click produces a concise toast.
- An invalid or outside-workspace candidate is treated as unavailable without
  revealing the rejected canonical path.
- A too-large or invalid-encoding file reports that specific reason.
- A Markdown render failure shows the source as escaped plain text rather than
  a blank panel.
- A watcher failure leaves the current snapshot readable and reports that live
  updates are unavailable.
- A stale event from a prior workspace or old document version is ignored.
- Closing a tab or changing workspace always disposes watcher subscriptions,
  even when a read or render is in flight.

No failure changes session metadata, terminal lifecycle, terminal input, or
the underlying document.

## Accessibility and keyboard behavior

- Document tabs use Dockview's normal keyboard focus and tab activation.
- Each tab has an accessible close label containing the filename.
- The reader is a navigable document region with its title as the accessible
  name.
- Rendered links are keyboard-focusable and show visible focus.
- Code blocks and tables remain horizontally scrollable without moving the
  whole app.
- **Copy path** and **Open externally** expose accessible names and success or
  failure feedback.
- Closing the active document returns focus to the previously active panel,
  normally the Terminal panel.

## Data flow

```text
coding tool prints workspace Markdown path
  -> xterm link provider identifies candidate/range
  -> preload sends a bounded eligibility batch with trusted session identity
  -> main returns only eligible candidate IDs
  -> provider exposes eligible ranges as terminal links
  -> user activates link
  -> preload sends terminal source + session ID + displayed candidate
  -> main obtains trusted workspace/session context
  -> main resolves, canonicalizes, contains, type-checks, bounds, and reads
  -> renderer receives sanitized workspace-relative document descriptor
  -> document-tab controller focuses existing key or adds panel within Terminal group
  -> Markdown panel renders safe GFM
  -> Electron directory watcher revalidates and emits later snapshot updates

rendered Markdown link points to another Markdown file
  -> panel sends document source + current document key + link candidate
  -> main resolves relative to the retained validated source document
  -> the same validation, read, and tab-deduplication flow continues
```

## Testing

### Candidate parsing

Table tests cover:

- relative, absolute, Unix, and Windows paths;
- uppercase `.MD`;
- quoted/backticked paths containing spaces;
- trailing sentence punctuation and Markdown punctuation;
- line, column, and fragment suffixes;
- multiple paths on one row;
- paths wrapped across terminal rows;
- precise xterm activation ranges;
- false positives such as missing extensions, URLs, and ordinary dotted text;
- batches larger than 16 candidates and candidates larger than 1,024 UTF-8
  bytes;
- positive and negative cache expiry;
- missing or unsafe syntactic candidates remaining plain terminal text.

### Electron security

Tests with temporary workspaces cover:

- session-directory and workspace-root resolution order;
- existing regular Markdown files;
- missing files and directories;
- extension and UTF-8 checks;
- the 2 MiB boundary;
- absolute and relative traversal;
- file and directory symlink escapes;
- Windows junction/reparse-point escapes where supported;
- file replacement between validation and read;
- activation revalidation after a positive eligibility result;
- sanitized responses without trusted or canonical absolute paths;
- terminal-source session validation;
- document-source relative resolution after the originating session is gone;
- rejected expired, closed-tab, and previous-workspace document keys;
- rejection of non-HTTP web schemes.

### Tabs and rendering

Renderer tests cover:

- first click creates and activates a panel in the Terminal group;
- duplicate canonical path focuses the existing tab;
- distinct files produce distinct tabs;
- tabs survive active-session changes and originating-session termination;
- internal Markdown links still open after originating-session termination;
- workspace change closes every document tab and subscription;
- safe GFM rendering with raw HTML disabled;
- anchors, internal Markdown links, and web links;
- copy/open-external actions;
- keyboard activation and focus return.

### Refresh

Tests cover:

- ordinary writes and atomic replacement;
- 150 ms debounce;
- duplicate watcher subscriptions;
- heading and fractional scroll restoration;
- no focus steal from background updates;
- deletion/unavailable state and reappearance;
- watcher failure;
- stale workspace/version event rejection;
- watcher disposal on tab close and workspace change.

Existing terminal attachment, selection, input, replay, resize, and
session-switching tests remain green.

## Documentation and issue tracking

Before implementation code:

1. Update `specs/orkworks-mvp.md` to allow explicit, bounded, read-only
   workspace Markdown viewing inside OrkWorks.
2. Update `specs/review-queue.md` only to clarify that terminal-clicked document
   tabs are an independent navigation surface, not automatic discovery or queue
   state.
3. Add or supersede an ADR if the renderer's bounded workspace-Markdown read
   authority materially changes ADR 0009 or ADR 0025's security boundary.
4. Create a scoped implementation issue and link the earlier session-plan
   handoff work without reopening its external-open scope.
5. Update `docs/agents/architecture.md`, `README.md`, and relevant Electron
   security documentation when implementation lands.

## Acceptance criteria

- [ ] An existing workspace-contained Markdown path printed in a live or
      historical terminal is visibly clickable.
- [ ] Missing, unsafe, and otherwise ineligible Markdown-looking candidates
      remain ordinary terminal text, and activation repeats validation.
- [ ] Clicking opens or focuses one safe, read-only document tab in the
      Terminal Dockview group.
- [ ] Multiple document tabs coexist with the Terminal tab and survive session
      changes or originating-session termination.
- [ ] The same canonical file never produces duplicate tabs.
- [ ] Changing workspace closes old document tabs and watchers.
- [ ] GitHub-flavored Markdown renders without executing raw HTML.
- [ ] Internal Markdown links use the same safe tab flow; web links allow only
      HTTP and HTTPS.
- [ ] Only canonical regular UTF-8 `.md` files no larger than 2 MiB and
      contained by the active workspace are read.
- [ ] The reader API never returns a trusted or canonical absolute filesystem
      path or exposes a general filesystem-read capability; absolute candidates
      printed in terminal output remain untrusted text.
- [ ] Open documents refresh after ordinary and atomic-replacement saves while
      preserving reading position and not stealing focus.
- [ ] Deleted or newly unsafe files retain their last readable snapshot with an
      unavailable state and can recover if the file reappears.
- [ ] Terminal input, selection, attachment, lifecycle, replay, and resizing
      behavior remain unchanged.
- [ ] No editing, general file viewer, automatic discovery, review queue, or
      review-state behavior is introduced.
