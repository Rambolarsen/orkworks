---
type: Plan
title: Privacy-qualified Taskmaster knowledge implementation
description: Deliver issue 529 through a reviewed Brain publisher, independently verified activation, and packaged offline/live evidence.
tags: [orkworks, taskmaster, knowledge, privacy]
status: active
---

# Privacy-qualified Taskmaster Knowledge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore Brain analysis from reviewed signed knowledge, including fresh offline installations and independently updated feeds.

**Architecture:** Brain exports exact reviewed source bytes, signs immutable publications, and joins its existing Pages deployment. Electron downloads and retains complete signed activation proofs; Rust independently verifies those proofs before admission, dispatch, and result acceptance. Reuse existing update scheduling, provider callbacks, persistence guards, and retained-inode locking.

**Tech Stack:** Existing Node crypto/TypeScript, Rust serde/sha2 with ed25519-dalek 2.2 (pkcs8/pem) and base64 0.22, Brain Node standard-library publisher/tests, existing GitHub Actions/Pages.

**Spec:** [Approved written design](../specs/2026-10-10-taskmaster-knowledge-distribution-design.md), approved by the owner’s “keep going” on 2026-10-10. [Knowledge specification](../../../specs/taskmaster-knowledge.md) remains authoritative. [Issue #529](https://github.com/Rambolarsen/orkworks/issues/529) tracks the whole outcome; [#782](https://github.com/Rambolarsen/orkworks/issues/782) admission stays closed until production prerequisites are delivered. [#769](https://github.com/Rambolarsen/orkworks/issues/769) consumes assessment guidance later; its action is outside this plan.

**Pass condition:** A packaged fresh offline app admits valid starter knowledge, automatically activates a newer real signed feed publication, and dispatches native/custom analysis under existing controls. Negative export, proof, cache, and dispatch cases retain eligible knowledge or make zero provider calls. Do not close #529 on fixture-only evidence.

## Global Constraints

- “Personal or project-specific assessments, project pages, experiment records, raw research, and repository-specific evidence are always excluded; there are no per-page exceptions to these exclusions.”
- “Knowledge is reference data, never executable tools, permission policy, or authority over repository instructions.”
- “There is no unsigned-JSON or app-packaging trust bypass.”
- “Bundle eligibility has no independent wall-clock age cutoff.”
- “There is no automatic provider fallback or model routing in v1.”
- “The app checks for updates at startup when due, then every six hours while running.”
- Signed payload and manifest entry must carry matching integer `privacyPolicyVersion: 1`.
- Preserve observation recommendations, Peon selection, executable trust, managed CLI policy, context bounds, manual exemptions, usage accounting, leases, dismissal watermarks, and instance/workspace identity.
- Keep Electron and renderer imports separate; pnpm is the only Node package manager in OrkWorks.
- Test keys are distinct from the application key and never confer production eligibility. Production key provisioning and exact public-content approval are owner gates.
- Work only in owned branches/checkouts. Brain publishing code uses a separate owner-authorized branch/PR. Never ship or stage the private source checkout.
- Prefix shell commands with `rtk`; command examples below already do so.

## Evidence, uncertainty, and complexity

**Least confident:** usable production signing configuration and exact approved content. API access proves access to Brain; an empty repository-secret list proves neither key absence nor availability. Build and verify publisher/consumer fixture paths first; hold production signing and final source promotion for concrete owner review.

**Project blind spot:** packaged instances share the cache, while existing writes use a fixed temporary filename. Reuse the history store’s advisory lock and platform replacement behavior; prove two-process preservation before using activation proofs there.

Resolved planning details:

- Native HTTP boundary is `providers.rs::poll_http_send_with_dispatch_gate`, called on every poll of the lazy reqwest send future in `HttpRunner::run_at_with_dispatch_gate`. Existing `ollama_send_rechecks_dispatch_gate_after_a_pending_poll` pins repeated polling. Native process boundary is `ProcessRunner::run_prepared_with_dispatch_gate` around `command.spawn` plus child preparation.
- Custom process boundary is `runtime/inference.rs::invoke_custom_inference_with_dispatch_gate`, nesting the existing current-custom guard and evaluator workspace callback around spawn. Keep lock order: harness snapshot → persistence/data → live workspace.
- Brain schemas permit arbitrary additional metadata. Export metadata instead comes from a reviewed closed policy. No general YAML tags/templates/includes or source traversal.
- The existing authenticated knowledge route inherits Axum’s 2 MiB default. Increase only that route to 6 MiB; the observation route’s 8 KiB limit remains.
- Existing Pages assembly replaces the complete site. Verify and retain an immutable archive, then join the existing visualization assembly.
- Crypto APIs checked against [ed25519-dalek 2.2](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.VerifyingKey.html) and [base64](https://docs.rs/base64/latest/base64/engine/general_purpose/index.html). Use strict Ed25519 verification; no custom cryptography.

| Dimension | Rating and treatment |
| --- | --- |
| Dependencies | 4 — two repositories plus owner-held signing/content; independent fixture work proceeds first |
| Blast radius | 4 — every Brain admission path; use existing guards and actual invocation tests |
| State changes | 3 — new additive proof files, legacy retained but ineligible |
| Reversibility | 3 — closed-gate client rollback; preserve archive/cache and correct at higher sequence |
| Uncertainty | Unknown — owner key/content and real packaged/live evidence still absent |

Total: **Incomplete**. Plan quality is assessed at the end. Unknown production evidence holds Tasks 6–7’s production operations, not Tasks 1–5’s reversible fixture implementation.

## Exact shared wire contract

All JSON is valid UTF-8, depth at most 32, and rejects duplicate object keys at every depth, including escaped aliases. Reject unpaired Unicode surrogate escapes in every JSON string, including keys and signed payload strings. All numeric tokens in these records must have the lexical form `0|[1-9][0-9]*` and value at most 9,007,199,254,740,991; reject negative zero, signs, fractional and exponent spellings even when numerically integral. Unknown fields follow these same lexical rules. Unknown signed payload/page metadata does not establish eligibility; this format’s publisher emits only the listed fields. Activation and envelope objects reject unknown fields.

| Record | Fields and bounds |
| --- | --- |
| Envelope | Exactly `payload: string`, `signature: string`; canonical padded standard base64, decoded signature 64 bytes; signature over exact UTF-8 payload bytes; envelope at most 2 MiB |
| Activation | Exactly `activationFormatVersion: 1`, `bundleEnvelopeBase64: string`, `manifestEnvelopeBase64: string`; canonical padded standard base64 of original envelope response bytes; each decoded envelope at most 2 MiB, complete serialized activation at most 6 MiB |
| Bundle | `formatVersion: 1`, `version` safe identifier (1–128 ASCII alphanumeric/dot/underscore/hyphen characters), `sequence` safe integer, `publishedAt` canonical UTC ISO timestamp with milliseconds, `privacyPolicyVersion: 1`, `capabilities` at most 16 unique safe identifiers (each ≤128 bytes), `pages` 1–256 |
| Page | Existing `id,title,type,status,content,sha256,relatedIds`; optional `parentId`, `applicability`, `provenance`. ID ≤256 ASCII safe relative Markdown path, no empty/dot/dotdot segments or backslashes. Title 1–512 UTF-8 bytes; type/status 1–128 bytes; content ≤64 KiB UTF-8; SHA-256 lowercase hex of exact content bytes. Related IDs unique, ≤256, all included. Parent included, not self, no hierarchy cycles. Applicability ≤32 unique strings each 1–128 bytes. Provenance ≤16 records exactly `title` (1–512 bytes) and `url` (HTTPS, ≤2048 bytes, no credentials/fragments); destinations explicitly reviewed in publisher policy. |
| Manifest | `formatVersion: 1`, `bundles` 1–1000 entries, signed envelope ≤2 MiB |
| Entry | `formatVersion`, `privacyPolicyVersion` positive safe integers; `sequence`, `version`, `sha256`, `path`. Path exactly `bundles/<safe version>.json`; digest over original bundle envelope bytes. Duplicate (format, policy, sequence), duplicate version, or duplicate path fails the manifest. |
| Verified identity | Bundle envelope SHA-256, format, signed policy, version, sequence, pinned public-key SPKI SHA-256; manifest bytes are retained proof but not bundle identity |

Select manifest entries with format 1 and policy 1 **before** maximum-sequence ranking. Other structurally valid compatibility classes are ignored, never used as fallback. Missing/non-integer policy fails parsing. Exactly one manifest entry must bind the activated bundle’s identity and raw digest. Same bundle attested by another valid manifest is idempotent; equal-sequence different bundle is rejected.

Assessment capability is the exact string `taskmaster-assessment-v1`; publisher policy separately names all reviewed assessment-guidance public IDs required before it emits that marker. Empty/incomplete guidance cannot emit it. Unknown bounded capabilities convey no authority.

Public page type is one of concept, principle, practice, playbook, reference, implementation-mapping, or generated index. Source directory/type must agree. Preserve status without promoting its maturity; reviewed applicability/provenance/parent/related fields are explicit policy values. Generate `index.md` with type `index`, status `active`, included public links and hierarchy. The reserved ID cannot be supplied by a source entry.

## Files and responsibilities

| Area | Files | Responsibility |
| --- | --- | --- |
| Protocol | `apps/desktop/electron/knowledgeProof.ts`, `strictJson.ts`; `crates/orkworksd/src/taskmaster/knowledge.rs`, `knowledge/strict_json.rs` | Independent bounded proof verification and opaque verified identity |
| Shared test data | `tests/fixtures/taskmaster-knowledge/`, `scripts/generate-knowledge-fixtures.mjs` | Public synthetic signed corpus, test-only key, deterministic regeneration |
| Cache | `apps/desktop/electron/knowledgeCache.ts`, existing `knowledgeUpdates.ts` | Locked proof transactions, update selection and fallback |
| Main/API | `apps/desktop/electron/main.ts`, Rust settings handler and `main.rs` | Exact activation transport, authenticated route limit, status |
| Runtime | `taskmaster/runtime.rs`, `runtime/inference.rs`, `evaluator.rs`, rollup helpers | Verified durable state, admission, actual dispatch and output guards |
| Brain policy/export | `support/taskmaster/allowlist.json`, `scripts/taskmaster/export.mjs`, `policy.mjs`, `tests/taskmaster-export.test.mjs` | Exact reviewed source export and generated index; no recursive inclusion |
| Brain publication | `scripts/taskmaster/publish.mjs`, `archive.mjs`, tests, existing Pages workflow | Signed append-only archive, validated staging, preserved visualization |
| Release/docs | Existing knowledge resources and packaging verifier; `docs/agents/taskmaster-knowledge-publication.md`; Brain publisher guide | Generated starter import, pinned-key consistency, operational evidence |

Task 1’s ADR records the protocol; select the next unused ADR number after checking the index and remote main, then update the index. This complements ADR 0054 rather than superseding managed CLI policy.

## Task 1: Electron proof verifier and shared signed corpus

**Files:** Create protocol/test-data files listed above and `apps/desktop/tests/knowledgeProof.test.ts`; create ADR and update index. Do not change admission or the existing starter yet.

**Interfaces:**

```ts
export type KnowledgeActivation = {
  activationFormatVersion: 1;
  bundleEnvelopeBase64: string;
  manifestEnvelopeBase64: string;
};
export type VerifiedKnowledge = {
  activation: KnowledgeActivation;
  bundle: KnowledgeBundle;
  identity: KnowledgeIdentity;
};
export function verifyKnowledgeActivation(bytes: Uint8Array, publicKey: string): VerifiedKnowledge;
export function verifyKnowledgeManifest(bytes: Uint8Array, publicKey: string): KnowledgeManifest;
export function selectKnowledgeEntry(manifest: KnowledgeManifest): KnowledgeManifestEntry;
export function createKnowledgeActivation(bundle: Uint8Array, manifest: Uint8Array): KnowledgeActivation;
```

Key argument belongs to trusted Electron construction/tests; no IPC or downloaded key may populate it. Production caller loads the packaged key. Freeze or clone verified values so later mutation cannot bypass identity.

- [ ] Record accepted ADR before code, including wire bounds, trust ownership, additive persistence, legacy recovery, and closed production gate.
- [ ] Write synthetic fixture generator using Node `generateKeyPairSync("ed25519")` once into clearly test-only files; subsequent regeneration uses that fixed test key. Include valid bundle/manifest/activation, same bundle with newer manifest, wrong key/signature/digest/policy, unsupported class, duplicate fields, invalid metadata/IDs/parent cycles, and malformed UTF-8/base64. Never use source Brain content.
- [ ] Write tests against the missing verifier and observe RED:

```ts
test("matching signed policy and attestation admit synthetic knowledge", () => {
  const result = verifyKnowledgeActivation(fixture("valid-activation.json"), fixtureText("test-public-key.pem"));
  assert.equal(result.bundle.privacyPolicyVersion, 1);
  assert.equal(result.identity.bundleSha256, fixtureText("valid-envelope.sha256").trim());
});
test("an unsigned bundle cannot activate knowledge", () => {
  assert.throws(() => verifyKnowledgeActivation(fixture("unsigned-bundle.json"), fixtureText("test-public-key.pem")));
});
```

Run: `rtk proxy node --experimental-strip-types --test tests/knowledgeProof.test.ts` from `apps/desktop`. Expect missing verifier/feature failure; then implement and require all corpus cases to pass.
- [ ] Implement strict JSON syntax/key scanner followed by JSON.parse, with decoded-key duplicate checks, depth/byte limits, and fatal UTF-8. Verify Ed25519 envelopes with Node crypto; enforce the shared table, exact digest binding and deterministic selection. No network or persistence in these modules.
- [ ] Add boundary cases for escaped duplicate keys, integer overflow, rejected lone surrogates and numeric tokens `1.0`, `1e0`, `-0`, both maximal envelopes, activation overflow, all page limits, two compatible entries ranking, and unsupported high-sequence entries not displacing eligible entries.
- [ ] Run tests and both Electron/renderer type checks; commit only Task 1 files.

## Task 2: Independent Rust verifier

**Files:** `taskmaster/knowledge.rs`, `knowledge/strict_json.rs`, `taskmaster/mod.rs`, Cargo.toml/lock; consume the shared corpus without editing its expected results.

**Interfaces:**

```rust
pub(crate) fn verify_activation(bytes: &[u8]) -> Result<VerifiedKnowledge, String>;
pub(crate) struct VerifiedKnowledge { /* private bundle, activation, identity */ }
impl VerifiedKnowledge {
    pub(crate) fn bundle(&self) -> &KnowledgeBundle;
    pub(crate) fn identity(&self) -> &KnowledgeIdentity;
    pub(crate) fn activation_bytes(&self) -> &[u8];
}
```

Only verifier constructs this type. Production verifier uses `include_str!` of the same app resource public key; a key-parameter helper exists privately under unit tests, never HTTP/config.

- [ ] Load rust-skills and writing-good-tests before Rust/test changes.
- [ ] Add corpus tests before implementation. Example RED:

```rust
#[test]
fn legacy_unsigned_knowledge_has_no_verified_identity() {
    assert!(verify_activation(include_bytes!("../../../../tests/fixtures/taskmaster-knowledge/unsigned-bundle.json")).is_err());
}
```

Run: `rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::knowledge`. Confirm expected missing feature, implement, rerun.
- [ ] Add ed25519-dalek 2.2 with pkcs8/pem and base64 0.22. Decode the packaged Ed25519 SPKI PEM; reject non-Ed25519/weak keys; use `verify_strict`. Reject noncanonical base64 by decode/re-encode equality.
- [ ] Use a bounded lexical pass enforcing the shared numeric spelling and surrogate rules, then a serde visitor preserving duplicate detection recursively before typed decoding; never deserialize signed policy into a permissive Value first. Enforce identical limits/identity and UTF-8 behavior to Task 1.
- [ ] Run the same valid/invalid corpus in Rust and TypeScript; add generated-size tests where files would be wasteful. Verify the production entry point rejects every fixture signed only with the test key.
- [ ] Run crate build, focused tests, formatting and commit. Admission remains closed.

## Task 3: Durable Electron proof cache and transport

**Files:** `knowledgeCache.ts`, `knowledgeUpdates.ts`, `main.ts`, existing updater tests, new `knowledgeCache.test.ts` and worker fixture. Reuse retained-lock/platform-replace patterns from `workspaceMemory.ts`.

**Interfaces:**

```ts
export class KnowledgeCache {
  constructor(options: { directory: string; publicKey: string; starterPath: string });
  load(): Promise<VerifiedKnowledge | null>;
  commit(candidate: VerifiedKnowledge): Promise<VerifiedKnowledge>;
}
```

Updater still exposes status and existing automatic settings. Synchronization sends `KnowledgeActivation`, not decoded `KnowledgeBundle`.

- [ ] RED: signed starter works offline; legacy starter/cache with arbitrarily high sequence cannot suppress a compliant starter; cache without signed manifest fails; tamper falls back to independently verified previous/starter.
- [ ] Implement bounded file reads (6 MiB), independent revalidation, highest verified sequence selection and retained legacy files. Keep in-memory eligibility separate from update error/status JSON.
- [ ] RED two-process tests: slower old writer adopts newer durable proof; equal sequence different bundle rejects; lock contention times out preserving starter; crash between previous and active leaves a verified candidate; unique temporary files cannot collide.
- [ ] Implement retained-inode fs-ext advisory lock with bounded retries; download outside lock, reread and verify within lock, compare full identity, write verified previous before active using unique mode-0600 flushed temporary files and platform-safe atomic replacement. Read back ambiguous outcomes. Never unlink the lock or hold it over network/sidecar calls.
- [ ] Adapt updater: verify manifest first, filter supported class before ranking, fetch bounded HTTPS with redirect/error/cancellation behavior preserved, verify complete candidate, commit, then synchronize actual selected durable proof. Same bundle/new manifest is idempotent.
- [ ] Run `rtk proxy node --experimental-strip-types --test tests/knowledgeProof.test.ts tests/knowledgeCache.test.ts tests/knowledgeUpdates.test.ts` plus TypeScript checks. Use `rtk proxy pnpm rebuild fs-ext` first if an Electron build last rebuilt its ABI. Commit.

## Task 4: Server-owned proof state and all analysis guards

**Files:** Existing runtime/inference/evaluator/rollup files, settings handler, route, activation/identity/rollup/admission tests. Extend existing guard APIs rather than adding a second authority service.

**Interfaces:** Runtime activation accepts activation bytes and verifies them itself. EvaluationSnapshot carries a full verified identity separately from prompt-selected pages. CapturedInference carries the same identity. Availability queries the runtime’s reverified proof. Never pass a decoded bundle directly into activation.

- [ ] RED through authenticated real route: compliant activation persists/restarts; standalone signed bundle/unsigned JSON fails; exactly bounded packet succeeds; oversized encoded/decoded body, wrong key, duplicate outer/policy fields, truncated/swapped attestation fail with no durable mutation/reservation.
- [ ] Change only authenticated knowledge route’s body cap to 6 MiB. Bound raw bytes before strict parse; preserve existing auth. Persist proof atomically with existing persistence lock, reread/reverify on open/reload; preserve legacy files but ignore their sequences.
- [ ] RED runtime state tests: downgrade, equal-sequence equivocation, same-bundle/new manifest idempotence without generation change, crash after generation advance, same-generation durable corruption. Implement monotonic verified identity and advance generation before publishing changed proof.
- [ ] RED native/custom background/manual tests with fixture-only verified entry: invalid/missing proof causes zero provider preparation, context collection and reservations; valid proof proceeds through real invocation boundary under existing configured/trusted provider.
- [ ] Add proof comparison inside `with_current_native_evaluation`, `with_current_custom_data`, reservations, cache/recommendation acceptance and native/custom rollup continuations. Relevant-page ranking never replaces full identity. Keep harness→persistence/data→workspace lock order.
- [ ] Pin the existing lazy native HTTP poll callback with controlled pending future plus local HTTP endpoint tests; pin native/custom process spawn with marker executable. Mutate activation/configuration and corrupt durable proof without incrementing generation before dispatch; require zero first dispatch. Corrupt/change after dispatch; require no stale result/cache/recommendation/rollup acceptance.
- [ ] Keep the current production admission refusal until Task 7’s compliant generated starter and verified publication exist. Positive tests use a private test-only verifier/key path; no runtime flag, user setting or environment bypass.
- [ ] Verify unchanged observation recommendations, manual accounting exemptions, provider trust/model independence, leases and dismissal behavior using existing suites. Run full Rust required checks and relevant desktop API tests; commit.

## Task 5: Brain reviewed exporter

**Checkout:** Clone/attach Brain in a separate owned checkout, read its root/scoped instructions and conventions/schemas/governance. Create `taskmaster-knowledge-publisher` branch from verified main, no direct source-main changes. Keep source contents private.

**Files:** Brain `support/taskmaster/allowlist.json`, `scripts/taskmaster/{policy,export}.mjs`, tests and guide. Policy starts empty; production export fails until reviewed pages are explicitly approved. Synthetic test roots contain fixture markers only.

**Policy/interface:**

```ts
type ExportPolicy = {
  privacyPolicyVersion: 1;
  pages: Array<{ sourcePath: string; reviewedSha256: string; id: string;
    title: string; type: string; status: string; parentId?: string;
    relatedIds: string[]; applicability: string[];
    provenance: Array<{ title: string; url: string }>;
    publicUrls: string[] }>;
  assessmentGuidanceIds: string[];
};
export function exportKnowledge(root: string, policy: ExportPolicy): ExportResult;
```

ExportResult contains bounded public bundle payload/index and private receipt inputs; signing is separate. Exact source path/hash binds all frontmatter and body bytes even when metadata is not exported.

- [ ] RED via `rtk proxy node --test tests/taskmaster-export.test.mjs`: one safe distilled fixture generates deterministic hierarchy/index; unlisted sibling fixture never appears; changed source hash refuses export.
- [ ] Implement exact regular-file reads with lstat on every ancestor; reject wildcard/absolute/traversal/backslash/duplicate paths, symlinks and excluded directory/type even when explicitly listed. Require approved type/directory mapping; no source discovery or recursive copying.
- [ ] Parse only a bounded literal frontmatter subset for required type consistency; reject tags, aliases, includes and executable constructs. Policy owns export metadata. Strip frontmatter from public content; preserve reviewed body bytes except explicit link rewriting.
- [ ] Implement Markdown link validation using a parser/tokenizer that covers inline/reference/autolinks, images, HTML, code boundaries and escapes. Internal destinations resolve only declared source entries to their public IDs. Reject excluded/unresolved destinations, embedded assets/raw HTML/data URLs; external URLs must exactly match page publicUrls. Never fetch or import a link target.
- [ ] RED fixtures put unique private markers in prose, frontmatter, link labels/destinations, relationships, provenance, assets, excluded classes and symlink/traversal targets; all forbidden entry routes fail, and no successful output includes excluded markers. Test private names/URLs in reviewed policy fields using artifact review fixtures, without claiming a regex proves arbitrary prose safe.
- [ ] Emit closed schema, generated index, deterministic sorted relationships, exact content hashes, preserved maturity/applicability, bounded safe provenance. Require all reviewed assessment IDs before emitting the capability; no #769 action implementation.
- [ ] Produce a private receipt with source revision, policy hash, reviewed file hashes, exporterFormatVersion and exporter implementation/dependency-lock hashes; do not embed private repository URLs/history/source tree in public files. Write unsigned preview/artifact diff for owner content review.
- [ ] Run exporter tests, existing Brain validation commands and commit owned branch. Prepare a draft source PR with synthetic evidence; content/publishing approval remains a later concrete gate.

## Task 6: Brain signed archive and Pages integration

**Files:** Brain `scripts/taskmaster/{publish,archive}.mjs`, `tests/taskmaster-publication.test.mjs`, existing `.github/workflows/publish-okf-pages.yml`, publisher guide.

**Archive:** Owner-authorized `generated-taskmaster-publications` branch contains immutable bundle envelopes, historical signed manifests and private receipts. Current manifest retains latest entry per (format, privacy policy) class; historical artifacts remain available at unchanged paths.

- [ ] RED bootstrap/missing-key tests: valid synthetic export plus test key signs exact payload bytes; absent/invalid key fails before archive/site mutation; changing export bytes at same sequence/version fails.
- [ ] Implement standard-library Ed25519 signing only after export validation. Allocate monotonic safe sequence from verified archive; deterministic public version from that sequence, reuse identical archived output on rerun, retain original publishedAt/signature bytes.
- [ ] RED archive tests: malicious extra file, malformed receipt, receipt-only source identity substitution, bad signature/digest/policy/path, conflicting history, missing referenced bundle all refuse staging; verify only explicit inventory is copied. Check every retained publication against its signed historical manifest and receipt’s artifact/policy identity. Resolve the receipt’s source commit in the trusted Brain main history, require it to be an ancestor of the workflow source revision, load that commit’s exact allowlist and source blobs, compare policy/file hashes, and regenerate the public payload using the recorded publication identity and the matching supported versioned exporter. Keep byte-transforming exporters in immutable versioned modules (initially `scripts/taskmaster/exporters/v1.mjs`); receipts pin their implementation and dependency-lock hashes against trusted source history and the owner-reviewed supported-exporter inventory. Changes that alter regeneration require a new exporter version, retaining the prior verifier/regenerator. Never execute arbitrary script paths or code supplied by the archive. Its bytes must equal the attested payload. Missing history or changed source identity fails closed; a mutable receipt is never approval evidence alone.
- [ ] RED recovery tests: empty first archive succeeds; malformed existing archive fails; interruption after archive commit before deployment reruns same publication; concurrent publisher refuses stale branch update; old compatible URLs retain identical bytes; a newer exporter version does not break old-version regeneration; every supported class remains in bounded current manifest.
- [ ] Integrate with existing Pages assembly, preserving visualization at root. Serialize publisher jobs; do not force-push or cancel a publishing transaction. Use minimal token permissions for archive commit and existing Pages deployment. Fetch complete trusted source history for receipt validation. Keep an owner-reviewed archive verification-key inventory so rotation can verify retained publications under earlier approved keys; clients still pin their release key and never learn trust from downloads. Verify signing configuration before mutation; never print key/env contents.
- [ ] Run local fixture staging/recovery tests and workflow validation; prepare reviewable source PR and unsigned public preview.
- [ ] **Owner gate:** present exact public content/artifact diff and requested secret name `TASKMASTER_KNOWLEDGE_SIGNING_KEY`. Reuse current pinned key only with matching owner-provisioned key; otherwise request explicit new-key/paired-starter trust approval. Do not generate/set production secret or publish before this gate is resolved.
- [ ] After approval and source PR checks/review, provision via owner-controlled workflow, publish immutable production artifacts, independently download/verify actual feed bytes with both consumer verifiers and record fingerprints/digests. Keep #529 open.

## Task 7: Generated starter, production admission and packaged evidence

**Files:** `apps/desktop/resources/knowledge/starter.json`, public-key.pem if explicitly rotated, release verifier/tests, user troubleshooting and agent publication guide, runtime production gate.

- [ ] Add resource-import verification test before import: take bundle/manifest from a verified production publication, construct activation preserving exact bytes, require both verifiers and pinned-key fingerprint agree. Starter metadata/receipt identifies source revision and bundle version without publishing private provenance.
- [ ] Copy generated activation as starter; remove handwritten content maintenance. Keep starter/public key and Rust embedded trust in the same client release. Do not commit private key, source pages or private receipt into OrkWorks.
- [ ] Remove the closed production admission refusal only after generated compliant starter and verified production feed are present. Availability derives solely from reverified proof. Preserve last-run history separately from current eligibility/update errors.
- [ ] Run full required Rust/desktop checks, release resource verifier, docs build and git diff checks. Run fresh explicit code review at current code SHA with effort appropriate for security/protocol/concurrency scope; split code PR boundaries if the diff becomes too large.
- [ ] Build host packaged application through existing release script. Record platform, app/source SHA, public-key fingerprint, starter/feed versions/digests and status/provider invocation evidence. Fresh isolated userData, offline startup → valid starter and Analyze now; publish reviewed newer version → automatic due update → sidecar reports new version. Exercise native/custom background/manual with approved configured providers; no fallback.
- [ ] Tamper/incompatible/unavailable/interrupted feed exercises retain working knowledge; restart and old offline timestamps remain valid. Do not attach prompts, credentials or private workspace evidence. Record other-platform evidence gaps explicitly; use existing CI/platform release evidence where available.
- [ ] Document promotion/review hash changes, publisher ownership, secret setup/backup/rotation, archive recovery, cache fallback and eligibility diagnostics. Attach reproducible evidence to #529/source PR/app PR.
- [ ] Before terminal PR state, tie off matching Taskmaster recommendation via sidecar accept/complete only after verified implementation; include packetMutation if present and recommendation ID in PR.
- [ ] Babysit each owned PR under repository budget, resolve review feedback, merge only after required checks/current code-review gate. Close #529 only with complete evidence; guarded finish-pr cleanup for our worktree.

## Mandatory requirement coverage

| #529 criterion | Delivery | Verification |
| --- | --- | --- |
| 1 strict exclusions | T5 policy/type/path gate | Mistaken allowlist excluded-class fixtures |
| 2 exact reviewed/default-excluded export | T5 hash policy | Unlisted sibling and changed-byte failures |
| 3 contents privacy | T5 preview + T6 owner review | Public artifact review and private-marker scan |
| 4 hierarchy/metadata/index/link safety | T1–2 schema, T5 exporter | Round-trip hierarchy + negative link/meta/asset/traversal fixtures |
| 5 same-pipeline starter/revision | T5 receipt, T7 import | Byte/digest match to actual signed publication |
| 6 signed Pages/pinned key/no credentials | T1–2 verification, T6 signing | Missing-key no mutation; production key agreement and artifact scan |
| 7 automatic immutable compatible feed | T3 selection, T6 archive/workflow | Two eligible publications, old URL bytes, normal due update |
| 8 negative exclusion fixtures | T5 exporter tests | Every forbidden entry route plus final artifact marker scan |
| 9 packaged E2E/fallback | T3 cache, T7 package | Fresh offline/new live signed update/tamper/incompatible/interruption/outage |
| 10 docs/ownership/rotation/evidence | T6–7 guides and issue evidence | Reproducible promotion/rotation/recovery walkthrough |
| 11 signed policy/capability | T1–2 schema, T5 guidance policy | Missing/non-integer/unsupported/mismatch rejection; guidance capability completeness |
| 12 signed starter + attestation | T1–3 proof, T7 resources | Unsigned/standalone/swapped manifest rejection; real route bounds/restart |
| 13 server admission/native/custom races | T4 guards, T7 reopening | Zero-call/no-reservation negatives, positive four-path dispatch, stale-output rejection |
| Extra cache concurrency | T3 retained lock | Two-process old/new/equivocation/crash/contention preservation |

## Reviewing-plans result

Pre-execution review must check this table against the current issue and spec, test interface consistency, scanner/Markdown grammar completeness, and whether owner gates block only dependent operations. The protocol, cache, runtime and publisher are coupled by the same end-to-end pass condition, so this is one plan with separate task review boundaries. No scope is removed to lower complexity.

Fresh-context adversarial review found three actionable gaps, all corrected here: lexical integer parity (`1.0`, `1e0`, `-0`), escaped lone-surrogate parity, and receipts whose source identities were not independently checked against trusted history. A scoped re-review confirmed those corrections and found a fourth actionable gap: historical reproduction had no exporter version. Receipts now pin a stable versioned exporter and dependency lock; tests require old publications to remain reproducible after a newer exporter is introduced. Shared negative fixtures now pin parsing; archive staging verifies original policy/source blobs and regenerated payload bytes. Self-review also corrected a fixture relative path and made retained old-key verification during rotation explicit.

**Scope/simplicity:** Ready for fixture implementation. All thirteen mandatory criteria map to delivery and checks above; use existing update/guard/lock mechanisms. No independent assessment action or provider expansion is included.

**Clarity:** Ready. Numeric/Unicode syntax, compatibility selection, proof identity and native/custom side-effect seams are explicit.

**Verification:** Ready for Tasks 1–5 and Task 6 fixture work. Production publication, starter import and admission reopening remain held for owner content/key evidence and Task 7 packaged checks. Ratings remain 4, 4, 3, 3, Unknown; total **Incomplete**. Final scoped review confirmed the exporter-version fix and found no material new breakage. Cross-model manual review was offered; no extra review was requested, so execution uses the completed fresh-context review findings.
