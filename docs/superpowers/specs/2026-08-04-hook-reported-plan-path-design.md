# Hook-reported plan paths

## Decision

Use a harness file-write hook to report the path of a newly written plan or
spec through the existing `planPath` field on `POST /sessions/:id/attention`.
The sidecar remains the authority that canonicalizes, contains, and validates
the path. Terminal-text detection remains only for harnesses without this
signal.

## Options considered

1. Keep tightening terminal-text matching. Rejected: it cannot distinguish an
   authored file from output that merely quotes one, and stale associations
   remain.
2. Add a generic hook-contract framework now. Rejected: issue #271 tracks that
   broader design; this fix only needs the existing marker-specific reporter
   branches.
3. Add Claude Code `PostToolUse` for `Edit|Write`. Selected: its documented
   payload supplies the actual `tool_input.file_path` after a successful write.

## Scope

The reporter scripts will extract Claude's file path, reduce it to a
workspace-relative path, and include it as `planPath` only when it is under an
accepted plan root. The existing attention report endpoint applies it
atomically. A later hook-reported path supersedes an earlier fallback value.

Codex is intentionally unchanged here. Its documented `PostToolUse` payload
for `apply_patch` exposes the patch command rather than a canonical file path,
so treating it as authoritative would recreate the parsing problem this change
removes. It retains the conservative terminal fallback until a reliable native
file-write payload is available.

## Checks

- Claude integration installs, probes, and removes the additional hook.
- POSIX and PowerShell reporters send a valid `planPath` for an accepted
  Claude file-write payload and omit all other paths.
- The sidecar accepts the reported path and the existing fallback stays active
  when no hook path is reported.
