---
type: Validation Evidence
title: Offline research capture and scope components
description: Independent synthetic retained-file capture, question, excerpt and task-scope bytes with source-only denial checks and explicit producer/history/consumer limits.
tags: [orkworks, research, sources, reports, qualification, validation]
status: draft
workflow_status: source-components-only
---

# Offline research capture and scope components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Base: `6cb9c362d0e13d2a221c3b6076605bc8c24cd9fd`.

The owner accepted the four choices in the
[research source design](../superpowers/specs/2026-11-12-research-source-codecs-design.md#scoped-owner-acceptance-and-first-component-packet)
for offline source-component fixtures only. This first bounded packet covers
ResearchCaptureV1, ResearchQuestionV1, ResearchExcerptV1 and ResearchScopeV1.
It supplies synthetic component evidence, not a real first-report producer,
legal command history or complete report consumer. Gate 2 remains Investigate.

## Retained packet and independent checks

The documentation-only artifacts are:

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-capture-scope.py): standard-library construction of synthetic bytes and domain hashes; imports no JavaScript checker, old source packet, repository capture or runtime module.
- [Retained JSON oracle](research-capture-scope-components.json): two captures, two questions and two byte-sliced file excerpts for one task; SHA-1 base strings are invented, not an actual Git revision claim.
- [JavaScript checker](research-capture-scope-components.mjs): exact closed fields, scalar/counter bounds, raw content hashes, source-domain hashes, ordered unique sets, complete capture references, task/repository/worktree/revision joins and UTF-8 slice/locator checks.
- [Tests](research-capture-scope-components.test.mjs): independent regeneration, output paths with spaces/percent signs, semantic denials resealed at their containing source boundaries, and exact byte/count limits.

The JSON packet wrapper `{context,captures,scope}` is test packaging, not a new
protocol or persistence record. context is an explicitly unqualified supplied
TaskKey/repository/source/base/object-format assumption. Every capture and scope
must join it exactly; that does not establish its authenticity. The checker takes
already decoded records. Its separate canonical byte decoder must be used to
reject duplicate members and malformed/noncanonical wire spellings before the
record checks; ordinary JSON.parse alone does not preserve duplicate evidence.

Reproduce from the repository root using the pinned Node 22 environment:

```bash
rtk proxy python3 docs/validation/generate-research-capture-scope.py --output /tmp/research-capture-scope-components.json
rtk proxy cmp docs/validation/research-capture-scope-components.json /tmp/research-capture-scope-components.json
rtk proxy node --test docs/validation/research-capture-scope-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

The retained packet measures:

| Quantity | UTF-8 bytes |
| --- | ---: |
| Raw capture occurrences | 29 |
| Complete canonical capture bodies, summed | 1,200 |
| Complete canonical scope body | 1,864 |

The first excerpt selects bytes 1–7 of `Aé🙂Z` plus a trailing LF in the capture:
the six selected bytes are exactly `é🙂`. The second selects bytes 0–8 of
`Retained data only.` plus LF. Locations use the exclusive end. Captured text
remains data; this packet performs no model delivery or instruction precedence
test. NFC normalization, line-ending repair, source fetch and truncation are
absent.

## Boundaries exercised

Checks distinguish raw UTF-8 from complete canonical JSON bytes. Tests cover
64 KiB raw capture versus one byte over; 16 complete 64 KiB occurrences at the
1 MiB aggregate versus a seventeenth capture; 2 KiB excerpt versus one byte over;
1 KiB complete file reference versus one byte over; and 32 KiB scope versus one
byte over. The 512 KiB capture-body check remains independent even though this
closed grammar's raw/leaf bounds constrain the reachable maximum below it.
Equal contents under separate source IDs remain separately measured occurrences.

Other denials cover unknown fields, invalid IDs/digests/revisions, cross-task or
foreign bindings, absolute/traversal/backslash paths, unused/unresolved captures,
empty/duplicate/unsorted sets, missing question text, whole-question and per-field
bounds, 32/33 questions, 64/65 excerpts, wrong reference variants and fields,
negative/fractional/unsafe slice offsets, overrun, split UTF-8 codepoints, wrong
exclusive endpoints, noncanonical decimal locators, stale references and wrong
source digest domains. Byte decoding rejects duplicate keys, BOMs, malformed
UTF-8, unpaired surrogates, alternate escapes, unsafe/fractional numbers and
noncanonical JSON spelling. SHA-1 and SHA-256 registered-format string joins are
both exercised without inspecting a native repository.

The initial 47 assertions failed against an empty checker for missing positive
measurements and missing denials. A subsequent byte-decoder/generator stub pass
exposed the expected missing canonical rejection and missing retained occurrences.
The final checks and fresh review evidence belong to the PR; these local
red/green observations do not establish any runtime behavior.

The Node 22 docs build passed all 1,376 tests (79 in this packet), metadata/index
checks and rendered-page/dead-link validation. Separate temporary-copy mutations
removing the excerpt path join, exact byte locator and containing scope cap each
made the expected denial tests fail. The owned checker was not mutated during
review. All 529 prior non-Markdown validation artifacts compare byte-for-byte to
the inspection base, and prior contract prose is preserved with appended routing.
The existing VitePress circular/large-chunk warnings concern unchanged bundler
configuration and do not alter this source-component evidence.

## Producer, history and consumer limits

No actual immutable Git capture, source custody, model delivery, UI approval,
reservation/allocation/attachment or ordinary lineage bridge is demonstrated.
There is no prepared-input codec, complete configuration/startup packet, plan/
approval/launch-source packet, active research control, current report CAS,
revocation/receipt lookup, report finding selection or report publication here.
The accepted source choices for later components remain available within their
scoped acceptance; this subset does not qualify them by association.

Body measurements above are **not** charged storage rows or admission evidence.
There are no object envelopes, metadata records, ledgers, CommitRefs, proof rows,
command read/write sets, direct/reverse indexes or transaction measurements in
this packet. The accepted shared child-source pool and run aggregate still need
complete key/projection/occurrence accounting in a separately closed catalog.
No storage format is selected. Existing format-3/4, R2, direct-execution child,
terminal and ledger packets are neither rewritten nor adopted.

The next source-component work is the full research configuration/startup and
original immutable source chain. Complete first-history fixtures must still wait
for research prepared input/current controls, actual source-owner modules,
collection/expectation/consumer membership, one complete catalog/version/
initializer, original generation history and atomic first-report effects under
the [first-report investigation](first-report-producer-contract.md). #1689 stays
open through those criteria. Existing accepted report/request/head/receipt and
configuration protocols remain unchanged. No runtime/native execution, adapter,
migration, model probe or executable runtime plan is authorized by this packet.
