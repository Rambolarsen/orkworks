---
type: Design
title: Project the exact owned Codex hook fragment
description: Accepted pure source projector and shared command template for complete Codex-owned hook bytes without retaining shared configuration.
tags: [orkworks, review, codex, source-contracts]
status: stable
---

# Project the exact owned Codex hook fragment

**Owner accepted this exact projection/retention boundary and coupled bounded
plan on 2026-11-22 with “go ahead” after reviewing draft #1959.**
Track [#1956](https://github.com/Rambolarsen/orkworks/issues/1956), a prerequisite
under #68/#66/#1899 and #738/#741. Audit base:
`b91b9571c15285df5d80f9a2f18dc93f7946fe71`.
Reuse the accepted [leaf contract](2026-11-20-review-configuration-leaves-design.md),
[isolation contract](2026-11-20-review-codex-isolation-design.md) and
[Codex source audit](../../validation/ordinary-review-codex-capture-audit.md).
The [bounded source-component plan](../plans/2026-11-22-review-codex-fragment-components.md)
adds no actual capture caller, installed hook read or prepared Start.

## Source findings and decision

`harness/integrations/codex.rs::probe` is a status/compatibility classifier.
It checks the six declared event arrays, marker commands and reporter existence,
but accepts additional group/hook keys and does not inspect other event keys.
`integration.rs::parse_json_object` uses ordinary serde JSON parsing.
`definition.rs::parse_strict_json` already rejects duplicate keys, trailing input
and excess bytes; it can be reused without exporting its detailed diagnostics.
The Codex command builder obtains platform/home through existing wrappers and
computes a six-event fingerprint. Antigravity now reuses `extract_marker` for its
separate `.agents/hooks.json`; do not change that classifier in this slice.

Choose a separate **pure projector for the standard POSIX Codex bundle**. The
future first prepared route remains direct macOS arm64; a platform-neutral pure
fixture is not platform/native qualification. The projector recognizes only the
exact source-generated portable reporter commands for
`$HOME/.orkworks/hook-scripts/report-harness-event.sh`, including the existing
Codex marker, event flag and bundle fingerprint. Custom paths, old markers,
partial bundles or customized owned fields are unavailable. Manual status,
install, repair, removal, reporter-existence checks and Windows behavior remain
unchanged. Do not silently rewrite a rejected carrier to make it usable.

Alternatives: modifying the existing Installed probe would change manual
compatibility; retaining/hashing the entire shared document copies unrelated
configuration; accepting caller-supplied expected commands cannot establish
source ownership. Neither option supplies the required closed owned bytes.

## Shared actual template, without effects

Separate the existing source renderer from ambient input resolution:

- Preserve existing wrapper selection of platform, reporter path and home.
  A pure portable-command formatting helper consumes the already resolved
  portable path; it must not resolve a home, read a file or change quoting.
- Extract Codex event flag/fingerprint command generation into one pure bundle
  builder. The actual manual `merge`/fingerprint path and projector use that
  builder, rather than maintaining two command/hash algorithms.
- Fingerprint order remains the current source `CODEX_EVENTS` order:
  SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse,
  Stop. Each base command has one trailing LF in the fingerprint input; the
  fingerprint flags are appended only after that digest, exactly as today.
- The projector constructs its standard POSIX base invocation internally using
  the same formatter and fixed portable suffix. It accepts no platform, marker,
  path, expected-template or capability argument from a caller. Its command
  generation never calls `ReporterPlatform::current`, `dirs::home_dir`, status,
  a file transaction, reconcile or native executable.

This is reuse of source rendering, not proof of the actual home, source asset
root or installed reporter. The later collector must independently join those
owners and the fixed standard path. Extraction must preserve literal current
POSIX and Windows commands/fingerprints, including source quirks; do not repair
unrelated quoting/platform behavior in this task.

## Input and ambiguity rules

Private crate boundary:

`project_standard_posix_codex_fragment(bytes: &[u8])
 -> Result<Option<OwnedCodexFragment>, CodexFragmentError>`

The input is borrowed carrier bytes supplied by a future qualified observer;
this function does not read `.codex/hooks.json`. Cap input at **256 KiB inclusive**.
Use the existing strict parser, retaining its recursion limit and trailing-input
rejection; do not introduce an unbounded-depth mode. A zero-length/invalid UTF-8/
BOM/truncated/non-object/duplicate-key carrier fails. Duplicate keys anywhere,
including unrelated sections, fail after JSON escape decoding. Empty object or
missing hooks can represent logical absence, but a present hooks value must be
an object and its event values arrays. These rules deliberately narrow capture
availability; they do not make the manual parser stricter.

For this policy, the exact case-sensitive owned namespace is
`orkworks:harness-integration:`. Inspect decoded string values **and object keys**
throughout the bounded carrier for that substring. This is an ambiguity guard,
not a shell parser or secret classifier. A mention in a comment-like user field
can therefore deny capture; nothing deletes that field or claims it executes.

- No namespace occurrence anywhere yields `Ok(None)` after the carrier/container
  checks. This means no recognizable owned fragment, not a missing physical file,
  valid native configuration, safe launch or evidence of uninstall history.
- A namespace occurrence is permitted only inside the `command` value of one
  exact expected group in its correct one of the six known event arrays.
- The complete owned group is exactly `{hooks:[{type:command, command:<expected>}]}`:
  one hook, no matcher, timeout, async, alternate type, extra key or sibling hook.
  Compare the whole parsed group against the internally generated expected shape.
- Every event must have exactly one such owned group. Any partial bundle,
  duplicate, foreign/legacy marker, wrong command/fingerprint/event, additional
  owned occurrence in an unknown event or elsewhere, or namespace-bearing key
  fails the entire projection. Never return a successfully projected subset.
- Other groups and root fields without the namespace remain ambient carrier
  input. Do not interpret them as a promise of native semantic validity or
  confinement. Their content must not appear in output or diagnostics.

A bounded recursive namespace check on a group determines whether it is an owned
candidate; whole-group equality and six unique event matches decide acceptance.
Also check root fields outside hooks, event names and unknown event groups.
This prevents the existing six-event-only traversal from silently omitting a
recognizable owned fragment. Unmarked unrelated commands remain ambient; source
history and complete real read-set qualification remain independent gates.

## Exact output and retention

`OwnedCodexFragment` owns only the reconstructed canonical native JSON bytes;
fields/construction are private and there is no deserializer. Expose a borrowed
`as_bytes()` accessor for the future leaf builder. Its existence proves only a
successful source projection, never configuration custody or native authority.
No user values or caller-supplied expected command are copied into the result.

Output is compact UTF-8 JSON, with one top-level `hooks` key. Its six event keys
follow `CODEX_EVENTS` order above, each value an array of its one expected group.
Group key order is `hooks`; hook keys are `type`, then `command`. Use serde JSON
string encoding; all standard POSIX generated values here are ASCII. No newline,
BOM, unrelated field, renderer response or serialization of the carrier is added.
Cap the **complete output at 16 KiB inclusive**, within the existing 256 KiB raw
resource cap. Changed source template bytes produce changed projection bytes
and therefore a different parent leaf commitment. Reject overflow; do not truncate.

Errors are a closed payload-free enum: InputTooLarge, MalformedCarrier,
UnsupportedFragment, OutputTooLarge. Map parser errors to the fixed category;
do not expose duplicate key names, commands, JSON paths, source snippets or an
underlying error chain. Do not log, persist, export or return carrier bytes.
Transient parsing can hold bounded input/parsed values; they are dropped on
return. The 256/16 KiB limits bound logical input/output work, not measured total
resident memory, native I/O or a complete physical capture envelope.

The eventual file inventory must include the projected hook fragment plus both
source/installed reporter and metadata-helper pairs. Active selection already has
its typed leaf field; trust observations, home/path custody, effective asset roots
and eligibility need their own qualified bindings. This task registers no actual
complete expected inventory or capture owner, and reads no such files. The first
role cannot treat `Ok(None)` as satisfying a required owned-fragment resource.

## Compatibility, verification and rollback

The normal install/status/uninstall paths keep their existing classification and
file side effects. Only their internal command generation is factored through
shared pure code; no stricter projector is called by those paths. Windows command
formatting and Antigravity’s shared portable formatter/marker behavior need
regression coverage even though Windows projection is out of scope.

Use independent complete byte/fingerprint literals for the standard POSIX bundle,
with additional byte-preservation fixtures for actual manual POSIX/Windows
builders. Keep fixture paths synthetic; never inspect installed or user hooks.
Projection tests must cover unrelated sentinel values omitted from output/errors,
duplicate escaped keys, nested duplicate keys, malformed containers, partial and
duplicate owned events, unknown event/key occurrences, forged templates, extra
executable fields, wrong fingerprint, all-absent namespace, inclusive input limit
and one extra byte. Exercise namespace occurrences outside all six known arrays.
The fixed standard bundle is below 16 KiB; do not invent an input-selectable
maximum-width template solely to make an unreachable output-overflow case.

TDD, full repository verification and current `/code-review low` apply after
approval. Rollback reverts source helpers/projector without rewriting metadata,
user configuration, installed assets or active sessions. There is no migration,
new dependency, renderer/API/schema activation or new permission. Actual carrier
observation, absence races, resource/executable custody, active/trust publication,
shared checked epoch, independent native/profile eligibility, physical admission
and prepared Start remain gated. Supported production roles remain empty.

## Bounded source execution evidence — 2026-11-22

The shared renderer and dormant projector are implemented. Independent Python/
hashlib literals pin complete POSIX/Windows command bundles and fingerprints;
canonical output is compared with the independent complete native JSON literal.
Seven focused projector tests passed, then the current 103-test integration
suite passed, including the actual manual merge/fingerprint golden regression,
Antigravity and Windows formatting cases, decoded namespace/duplicate keys,
retained recursion limit and both logical-absence/owned positive 256 KiB input
boundaries. No installed/user configuration or native coding tool was read.
Full repository verification passed: Rust3,639/0fail/24ignored plus integration
suites, reporter32, desktop1,795/0fail/5skipped, native ABI/type-check/build,
docs2,566/site build and close-out checks. Final review is recorded in #1959.

The new projector has no production capture caller. Its module is explicitly
source-only; parsing a copied carrier into exact source bytes establishes no
file/executable custody, complete inventory, native eligibility, shared epoch
publication, physical admission or prepared Start behavior.
