# New-session image context

Status: approved by the repository owner on 2026-10-23 (“go ahead”). Implementation and delivery qualification are in progress.
Issue: [#979](https://github.com/Rambolarsen/orkworks/issues/979).
Decision: [ADR 0089](../docs/adr/0089-new-session-image-context.md).

## Outcome and scope

A user starting an ordinary agent session can paste a screenshot, drag image
files into the composer, or select multiple images with an Attach images button.
All three inputs populate one ordered list of thumbnails with accessible filenames
and Remove controls. The same images accompany the initial prompt. Image-only
starts are allowed, with the fixed instruction “Describe the attached images.”
The user explicitly requested all three input methods on 2026-10-23.

This covers the transient New agent session composer only. Existing-session
attachments, PDFs, remote URLs, screenshot capture, image editing, and automatic
Taskmaster attachments are outside this change.

## Composer behavior

- Paste image clipboard items while focus is within the composer. Preserve normal
  text paste when no image items are present. For mixed image/text clipboard
  contents, attach the images and insert any plain text at the prompt selection
  once; never insert HTML or duplicate the same clipboard image via two APIs.
- Accept file drops within the composer and prevent browser navigation there.
  Ignore drag operations outside it. Reject directories, URL-only drops, and
  unsupported files with an inline explanation.
- The keyboard-accessible Attach images button opens a multi-file picker.
  Selecting the same file again after removal works. Preserve selection order;
  separate intentional additions may contain identical images.
- Support PNG, JPEG, and WebP. Proposed limits are eight images, 5 MiB per image,
  20 MiB combined source bytes, 16 megapixels per image, and 40 megapixels
  combined. Display names are at most 255 UTF-8 bytes; image-bearing requests
  accept at most 64 KiB of UTF-8 prompt text. Validate actual
  content and successful decoding, not only extension or declared MIME type.
  Preserve original bytes; no implicit compression, conversion, or cropping.
- Process each addition as an atomic batch: on any rejected item keep the existing
  list and identify the invalid item. Disable Start while ingestion is pending.
  Keep ingestion bounded before reading or decoding full contents. Inspect image
  dimensions before full decode; validate one full-size image at a time, release
  it before the next, and retain previews at most 512 pixels on either edge.
  Browser previews must not retain all original-size decoded surfaces.
- Revoke thumbnail object URLs when removed or unmounted. Attachments stay in
  memory until Start; do not persist them in localStorage or workspace UI memory.
- Cancel discards the draft. Workspace switching discards it through the existing
  composer lifecycle; late async ingestion/confirmation must not cross workspaces.
- Freeze the submitted draft while Start is pending. A synchronous error preserves
  prompt/images and displays its reason. After successful creation, the existing
  observable creating/error session lifecycle owns asynchronous launch failures.
- Changing coding tool retains attachments. Unsupported tools show an explanation
  and disable Start until the images are removed or a supported tool is selected.
  Model image capability may still be rejected by the tool/provider; do not invent
  a model allow-list or change the model silently.

## Ownership and transport

The renderer owns browser File/Blob ingestion and preview state. Electron main
owns a narrow create-session-with-images IPC call. Its arguments contain the
selected workspace identity, normal creation options, and bounded image bytes
with display names. Never accept renderer-supplied local filesystem destinations
or read arbitrary paths on behalf of the renderer. Revalidate lengths, types,
counts, and the selected workspace in main. Keep Electron/renderer types in each
side's own directory, following the existing intentional duplication rule.

Main captures the workspace generation and exact sidecar identity at IPC receipt,
then rechecks both immediately before forwarding. A switch before submission
cancels the request; never retarget it to the new workspace. Main uses the captured
sidecar and its main-only authentication secret. A switch after submission cannot
undo an accepted creation in the original workspace, but its late response must
not navigate or update the new workspace; the old session remains discoverable
when returning to its workspace. Extend POST /sessions additively with optional
initialImages entries containing name, mimeType, and dataBase64. Nonempty images
require the existing main-only token; existing text-only callers remain compatible.
Bound this route to 28 MiB encoded JSON and enforce the decoded limits above;
do not increase unrelated route limits. Invalid or unsupported image requests fail
before creating a session or writing files. Never log image payloads or put bytes
in session-list responses, task summaries, or Peon inference inputs.

The sidecar is the sole owner of an image bundle under the selected workspace's
private OrkWorks storage, scoped by generated session ID. Generate storage names
and extensions from validated content; display names are inert labels. Use private
permissions (0700 directory/0600 files on Unix, equivalent owner-only access on
Windows), exclusive file creation, and no-follow/contained operations. Decode
with bounded dimensions before acceptance; decoder choice is an implementation
choice and any added dependency must be justified against existing libraries.

Write files and a bounded ownership manifest into a uniquely named staging
directory. Flush and atomically rename the complete bundle to its generated
session-ID destination before persisting the creating session record; launch
only after both complete. A crash between publication and metadata persistence
leaves an orphan bundle, never a launched session with missing files. A crash
after persistence leaves a complete bundle for existing startup reconciliation.
Any pre-acceptance failure removes only this request's files and returns an error. Keep accepted files with remembered
session state, including after runtime exit, so native resume can still resolve
references. Forget/retention deletes the bundle along with the owning session;
workspace switching and renderer detachment do not delete it. On startup, incomplete staging
and published bundles without a persisted session record are cleanup candidates
only after exclusive ownership and abandonment are established. Never remove a
bundle referenced by retained metadata or belonging to another live writer. If
ownership or metadata reads are uncertain, retain files and report a diagnostic. This adds no migration of
existing session records: missing bundles mean an older text-only session.

## Coding-tool delivery

Use a small compiled delivery policy alongside the existing harness launch
adapter, not user-configurable shell interpolation. Expose its resolved support
to the composer and enforce it again server-side. Custom tools and the generic
shell default to unsupported. Installed-tool version/feature evidence must gate
support; names alone are insufficient. Do not widen filesystem access, override
permissions, alter global configuration, or automate trust/approval dialogs.

Target these routes, enabling each only after an OrkWorks-launched fixture verifies
that the model can distinguish the image content:

| Coding tool | Delivery route | Current evidence and remaining gate |
| --- | --- | --- |
| Codex | Generated file arguments through native `--image` and the text as one positional argv prompt; disable queued PTY submission for this route | [Official CLI documentation](https://learn.chatgpt.com/docs/codex/cli) documents image context; installed `codex --help` advertises `--image <FILE>...` on 2026-10-23. Verify images and queued text form exactly one initial turn, including image-only starts; help alone does not establish that behavior. |
| Claude Code | Explicit local image references in the initial prompt, instructing the agent to inspect each image | [Official image workflow](https://code.claude.com/docs/en/common-workflows#work-with-images) supports image paths. Verify private-storage reads under ordinary permissions and paths with spaces. |
| OpenCode 2 | Explicit local image references and image-capable `read` in the initial prompt | [Official attachments documentation](https://dev.opencode.ai/v2/docs/attachments/) documents image processing by `read`. Verify the installed V2 runtime reads private files and passes their images to the model. |
| Copilot | Interactive image file references using documented `@` syntax | [Official CLI workflow](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/overview) documents image references. Verify bracketed-paste interpretation and filenames with spaces before enabling. Do not use its noninteractive-only attachment flag for an interactive session. |

Any unverified route remains disabled with an honest reason. Record exact tool
version, platform, fixture outcome, and provider/model with delivery evidence.
A provider rejection becomes an actionable startup/runtime error; never silently
launch without the selected images. Treat successful PTY submission as delivery,
not proof of model understanding. Preserve existing startup-readiness, trust
chooser, cancellation, process-isolation, and single-owner initial-prompt rules.
Text prompts must not interpolate filenames as executable shell syntax. Pass
argv directly and serialize file references as data for each tool's syntax.

## Acceptance and implementation outline

1. Add bounded shared-purpose ingestion logic and a single attachment list to
   NewSessionDialog, with paste/drop/picker UI and lifecycle/error handling.
2. Extend the renderer/main IPC and authenticated session-creation boundary;
   sidecar validates and owns the private bundle and retention cleanup.
3. Integrate verified delivery policies with harness launch and existing queued
   prompt submission. Keep unverified tools disabled and plain-text sessions
   unchanged. Update runtime and harness evidence references as routes qualify.
4. Verify UI ingestion and preview removal, mixed clipboard content, rejected
   batches, limits, corrupt images, repeated selection, workspace/cancel races,
   failed Start/retry, and unsupported tool selection. Backend tests cover auth,
   decoded/encoded bounds, inert names, cleanup, retained resume, and old clients.
5. For every enabled route, use an image with a distinctive non-text visual and
   verify the agent identifies it from the actual image. Include multiple images,
   image-only starts, paths with spaces/non-ASCII, and a denied image read.
   Fake CLI argv fixtures are supplemental, not capability evidence.

After implementation run `bash scripts/verify-repo.sh` and the scoped desktop
and Rust checks required by their AGENTS.md guides. At close-out run documentation
checks and the worktree hook. Runtime code needs the repository's `/code-review
low` gate before merge. Documentation alone does not establish a shipped feature.

## Compatibility, recovery, and review

Omitting initialImages preserves the existing API and initial-prompt flow. No
image bytes are added to native session identity or attention metadata. A revert
can disable image submission while leaving accepted bundles with their retained
sessions; do not delete user context during rollback. Old binaries may ignore an
unknown optional JSON field, so main must positively negotiate image support
before submitting images and refuse a sidecar without it. Implement this as an
explicit additive capability in the existing sidecar handshake, not a version guess.

Preparation review: Ready for execution; the owner approved the contract on 2026-10-23.
Dependencies 3 (renderer, Electron, sidecar, and known tool adapters); blast radius
3 (session startup, with a separate optional path); state changes 3 (additive
private bundles and optional API field); reversibility 3 (disable submission,
retain accepted files); uncertainty 3 (documented routes, bounded installed
fixtures still required). Total 15/25; these ratings do not grant approval.

Material review checks: user requirements all covered; no generic upload service
or new agent runtime; capability evidence remains an implementation gate; local
storage and sandbox permissions must be proven together. The owner approved runtime implementation on 2026-10-23.

Builder review: a fresh read-only reviewer identified missing aggregate decoded
image bounds, name/prompt overhead bounds, crash publication ordering, and exact
workspace-generation fencing. The contract above incorporates each correction.
Delivery fixtures remain required evidence, rather than claims of current support.
The doc-check README warning needs no README edit: this is a proposed contract,
not a change to current architecture or shipped feature claims. The initial
worktree warning classified the branch as merged because it still pointed at
main before its first commit; it does not establish a merged PR.
