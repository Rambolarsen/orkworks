---
type: Validation Evidence
title: Offline original research source chain
description: Independent synthetic research plan, approval, reservation, allocation, attachment and assignment source bytes with original-identity joins and explicit history limits.
tags: [orkworks, research, sources, qualification, validation, history]
status: draft
workflow_status: source-components-only
---

# Offline original research source chain

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Inspection base: `faf43417f5a89098aceaac63517b87d5af036a48`.

This continues the owner's [offline source-component acceptance](../superpowers/specs/2026-11-12-research-source-codecs-design.md#scoped-owner-acceptance-and-first-component-packet)
after the [configuration/startup packet](research-startup-components.md).
It adds ResearchSourcePlanV1, approval request/source/receipt, original reservation,
allocation, attachment and assignment source. It validates synthetic source
shapes, bytes and joins. It does not establish actual approval, admission,
successful one-attempt reservation, native allocation, unique attachment or legal
transaction history. Gate 2 remains Investigate; no runtime/native execution.

## Retained evidence and independent construction

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-chain.py) constructs new research records and domain hashes independently of JavaScript. Its only input is the unchanged synthetic research startup packet.
- [JSON oracle](research-chain-components.json) embeds that exact startup packet and adds unqualified bindings, source records, whole-body fingerprints and four metadata width examples. This outer wrapper is test packaging, not a protocol or storage catalog.
- [Checker](research-chain-components.mjs) composes the prior startup checker, common native-descriptor syntax and explicit original-source joins. No old child or report validator is widened.
- [Tests](research-chain-components.test.mjs) regenerate the oracle, preserve its input, reseal/relink semantic mutations and check source, counter, domain, identity and width boundaries.

The unqualified context supplies only a future prepared-input binding and four
symbolic CommitRefs. The prepared input has a bounded preparationRevision and
digest spelling; no body/codec or successful preparation is invented. It is not
the old DirectDecisionV1(false) input. Commit counters use the common positive
uint64 spelling; neither epoch nor counter values select a storage format. The
synthetic refs have no proof manifests, original producer events, causal-order
qualification, current grants or read authority.

The record checker accepts already decoded objects. Stored canonical source bytes
must first pass the capture/scope packet's strict canonical byte decoder; ordinary
JSON.parse alone loses duplicate-member evidence. This packet introduces no new
wire decoder or permissive storage parser. Retained startup artifacts continue
through their existing canonical decoder inside the composed startup checker.

## Original immutable joins

The plan binds the complete original run/configuration/scope, one task/group and
one batch at maxParallelChildren 1. Its initial prompt equals the configuration
assignment description. ParentPlanBinding retains its exact ten fields and joins
the supplied, explicitly unqualified prepared-input binding. The group has the
approved group ID/base, a bounded allowed branch spelling and NativePath-shaped
absolute path. These strings do not prove an existing clean Git base or native
branch/path authority.

The approval request and source use the distinct accepted research action and
request/approval digest domains. The source/receipt preserve the same original
plan, approval ID, review context and request bytes. A matching digest is not
proof Main originated the ID or the user saw/approved the bytes. The receipt
CommitRef is only linked to the supplied unqualified original-ref example.

Reservation retains the original ApprovalRef, exact configuration/scope and the
positive pair expectedTaskVersion + 1 = assignmentTaskVersion, bounded by
2^31−1. Attachment and assignment identity use that frozen version, reservation,
child/parent sessions and original configuration. No live task counter, report
CAS or automatic adoption of a later approval is introduced. Version-1 R2 report
requests remain their original synthetic components and are not imported here.

Allocation joins the original reservation/group/repository and exact planned
root. Its linked-worktree descriptors differ from the source root/private path
and object descriptors. This detects source-descriptor reuse only; canonicality,
native object existence/freshness and branch/base agreement still need their real
owner's evidence. It performs no filesystem allocation or native identity lookup.
Attachment joins that original allocation and frozen child identity. Assignment
source projects the original configuration's sorted criteria and declared rubric
order and retains the exact plan/scope/approval/reservation/allocation/attachment.

All selected plan/reservation/allocation/attachment domains are research-specific.
Whole-body request/approval/assignment and research-source-evidence fingerprints
are independently retained and checked. The full v3 configuration domain and
existing report/request/head/receipt protocols remain unchanged.

## Body measurements and containing bounds

The independent oracle measures:

| Quantity | Canonical UTF-8 bytes |
| --- | ---: |
| Plan body | 1,651 |
| Declared approved-source body aggregate | 18,193 |
| Eight new plan/request/approval/receipt/reservation/allocation/attachment/assignment bodies, summed | 10,305 |
| Approval metadata key/value wrapper | 1,825 |
| Reservation metadata key/value wrapper | 1,609 |
| Allocation metadata key/value wrapper | 2,142 |
| Attachment metadata key/value wrapper | 1,650 |

The approved-source aggregate includes the full plan/configuration/scope/capture
bodies plus every raw capability-artifact occurrence in the retained evidence
list. Equal bytes under separate reference IDs count separately. References
already inside a body's canonical bytes are not another retained target copy.
The checker enforces the accepted 2 MiB body aggregate, 64 KiB plan body and
4 KiB approval receipt ceilings. This is the declared source package, not a
producer's transaction, transport envelope or storage quota. Additional retained
occurrences in a future producer must also count there; this packet creates no
deduplication allowance and does not alter the old direct-execution packet.

The four metadata examples use the accepted research family/key/value shapes.
The measured width is canonical `{key,value}`, retaining all supplied links;
the family name and array order are test packaging. Complete wrappers must fit
16 KiB. The allocation wrapper is tested at exactly 16,384 bytes and one byte
over with otherwise valid source records. No storage row-domain, object envelope,
committed projection hash or direct/reverse index is selected by those examples.
Outer lastCommit and approval receipt.commit exclusions remain for the eventual
reviewed catalog/proof/accounting implementation, not a hidden proof in this packet.

Other positive/negative boundaries cover the highest original expected/frozen
pair, prepared-input revisions 64/65, the full positive uint64 CommitRef maximum,
4 KiB UTF-8 scope text plus one byte and 256-byte branch names plus one byte.
Semantic mutations cover mixed plans/configurations/scopes/approvals, changed
review context, parallel/batch changes, invalid original versions, live-counter
fields, source-object adoption, unplanned roots, wrong frozen identities,
criteria/rubric changes, metadata family/key/link drift and direct-execution
source-domain substitution. The 2 MiB aggregate, 64 KiB plan and 4 KiB receipt
checks do not claim separate near-limit witnesses for every coupled source maximum.

## Reproduction and qualification limits

The first two assertions failed against explicit empty generator/checker stubs.
After the independent generator was added, missing checker measurements and
semantic denials still failed before implementation. Regeneration is byte-exact,
including a copied generator/input under paths with spaces and percent signs.
The generator writes explicit UTF-8/LF and Git pins the new oracle to LF.

Pinned Node 22 verification passed all 1,505 documentation tests, including 67
new tests, metadata/index and rendered-page/dead-link validation. OKF regression,
doc drift and diff whitespace checks passed. Temporary-copy mutations removing
the original expected/frozen pair, frozen attachment identity or metadata width
guard each failed an intended test. The mutation exercise first exposed a test
gap where changing both attachment and assignment identities let the latter
check mask removal of the former; a dedicated attachment-only case now closes
that gap. All 537 prior non-Markdown validation artifacts are byte-for-byte
preserved and three prior concept bodies remain append-only. The owned checker
was not mutated during review. Existing VitePress circular/large-chunk warnings
concern unchanged bundler configuration; worktree inventory warnings are reviewed
separately in the PR and do not qualify this source packet.

Use pinned Node 22 from the repository root:

```bash
rtk proxy python3 docs/validation/generate-research-chain.py --output /tmp/research-chain-components.json
rtk proxy cmp docs/validation/research-chain-components.json /tmp/research-chain-components.json
rtk proxy node --test docs/validation/research-chain-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

There is no source-owner factory, prepared-input codec, active research control,
first-report CAS, current generation/capability, revocation/receipt lookup or
ordinary lineage bridge. Source records cannot establish successful event order
or renew authority. No fresh subject/ledger, workspace/run resource inventory,
shared-pool accounting, complete catalog/version/initializer, consumer/collection/
expectation/index membership, atomic first-report effects, recovery or custody
proof is supplied. No old format-3/4 history, source or receipt is adopted.

The next written contract work is research prepared input and current controls,
including original-versus-live task versions and exact source-owner command/effect
boundaries, before complete transaction/history fixtures. The
[first-report investigation](first-report-producer-contract.md) remains the map
for those dependencies. #1689 and #1089 remain open; existing logical acceptance
does not authorize runtime/native execution, adapters, migrations, model probes
or executable runtime plans.
