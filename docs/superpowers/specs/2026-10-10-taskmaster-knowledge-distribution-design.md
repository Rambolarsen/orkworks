---
type: Design
title: Privacy-qualified Taskmaster knowledge distribution
description: Reviewed Brain export, signed offline and feed knowledge, and verified Taskmaster admission for issue 529.
tags: [orkworks, taskmaster, knowledge, privacy, distribution]
workflow_status: accepted
---

# Privacy-qualified Taskmaster knowledge distribution

- Date: 2026-10-10
- Status: accepted; revised written design approved by the owner in chat on 2026-10-10
- Tracking: [#529](https://github.com/Rambolarsen/orkworks/issues/529)
- Runtime prerequisite: [#782](https://github.com/Rambolarsen/orkworks/issues/782), completed with admission deliberately closed
- Dependent assessment: [#769](https://github.com/Rambolarsen/orkworks/issues/769), related to [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Authority: [knowledge specification](../../../specs/taskmaster-knowledge.md), especially Knowledge distribution, Analysis, and Validation
- Architecture baseline: [ADR 0054](../../adr/0054-taskmaster-honors-managed-cli-policy.md) and [Taskmaster architecture](../../agents/architecture.md)

## Outcome and pass condition

Restore Brain-backed background analysis and **Analyze now** using only
reviewed, distilled, cryptographically verified reference knowledge. A fresh
offline packaged application must use a verified generated starter. An installed
application must activate a newer eligible signed publication without a binary
release. Invalid or unavailable updates must preserve eligible working knowledge.

Success requires a packaged-app demonstration plus tests proving that legacy
knowledge, forged policy fields, invalid signatures, and mismatched attestations
cannot reach either native or custom providers. Observation recommendations,
provider trust, context limits, managed CLI policy, and explicit Fix with AI
remain governed by their existing contracts.

This design covers the publisher in the owner's private Brain repository and the
consumer in OrkWorks. It does not implement **Assess workflow**, upload workspace
data, ingest raw research, distribute the private repository, introduce provider
fallback, or grant knowledge any execution authority.

## Investigated baseline

The investigation used OrkWorks main at `159b08d0` and the current Brain repository
through authenticated read-only GitHub requests. No Brain contents were selected
or approved for public export by this investigation.

- `apps/desktop/electron/knowledgeUpdates.ts` already verifies Ed25519 feed
  envelopes, bounded HTTPS downloads and manifest checksums, maintains active
  and previous caches, and checks on the existing six-hour schedule. Its starter
  is unsigned JSON; caches retain signed bundles without matching manifest proof.
- `synchronizeKnowledge` sends decoded bundle data through the existing
  Electron-authenticated `POST /settings/taskmaster/knowledge` route.
- `TaskmasterRuntime::open`, `reload_durable`, and `activate_knowledge` accept
  structurally valid plain bundle JSON. They do not establish signed privacy
  eligibility. The current unconditional `brain_knowledge_availability` refusal
  protects manual admission, scheduling, worker entry, and settings availability.
- Existing runtime guards bind settings, provider identity, workspace, and
  generation through reservations, actual dispatch, and output acceptance.
  Relevant-page selection uses a separate prompt snapshot; it must not weaken
  the full bundle identity used for dispatch.
- Brain's existing Pages workflow deploys a freshly assembled visualization
  artifact. A separate deployment would overwrite that site; publication must
  join the existing site assembly. The existing recursive Markdown exporter
  has a different boundary and must not become the Taskmaster exporter.
- The accepted knowledge spec already forbids individual exceptions for private
  projects, assessments, experiment records, and raw research. #529's older
  context sentence about permitted exceptions is historical, not current scope.
- Repository-secret listing returned no Brain repository secrets. That does
  not prove the absence of environment or organization secrets, or establish
  access to the private key matching the application's current public key.

## Export and content review

Use a dedicated Taskmaster export policy in Brain's repository support
configuration, outside the general knowledge-bundle scan. The policy lists exact
source paths, their reviewed full-file SHA-256 hashes, stable public page IDs,
and approved export metadata. Wildcards and recursive inclusion are forbidden.
An unchanged reviewed page may publish automatically; any source-byte change
requires renewed review and an updated approved hash. New and unclassified pages
are excluded by default.

Stable public IDs apply across publications, not just within a bundle. Use the canonical repository-relative source path in the fixed Brain repository as source identity. Record exact source-path/public-ID pairs in private receipts and reconstruct cumulative ownership from independently verified archive history before exporting/signing a candidate. Reject changing an existing source's ID or reusing its ID for another source, including after omission or deprecation; unchanged/reintroduced pairs remain valid. Source moves have no implicit migration, and `index.md` remains reserved. Only the verified initially empty archive root permits empty ownership. The private mapping never enters Pages or the application bundle.

An owner-reviewed export change includes the proposed public artifact diff,
not just a path inventory. Review prose, headings, link labels and destinations,
code blocks, frontmatter, provenance, and relationship labels for private names,
project details, excerpts, URLs, identifiers, credentials, or sensitive history.
A hash binds the review to exact bytes; automated scanning supplements review
and cannot prove that arbitrary prose is free of private information.

The exporter must:

- Reject excluded page classes even when mistakenly allowlisted. A reusable
  distilled page must live under an eligible class and pass content review;
  a project assessment or experiment never gains a per-page exception.
- Read only declared regular files from the fixed source checkout. Reject
  absolute paths, traversal, symlinked files or ancestors, and aliases escaping
  the source root. Reject duplicate source entries and public IDs.
- Serialize an explicit metadata schema rather than arbitrary frontmatter.
  Keep the existing page identity, title, type, status/maturity, content hash,
  and related IDs. Add bounded optional parent identity, applicability labels,
  and reviewed public provenance as needed to preserve the source hierarchy.
  Every exported string is part of the reviewed artifact. Parse source status as well as type and require the plan's explicit non-promoting mapping: preserve concept/principle lifecycle and practice maturity exactly; required missing status fails. Other eligible types preserve a present literal status; only absent status permits the reviewed `active` lifecycle default. A reviewed file hash does not authorize policy metadata that promotes its source.
- Resolve internal Markdown links and relationships only to included public
  IDs. Reject unresolved/excluded targets rather than recursively importing
  them or merely hiding their URLs. Public external citations require explicit
  reviewed destinations. Reject embedded assets, raw HTML, and asset/data URLs
  in the initial exporter; they have no implicit copy or fetch path.
- Generate the application index from included pages and approved hierarchy.
  Never copy Brain's root index, history, assessments, or source-directory tree.
- Validate the finished serialized artifact, including metadata and index,
  against the excluded-content fixtures and all client bounds.

Preserve the existing bundle format where its additive fields remain compatible.
Unknown fields never establish eligibility. The implementation plan must specify
identical parsing and bounds in both consumers, including duplicate-key rejection
for signed policy and identity fields. Do not use a permissive YAML/Markdown
parser to execute tags, includes, templates, or other source instructions.

Record source commit, allowlist revision, reviewed page hashes, bundle version,
and output digest in a private publisher receipt. Public artifacts retain the
bundle version and digests needed to connect them to that receipt; private
repository URLs, commit messages, and unreviewed provenance do not ship. The
packaged starter is copied from the corresponding validated signed publication,
never maintained as a separate handwritten text source.

## Signed publication and archive

Retain the existing Ed25519 envelope convention: a UTF-8 JSON payload string
and a base64 signature over its exact bytes. The manifest's SHA-256 is over the
exact signed bundle-envelope bytes, before any parse/reserialization.

The signed bundle payload contains `formatVersion`, `version`, monotonic
`sequence`, `publishedAt`, integer `privacyPolicyVersion: 1`, pages, and a
bounded `capabilities` array. Each matching signed manifest entry carries
format, version, sequence, digest, relative bundle path, and the same supported
integer privacy policy version. Reject absent, non-integer, unsupported, or
mismatched policy values even if the signature is valid.

Sequence is a nonnegative safe integer, including zero. The exact wire grammar in the [implementation plan](https://github.com/Rambolarsen/orkworks/blob/main/docs/superpowers/plans/2026-10-10-taskmaster-knowledge-distribution.md#exact-shared-wire-contract) fixes timestamp spelling and Gregorian calendar validity, manifest-entry types/bounds, and structural validation of every compatibility class before selection. Both independent verifiers consume shared positive and malformed-entry fixtures for these rules. Provenance uses the plan’s bounded canonical ASCII HTTPS subset, with no userinfo or fragment, rather than permissive URL normalization.

Grant `taskmaster-assessment-v1` only when the reviewed distilled assessment
entry point and required general guidance are present. Do not export the full
private assessment process or claim that this capability implements #769.
The final #529 evidence must identify the reviewed guidance and capability;
until then the dependent assessment prerequisite remains unavailable.

Integrate publication into Brain's existing Pages assembly and deployment,
preserving its current visualization. Supply only validated public Taskmaster
artifacts under `orkworks-knowledge/`. Verify the private archive first, then stage a separate allowlist of public bundle envelopes and signed manifests only. Private receipts, source identities/review hashes, policy/source blobs, and archive metadata never enter the Pages tree; final-tree tests enforce their absence. Never stage the source checkout or
general recursive Brain export into that subtree.

Retain signed artifacts and their manifest history in an owner-authorized
generated-publication branch in the private Brain repository. This is a durable
artifact archive, not a development branch. Archive writes use the repository's
publishing workflow with minimal required permissions. Serialize publisher runs;
refuse a conflicting branch update rather than force-pushing. Each Pages build
assembles the archived compatible versions plus any newly validated publication.
A rerun reuses identical archived bytes; a version or sequence collision with
different bytes fails. Advance the public manifest only after the referenced
immutable bundle exists in the assembled site.

Treat the archive as untrusted input on every build. Pin an owner-approved initially empty archive root, require no-force/no-delete branch protection, and verify complete linear ancestry plus preservation of every prior immutable publication inventory. Removing an entire bundle/manifest/receipt set must fail even when the remaining records are internally consistent; missing history cannot bootstrap an existing archive. Before staging retained
artifacts, verify their signatures, supported privacy policy, manifest/digest
bindings, safe paths, bounded schema, and the private export receipt that records
the reviewed source identity. An arbitrary artifact-branch file, a legacy bundle
without eligible proof, or a mismatched receipt cannot enter Pages merely because
it was already archived. Copy only the verified artifact inventory; never the
whole branch checkout.

The publisher's first run starts from an explicitly empty archive. Recover a
cancelled/failed deployment from the last committed verified archive; do not
allocate a second sequence to identical already-archived output. Test first-run
bootstrap, missing/malformed archive state, tampered retained artifacts, and
cancellation after archive commit but before Pages deployment. The next successful
build must recover the same immutable bytes and preserve previous public URLs.
A rejected archive must not replace the currently deployed site.

Missing or invalid signing configuration fails the publication job before any
new Taskmaster artifacts or archive mutation. Validate the approved export
before accessing the signing key. Provisioning, replacing, or rotating that
secret is an owner-controlled operation, not implied by repository read access.
No private key or private-repository credential enters app resources, published
artifacts, logs, or test fixtures.

Use the current pinned public key only if the owner can provide its matching
publishing key. Otherwise explicitly approve a new key,
generate the signed starter with its matching key, and include that starter and
public key together in both consumer verification paths in the same client
release. Do not require shipping a key-only client before its signed starter
exists. Keep older signed publications available for older compatible clients.
Document the key fingerprint, custodianship, backup, and rotation procedure.
Normal rotation requires a client trust update; never trust a key supplied by
the downloaded bundle or silently rotate trust through reference data.

## Verified activation and durable state

Introduce a bounded activation record containing the original signed bundle
envelope and its signed manifest attestation. Reuse the existing authenticated
knowledge route; decoded bundle JSON alone cannot activate eligible knowledge.

Electron keeps responsibility for feed selection, download, verification,
scheduling, and cache fallback. The sidecar independently validates the received
activation record using the application-pinned public key, and derives an
internal verified-knowledge value that callers cannot construct from unsigned
fields. Public-key input must not be caller-selectable.

Both verifiers require matching signature, envelope digest, format, sequence,
version, privacy policy, page digests, relationship containment, and content/size
bounds. Select exactly one matching manifest entry; conflicting duplicate
identities are invalid. Keep existing per-response and per-page bounds and impose
an explicit aggregate bound on activation records in HTTP and durable storage.

The activation record has exactly three fields: integer `activationFormatVersion:
1`, `bundleEnvelopeBase64`, and `manifestEnvelopeBase64`. Base64 encodes the
original response bytes; verification decodes them without reserializing either
envelope. Require canonical base64, valid UTF-8, duplicate-key rejection, and no
unknown activation fields. Count container nesting from zero before the root: every entered object or array, including a root container at depth one, adds one; scalars add none. Reject depth above 32 independently at each JSON boundary, with shared 31/32/33 object, array and mixed fixtures. Keep each decoded envelope at most 2 MiB and the
serialized activation record at most 6 MiB; two maximum-sized envelopes require
about 5.34 MiB after base64 encoding. Apply the 6 MiB limit only to the authenticated
knowledge route and its durable/cache records. The route currently inherits
Axum's 2 MiB JSON limit; the workflow-observation route's separate 8 KiB cap stays
unchanged. Retain the existing 64 KiB page and 256-page bounds.

Retain the complete signed manifest that attests the selected bundle, never an
unsigned extracted entry. Its existing 2 MiB/1,000-entry bounds remain explicit.
Keep immutable bundle URLs and historical manifests in the archive; the current
feed manifest may retain the latest entry for each supported compatibility
class rather than every historical release. Each compatibility class must have
a deterministic client selector, with format and privacy policy checked before
sequence ranking. If the bounded current manifest can no longer represent every
supported class, publication fails pending an explicit format migration; it
never silently drops compatible clients.

Boundary tests submit the largest compact protocol-valid activation (two 2 MiB envelopes, 5,592,491 serialized bytes) through the real Electron client and authenticated HTTP route, restart from it, and reject excess encoded/decoded size, malformed
base64/UTF-8, duplicate fields, and truncated or substituted signed manifests
without changing active knowledge or reserving inference usage. Separately prove the route accepts otherwise-valid JSON padded with legal whitespace to exactly 6 MiB and rejects one extra byte; prove the client request guard against a recording local server using transport-only compact JSON payloads at those sizes. The real client's JSON.stringify cannot produce a protocol-valid 6 MiB compact packet from two bounded envelopes.

Package the generated starter with its signed manifest attestation and the same
pinned public key. Feed caches retain the activation proof as one atomic record,
rather than independently replaceable bundle and manifest files. Use the same
verification path for starter, active cache, previous cache, and sidecar startup.
A standalone signed bundle without matching retained manifest proof is ineligible.

Electron's cache is shared by packaged instances in the same installation.
Its existing fixed `active.json.tmp` writer is not a concurrency contract.
Use a retained-inode OS advisory lock following the existing `fs-ext` history
pattern, with bounded acquisition and no age-based lock eviction. Download and
verify outside the lock; then lock, reread/reverify durable active and previous
records, and compare sequence plus full bundle identity before committing.
Do not hold a cache lock across a network request or a sidecar request. Derive a storage namespace from the packaged trusted key's SPKI SHA-256: active/previous proofs, advisory status and retained lock live under `keys/<fingerprint>/` inside the installation knowledge directory. Overlapping old/new pinned-key clients never overwrite or delete another key's records, use them as fallback, or apply their sequence floor. The sidecar's additive proof likewise lives at `<Taskmaster root>/knowledge/<fingerprint>/activation.json`, derived from its embedded key. Rotation does not broaden client trust; concurrent-key fixture tests prove preservation.

Use unique same-directory temporary files, flush them, and atomically replace
records using the existing platform-safe replacement pattern. Preserve a verified
previous record before replacing active. Under the lock, a slower writer must
adopt an already committed newer eligible record rather than downgrade it;
equal-sequence different content is refused. After an ambiguous replacement,
read back the expected identity before claiming success. On lock contention or
write failure, keep the last independently verified in-memory/starter knowledge
and report the update failure. Reconcile the in-memory selection with durable
state before synchronization; advisory status cannot confer eligibility.

Add two-process cache tests for older/newer writers, equal-sequence conflicts,
crashes between previous/active replacement, and lock contention. Assert that
no writer clobbers another's temporary file, reports an uncommitted activation,
or discards every verified fallback. The updater also compares an equal-sequence signed feed entry with the current verified identity before skipping download; a different version or envelope digest is an update error, not a successful check. This is short file-update serialization,
not peer-instance discovery or analysis coordination.

Persist the proof atomically under Taskmaster's existing global knowledge store,
using its existing persistence lock order. Verify before touching the current
record; refuse a lower sequence and refuse different content at the same
sequence. Idempotence uses bundle identity, not the bytes of its manifest: a
newer valid manifest attesting the same bundle is not conflicting knowledge and
does not advance evaluation generation. Keep either matching verified attestation.
Test same-bundle/different-manifest activation as well as equal-sequence different
bundle rejection. Advance the existing evaluation generation before publishing
new knowledge; a crash may invalidate old work but
must never admit old work against new knowledge. Re-read and verify durable
knowledge whenever a runtime view reloads it.

Legacy plain starter/cache/sidecar records remain ineligible and cannot set a
sequence floor that prevents recovery to a compliant signed starter. Preserve
legacy files for diagnosis rather than treating them as trusted fallback.
Among eligible snapshots, prefer the highest verified sequence. A damaged active
cache can fall back to a verified previous snapshot or starter. If no eligible
snapshot exists, report unavailable and invoke no provider. An unavailable,
incompatible, tampered, oversized, redirected, cancelled, or interrupted update
must not displace eligible knowledge.

Eligibility has no independent age cutoff and requires no feed connection at
dispatch. Preserve the existing startup-when-due and six-hour running checks,
the automatic-update toggle, and cancellation/generation handling.

## Analysis admission and identity

Replace the unconditional refusal only when the complete verified activation
path and generated starter are delivered. Make eligibility a server-owned
runtime query shared by status, manual admission, background scheduling, and
worker entry. Refuse missing or invalid proof before provider preparation,
context collection, or a usage reservation. Preserve lease-protected run-status
recovery and deterministic observation evaluation.

Carry the full verified bundle identity in the evaluation snapshot: its exact
envelope digest, signed policy, version, sequence, and trusted key identity.
Relevant-page ranking changes only prompt selection, not this identity.
Revalidate knowledge together with effective settings, provider/trust identity,
workspace instance, and generation at reservation, the actual native/custom
dispatch boundary, and result acceptance. Knowledge replacement or corruption
invalidates queued/in-flight work; stale output cannot populate caches,
recommendations, or semantic rollups.

Extend the current guards rather than assuming their checks are equivalent.
The native `with_current_native_evaluation` guard compares current knowledge
and must compare the verified proof identity. Add that same identity to
`CapturedInference` and reverify/compare it in `with_current_custom_data`;
its current generation/settings/trust checks alone do not detect durable proof
corruption without a generation change. Apply the same proof check at output
and rollup acceptance, including existing native generation-only continuations.

Preserve the existing lock order: harness snapshot, Taskmaster persistence and
data, then the live workspace guard at dispatch. Perform proof revalidation
inside those guards through the existing native send/custom process-spawn
callback; release locks before waiting for inference output. The implementation
plan must pin the exact native HTTP-send callback and process-spawn seam with
tests, rather than adding a check followed by an unguarded send. Include a
same-generation durable-proof corruption test as well as normal activation and
configuration-change races. A narrowing configuration mutation retains its
existing invalidation behavior.

Preserve background accounting and manual exemptions, the active Brain
recommendation gate, dismissal watermarks, provider compatibility and executable
trust, Peon's independent selection, and managed CLI policy. This change does
not add a provider, execute a recommendation, or alter Fix with AI authority.
Show current knowledge availability separately from the last recorded run
outcome and separately from update errors.

## Verification and requirement coverage

| Requirement from #529 | Delivery boundary | Required evidence |
| --- | --- | --- |
| Strict exclusions; no exceptions | Brain policy and reviewed allowlist | Mistakenly allowlisted excluded classes fail; owner-reviewed exported artifact diff |
| Direct reviewed export, default exclusion | Brain exporter | Exact-byte hash mismatch and new/unclassified pages cannot publish |
| Content privacy | Review and final-artifact validation | Private markers in prose, labels, metadata, URLs and provenance do not appear in any public artifact |
| Hierarchy, stable IDs, maturity, applicability, provenance, index | Export schema, generated index and cumulative private receipt ownership | Cross-publication rename/reuse/omission rejection and valid reintroduction; deterministic output; internal IDs/links resolve only inside bundle; client round-trip preserves metadata |
| Generated starter and source revision | Publisher receipt and resource import | Starter matches signed publication bytes; private receipt records source revision and version |
| Signed publication and pinned key | Publisher and both verifiers | Missing key fails before mutation; wrong key/signature/digest/manifest is rejected |
| Automatic immutable distribution | Durable archive and existing Pages workflow | First-run bootstrap; retained-artifact/receipt verification; cancellation recovery; older compatible URLs and unchanged bytes; idempotent rerun |
| Exclusion edge cases | Export negative fixtures | Links, frontmatter, HTML/assets, traversal and symlink cases cannot import private markers |
| Packaged offline and update behavior | Desktop packaging and live feed | Fresh offline starter, newer signed auto-activation, version reporting, offline/tampered/incompatible/interrupted fallback |
| Promotion, ownership, key rotation, troubleshooting | Publisher/consumer documentation | Documented reproducible promotion and recovery exercise with evidence links |
| Signed policy and assessment capability | Payload, manifest and reviewed guidance | Missing/legacy/non-integer/unsupported/mismatched policy fails; assessment marker only accompanies reviewed required pages |
| Signed starter attestation | Packaged activation record and bounded transport | Unsigned starter, standalone signed bundle and swapped manifest cannot enable inference; actual-route size/encoding limits and restart round-trip |
| Server-owned admission/revalidation | Sidecar runtime and real dispatch paths | Native/custom background and Analyze now positive path plus zero-call/no-reservation negatives; activation/configuration and same-generation proof-corruption races reject stale work |
| Concurrent cache preservation | Electron installation cache transaction | Two-process old/new and conflicting writers; crash/lock-contention recovery retains verified fallback |

Use shared signed fixtures in TypeScript and Rust, with test-only keys distinct
from the application key. Run the existing knowledge-update, activation,
identity, and settings/admission suites with assertions at actual provider
invocation boundaries, not only helper-return values. Include process restart,
durable corruption, equal-sequence equivocation, legacy sequence poisoning, and
changed-knowledge output rejection. Observe deterministic recommendations while
Brain admission is unavailable.

For the packaged demonstration, record app/source version, platform, public-key
fingerprint, bundle versions/digests, feed publication, activation status, and
observed provider dispatch. Do not attach prompts, credentials, private source
pages, or workspace evidence. Follow existing provider trust and context settings
for the explicit live run. Host-platform evidence alone does not prove native
packaging behavior on another platform; disclose any remaining platform gap.

## Delivery boundaries and owner gates

The cross-repository outcome has three linked review boundaries: publisher and
reviewed guidance; consumer proof/activation/admission; generated resources and
packaged verification. Keep the closed prerequisite until all required pieces
are available. A passing exporter test or an application fixture signed with a
test key is not production readiness, and partial PRs must not close #529.

After written-design approval, prepare the implementation plan using
`writing-plans` and `reviewing-plans`, with a concrete delivery and verification
mapping for every row above. Record the signed activation/admission protocol in
an accepted ADR before runtime implementation. Preserve ADR 0054's managed-policy
decision; this work does not reverse it.

Brain source changes follow its scoped instructions and an owner-authorized
branch/PR workflow for publisher code. Final allowlist/content approval and
production signing provisioning are explicit owner gates. Drafting this design
does not provision a key, approve source-page contents, publish the feed, or
enable inference. Keep #529 open until its attached evidence demonstrates the
whole outcome. Reconcile its obsolete context wording during implementation
tracking without weakening the current acceptance criteria.

## Uncertainty and blind-spot checkpoint

**What am I least confident about?** Production signing and the exact safe
allowlist. Repository/API reads proved source access, the existing Pages assembly,
and the empty repository-secret list. They did not prove possession of a matching
private key or content approval. Hold dependent publication and starter generation
until owner-controlled key provisioning and exact artifact review are evidenced.

**What might the project be missing?** A working download signature check is not
durable server-owned eligibility, and a successful Pages deployment does not
retain older bundle URLs automatically. Source tracing confirmed both gaps.
The retained activation proof, sidecar startup/dispatch verification, and durable
publication archive address them. Reviewed content hashes also prevent automatic
daily publication from silently treating changed page prose as previously approved.

| Complexity dimension | Initial rating and evidence |
| --- | --- |
| Dependencies | 4 — coordinated private publisher, public consumer, owner-held key and page review |
| Blast radius | 4 — knowledge admission affects every native/custom Brain analysis path |
| State changes | 3 — additive signed proof/cache state; legacy data becomes unusable for analysis and is preserved |
| Reversibility | 3 — revert consumer to the currently closed gate, preserve archive/cache, and publish corrections at a higher sequence |
| Uncertainty | Unknown — final reviewed content and usable production signing configuration are not established |

Total: incomplete because production evidence is unknown. No numeric sum is
claimed. The consumer rollback is the current closed-gate version; it preserves
cache/archive records and therefore does not require pretending an older client
understands the new activation format. Key availability and content approval
remain explicit prerequisites to production delivery.

## Reviewing-plans result — 2026-10-10

Reviewed this design against every #529 acceptance criterion, the accepted
knowledge spec, actual updater/cache and sidecar guard code, and the existing
Brain Pages workflow. Initial result: **Revise**. Corrected the following gaps
in this document; no implementation or publication occurred.

| Finding | Evidence and consequence | Design correction and verification |
| --- | --- | --- |
| Activation representation and bounds were deferred | Knowledge handler uses JSON without a route-specific body override; Axum defaults to 2 MiB, while two signed envelopes and encoding can exceed it | Exact base64 activation schema, 2 MiB decoded-envelope/6 MiB aggregate limits, route-only override, signed-manifest retention and actual-route boundary tests |
| Atomic replacement did not address concurrent cache writers | Packaged instances share the userData cache; `KnowledgeUpdates.atomic` uses one fixed temporary filename and in-memory sequence selection | Retained advisory lock, durable reread/sequence comparison, unique flushed temporary files, platform-safe replacement, read-back and two-process crash/contention tests |
| Archive reuse had no explicit verification/recovery check | The proposed publication branch is another input to public Pages; current Pages deploys a fresh artifact each time | Verify every retained publication and receipt before staging; first-run, tampered-archive and archive-commit/deployment interruption fixtures |

The review also clarified coordinated key/starter release ordering and the
native/custom guard differences, including corruption without a generation
change. The requirement matrix covers all thirteen issue criteria plus cache
concurrency; no requirement was dropped to simplify delivery. The archive is
justified by immutable URL retention across full-site Pages replacement; reuse
the existing updater, dispatch callbacks, persistence guards, and locking pattern
rather than introduce a second knowledge service.

**Scope/simplicity:** pass for written-design review; retain the coupled
publisher/consumer/offline end-to-end outcome and use its three delivery
boundaries for implementation planning.

**Clarity:** pass for written-design review; signed transport, cache transaction,
archive recovery, and owner-controlled gates are explicit. Package/module choice
and detailed test scheduling belong in the subsequent implementation plan.

**Verification:** pass for written-design review; all mandatory requirements
have delivery and observable positive/negative checks. Before that plan is
marked Ready, pin the native HTTP-send test seam and the exact export metadata
schema/compatibility selector; before production delivery, provide approved
content, matching signing configuration, and packaged/live-feed evidence.

Final design quality: **Ready for written-design review**. Implementation
readiness: **Investigate**, pending the listed evidence and normal approval/ADR/
implementation-plan gates. Complexity ratings remain 4, 4, 3, 3, Unknown;
total incomplete. This review is not implementation approval.
