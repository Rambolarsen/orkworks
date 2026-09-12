# Coding tool settings regression fixes

Approved scope: restore expand/collapse without discarding unsaved command-path edits, investigate Claude Code integration installation on Windows, and cover confirmed defects with regression checks.

The details subsection remains mounted. Its `hidden` attribute must take precedence over the flex layout. The disclosure button and row retain their existing event handling.

The running Windows sidecar reports Claude Code as detected and enabled, with registration `absent`, ownership `none`, and activation `disabled`. An off/on retry leaves the same message. Trace the existing renderer → preload → Electron orchestration → sidecar path; preserve explicit confirmation and truthful registration status. Do not label an absent integration healthy.

This restores existing settings and integration behavior under the MVP's resolved harness capabilities and integrations scope. It adds no capabilities, dependencies, or architecture changes.

## Confirmed installation defects

The user confirmed this affects all tools and that they accept the native installation dialog. The shared stable PowerShell reporter is older than the packaged reporter. Updating an existing reporter reproduces Windows error 32: `write_new_file_atomically` holds its replacement file open during `ReplaceFileW`. Close it after flushing and syncing, as the configuration transaction already does.

The grouped sidecar route returns registration `error` with diagnostics in an HTTP 200 body. Electron currently classifies that body as a successful operation, allowing a later absent-status refresh to discard the error. Classify a nominally successful result with registration `error` as failed in the shared result projection, retaining diagnostics for all mutation entry points.

No harness contract changes: launch, resume, identity capture, voice, capacity, and hook payloads retain their existing declarations. The fix is in shared reporter publication and error presentation.
