---
type: Validation
title: Offline ordinary-review launch inputs
description: Canonical proposal and prepared-input components with explicit logical-only source and capability assumptions.
tags: [orkworks, review, qualification, validation]
status: stable
---

# Offline ordinary-review launch inputs

The owner accepted the [typed input design](../superpowers/specs/2026-11-19-review-launch-input-design.md)
and [coupled plan](../superpowers/plans/2026-11-19-review-launch-input-components.md)
on 2026-11-19 with “keep going”. This evidence concerns finite logical values,
not a production proposal producer or Start. Track
[#66](https://github.com/Rambolarsen/orkworks/issues/66),
[#68](https://github.com/Rambolarsen/orkworks/issues/68) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899).

## Reproduction

```sh
node --test docs/validation/ordinary-review-launch-input-components.test.mjs
```

Use the pinned Node 22. The [component](ordinary-review-launch-input-components.mjs),
[tests](ordinary-review-launch-input-components.test.mjs) and
[literal vectors](ordinary-review-launch-input-vectors.json) are isolated under
docs/validation. Golden complete JSON strings and domain-separated SHA-256 were
computed independently using Python standard-library JSON/hashlib, before the
implementation existed. The vector retains both bytes and measured lengths.
No preceding #1919 codec/test/vector bytes are changed.

## Current logical evidence

The focused suite currently passes **163 tests**. Initial execution failed with
no implementation. Further RED/GREEN tests establish that an exhausted proposal
can be decoded as history but cannot qualify fresh inspection, and that original
null-draft and explicitly edited draft paths join different captured facts.

The finite cases cover closed nested keys and every prepared scalar/array leaf;
valid changes alter the golden digest, and unsupported changes deny encoding.
Malformed UTF-8, duplicate/escaped/reordered keys, BOM/whitespace, Unicode
surrogates/controls, overflow/noncanonical counters and sparse argv are denied.
Non-BMP, combining marks, CRLF/tab and U+2028/U+2029 remain exact; no normalization
or native-path conversion is performed. Native paths receive lexical absolute
shape checks only, with no filesystem/canonicality witness.

Valid exact complete **24 KiB proposal**, **12 KiB configuration**, **48 KiB
prepared** and **24 KiB canonical request** constructions pass and one extra
byte denies. Independent JSON byte recounts select bounded fields within their
accepted raw UTF-8 domains. Individual 16 KiB instruction, 2 KiB request text,
256-byte model, 128-byte IDs, 64 argv entries/1 KiB each, 4 KiB paths,
2 KiB artifact references/anchors and 1 KiB branch cases are included. These are
specific boundary constructions, not a complete maximum valid production state.
The request codec defines a canonical offline component representation; this
does not establish a production HTTP parser or widen the accepted raw request cap.

The join oracle requires explicit current proposed/no-target status and literal
positive simulated source/configuration predicates, nonexpired 30-second entry,
exact frozen values and selected original/edited draft. Replacing any synthetic
owner reference denies even with identical labels. Source runtime/lifecycle/cwd,
Git common directory/head, actual design/implementation slot, pin/epoch/capture,
workspace incarnation and resolved configuration changes deny the old entry.
The two source/admission orders are logical comparisons, not concurrency proof.

All **24/24 temporary-copy mutations** were detected by existing assertions:
canonical equality, captured caller value, exact owner identity, five current
snapshot joins, three draft joins, literal source/configuration predicates,
expiry, final instruction/configuration epoch digest inclusion, four complete
byte caps, proposal exhaustion, original/edited mode, negative zero and dense
argv. A first mutation run exposed a malformed tool ID masking a join test; the
case now uses a valid mismatched ID. Temporary copies were deleted; repository
code was never patched by that audit. Fresh whole-task review found no required
issues; its optional field-check readability suggestion was applied and the
focused suite passed again.

## Qualification limits

The entry holds a frozen prepared value, the original null or explicit three-field
draft input, prepared/expiry times and synthetic JavaScript owner tokens. Current
facts contain the five snapshots/owner tokens, status/target, simulated eligibility
predicates and time. These are test-only records, not public API schemas or actual
Rust Store/runtime/configuration references. A matching digest or literal true
predicate cannot qualify real authority. The supported production profile set
remains empty.

No complete effective catalog/integration leaf codec, actual request/proposal
producer, revision/history publication, executable/native/model eligibility,
immutable external environment or prompt delivery is qualified. Complete preview
entry/view/response codecs and their separate 64 KiB budget witnesses remain
missing. Physical registry initialization, retained-attempt protection, recovery,
retention/export/downgrade compatibility, source lock ordering and prepared Start/
recommendation lifecycle publication remain separate gates.

No application code imports the module; no RecommendationType/API, metadata,
provider or native coding-tool session is changed. No runtime/storage/role
activation or broader user-request producer approval follows from these tests.

## Repository regression evidence

After implementation, `bash scripts/verify-repo.sh` passed under Node22: 3,603
Rust tests plus integrations (22 ignored), 32 reporter tests, native ABI check,
desktop type-check/build and 1,786 tests (5 skipped), docs 2,310 and site build.
Whitespace and doc/worktree checks passed. Existing compiler/build warnings are
outside this diff; README drift needs no shipped-behavior update for these
offline components. Foreign worktrees were preserved. Broad regression success
does not expand the logical qualification limits above.
