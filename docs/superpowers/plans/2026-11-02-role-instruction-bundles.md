---
type: Implementation Plan
title: Supplied role instruction validation and rendering
description: A bounded pure Rust instruction composition unit following the accepted role configuration core.
tags: [plans, roles, instructions, validation]
status: stable
workflow_status: implemented
---

# Supplied role instruction validation and rendering

Outcome: one dormant, data-only Rust module validates the structure of supplied
instruction inputs and returns deterministic rendered bytes plus their SHA-256.
Pass condition: exact golden bytes and rejection fixtures pass, with no production
caller or new side effect. Tracking: [#1303](https://github.com/Rambolarsen/orkworks/issues/1303).
The owner approved inline implementation of reviewed source
`8b668a447fbb5ee97777223f86be2ac73ebc82f1` with “yes, go ahead”.

## Accepted basis and bounded scope

Read the accepted [#741 core brief](../specs/2026-11-02-role-configuration-component-review.md)
and the canonical [composition](../specs/2026-10-04-agent-role-configuration-design.md#repository-role-and-assignment-composition),
[resource closure](../specs/2026-10-04-agent-role-configuration-design.md#referenced-skill-instruction-resources),
[byte/digest](../specs/2026-10-04-agent-role-configuration-design.md#canonicalization)
and [bounds](../specs/2026-10-04-agent-role-configuration-design.md#bounds-compatibility-and-persistence)
contracts. This is the first instruction-composition portion of the proposed
configuration validator/renderer, not the whole configuration implementation.

Implementation choices proposed here: keep a private Rust module under
`taskmaster`, use owned UTF-8 strings and existing `sha2`/`hex`, and accept an
internal projection of already supplied snapshots. Introduce no dependency,
wire format, persisted record or extension to the accepted logical schemas.
No filesystem discovery, source fetch, storage, model/native calls, endpoint,
settings edit, session/worktree action, approval or adapter mapping is included.
Natural-language applicability/conflict interpretation remains reviewed planning
work. A successful return certifies only the checks enumerated in this plan.

Full assignment v1/v2/v3 and bootstrap v1/v2 records are outside this API. It
neither parses, constructs, serializes nor hashes them; there is no generic
configuration JSON entry point or fallback through a legacy v1 shape. Future
version-specific consumers must validate their full descriptors and preserve the
accepted child-v3/root-v2 output routing before projecting these instruction
inputs. The result cannot establish configuration validity, plan approval, source
freshness, delivery, permissions or native support. #740 stays closed; supported
production roles remain empty. No live probe is necessary for this unit.

## Module and interfaces

At inspected base `55e4863e`, `taskmaster/coordinator.rs` has its own dormant plan
records and canonical JSON helper; the proposed role configuration types have no
runtime implementation. Leave that serializer, its callers and its wire bytes
unchanged. This module needs only exact text hashing, so extracting a shared
canonicalization framework would add no value.

| File | Responsibility |
| --- | --- |
| `crates/orkworksd/src/taskmaster/role_instructions.rs` | Internal input types, bounded structural validation, deterministic composition and exact text digest; keep helper functions private. |
| `crates/orkworksd/src/taskmaster/role_instructions_tests.rs` | In-crate unit tests and literal/checked-in golden comparisons. |
| `crates/orkworksd/tests/fixtures/role-instructions/` | Synthetic input notes and independently computed expected UTF-8 output/digest; no real repository instructions or secrets. |
| `crates/orkworksd/src/taskmaster/mod.rs` | Declare the dormant module and test module only; no production call site. |
| `.gitattributes` | Preserve the golden fixture’s intentional mixed LF/CRLF bytes without text conversion. |

Use one public-to-crate entry point:
`render_instruction_bundle(input: &InstructionBundleInput) -> Result<RenderedInstructionBundle, InstructionBundleError>`.
Validation is private and completes before allocating the rendered output. The
result exposes immutable `text: String` and `sha256: String` through accessors;
its constructor/state is private so callers cannot fabricate a validated result.
The module stays dormant following the coordinator precedent; any dead-code
allowance is scoped to this new module, not the crate.

The internal input has these explicit parts, with no serde derives or wire claims:

- `task_id`; `coordination_text`; `assignment_text` (already composed assignment,
  rubric, criteria, output and escalation text supplied by a future caller).
- One role projection: `id`, the six-role enum, exact `instructions`,
  `instructions_digest`, and `source_snapshot_digest` supplied by the caller.
- Rule projections: `id`, `source_identity`, `source_path`, `precedence`,
  `authority`, exact `content`, `content_digest`.
- Skill projections: `id`, exact `content`, `content_digest`, mandatory/optional
  requirement, `requirement_source_ids`, `resource_ids`.
- Resource projections: `id`, `skill_id`, exact `content`, `content_digest`.
- Requirement projections: `source_id`, `source_digest`, `task_id`,
  `required_skill_ids`, bounded `source_locations`, and `applicability_reason`.
- `reported_unresolved_source_ids` and `reported_conflicts` (pairs of source IDs
  plus bounded explanation). Any entry blocks rendering; empty means no blocker
  was reported, not that this library verified arbitrary prose or owner review.

These are deliberately projections, not aliases for the complete snapshot wire
records. Version, provenance, role-template `contentDigest`, selection reason,
source-reference resolution and full assignment/permission validation remain the
future caller's responsibility. Role `source_snapshot_digest` is an opaque
caller-supplied identity: check its syntax and manifest equality, but do not claim
to recompute the full role-template descriptor from a partial projection. Rehash
`instructions_digest` and every rule/skill/resource content digest from the exact
supplied UTF-8 text. Never treat the opaque source identity as proof of its origin.

Errors identify the failed invariant and relevant IDs/indices, without copying
instruction text into diagnostic strings. Bound reported unresolved/conflict
inputs to the existing blocker-record ceiling of 16 KiB, with at most 16 source
references; they report a blocker rather than permitting partial rendering. Bound
the returned diagnostic to the
existing 2 KiB reason limit; one deterministic first error is sufficient.

## Structural checks and render contract

- IDs use the accepted nonempty ASCII alphabet and 128-byte bound; digests are
  exactly 64 lowercase hexadecimal characters. Roles are exactly the six accepted
  values. Reject duplicates within namespaces; rule and role source IDs must also
  be distinct so each requirement source is unambiguous.
- Require one manifest per supplied rule and role; reject extra/missing sources,
  wrong source digest/task ID, duplicate required IDs/source locations and missing
  applicability reason for an empty requirement set. For rules the source digest
  is their content digest; for the role it is `source_snapshot_digest` above.
- The union of required skill IDs must resolve to selected mandatory skills.
  Each mandatory skill names exactly all contributing sources; optional skills
  have none and cannot satisfy a mandatory requirement. Reject dangling/duplicate
  requirement-source references. No skill can add authority.
- Rules arrive in contiguous precedence order `0..n-1`; reject gaps or reordered
  arrays instead of sorting them. Authority is binding. Validate source paths as
  accepted relative `/` paths; reject absolute/backslash/empty/dot/parent/NUL
  components. Reject duplicate `(source_identity, source_path)` locators.
- Every resource belongs to one selected skill, occurs once in its resource-ID
  set, and resolves once in the resource array. Reject dangling, duplicate,
  unowned and cross-skill references. This checks the supplied closure; it does
  not discover transitive references from prose or fetch missing content.
- Apply 16 skills, 32 combined rules/resources, 32 resource IDs per skill,
  33 manifests, 16 required IDs and 32 source locations/source IDs per record,
  512 bytes per source location and 2 KiB per reason/source path. Byte counts use
  UTF-8, with checked addition. The complete rendered text, including framing,
  must be at most 262,144 bytes. Reject before output allocation; never truncate.
  Full 1 MiB configuration and 2 MiB plan checks belong to their future codecs.

Frame each section exactly as the following ASCII header, raw UTF-8 content, and
footer. `<kind>` is one of the literal lowercase names below, `<id>` is validated
ASCII, `<digest>` is the exact content SHA-256 and `<n>` is decimal UTF-8 content
length with no leading zero. The newlines shown are one LF each:

```text
=== orkworks:<kind> id=<id> sha256=<digest> bytes=<n> ===
<exact content bytes>
=== /orkworks:<kind> ===
```

The code block describes the line grammar: do not add blank lines between header,
content and footer. The footer always contributes one LF before it, even when the
content already ends in LF; preserve the original content unchanged. The framing
is explanatory prompt text, not an escape mechanism, instruction-priority
boundary or parser for later reconstructing trusted snapshots. Content may itself
contain header-looking text; retain it verbatim and make no injection guarantee.

Concatenate frames with no extra separator in this order:

1. `coordination` / ID `coordination`, then `rule` frames in supplied precedence.
2. `role` / the selected template ID.
3. `assignment` / `task_id`.
4. Mandatory `skill` frames sorted by ID bytes, then optional skills sorted by ID
   bytes. Each skill is immediately followed by its `resource` frames in resource
   ID byte order, independent of input set order.

Hash the complete resulting UTF-8 text with plain SHA-256, matching
`renderedInstructionsDigest` text semantics; there is no configuration-domain
prefix. The framing is a proposed implementation choice, to be pinned by fixtures
when this plan is approved. Future changes need reviewed version/compatibility
handling before any configuration consumer relies on these bytes.

## Delivery and verification

One implementer owns this coupled module; inline execution is the recommended
method. Use the local TDD skill once code is approved. Do not delegate separate
writers for validation and rendering: they share the same private representation.

### Task 1: Structural validator and input types.

 Add failing unit cases, then implement
   bounded checks. Cover valid empty rule/skill/resource sets with an explicit
   empty role manifest, all six role values, invalid IDs/lowercase digests,
   mismatched content, wrong task/source digest, duplicate IDs/locators/manifests,
   precedence gaps, missing/optional mandatory skills, incomplete requirement
   sources, duplicate source locations, all closure ownership failures and reported
   unresolved/conflicting input. Do not add a prose conflict parser.
### Task 2: Deterministic composition.

 Add an independently specified golden bundle
   containing a rule, role, assignment, mandatory and optional skills, resources,
   CRLF, non-ASCII/combining text, trailing/no trailing LF and header-looking
   content. Record literal expected bytes and a digest computed independently
   with Python `hashlib` or `shasum`, not by the implementation under test. Then
   implement framing/order and byte preflight. Assert set permutations produce
   identical output without mutating the input, swapping rules with updated
   contiguous precedence changes output (an unchanged-index reorder is rejected),
   any content mutation
   changes its digest, and the accepted `Delegate.` plus LF instruction fixture
   still hashes to the canonical spec value.
### Task 3: Boundaries and close-out.

 Test each collection limit at/over its boundary,
   exact 262,144-byte rendered output and one byte over (including multibyte
   content/framing), and integer-safe length arithmetic. Inspect the diff/call
   graph for no production callers, serde/full-configuration codecs, external I/O,
   new dependencies or changes to coordinator bytes. Record the precise supported
   instruction-only surface in this plan/issue, not a shipping role claim.

For each code task, observe a meaningful failing assertion before the minimal
implementation, then run the focused suite:
`rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml role_instructions`.
Expected: all intended cases execute and pass, not zero filtered tests. Golden
expectations must not be regenerated from actual output to make failures pass.
At close-out run `rtk proxy bash scripts/verify-repo.sh` and the sidecar guide's
build/test/fmt checks with their required Node/desktop dependency preparation;
reuse equivalent successful commands from verify-repo rather than repeating them.
Complete current-code-diff `/code-review low`, PR CI, doc-check and worktree hook.
Docs-only preparation of this plan uses docs build/checks, not the Rust suite.

## Investigated risks and review disposition

The main uncertainty was whether a small renderer would accidentally become a
full configuration codec. The accepted record/version sections include repository
bindings, typed review input and structured output/reporting; this plan keeps
those records out of the API rather than supplying partial serialized defaults.
Existing coordinator canonical JSON was inspected; it is unnecessary for a text
renderer and remains untouched. The prose applicability boundary and partial
native evidence are already explicit in the accepted core and remain unchanged.
Rollback before integration is removal of this dormant module/test declaration;
there is no persistent state to restore or migration to reverse.

| Dimension | Rating and evidence |
| --- | --- |
| Dependencies | 1 — One module owner; existing hash libraries; no external producer required for synthetic inputs. |
| Blast radius | 1 — No production caller; existing coordinator/session behavior unchanged. |
| State changes | 1 — Pure in-memory computation only. |
| Reversibility | 1 — Remove dormant module and tests; no stored records. |
| Uncertainty | 2 — Byte framing is a new proposed local choice, pinned independently before integration. |

Total: 6/25, a planning judgment, not an approval threshold.
Self-review disposition: **Ready for owner review**. Every in-scope invariant has
an implementation/check above. No full-configuration or native qualification is
claimed. The owner subsequently approved this concrete plan for inline implementation;
that approval retains the stated boundary. No further general research campaign is required.


## Implementation result

The owner-approved bounded unit is implemented as a dormant module. Eighteen
focused tests pass: structural rejection groups, independent golden text/hash,
set-order invariance, meaningful rule-order/content sensitivity, inclusive
collection/metadata/blocker bounds, exact 262,144-byte rendered admission and
checked overflow without large allocation. The initial validator and renderer
tests were observed failing against their respective scaffolds before the
corresponding implementation. Boundary tests exercise the completed invariants.
The golden fixture retains its exact bytes even with `core.autocrlf=true`.

A fresh `/code-review medium` (size escalation for more than 500 changed code/test
lines) covered code head `3297045d8c27c22fd80326d23dab662d6dac34ae`; it found no
Critical or Important issues and no declined-to-judge items. One minor remains:
portable path checks also reject a POSIX relative filename such as `a:notes.md`.
No production caller, full-configuration codec/digest, permission/native adapter,
storage or runtime authority is added. Full repository and PR CI results are
recorded with [#1303](https://github.com/Rambolarsen/orkworks/issues/1303)'s
implementation PR; the supported production-role set remains empty.
