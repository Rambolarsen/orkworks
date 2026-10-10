# Signed Taskmaster knowledge activation proof

- Status: accepted
- Deciders: repository owner
- Date: 2026-10-10

## Context

Taskmaster knowledge is reference data and must remain ineligible for Brain-backed inference unless the application can prove that a privacy-policy approved bundle and its signed compatibility attestation came from the trusted publisher. A decoded bundle, app packaging, a cache, or a renderer request alone does not provide that proof. Electron and the Rust sidecar must independently verify the same bounded wire record before any later runtime admission work can use it. This decision complements [ADR 0054](0054-taskmaster-honors-managed-cli-policy.md) and does not change the managed CLI policy it records.

## Decision

Represent activation as exactly three JSON fields: integer `activationFormatVersion: 1`, `bundleEnvelopeBase64`, and `manifestEnvelopeBase64`. Each field contains canonical padded standard base64 of the original signed-envelope response bytes. An envelope is at most 2 MiB; the complete serialized activation is at most 6 MiB. Envelopes contain exactly `payload` and `signature`, use canonical padded standard base64, and carry a 64-byte Ed25519 signature over the exact UTF-8 payload bytes. Verify with a trusted application key supplied by the Electron construction boundary; no renderer, downloaded payload, or activation record can select that key.

Parse JSON as fatal UTF-8 with a 32-level maximum depth. Reject duplicate keys
after JSON string escape decoding, unpaired surrogate escapes, non-canonical
numeric token forms, and values above JavaScript's safe integer maximum. Apply
the same lexical rules to unknown fields. Activation and envelope objects
reject unknown fields.

Bundles contain 1–256 pages, at most 16 unique safe capability identifiers,
and signed `privacyPolicyVersion: 1`. Page content is at most 64 KiB of UTF-8;
titles are 1–512 bytes; type and status are 1–128 bytes. Page type is one of
`concept`, `principle`, `practice`, `playbook`, `reference`,
`implementation-mapping`, or `index`. Each page has at most 256
unique related IDs, 32 unique applicability strings, and 16 provenance records.
Provenance titles are at most 512 bytes and HTTPS URLs at most 2,048 bytes,
without credentials or fragments. Page IDs are unique, lowercase-`.md` relative
paths matching
`^(?:[A-Za-z0-9_][A-Za-z0-9._-]*/)*[A-Za-z0-9_][A-Za-z0-9._-]*\.md$`, with at
most 256 ASCII bytes. Parents and related IDs must resolve within the bundle;
parent cycles are invalid.

Each manifest entry carries positive safe-integer format and policy values and
a nonnegative safe-integer sequence. The bundle sequence uses the same range,
including zero. Each entry also carries a safe version and a path exactly
`bundles/<version>.json`. The signed
manifest binds the original bundle-envelope bytes with lowercase SHA-256 and
has at most 1,000 entries. Bundle and manifest policy values must match. Select
format 1 and policy 1 entries before sequence ranking; require exactly one
entry to attest the activated bundle. Ignore other structurally valid
compatibility classes rather than allowing them to displace an eligible entry.
A same-bundle attestation from another valid manifest is idempotent; different
content at an equal sequence is equivocation.

The verified value is immutable and carries full bundle identity: envelope digest, format, signed policy, version, sequence, and SHA-256 fingerprint of the pinned public key's SubjectPublicKeyInfo. Manifest bytes remain retained as proof but do not form bundle identity. Knowledge values never grant executable capability, permission policy, or authority over repository instructions. There is no unsigned-JSON or app-packaging trust bypass and no independent wall-clock age cutoff for eligible cached knowledge.

Activation proof persistence is additive. Legacy plain starter, cache, and sidecar records remain available for diagnosis but are not eligible and cannot set a sequence floor that blocks recovery to a valid signed starter. Durable replacement, cache concurrency, sidecar revalidation, and analysis admission remain separately implemented behind the contracts in the accepted knowledge specification. Until the full approved export, signed starter, publication, and runtime verification gates are met, Brain-backed analysis remains closed.

## Consequences

Electron can verify feed, cache, and starter packets through one proof API, and the sidecar can independently apply the same wire contract. Test-only signing keys and synthetic pages prove protocol behavior but never confer production eligibility. Production key provisioning, reviewed public content, publication, and opening the analysis gate remain explicit owner-controlled prerequisites.

This protocol preserves the independent Taskmaster selection and the managed CLI behavior in ADR 0054. Deterministic observation recommendations continue while Brain-backed analysis is unavailable.
