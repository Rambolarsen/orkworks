---
type: Validation Evidence
title: Original research preparation authorization components
description: Independent synthetic allocated-prestate intent, authorization digest and ordinary lineage lookup examples without a storage profile or runtime authority.
tags: [orkworks, research, authorization, lineage, qualification]
status: draft
workflow_status: offline-components-only
---

# Original research preparation authorization components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `04d02e40`.

The owner [accepted the three research authorization choices](../superpowers/specs/2026-11-13-research-preparation-authorization-design.md#scoped-owner-acceptance--2026-11-15)
for offline components only. This packet supplies separate allocated snapshots,
intent/auth source bytes and an unchanged ordinary schema2 member. Original
C/command/proof, source/native identity, private permit and complete membership
are supplied assumptions. Gate 2 remains **Investigate**.

## Independent artifacts and scope

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-authorization.py) reads only the unchanged prefix JSON. It independently derives allocated snapshots, intent, authorization source digest, ordinary member and byte measurements; it never invokes JavaScript.
- [JSON oracle](research-authorization-components.json) separates captured allocated controls from the old running/report examples. It has no storage header, manifest, row/change hash, selected format, SQL or actual permit.
- [JavaScript checker](research-authorization-components.mjs) independently derives the same finite decoded components, reuses the existing prefix source validator and compares joins, source digest, supplied resolution and byte measurements. It is a finite oracle checker, not a general research wire decoder, publisher, reducer or linked-store reader.
- [Tests](research-authorization-components.test.mjs) independently regenerate bytes, check semantic denials after resealing, pin digest/size literals and check independent limits and paths containing spaces/percent signs.

The input SHA-256 fingerprints documentation bytes only. Symbolic origin hashes
are explicitly hashes of unqualified labels, not storage commitments. Old
format-3/4/6 source/proof bytes are not imported, converted or rewritten. The
conditional 51-name catalog remains incomplete; no next format is selected.

## Allocated intent and source bytes

The original prefix source chain supplies configuration, retained files/scope,
approval, reservation, allocation and deterministic intended attachment/assignment
bodies. The checker validates that source chain through its unchanged validator.
These bodies are not native custody/allocation or complete original proof evidence.

Four new snapshots explicitly construct every field from original source records
and the accepted control shapes, without copying later running-view controls. Their
keys remain RunKey/PlanRef/TaskKey/TaskKey; their origins are labelled supplied
prestate refs. Run version is 7, live task version 4, task launching, launch
allocated with null attachment/spawn runtime. Frozen P remains at task version 2.
Plan is active under the original approval; run current input stays original
input-2 with no blocker, pause or clarification. These are the illustrative
no-intervening-event values, never production admission constants. The old
run-10/11, task-7 views remain unchanged and are not allocated prestates.

Intent binds the complete P, original parent generation, expected H, ordinary
epoch/G/original creating string and deterministic **new** attachment/source.
producerSource uses research-assignment-source and its whole-body research digest.
The copied controls are complete decoded prestate values; their lastCommit links
remain inside the hashed intent. The source digest uses
`orkworks.research-preparation-authorization.v1`, one LF and canonical UTF-8 body.
There is no self-digest/new C inside that body, secret or restored private permit.

Sequence "1", ordinary G "0", admission epoch and all origin refs are example
inputs, not issued identity, chronology or live inventory evidence. The checker
compares full supplied P and permit identities across the two named authorization
families; it does not discover complete historical/tombstone membership. Absence,
source ownership, currentness, physical outcome and all direct proofs remain
explicit assumptions even when the finite checks pass.

## Ordinary member and supplied original-command lookup

The member retains exactly schemaVersion, assignmentIdentity, attachmentId,
assignmentSourceDigest and preparationCommit. The lineage pairs that member with
the exact original createdAt string. assignmentSourceDigest is the original
research assignment digest, never the authorization digest or current config hash.

The supplied lookup starts from exact original P/C. Its command kind is
research-source-attach and selects research_preparation_authorizations plus
research-assignment-source. The checker compares complete original assignment,
attachment, authorization and lineage and refuses unknown commands, wrong child
owner/family, duplicate P/permit, changed C and byte drift. No caller sourceKind
field, role inference, table-presence fallback or ordinary schema bump is added.

Original C/command are **supplied synthetic facts**, not generated manifests or
recognized physical-profile evidence. The packet contains no successful
child-source-attach lookup example and does not qualify any deployed reader.
Existing format6/schema2 readers must still refuse unknown research linked
profiles; this packet cannot establish an eligible research store or reader.

## Measured occurrences and independent limits

| Finite quantity | Canonical UTF-8 bytes |
| --- | ---: |
| Complete run/plan/task/launch key-value wrappers | 2,277 / 1,348 / 989 / 2,742 |
| Full P plus auth committed projection | 10,439 |
| Full P plus complete auth wrapper | 10,639 |
| Ordinary member only | 803 |
| Supplied authorization/member joint parts only | 11,033 |

The auth charge excludes only its outer lastCommit. All four copied snapshots,
old H and source origins remain charged, in addition to existing independently
retained source/control occurrences. This is one projected occurrence under the
existing shared 8 MiB/run, 64 MiB/workspace source pools and 32 MiB run aggregate.
No ledger balance, complete resource delta, manifest, index charge or complete
transaction is supplied, so this is not a capacity/reconciliation proof. The five
fresh assignment zero rows and all initializer obligations remain unchanged.

Tests independently enforce 1 MiB run, 16 KiB plan/task, 32 KiB launch, 2 MiB auth
wrapper and the separate 1 MiB ordinary record/joint-job ceilings. A width-only
padded run fits exactly its own 1 MiB wrapper but the supplied auth/member parts
already exceed the joint-job ceiling. That padding is a measurement vector,
not a valid control body. Smaller source/worker/command/discovery/physical limits
still win; small example parts prove no coupled maximum or complete job fits.

## Policy oracle boundary and checks

The nine policy labels cover revoke-first; permit-before-volatile-close with
possible save/no spawn; durable H/G drift; ownership loss; proven rollback with
spent sequence; unknown outcome with retained ownership/no retry; committed lost
response with no spawn; and workspace switch with detached publication/original
pair retention. The checker compares these labels to the accepted contract.
It does not schedule threads, arbitrate revocation, issue a permit, enforce leases,
run SQL, prove rollback or simulate a complete lifecycle. Actual permits and
source/native/coverage/currentness/proof/outcome facts remain unqualified.

The initial generator and checker stubs failed the allocated-prestate and empty
packaging tests. With the oracle generated, 38 checks failed against the empty
checker before implementation. The final suite adds literal source measurements
and an independent lower-job-limit witness. Regeneration must preserve every
previous validation artifact and the historical proposal body.

From the repository root with pinned Node 22:

```bash
rtk proxy python3 docs/validation/generate-research-authorization.py --output /tmp/research-authorization-components.json
rtk proxy cmp docs/validation/research-authorization-components.json /tmp/research-authorization-components.json
rtk proxy node --test docs/validation/research-authorization-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

Complete research catalog/version/initializer and source-to-index correspondence,
positive continuation codecs, actual source/revocation/ordinary writer coverage,
metadata/native/lease/physical feasibility and atomic history/recovery remain
separate gates. No source/runtime/native/model execution, Rust/Electron code,
physical database/adapter, migration or executable product plan is supplied.
#1689/#1089 remain open; #740 remains closed research. The next written prerequisite
is closure of the remaining research continuation/catalog/proof/index ownership
before any full-state/history fixtures; this packet supplies no such closure.

## Review and verification disposition

One bounded round used two fresh read-only Luna reviewers for correctness and
coverage/evidence. Findings were addressed: both implementations now construct
allocated controls explicitly from original source records instead of cloning
later views, and the plan promises command/source-family rejection rather than
unimplemented physical-profile recognition. Independent regeneration preserved
the exact oracle bytes after that construction correction.

All 42 new component tests pass. Five guard removals in temporary copies were
caught by their intended denial tests: allocated snapshot joins, source-family
binding, captured H, authorization digest domain and the independent joint-job
ceiling. Owned files were not mutated for this audit. The docs pipeline passes
all 1,704 tests and the Node 22 VitePress build, metadata/index and rendered-page
links; all 20 OKF regression tests pass. Existing circular/large-chunk warnings
concern unchanged bundler configuration and need no edit for this packet.

The full repository verifier stopped at prerequisites because desktop dependencies
are not installed in this docs-only worktree; no full-repository success is
claimed. Documentation drift and worktree checks pass; the worktree hook's
separate session-workflow-split stale-worktree notice concerns an unrelated owner.
Task-worktree metadata reporting was unavailable (invalid session environment);
work remained in the isolated owned checkout without changing primary residue.
Hosted review/CI and original-artifact preservation are recorded separately in
the PR. None of these checks qualifies the open runtime/profile/proof gates.
