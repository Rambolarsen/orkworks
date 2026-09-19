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
- Windows uses `ReplaceFileW` for existing targets and `MoveFileExW` with
  `MOVEFILE_WRITE_THROUGH` for new targets;
- Unix continues to open and sync directories, reporting real failures.

This avoids asking users to run OrkWorks elevated or weakening file-write and
rename errors. Treating only `PermissionDenied` as benign was rejected because
it could hide a genuine ACL failure on a path that should otherwise be
accessible.

## Testing

Use the existing Windows recommendation acceptance regression, which currently
reproduces the failure while listing the recommendation after a graph
transaction. Add a platform-specific helper test documenting that directory
sync is best-effort on Windows, while retaining the missing-directory failure
test on Unix. Run the focused Rust tests, full Rust formatting/tests, and the
desktop type/test checks required by the scoped instructions.
