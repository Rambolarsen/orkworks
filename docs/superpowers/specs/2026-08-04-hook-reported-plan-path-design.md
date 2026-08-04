# Hook-reported plan paths

## Decision

Use a harness file-write hook to report a newly written plan or spec through a
dedicated path-only sidecar endpoint. The sidecar canonicalizes, contains, and
validates the reported path before persisting its workspace-relative form. The
report does not alter attention. Terminal-text detection remains only for
harnesses without this signal.

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

Claude's installed `PostToolUse` hook matches `Write|Edit` and forwards its
raw `tool_input.file_path`. The sidecar accepts only an existing regular
Markdown file beneath `docs/superpowers/plans/`,
`docs/superpowers/specs/`, or `specs/`, rejecting control characters,
workspace escapes, symlinks, and all other paths. It stores the normalized
relative path and emits an event. A hook report always replaces a previous
fallback value; fallback still writes only when no association exists, so both
arrival orders preserve the hook value.

Codex is intentionally unchanged here. Its documented `PostToolUse` payload
for `apply_patch` exposes the patch command rather than a canonical file path,
so treating it as authoritative would recreate the parsing problem this change
removes. It retains the conservative terminal fallback until a reliable native
file-write payload is available.

## Checks

- Claude integration installs, probes, and removes the additional hook.
- POSIX and PowerShell reporters forward the raw Claude file path without
  changing attention.
- The sidecar accepts a valid hook path, rejects absolute/escaping/control/
  non-Markdown/symlink paths after normalization, and keeps attention intact.
- Hook-first and fallback-first arrival orders both retain the hook path.
- Add ADR 0036 plus its index entry, update ADR 0035's cross-reference, the
  plan-review spec, and the harness-contract register.
