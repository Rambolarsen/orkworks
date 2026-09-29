# Codex native session IDs use a temporary hook report mailbox when sandboxed networking blocks loopback

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-09-28
- Issue: [#673](https://github.com/Rambolarsen/orkworks/issues/673)

## Context

Codex's trusted OrkWorks hooks parse the native thread ID, but a hook's HTTP
request to the local sidecar can fail while Codex's command network sandbox
blocks loopback. Enabling general localhost access would also let ordinary
commands reach local services, so it is broader than this identity-reporting
need. Codex hooks still need a narrow way to deliver a native ID to their
owning live OrkWorks session.

The existing per-session report token is delivered through the child process
environment and checked in memory. It must not be persisted or logged. As
[ADR 0068](./0068-codex-subagents-share-owning-session-identity.md) records,
the token authenticates the OrkWorks session but cannot prove which child
process sent a report.

## Decision

For each live Codex runtime, the sidecar creates an operating-system temporary
directory with private directory permissions and gives its path to that
session's hook reporter. The Codex reporter atomically publishes bounded,
uniquely named JSON files containing only harness-session report fields. It
does not write the report token into the mailbox.

The owning runtime consumes only reports in its own mailbox while it is live,
draining it once more before terminal cleanup so a short-lived session cannot
lose a report already queued when the process exits. It uses the report token
held in memory and the same harness-session authorization,
fingerprint/provenance, and metadata merge path as the HTTP handler, and
re-verifies the token immediately before applying each report rather than
only at the start of a scan pass. The existing recorded-reset and
authenticated root `SessionStart(source=clear)` guard remains the only way to
replace an accepted Codex ID. Invalid and stale reports are discarded, and
abandoned reporter staging files are aged out so they cannot occupy the
per-pass scan budget indefinitely. Mailbox files are removed as they are
consumed.

The sidecar pins the mailbox directory while the runtime is active. Unix
scans, opens, and removals are relative to that directory handle and refuse
symlinks; Windows holds a directory handle that allows report creation but
prevents the mailbox path from being renamed or replaced. The mailbox
*directory* itself is never removed by path on Unix at runtime cleanup: the
path stays writable by the Codex child for the mailbox's whole lifetime, so
any check-then-remove sequence keyed on the path, rather than the held
handle, is a TOCTOU a child could use to make cleanup delete an unrelated
directory later occupying that pathname. The directory is leaked instead,
left for the OS temp-directory lifecycle rather than removed unsafely.
Windows is not exposed to this: its directory handle holds an exclusive share
mode for the runtime's whole lifetime, so nothing can occupy or replace that
pathname while cleanup runs, and its directory is removed normally.

The mailbox carries Codex native session identity only. It does not relay
attention or other hook reports, add general localhost access, or change
ordinary command networking. Failure to create a mailbox leaves the existing
HTTP reporter path in place and emits only redacted diagnostics.

The mailbox path is an additional per-session reporting capability. Since
child processes inherit the reporting environment, this design does not
distinguish the configured Codex hook from another child process holding that
capability. It does not claim operating-system process authentication. To
keep that capability scoped to its owning session, the mailbox environment
variable is stripped from every child's forwarded environment before a
runtime sets its own value, so a non-Codex session, or a Codex session whose
mailbox failed to create, never inherits another session's mailbox path from
the parent sidecar process.

## Consequences

Codex can report the native ID without requiring ordinary Codex commands to
connect to the sidecar over loopback. A temporary filesystem channel adds
bounded parsing and cleanup work to the live Codex runtime. The report token
remains memory-only; the mailbox contains no token or prompt text. Other
harness reporting transports are unchanged.

This channel preserves the trust limits already documented in ADR 0068: the
session capability and hook fingerprint establish accepted report provenance
for the OrkWorks session, but not the identity of the operating-system
process that produced it.
