---
type: Design
title: Privacy-qualified Taskmaster knowledge distribution
description: Reviewed Brain export, signed offline and feed knowledge, and verified Taskmaster admission for issue 529.
tags: [orkworks, taskmaster, knowledge, privacy, distribution]
status: proposed
---

# Privacy-qualified Taskmaster knowledge distribution

- Date: 2026-10-10
- Status: proposed; approach approved in chat, written-design review pending
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
  Every exported string is part of the reviewed artifact.
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

Grant `taskmaster-assessment-v1` only when the reviewed distilled assessment
entry point and required general guidance are present. Do not export the full
private assessment process or claim that this capability implements #769.
The final #529 evidence must identify the reviewed guidance and capability;
until then the dependent assessment prerequisite remains unavailable.

Integrate publication into Brain's existing Pages assembly and deployment,
preserving its current visualization. Supply only validated public Taskmaster
artifacts under `orkworks-knowledge/`. Never stage the source checkout or
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

Missing or invalid signing configuration fails the publication job before any
new Taskmaster artifacts or archive mutation. Validate the approved export
before accessing the signing key. Provisioning, replacing, or rotating that
secret is an owner-controlled operation, not implied by repository read access.
No private key or private-repository credential enters app resources, published
artifacts, logs, or test fixtures.

Use the current pinned public key only if the owner can provide its matching
publishing key. Otherwise explicitly approve a new key and ship its public half
in both consumer verification paths before publishing an eligible starter.
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
The implementation plan must reconcile this aggregate with the route body limit
before coding; no unbounded manifest history is sent with every activation.

Package the generated starter with its signed manifest attestation and the same
pinned public key. Feed caches retain the activation proof as one atomic record,
rather than independently replaceable bundle and manifest files. Use the same
verification path for starter, active cache, previous cache, and sidecar startup.
A standalone signed bundle without matching retained manifest proof is ineligible.

Persist the proof atomically under Taskmaster's existing global knowledge store,
using its existing persistence lock order. Verify before touching the current
record; refuse a lower sequence and refuse different content at the same
sequence. Identical activation is idempotent. Advance the existing evaluation
generation before publishing new knowledge; a crash may invalidate old work but
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

Use the current persistence, harness, workspace, and dispatch guards in their
existing lock order. The implementation plan must identify the concrete
linearization point for both transports, including native HTTP and custom
process start, so a check followed by an unguarded send is not presented as
revalidation. A narrowing configuration mutation must retain its existing
invalidation behavior.

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
| Hierarchy, stable IDs, maturity, applicability, provenance, index | Export schema and generated index | Deterministic output; internal IDs/links resolve only inside bundle; client round-trip preserves metadata |
| Generated starter and source revision | Publisher receipt and resource import | Starter matches signed publication bytes; private receipt records source revision and version |
| Signed publication and pinned key | Publisher and both verifiers | Missing key fails before mutation; wrong key/signature/digest/manifest is rejected |
| Automatic immutable distribution | Durable archive and existing Pages workflow | A later deployment retains older compatible bundle URLs and unchanged bytes; rerun is idempotent |
| Exclusion edge cases | Export negative fixtures | Links, frontmatter, HTML/assets, traversal and symlink cases cannot import private markers |
| Packaged offline and update behavior | Desktop packaging and live feed | Fresh offline starter, newer signed auto-activation, version reporting, offline/tampered/incompatible/interrupted fallback |
| Promotion, ownership, key rotation, troubleshooting | Publisher/consumer documentation | Documented reproducible promotion and recovery exercise with evidence links |
| Signed policy and assessment capability | Payload, manifest and reviewed guidance | Missing/legacy/non-integer/unsupported/mismatched policy fails; assessment marker only accompanies reviewed required pages |
| Signed starter attestation | Packaged activation record | Unsigned starter, standalone signed bundle and swapped manifest cannot enable inference |
| Server-owned admission/revalidation | Sidecar runtime and real dispatch paths | Native/custom background and Analyze now positive path plus zero-call/no-reservation negatives; activation/configuration races reject stale work |

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

Total: incomplete because production evidence is unknown. Design quality:
ready for written-design review; implementation planning must resolve the
specified parser/body bounds and dispatch linearization, and dependent production
work must wait for the owner gates. No implementation readiness is claimed.
