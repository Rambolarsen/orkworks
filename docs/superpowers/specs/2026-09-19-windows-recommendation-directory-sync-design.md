# Windows recommendation handoff directory-sync design

## Problem

On Windows, accepting a Taskmaster recommendation can fail with the renderer
toast `Couldn't send the fix to the session.` The sidecar has already written
the recommendation transaction's files, but recovery then reports
`PermissionDenied` while trying to open the transaction directory for a Unix-
style directory `fsync`.

This is an API-semantics mismatch, not a missing user ACL: Windows directory
handles require special Win32 open flags, while `std::fs::File::open` requests
a normal file handle. The actual file writes and Windows atomic publication
can succeed before this durability-only check fails.

## Chosen approach

Make recommendation-store directory syncing best-effort on Windows by making
that helper return success without opening the directory. Keep the existing
file-level durability and publication behavior intact:

- staged files are flushed before publication;
- temporary files are closed before Windows replacement so `ReplaceFileW` does
  not fail on an open temporary handle, including the single-record `put`
  path used when accepting a recommendation;
- Windows uses `ReplaceFileW` for existing targets and `MoveFileExW` with
  `MOVEFILE_WRITE_THROUGH` for new targets;
- Unix continues to open and sync directories, reporting real failures.

This avoids asking users to run OrkWorks elevated or weakening file-write and
rename errors. Treating only `PermissionDenied` as benign was rejected because
it could hide a genuine ACL failure on a path that should otherwise be
accessible. Windows cannot provide the same directory-entry crash durability as
Unix here: `MoveFileExW` requests write-through for new files, but
`ReplaceFileW` has no equivalent write-through flag. The implementation should
document this platform limitation rather than imply that directory metadata is
fully power-loss durable.

## Testing

Add a Windows regression that exercises the full failure path: publish a graph
transaction, recover it, list the recommendation, and accept the recommendation
into a live session. Add a platform-specific helper test documenting that
directory sync is best-effort on Windows, while retaining the missing-directory
failure test on Unix. Ensure the Windows CI test list runs the recommendation
store and acceptance tests. Run the focused Rust tests, full Rust formatting and
tests, and the desktop type/test checks required by the scoped instructions.
