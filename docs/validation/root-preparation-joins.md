---
type: Validation Evidence
title: Root preparation source joins
description: Independent synthetic draft, approval, receipt and retained-source bytes with strict reconstruction and storage-key commitments, without admission or custody proof.
tags: [orkworks, hierarchy, preparation, approval, codecs, validation]
status: draft
---

# Root preparation source joins

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744).
Source: [accepted producer flow](../superpowers/specs/2026-10-25-root-preparation-producer-design.md#owner-acceptance-and-handoff--2026-10-25)
and [accepted root source slice](../superpowers/specs/2026-10-27-root-bootstrap-source-schemas-design.md#owner-acceptance-and-fixture-handoff--2026-10-28).
The owner authorized this bounded follow-up after PR #1128. No new schema,
quota, authority default or native mechanism is selected here.

## Retained packet

[root-preparation-join-vectors.json](root-preparation-join-vectors.json) has six
independent Python-derived canonical vectors: complete RootPreparationDraftV1,
RootPreparationApproveRequestV1, RootPreparationReceiptV1, retained source,
root_drafts row and root_preparations row. The retained source and its row have
the same logical value; their raw fingerprints and domain-separated row
commitments have different purposes. The generator
`generate-root-preparation-join-vectors.py` reads the preserved
[root-bootstrap catalog](root-bootstrap-vectors.json) by exact file fingerprint,
then uses only Python standard-library JSON and SHA-256. It neither imports the
JavaScript checker nor inspects live sources, sessions, configuration or credentials.

The existing complete bootstrap is deliberately unverified, with no reference
inputs or materialized deliverable. Its exact run, repository/worktree descriptors,
settings/model/configuration artifacts and all earlier byte catalogs remain
unchanged. Synthetic commit links retain the existing CommitRef shape; their
digests explicitly label absent manifest evidence. They are not fabricated
ChangeManifest proofs or a claim that an ineligible root was admitted.

Run from the repository root with Node 22:

```bash
python3 docs/validation/generate-root-preparation-join-vectors.py --output /tmp/root-preparation-join-vectors.json
cmp docs/validation/root-preparation-join-vectors.json /tmp/root-preparation-join-vectors.json
node --test docs/validation/root-preparation-joins.test.mjs
pnpm --dir docs docs:check
```

## What the checker establishes

[root-preparation-joins.mjs](root-preparation-joins.mjs) is documentation-only.
It consumes complete logical sources, reuses the existing bootstrap/run/approval
checkers and reconstructs the original draft from retained bootstrap, run and
approval metadata. It rehashes the existing draft and approval-request domains,
compares every repeated root identity and validates each complete receipt and
CommitRef shape. It checks the 1,081,344-byte draft/source and 4,096-byte receipt
ceilings plus the existing smaller nested source limits. It does not introduce
a new source domain, raw request log or latest-configuration alias.

The packet entry point requires exact canonical wire bytes for draft, request
and retained source, plus separately retained original reviewed-draft and
approval-request bytes in its fixture context. It decodes strict UTF-8 and
canonical stored JSON before interpretation, then compares the exact frozen
bytes independently of their stored hashes. The original byte context is a
test prerequisite, not a caller credential or proof that a real trusted UI
presented those bytes. Hash repair cannot make a substituted source match it.

The retained source reconstructs the exact approval tuple and request bytes;
its receipt binds the full #744 root AssignmentIdentity, bootstrap/run/draft
hashes, approval ID and request digest. Both source commit links must agree.
The strict storage membership key is the exact workspace for root_drafts and
exact workspace/run tuple for root_preparations. Row commitments use the existing
`orkworks.hierarchy-storage-row.v3` domain over canonical `{table,key,body}`.
Only root_drafts.lastCommit, root_preparations.lastCommit and
root_preparations.receipt.commit are omitted at their named paths. Approval,
context, generation, receipt identity and complete original source bytes remain
committed. Omitted links are still validated; exclusion never means ignore drift.

[root-preparation-joins.test.mjs](root-preparation-joins.test.mjs) compares all
six independent byte/hash vectors and exercises source substitutions, changed
approval tuples, closed-field/version/tag failures, invalid stored JSON/UTF-8,
root-versus-child identity, Counter encodings and wrong row keys/links. It checks
that repaired hashes and changed commit links cannot erase the original source
or approval binding, while legal changes to excluded link fields leave row
commitments unchanged. The initial stub failed fingerprint and missing-rejection
assertions before the checker implementation.

## Qualification boundaries and next work

Success is labelled `synthetic-preparation-joins-only`. A structurally coherent
receipt is no record of admission, current UI authority, trusted presentation,
run state, fence reservation, capability, native commit or parent startup.
Matching exact bytes do not make a revoked review context current. The packet
performs no receipt lookup, idempotent retry, cancellation, recovery, fence-file
operation, SQL write, model/native probe or runtime integration. It supplies no
claim-first exception; these preparation operations remain excluded from #744's
claimed-mutation scope.

CommitRef syntax/equality and row-domain bytes are checked without claiming
that any referenced manifest, proof retention, predecessor chain, store header
or complete changed-row set exists. The pending draft row and retained source
row are separate component examples, never two simultaneous rows in an admitted
state: successful admission must consume the exact draft atomically. No
before/after store witness, source-custody mechanism, aggregate/history quota,
summary-capacity branch or complete #742 membership/attachment effect is supplied.
Neither a source-row fingerprint nor an old synthetic transaction can fill
those gaps. Existing empty-output store witnesses retain their historical seed
and are not upgraded or rebound by this packet.

Next, qualify complete initial admitted run/parent/summary membership and
changed-row effects, including both capacity-fitting and never-initialized
summary branches and all shared ledgers/dependencies. Source-history/aggregate
limits, retained source custody, native excluded-action cutoff and parent
attachment proof remain separately owned gates. Keep #741/#742/#744/#746 open.
All role profiles remain UNVERIFIED/no-go; #740 stays deferred until November,
and separately approved runtime/storage/native plans remain required.
