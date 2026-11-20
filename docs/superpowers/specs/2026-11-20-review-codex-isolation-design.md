---
type: Design
title: Freeze ordinary-review Codex isolation before capture
description: Accepted pre-capture isolation contract and bounded source probe/render separation, with native and prepared-creator gates retained.
tags: [orkworks, review, codex, launch]
status: stable
---

# Freeze ordinary-review Codex isolation before capture

**Owner accepted this isolation ordering, first-profile boundary and coupled bounded
plan on 2026-11-20 with “keep going” after reviewing draft #1946.**
Track [#1941](https://github.com/Rambolarsen/orkworks/issues/1941), a prerequisite
of #68/#66/#1899 and #738/#741. Source base:
`bc1c9d9475a07f87578bf3f7fba84e2fa4a022be`.
The [source audit](../../validation/ordinary-review-codex-capture-audit.md)
identified the late command mutation; this contract resolves its intended
ordering without claiming that the actual capture/creator owners exist.

Reuse the accepted [prepared-input contract](2026-11-19-review-launch-input-design.md),
[leaf contract](2026-11-20-review-configuration-leaves-design.md),
[admission source contract](2026-11-19-review-source-qualification-design.md)
and [ADR 0072](../../adr/0072-codex-session-process-isolation.md).
The coupled [source-component plan](../plans/2026-11-20-review-codex-isolation-components.md)
implements only the real Rust probe/render separation and its manual-launch
regressions. It does not activate a prepared runtime path, add a capability row,
perform a native qualification run or implement the future evidence owner below.

## Decision and alternatives

Resolve isolation **before** producing the integration leaf and final prepared
command. Preview/status/Start must consume the same literal command; they never
run a help probe, prepend a flag or choose a different runtime after inspection.
The first accepted prepared Codex route is direct macOS arm64 with independently
qualified exact executable/version/model support for `--no-daemon`. No such role
is currently qualified. Other platforms, wrappers, legacy no-option versions,
resume and the owned native app-server route remain unavailable for this profile.
These restrictions do not narrow the existing manual launcher.

| Alternative | Disposition |
| --- | --- |
| Pre-capture evidence plus frozen literal direct command | Accepted. Fits the existing complete-byte commitments; keeps slow effects outside read-only capture. |
| Symbolic argv or isolation decision deferred to Start | Reject for this slice. It changes the accepted literal preview/configuration promise and requires a separate reviewed contract. |
| Keep a shared daemon or silently skip isolation | Reject. ADR 0072 records cross-session environment/capability reuse; stock presence is not an isolation guarantee. |
| Probe from preview/status or renew on retry | Reject. Inspection is read-only; a process probe is an effect and cannot refresh the accepted preview lifetime. |
| Copy Settings/version-cache text into a new boolean | Reject. The version cache has different truncation/stream policy and no exact configuration/executable/model custody proof. |

The adapter's no-op `augment_launch` remains only one stage: catalog stock
resolution produces the base command; qualified isolation preparation produces
the final integration command. `catalogLeaf.resolvedCommand` and
`integrationLeaf.finalCommand` need not have identical argv. The latter equals
configuration argv exactly, and both cwd values equal the prepared workspace.
The existing #1933 logical join already has this relation; no leaf fields,
digest domains, budget or runtime-policy literal changes are proposed.

## Separate classification, probing and rendering

Refactor `crates/orkworksd/src/runtime/codex_launch.rs` into three private crate
seams, keeping the existing `isolate_session` wrapper and its call site:

- `IsolationAdvertisement::{Advertised, NotAdvertised}` classifies successful
  bounded help output. This is parser output, never capability or launch authority.
- `probe_isolation_advertisement(&CommandSpec)` retains actual command resolution,
  filtered probe environment, `--help`, cwd, null stdin, process ownership,
  errors and logging, returning that enum only after a successful probe.
- `render_isolated_arguments(&[String], IsolationAdvertisement) -> Vec<String>`
  is deterministic and has no filesystem, process, environment, clock or owner
  access. It preserves every argument exactly, inserting one `--no-daemon` at
  index zero only when Advertised and no equal flag occurs before the first `--`.

A small pure `isolation_advertisement(stdout: &[u8], stderr: &[u8])` helper owns
classification. Preserve the current lossy UTF-8 conversion, literal LF between
streams, ANSI stripping and exact first whitespace token check. Similar option
names or a substring are not advertisements. Probe policy remains three seconds,
64 KiB **per output stream**, success required, overflow rejected, using the
shared probe-process ownership/cleanup contract. No version-cache substitute,
new retry, output retention or new native process is introduced by extraction.

`isolate_session(&mut CommandSpec)` calls the probe then replaces only args with
the pure rendered result. On an error it leaves command bytes unchanged and
returns the existing classified error. Successful older help remains
NotAdvertised and preserves arguments for manual launches. Existing manual new
launch/exact resume, cancellation checks and capability filtering keep their
current ordering. No runtime caller may use this enum alone to skip the probe.

## Future source-owned evidence before pure capture

This section constrains the later capture producer, **outside the
bounded implementation plan**. Missing actual owner/custody/native evidence keeps
that producer unavailable; a synthetic token must never stand in for it.

An authorized availability/qualification operation, separate from opening or
refreshing a preview, may run discovery before capture. This design does not add
that operation, UI, schedule or route, and does not authorize running a native
probe in this task. The existing manual startup probe stays manual-startup-only.

A future configuration owner must hold an opaque, nonserialized isolation-evidence
reference with all of these exact bindings:

- The same live configuration owner and checked positive epoch used by the
  catalog/integration/prepared values, including failed publication and observed
  A→B→A. An ID, equality of paths, registry revision or cache generation is insufficient.
- Canonical absolute executable and exact live executable/custody reference;
  source-owned build, direct macOS arm64 route, independently verified tool
  version, explicit selected model and capability-evidence identity/revision.
- Exact workspace owner/generation/root and base stock program/argv/cwd. The
  separate integration/root/home/asset/context owners remain required by #1933;
  the isolation reference does not qualify their read set.
- Completed successful Advertised result from the exact bound executable and
  probe context, with the existing filtered environment policy. Retain the
  result and bindings, not raw help, ambient environment, credentials or tokens.
  An observed context change invalidates the reference before another capture.
- Exact final program/argv/cwd obtained from that base and pure rendering, within
  all existing command/configuration/prepared byte limits. Insertion can turn
  64 base argv entries into 65, or overflow the 12 KiB configuration: deny instead
  of truncating, dropping another argument or raising a limit.

The independently qualified native/profile evidence must cover the actual
isolation behavior and selected model. Help advertisement is necessary for this
first profile but not sufficient: an arbitrary executable can print that option.
Missing, NotAdvertised, failed, timeout, oversized, cancelled or cleanup-unconfirmed
results cannot publish usable evidence. A later successful manual probe cannot
retroactively qualify a previously unavailable preview.

Executable metadata in `runtime/codex_native/launch.rs::ExecutableIdentity` and
`VersionProbeEpoch` show existing revalidation seams, but neither is promoted here
to complete custody or shared checked-epoch coverage. Metadata equality and an
unchanged filename cannot prove replacement/ABA absence or native role support.
A producer must qualify these owners before implementation/activation. External
unobserved changes/ABA retain the accepted residual boundary; every observed
mismatch permanently invalidates the affected preview.

Pure capture reads only already-owned immutable values/references. It performs no
probe, repair, install, disk write or hidden availability refresh. Slow bounded
source observation happens outside short application guards, then the final
owner/epoch checks publish the exact prepared input. A missing/stale evidence
reference returns unavailable/changed, requiring a new explicit preparation.
Status/Start cannot renew the 30-second preview or resurrect invalidated evidence.
No second raw-output/leaf/evidence cache, public schema or persistent record is
introduced. Existing preview retention limits and exact live references apply.

## Future admission and startup consumption

The eventual private prepared creator must consume a source-owned route proof
bound to the admitted attempt and frozen command. A request field, renderer
boolean, parser enum or ordinary `CreateSessionCommand` cannot select it.

1. Revalidate the existing recommendation/source/artifact/workspace/configuration
   and native eligibility owners, including the isolation reference, without
   probing. Deny a different model, base argv, command, epoch, runtime route,
   executable identity or observed environment/context.
2. Serialize the final controlled-source check and first creation mutation through
   the qualified owner/admission fences from #1919/#1933. Reserve at most one
   attempt and bind the exact command and route proof to it. The physical
   admission mechanism and lock ordering remain separate qualification work;
   this sequence is a required boundary, not a claim about the current creator.
3. Keep fresh session/report capability and mailbox allocation after admission,
   under the eventual exact reporting policy. Those values never enter leaves.
   Allocation/fallback choices still need their own qualified boundary.
4. At runtime consume the exact bound command through the prepared direct route.
   Do not call `codex_launch::isolate_session` or `codex_native::eligible` again,
   mutate argv, switch to a newly eligible native route, resolve a different
   executable or silently fall back to the ordinary creator. Revalidate the live
   route/custody proof and cancellation before effects. Failure leaves the
   attempt's truthful existing outcome; an ambiguous post-create result does not
   authorize respawn.

The bounded plan does not add this route proof, bypass flag, alternate runtime
branch, physical registry or Start. Ordinary callers continue through the current
startup probe/native eligibility paths. Before the prepared runtime consumer is
implemented, synchronize this accepted ADR extension and all source/capability/
producer gates; approval of extraction alone cannot enable it.

## Verification, compatibility and recovery

The source-component pass condition is behavior-preserving extraction tested
through the actual existing manual wrapper, plus pure classification/render
cases. Include split stdout/stderr, ANSI, similar option names, flags before and
after `--`, empty arguments, exact ordering, no mutation on error, current
environment exclusion, failed/oversized/hung probes and cancellation regressions.
Synthetic executable fixtures exercise probe ownership, not Codex/native roles.

Future producer tests must additionally deny foreign/stale owners, same-path
replacement, failed publication, observed A→B→A, changed model/platform/context,
NotAdvertised, missing native evidence and over-budget rendered commands. A
prepared-start fixture must prove zero discovery children after preview and
exact PTY argv; the existing two-session isolation regression must still pass.
None of these future producer tests is claimed by parser tests or this document.

No storage migration, lockfile/dependency change or installed asset update is
needed for extraction. Rollback reverts that source refactor to the current
single wrapper without changing metadata, installed hooks, active sessions or
manual behavior. The future prepared consumer needs a separately qualified
rollback/unknown-attempt policy before activation. Production-role eligibility
remains empty.

## Source-component execution evidence — 2026-11-20

The bounded extraction is implemented in `runtime/codex_launch.rs`. The new
pure tests first failed for the absent enum/classifier/renderer seams; the
focused suite then passed 11/11 (two pure tests with 18 independent literal
cases plus nine actual-wrapper synthetic tests). Existing failure, oversized,
timeout and legacy-help tests now compare the complete unchanged command where
applicable. Five existing runtime Codex regressions passed, including separate
new/resume reporting environments and deletion during successful/failed probing.
The full Rust unit suite passed 3,622/0fail/24ignored and its integration suites
passed. Full repository verification passed: reporter 32, desktop 1,793/0fail/5skipped,
ABI/type-check/build, docs 2,539/site build, and close-out checks. Final review
evidence is recorded in #1946.

No source outside this module consumes the new classification/render seams;
`session_runtime.rs` and native eligibility remain unchanged. This is actual
source refactoring with synthetic process evidence, not native Codex capability,
complete executable custody, capture-owner qualification or prepared Start.
