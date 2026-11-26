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
The production sidecar currently restricts native Codex image delivery to macOS
arm64. On 2026-11-26, the installed WinGet x64 Codex
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
2. With production capability disabled, qualify attachment delivery through
   actual storage, the ordinary PTY, and the resolved native Codex executable.
   For multiple images, compare the inline payloads to the published files
   byte-for-byte and in order. For an image-only prompt, verify its inline
   payload in the same way. Capture the request with a disposable local
   Responses endpoint. Confirm that each successful start submits one initial
   user turn and that paths with spaces and non-ASCII resolve to the verified
   bundle at dispatch. A locked image must report a typed error before Codex
   starts. Retain the executable version, platform, provider/model, and cleanup
   outcomes. Use the authenticated disposable run as evidence of native startup.
   Record the model's visual answer for diagnosis. Evaluate delivery using the
   captured payloads and startup evidence.
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
path. It attempts two normal OrkWorks PTY starts with readable files, one
with multiple images and text and one with an image-only prompt. The third
case holds an exclusive read lock before preflight and requires a typed
startup error with no Codex session PID. The fixture checks the published
volume and file IDs at dispatch. Inspect the retained output for the version,
model, and one initial turn for each successful start. The current ignored
test still checks historical visual answers. Use the payload comparison and
startup evidence above to evaluate delivery.

The focused Windows image suite passed 11 tests on Windows x64. The live
fixture remains opt-in, and the separate recovery helper runs only as a
synthetic subprocess. The suite covers protected ACL shape, refusal of an
inherited ACL, reparse and hard-link refusal, no-replace publication, pinned
dispatch identity, share-delete readers, rollback and process-exit recovery.
Process exit does not establish power-loss or atomic-media flush ordering.

Before compiling the Windows backend into production, confirm the remaining
storage checks.

- Have a second ordinary Windows account attempt to open an attachment by
  its known path. Require denial and confirm that the owner can read it.
- Confirm native NTFS rename and flush ordering through publication and
  session-record persistence. Reuse the same-volume no-replace and
  process-exit recovery evidence for those parts of the sequence.
- Verify Forget and retention for a text-only session with a UUID on a root
  that is ineligible for image storage. The current deletion flow attempts
  to open the private image store for any UUID. An image-profile error can
  stop ordinary deletion before metadata removal. Check that ambiguous
  ownership still preserves any bundle and its metadata.

The native storage tests passed on fixed local NTFS (five passed, with the
subprocess helper ignored when run directly). The existing retention suite
and Forget handler tests passed (25 and three). These tests exercise
ordinary deletion and recovery on the current profile. A regression using
two text-only UUID sessions under a disposable directory alias reproduced
the gap. Forget returned an application error and age retention left its
expired record. The Windows image anchor refused the alias before checking
whether the image store existed. The owner approved a read-only absence
check for this case. Resolve and retain the ordinary metadata directory
identity, then look up the image-store name relative to that handle.
Revalidate the identity and absence before deleting the session record.
Open and remove metadata files relative to the same retained directory on
this path. A present or unverifiable image store keeps the cleanup error.
That evidence covers the final lookup. Ordinary creation assigns a fresh
UUID, so it cannot publish images for the record being deleted. Use the
existing verified image cleanup for eligible roots.

The regression now passes for Forget and age retention. A second test
preserves the records when an image store appears before the final check
or the directory alias changes. The Windows backend and launch override
are test-only.

## Windows live fixture result (2026-11-26)

The owner authorized the three disposable starts. The fixture used the native
WinGet Codex 0.172.0 executable, the installed login, and the configured default
model. The actual model identifier was not retained. The multiple-image case
ran at `6455ca3b`. The remaining cases ran with the ACL verification and case
selection changes committed as `86ce3b7f`.

| Case | Observation | Qualification result |
| --- | --- | --- |
| Multiple images and prompt | Codex answered “Red, white, blue, white.” The expected order was red, green, blue, orange. | Failed the color check. |
| Image-only | Codex described a solid red circle on white and omitted the green rectangle. | Failed the color check. |
| Denied read | With the exclusive read lock held, Codex continued the text turn, tried local inspection, and reported that the file was locked by another process with no accessible image. | The terminal showed the lock explanation. The fixture did not observe its expected OS error and the launch continued. |

An offline probe regenerated both PNGs using the unchanged fixture generator,
decoded them, and verified the dimensions (256 × 128), the colored centers of
both shapes, and the white background. This does not reveal the image bytes
the provider consumed. The common omission of the right-hand shape remains
unexplained.

The total was three starts. No live case was repeated. The batch does not
qualify the Windows route. The ACL review correction replaced direct reads
of the returned ACL header with `GetAclInformation` and bounds the ACE bytes
before comparing them. All 11 focused Windows image tests passed afterward.

## Follow-up offline payload trace and preflight

The disposable loopback provider captured native Codex 0.172.0 Responses
requests for the requested GPT-5.4 model. An isolated Codex profile used an
unauthenticated local endpoint. The harness saved only fixture PNG bytes and
served fixed synthetic replies. This trace measures local preparation and
serialization without making a real inference call.

| Native startup case | Source PNG bytes | Request PNG bytes | Comparison |
| --- | --- | --- | --- |
| Multiple images, first attachment | 1123 | 1123 | Byte-identical, SHA-256 `962df75e9af659ec944fd5ab1731ce22a791efff947d893a35aff22ca29d5044` |
| Multiple images, second attachment | 1122 | 1122 | Byte-identical, SHA-256 `4023e88fb0ee67c3642bfea7fae7b64985ffae15237d77b2b5f3003b421515e2` |
| Image-only | 1123 | 1123 | Byte-identical to the first attachment above |

The initial loopback attempt hit folder trust before it could send an image.
The disposable profile then used the native Git common root Codex identified.
The multiple-image request supplied both PNGs. A separate image-only run
bound the inline image to its exact published path and completed successfully.
These captures match the existing offline check of both shape colors. The
trace establishes the client boundary for the synthetic provider. Hosted
processing and model interpretation are outside that boundary.
[Codex 0.172.0 image serialization](https://github.com/openai/codex/blob/rust-v0.172.0/codex-rs/protocol/src/models.rs#L1941-L1994)

The locked-file regression failed when the lock was acquired after preflight.
It passed after moving the lock before path verification. The qualification
route now records `image_attachment_unavailable` using the existing startup
failure path and the desktop's local message catalog. The offline denied case
recorded `status=error`, the attachment-specific code, and no Codex session
PID. This covers a read failure present at preflight. Acknowledgement from
Codex for a later read failure remains unverified.

The focused image suite passed 12 tests, the Rust diagnostic suite passed
two, and the desktop diagnostic tests and type-check passed.

## Follow-up live result (2026-11-26)

The installed Codex 0.172.0 rejected GPT-5.4 as retired before either readable
case submitted a turn. The denied-read case passed in that batch. OrkWorks
reported `image_attachment_unavailable` with no Codex session PID.

The two readable cases ran once each with the recommended replacement,
`gpt-6-sol`, selected for only the fixture process.

| Case | Observation | Qualification result |
| --- | --- | --- |
| Multiple images and prompt | Codex answered red, white, blue, white. The expected order was red, green, blue, orange. | Failed the color check. |
| Image-only | Codex described the red circle on the left of a white rectangular background. It omitted the green rectangle. | Failed the color check. |

The color errors repeat the earlier live results. The offline trace verifies
the PNG bytes dispatched by the installed client to a synthetic endpoint.
The cause of the live discrepancy remains open. This batch left the
original visual check unmet.

## Direct Windows Codex control (2026-11-26)

A single interactive PowerShell launch used the same installed Codex 0.172.0
executable, GPT-6 Sol, current login, two hash-verified fixture PNGs in the
same order, and the exact multiple-image prompt. It ran from the qualification
worktree with the CLI's read-only sandbox setting. The CLI opened both local
PNGs directly with `--image` and displayed two image attachments before
submitting the prompt.

The direct run answered **red, white, blue, white**, matching the prior
OrkWorks run. The expected order was red, green, blue, orange. The
multiple-image color error therefore also occurs in a direct Codex session.
Together with the byte-identical offline payloads, this gives evidence about
the OrkWorks handoff independent of the model's color answer. The direct
control covered the multiple-image case. The image-only result and the
native cause of the color error remain unresolved.
