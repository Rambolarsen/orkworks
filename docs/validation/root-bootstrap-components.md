---
type: Validation Evidence
title: Root bootstrap source-byte evidence
description: Independent synthetic canonical root bootstrap bytes, exact instruction rendering and adversarial structural checks without native or runtime qualification.
tags: [orkworks, hierarchy, bootstrap, codecs, validation]
status: draft
---

# Root bootstrap source-byte evidence

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741) and
[#742](https://github.com/Rambolarsen/orkworks/issues/742).
Source: [accepted root-only source slice](../superpowers/specs/2026-10-27-root-bootstrap-source-schemas-design.md#owner-acceptance-and-fixture-handoff--2026-10-28).

## Artifacts and reproduction

[root-bootstrap-vectors.json](root-bootstrap-vectors.json) contains eight
independent Python-derived vectors: the six-action permission profile, startup
settings, pinned and tool-managed model policies, effective configuration
projection, unverified capability evidence, a complete unverified bootstrap and
its exact rendered instructions. Each vector retains canonical JSON, inclusive
UTF-8 byte count and raw SHA-256 fingerprint. The full bootstrap includes all
source-owned repository/worktree descriptors rather than digest placeholders.
Its run is independently sealed with the matching bootstrap, goal and bindings.

`generate-root-bootstrap-vectors.py` uses
Python standard-library JSON and SHA-256. It imports no JavaScript checker and
reads no native filesystem/session/configuration data. Every path, executable,
object identity, adapter/model label and evidence artifact is synthetic. The
older component catalogs, generators and checkers remain unchanged.

Run from the repository root with Node 22 or newer:

```bash
python3 docs/validation/generate-root-bootstrap-vectors.py --output /tmp/root-bootstrap-vectors.json
cmp docs/validation/root-bootstrap-vectors.json /tmp/root-bootstrap-vectors.json
node --test docs/validation/root-bootstrap-components.test.mjs
pnpm --dir docs docs:check
```

[root-bootstrap-components.mjs](root-bootstrap-components.mjs) is a
documentation-only consumer, never a production dependency. Its component checks
cover closed fields, finite lengths/counts, sorted sets, exact owner joins,
manifest-to-skill contributor projections, profile/tool pairing, retained
settings/model/configuration hashes, ten evidence surfaces, calendar timestamps
and exact framed rendering. Boundary cases preserve the distinct owning
4,096-byte repository/worktree and 2,048-byte provenance/executable path limits. It recomputes repository, worktree, evidence,
bootstrap and run digest domains without changing any existing domain. The
stored-byte decoder rejects duplicate keys, noncanonical spelling, invalid UTF-8
and unpaired surrogates. Callers checking parsed objects cannot recover duplicate
wire keys; byte fixtures always pass through the strict decoder first.

[root-bootstrap-components.test.mjs](root-bootstrap-components.test.mjs) compares
against independent golden bytes and mutates authority declarations, source
identities, hashes, artifact retention, automatic/supplied ordering and rendering.
The initial minimal stub failed the fingerprint and missing-rejection assertions;
the implemented checker is then tested with the existing component suites in
docs:check. Header-like text, CRLF and decomposed Unicode are retained literally;
they cannot terminate a length-delimited block or silently normalize its hash.

## Qualification boundaries and next work

A successful check returns `unverified-source-bytes-only`, never an approvable
draft, admitted run, verified profile or authority token. This initial packet
intentionally accepts only capability snapshots whose decision is unverified;
other support decisions need independent real evidence and are outside this
consumer. Retaining a fixture or primary-reference label does not prove the
claim it contains. The checker cannot judge whether an unverified check's retained citation actually
explains its evidence gap; that source relevance requires written review even
when a reference is labelled primary. Secret detection, adapter setting recognition, model-policy
semantics, platform/translation identity and actual content-delivery precedence
require qualification independent of these bytes.

The first complete root has no reference inputs and no materialized deliverable.
Its six coordination actions are static ceilings. The checker cannot exercise
bearers, exact-plan execution grants, current stage/run fences, cross-subject
report authority, native permission enforcement or revocation races. The source
proposal's runtime denial oracles remain obligations for a separately approved
runtime plan. Root result tools and server-artifact declarations can be checked
structurally; a usable nonempty output still requires an approved producer/wrapper.
The empty contract grants neither success nor a way to omit a required artifact.

Rule applicability, natural-language output conflicts, source-locator resolution,
required graph-skill applicability and evaluator suitability remain review
obligations. Source-byte validity is not evidence that those semantic checks
passed. Synthetic native path/object descriptors establish no registration,
canonical native path, opened-handle identity, source custody or actual executable
inventory. All six role profiles remain UNVERIFIED/no-go; no old synthetic seed
is upgraded by the new packet. #740 remains deferred until November.

Next, join the accepted draft/approval/receipt and retained source envelopes,
then qualify complete initial-state/attachment/storage-row membership and atomic
transition effects, aggregate/custody limits and native commit/revocation order.
This packet checks the full bootstrap-to-run source binding; it does not claim
a complete source-valid preparation transaction, durable storage, migration,
native execution or production launch eligibility. Keep #741/#742/#744/#746 open.
