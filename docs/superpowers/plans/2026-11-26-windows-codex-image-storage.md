---
type: Implementation Plan
title: Windows Codex image storage and delivery
description: Proposed private Windows bundle storage and qualification for new Codex sessions with images.
tags: [orkworks, sessions, images, windows]
status: draft
workflow_status: active
---

# Windows Codex image storage and delivery

[Issue #1674](https://github.com/Rambolarsen/orkworks/issues/1674) delivers the
existing New session image flow on Windows. The [image-context contract](../../../specs/session-image-context.md)
and [ADR 0089](../../adr/0089-new-session-image-context.md) govern this work.
The sidecar currently restricts native Codex delivery to macOS arm64 and has no
Windows bundle implementation. On 2026-11-26, the installed WinGet x64 Codex
binary returned `codex-cli 0.172.0` and advertised `-i, --image <FILE>...`.
Both bounded probes exited successfully. This confirms the installed interface.

## Proposed first Windows backend

- Qualify Windows x64 with a local NTFS workspace metadata root and the installed
  native Codex executable that passes the live fixture. Require that exact
  qualified stable version and successful bounded version/help checks before
  initial activation. Resolve the same executable for the probe and native PTY
  launch. Shell shims and different builds require their own evidence.
- Keep images in the existing workspace metadata root under `session-images`.
  From a verified local volume root, walk one validated component at a time
  to the metadata root. Open existing components without following reparses
  and verify kind and attributes on each returned handle. Retain the parent
  handles through owned operations. Reject unsupported or uncertain roots.
- Create each private object exclusively with a protected DACL granting access
  to the sidecar user's SID and verify ownership, DACL and object kind through
  its handle. Prefer documented, single-component parent-handle `NtCreateFile`
  operations for atomic creation and no-follow reopen. Verify the exact options
  on the target platform before relying on them. Regular files must have one
  hard link. Treat every reparse, unexpected entry, or ambiguous handle as a
  failure. The same user runs Codex, so its real startup read must also pass.
  [Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)
  · [Security descriptors for new objects](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptors-for-new-objects)
  · [GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo)
- Reuse the existing nonblocking image lock, bounded owner manifest, generated
  names, staging directory and accepted-bundle lifetime. Flush the manifest
  and images and publish by same-directory, no-replace rename on the owned
  staging handle before persisting session metadata. Qualify rename, flush
  ordering and process interruption on NTFS. Remove verified files and
  directories by handle. Keep the manifest until owned leaf cleanup succeeds.
  [Same-directory native rename](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
  · [Microsoft SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
  · [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
- Reuse ordinary JSON retention and recovery. Hold the cleanup lease until
  metadata deletion, and keep retry ownership on failure. Startup recovery
  requires an owned manifest and definitive absence of both live and corrupt
  session records. Preserve a bundle if metadata or its object identity is
  uncertain. The separate SQLite cleanup route requires its own qualification.

## Implementation and checks

1. After scoped storage-design approval, record the Windows extension to ADR
   0089 and implement the backend behind existing image storage interfaces.
   Use native synthetic fixtures to verify owner access and denial to another
   ordinary identity, exclusive creation, reparse and hard-link refusal,
   non-replacement, cleanup contention and retry, and failures on either side
   of session-record persistence. Verify that neither publishing nor recovery
   modifies a link target, an unrelated bundle, or retained metadata.
2. With production capability still disabled, adapt the existing ignored
   `qualify_installed_image_delivery` test for an owner-run disposable Windows
   session. Exercise the actual storage, ordinary PTY, and resolved native
   executable. Retain the four visual results in order for multiple images, the
   image-only description, exactly one initial turn in each success case, paths
   with spaces and non-ASCII, and a denied read with visible failure. Check the
   path handed to Codex resolves to the same verified bundle at dispatch.
   Record CLI version, platform, provider/model, and cleanup outcomes. This
   coding session can run synthetic tests. The owner runs authenticated live
   qualification in a separate disposable context.
3. Enable the qualified route through the existing handshake and enforce it
   again at session creation. Require the eligible storage backend as well as
   the executable probe. Cover unsupported versions, failing probes,
   workspace changes, image failures, text-only startup and macOS compatibility.
   Update the source-level harness and user documentation. Run the focused native
   regressions, full repository verification, and current-diff code review
   before merging the activation.

## Review and recovery

The owner approved the Windows custody boundary on 2026-11-26. Implementation and native qualification are in progress.
The backend and existing launch/retention callers are known (dependencies 3),
and normal session creation shares the change (blast radius 3). The bundle
format and API are additive (state changes 2). Disable capability to roll back
while retaining cleanup for existing bundles (reversibility 3). NTFS handle
semantics, pathname handoff and native prompts still require target evidence
(uncertainty 4). If the backend cannot establish containment or publication
ordering, keep the route disabled and bring back the specific design gap.

The current scope preserves the same local-user security model as Unix,
including the user's ability to access their own bundle. Owner-only ACLs are
the filesystem boundary. Confirm normal launch works without changing access
permissions, Codex sandbox settings, or the workspace's trust configuration.

## Running the disposable Windows fixture

Run the ignored test in an owner-controlled Windows x64 checkout using the
installed native executable and existing Codex login. The checkout must be
trusted through the normal Codex workflow. Optionally set
`ORKWORKS_IMAGE_FIXTURE_MODEL` to choose the model to qualify.

```powershell
$env:ORKWORKS_IMAGE_FIXTURE_CODEX = "$env:LOCALAPPDATA\Microsoft\WinGet\Links\codex.exe"
cargo test --locked --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd image_context::tests::qualify_installed_image_delivery -- --exact --ignored --nocapture
```

Use the absolute native executable path for the actual install if it differs.
The fixture creates temporary NTFS storage with spaces and non-ASCII in its
path. It makes at most three normal OrkWorks PTY starts: ordered multiple
images with text, image-only, and an image-only start that encounters an
exclusive Windows read lock. It checks the published volume/file identities
at dispatch. Inspect the retained output for the version, actual model,
expected visual results and one initial turn for each successful start,
plus the visible denied image read.

The focused Windows image suite passed 11 tests on Windows x64. The live
fixture remains opt-in, and the separate recovery helper runs only as a
synthetic subprocess. The suite covers protected ACL shape, refusal of an
inherited ACL, reparse and hard-link refusal, no-replace publication, pinned
dispatch identity, share-delete readers, rollback and process-exit recovery.
Process exit does not establish power-loss or atomic-media flush ordering.

Before compiling the Windows backend into production, confirm publication
ordering on the supported NTFS profile and denial to another ordinary user.
Preserve ordinary text-only Forget and retention on storage ineligible for
images. The current backend and the launch override are test-only.
