use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{pkcs8::DecodePublicKey, Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

mod strict_json;

const MAX_ACTIVATION_BYTES: usize = 6 * 1024 * 1024;
const MAX_ENVELOPE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const LOWER_SHA256: &str = "0123456789abcdef";
const PAGE_TYPES: &[&str] = &[
    "concept",
    "principle",
    "practice",
    "playbook",
    "reference",
    "implementation-mapping",
    "index",
];
const APP_PUBLIC_KEY_PEM: &str =
    include_str!("../../../../apps/desktop/resources/knowledge/public-key.pem");

fn deserialize_present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgePage {
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(rename = "type")]
    pub(crate) page_type: String,
    pub(crate) status: String,
    pub(crate) content: String,
    pub(crate) sha256: String,
    pub(crate) related_ids: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_present")]
    pub(crate) parent_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present")]
    pub(crate) applicability: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_present")]
    pub(crate) provenance: Option<Vec<KnowledgeProvenance>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct KnowledgeProvenance {
    pub(crate) title: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnowledgeBundle {
    pub(crate) format_version: u8,
    pub(crate) version: String,
    pub(crate) sequence: u64,
    pub(crate) published_at: String,
    pub(crate) privacy_policy_version: u8,
    pub(crate) capabilities: Vec<String>,
    pub(crate) pages: Vec<KnowledgePage>,
}

#[derive(Clone, Debug)]
pub(crate) struct KnowledgeIdentity {
    pub(crate) bundle_sha256: String,
    pub(crate) format_version: u8,
    pub(crate) privacy_policy_version: u8,
    pub(crate) version: String,
    pub(crate) sequence: u64,
    pub(crate) public_key_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Activation {
    activation_format_version: u8,
    bundle_envelope_base64: String,
    manifest_envelope_base64: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedEnvelope {
    payload: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    pub(crate) format_version: u8,
    bundles: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    format_version: u64,
    privacy_policy_version: u64,
    pub(crate) sequence: u64,
    pub(crate) version: String,
    pub(crate) sha256: String,
    path: String,
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedKnowledge {
    bundle: KnowledgeBundle,
    identity: KnowledgeIdentity,
    activation_bytes: Vec<u8>,
}

impl VerifiedKnowledge {
    pub(crate) fn bundle(&self) -> &KnowledgeBundle {
        &self.bundle
    }

    pub(crate) fn identity(&self) -> &KnowledgeIdentity {
        &self.identity
    }

    pub(crate) fn activation_bytes(&self) -> &[u8] {
        &self.activation_bytes
    }
}

pub(crate) fn verify_activation(bytes: &[u8]) -> Result<VerifiedKnowledge, String> {
    verify_activation_impl(bytes, APP_PUBLIC_KEY_PEM)
}

#[cfg(test)]
fn verify_activation_with_key(
    bytes: &[u8],
    public_key_pem: &str,
) -> Result<VerifiedKnowledge, String> {
    verify_activation_impl(bytes, public_key_pem)
}

fn verify_activation_impl(bytes: &[u8], public_key_pem: &str) -> Result<VerifiedKnowledge, String> {
    if bytes.len() > MAX_ACTIVATION_BYTES {
        return Err("activation exceeds 6 MiB".into());
    }
    let activation: Activation = strict_json::parse(bytes, MAX_ACTIVATION_BYTES)?;
    if activation.activation_format_version != 1 {
        return Err("activationFormatVersion must equal 1".into());
    }
    let bundle_envelope_bytes =
        canonical_base64(&activation.bundle_envelope_base64, "bundle envelope")?;
    let manifest_envelope_bytes =
        canonical_base64(&activation.manifest_envelope_base64, "manifest envelope")?;
    if bundle_envelope_bytes.len() > MAX_ENVELOPE_BYTES
        || manifest_envelope_bytes.len() > MAX_ENVELOPE_BYTES
    {
        return Err("signed envelopes exceed 2 MiB".into());
    }
    let key = parse_public_key(public_key_pem)?;
    let bundle_payload = verify_envelope(&bundle_envelope_bytes, &key)?;
    let bundle: KnowledgeBundle = strict_json::parse(&bundle_payload, MAX_ENVELOPE_BYTES)?;
    validate_bundle(&bundle)?;
    let manifest_payload = verify_envelope(&manifest_envelope_bytes, &key)?;
    let manifest: Manifest = strict_json::parse(&manifest_payload, MAX_ENVELOPE_BYTES)?;
    validate_manifest(&manifest)?;

    let digest = sha256_hex(&bundle_envelope_bytes);
    let expected_path = format!("bundles/{}.json", bundle.version);
    let matching = manifest
        .bundles
        .iter()
        .filter(|entry| {
            entry.format_version == u64::from(bundle.format_version)
                && entry.privacy_policy_version == u64::from(bundle.privacy_policy_version)
                && entry.sequence == bundle.sequence
                && entry.version == bundle.version
                && entry.sha256 == digest
                && entry.path == expected_path
        })
        .count();
    if matching != 1 {
        return Err("signed manifest must contain exactly one entry for the activated bundle identity and envelope digest".into());
    }
    let (_, public_key_der) = decode_public_key_pem(public_key_pem)?;
    let identity = KnowledgeIdentity {
        bundle_sha256: digest,
        format_version: bundle.format_version,
        privacy_policy_version: bundle.privacy_policy_version,
        version: bundle.version.clone(),
        sequence: bundle.sequence,
        public_key_sha256: sha256_hex(&public_key_der),
    };
    Ok(VerifiedKnowledge {
        bundle,
        identity,
        activation_bytes: bytes.to_vec(),
    })
}

#[cfg(test)]
fn verify_manifest_with_key(bytes: &[u8], public_key_pem: &str) -> Result<Manifest, String> {
    let key = parse_public_key(public_key_pem)?;
    let payload = verify_envelope(bytes, &key)?;
    let manifest: Manifest = strict_json::parse(&payload, MAX_ENVELOPE_BYTES)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn verify_envelope(bytes: &[u8], key: &VerifyingKey) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err("signed envelope exceeds 2 MiB".into());
    }
    let envelope: SignedEnvelope = strict_json::parse(bytes, MAX_ENVELOPE_BYTES)?;
    let signature_bytes = canonical_base64(&envelope.signature, "signature")?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| "Ed25519 signature must be 64 bytes")?;
    let payload = envelope.payload.into_bytes();
    key.verify_strict(&payload, &signature)
        .map_err(|_| "signature verification failed")?;
    Ok(payload)
}

fn parse_public_key(pem: &str) -> Result<VerifyingKey, String> {
    let (key, _) = decode_public_key_pem(pem)?;
    if key.is_weak() {
        return Err("trusted public key must not be weak".into());
    }
    Ok(key)
}

fn decode_public_key_pem(pem: &str) -> Result<(VerifyingKey, Vec<u8>), String> {
    let der = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    if der.is_empty() {
        return Err("trusted public key PEM is invalid".into());
    }
    let der = canonical_base64(&der, "trusted public key")?;
    let key = VerifyingKey::from_public_key_der(&der)
        .map_err(|_| "trusted public key must be Ed25519 SPKI")?;
    Ok((key, der))
}

fn canonical_base64(value: &str, label: &str) -> Result<Vec<u8>, String> {
    let decoded = STANDARD
        .decode(value)
        .map_err(|_| format!("{label} must use canonical padded standard base64"))?;
    if STANDARD.encode(&decoded) != value {
        return Err(format!("{label} must use canonical padded standard base64"));
    }
    Ok(decoded)
}

fn validate_bundle(bundle: &KnowledgeBundle) -> Result<(), String> {
    if bundle.format_version != 1 || bundle.privacy_policy_version != 1 {
        return Err("bundle format and privacy policy versions must equal 1".into());
    }
    safe_identifier(&bundle.version, "bundle.version")?;
    if bundle.sequence > MAX_SAFE_INTEGER {
        return Err("bundle.sequence exceeds safe integer range".into());
    }
    if !canonical_timestamp(&bundle.published_at) {
        return Err(
            "bundle.publishedAt must be a canonical UTC ISO timestamp with milliseconds".into(),
        );
    }
    validate_unique_strings(&bundle.capabilities, "bundle.capabilities", 16, 128, true)?;
    if bundle.pages.is_empty() || bundle.pages.len() > 256 {
        return Err("bundle.pages must contain 1 to 256 pages".into());
    }
    let mut pages = HashMap::new();
    for page in &bundle.pages {
        page_id(&page.id)?;
        bounded_string(&page.title, "page.title", 1, 512)?;
        bounded_string(&page.page_type, "page.type", 1, 128)?;
        if !PAGE_TYPES.contains(&page.page_type.as_str()) {
            return Err("page.type is unsupported".into());
        }
        bounded_string(&page.status, "page.status", 1, 128)?;
        if page.content.len() > 64 * 1024 {
            return Err("page.content exceeds 64 KiB".into());
        }
        if !is_lower_sha256(&page.sha256) || page.sha256 != sha256_hex(page.content.as_bytes()) {
            return Err("page.sha256 must match exact UTF-8 content bytes".into());
        }
        validate_unique_strings(&page.related_ids, "page.relatedIds", 256, 256, false)?;
        for id in &page.related_ids {
            page_id(id)?;
        }
        if let Some(parent) = &page.parent_id {
            page_id(parent)?;
        }
        if let Some(applicability) = &page.applicability {
            validate_unique_strings(applicability, "page.applicability", 32, 128, false)?;
        }
        if let Some(provenance) = &page.provenance {
            if provenance.len() > 16 {
                return Err("page.provenance must contain at most 16 records".into());
            }
            for item in provenance {
                bounded_string(&item.title, "provenance.title", 1, 512)?;
                bounded_string(&item.url, "provenance.url", 1, 2048)?;
                validate_url(&item.url)?;
            }
        }
        if pages.insert(page.id.as_str(), page).is_some() {
            return Err("page IDs must be unique".into());
        }
    }
    for page in &bundle.pages {
        for related in &page.related_ids {
            if !pages.contains_key(related.as_str()) {
                return Err("related page is missing".into());
            }
        }
        if let Some(parent) = &page.parent_id {
            if parent == &page.id || !pages.contains_key(parent.as_str()) {
                return Err("page has invalid parent".into());
            }
        }
    }
    for page in &bundle.pages {
        let mut visited = HashSet::new();
        let mut parent = page.parent_id.as_deref();
        while let Some(id) = parent {
            if !visited.insert(id) {
                return Err("page parent relationships contain a cycle".into());
            }
            parent = pages.get(id).and_then(|page| page.parent_id.as_deref());
        }
    }
    Ok(())
}

fn select_manifest_entry(manifest: &Manifest) -> Result<&ManifestEntry, String> {
    manifest
        .bundles
        .iter()
        .filter(|entry| entry.format_version == 1 && entry.privacy_policy_version == 1)
        .max_by_key(|entry| entry.sequence)
        .ok_or_else(|| "manifest has no supported format and policy entry".into())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.format_version != 1 {
        return Err("manifest.formatVersion must equal 1".into());
    }
    if manifest.bundles.is_empty() || manifest.bundles.len() > 1000 {
        return Err("manifest.bundles must contain 1 to 1,000 entries".into());
    }
    let mut tuples = HashSet::new();
    let mut versions = HashSet::new();
    let mut paths = HashSet::new();
    for entry in &manifest.bundles {
        if entry.format_version == 0
            || entry.format_version > MAX_SAFE_INTEGER
            || entry.privacy_policy_version == 0
            || entry.privacy_policy_version > MAX_SAFE_INTEGER
        {
            return Err(
                "manifest format and privacy policy versions must be positive safe integers".into(),
            );
        }
        if entry.sequence > MAX_SAFE_INTEGER {
            return Err("manifest sequence exceeds safe integer range".into());
        }
        safe_identifier(&entry.version, "manifest.version")?;
        if !is_lower_sha256(&entry.sha256) {
            return Err("manifest entry sha256 must be lowercase hexadecimal".into());
        }
        let expected = format!("bundles/{}.json", entry.version);
        if entry.path != expected {
            return Err("manifest entry path must match its safe version".into());
        }
        if !tuples.insert((
            entry.format_version,
            entry.privacy_policy_version,
            entry.sequence,
        )) {
            return Err("manifest has duplicate format/policy/sequence entries".into());
        }
        if !versions.insert(&entry.version) {
            return Err("manifest has duplicate versions".into());
        }
        if !paths.insert(&entry.path) {
            return Err("manifest has duplicate paths".into());
        }
    }
    Ok(())
}

fn validate_unique_strings(
    items: &[String],
    label: &str,
    max_items: usize,
    max_bytes: usize,
    identifier: bool,
) -> Result<(), String> {
    if items.len() > max_items {
        return Err(format!("{label} exceeds {max_items} items"));
    }
    let mut unique = HashSet::new();
    for item in items {
        bounded_string(item, label, 1, max_bytes)?;
        if identifier {
            safe_identifier(item, label)?;
        }
        if !unique.insert(item) {
            return Err(format!("{label} entries must be unique"));
        }
    }
    Ok(())
}

fn bounded_string(value: &str, label: &str, min: usize, max: usize) -> Result<(), String> {
    if value.len() < min || value.len() > max {
        return Err(format!(
            "{label} must be between {min} and {max} UTF-8 bytes"
        ));
    }
    Ok(())
}

fn safe_identifier(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(format!("{label} must be a safe identifier"));
    }
    Ok(())
}

fn page_id(value: &str) -> Result<(), String> {
    if value.len() > 256 || !value.ends_with(".md") {
        return Err("page ID must match safe relative Markdown path grammar".into());
    }
    let body = &value[..value.len() - 3];
    for segment in body.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.starts_with('.')
            || !segment.as_bytes()[0].is_ascii_alphanumeric() && segment.as_bytes()[0] != b'_'
            || !segment
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err("page ID must match safe relative Markdown path grammar".into());
        }
    }
    Ok(())
}

fn validate_url(value: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 2048
        || !bytes.iter().all(|byte| (0x21..=0x7e).contains(byte))
    {
        return Err("provenance URL must contain 1 to 2048 printable ASCII bytes".into());
    }
    if !value.starts_with("https://") || value.contains('\\') || value.contains('#') {
        return Err(
            "provenance URL must use canonical HTTPS syntax without backslashes or fragments"
                .into(),
        );
    }
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let escape = bytes
                .get(index + 1..index + 3)
                .ok_or("provenance URL has an incomplete percent escape")?;
            if !escape
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(byte))
            {
                return Err("provenance URL percent escapes must use uppercase hexadecimal".into());
            }
            index += 2;
        }
        index += 1;
    }
    let parsed = url::Url::parse(value).map_err(|_| "provenance URL must be a valid HTTPS URL")?;
    let authority = value["https://".len()..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if parsed.scheme() != "https"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || authority.contains('@')
        || parsed.as_str() != value
    {
        return Err(
            "provenance URL must be a canonical HTTPS URL without credentials or fragments".into(),
        );
    }
    Ok(())
}

fn canonical_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 24
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'.'
        || b[23] != b'Z'
    {
        return false;
    }
    for (start, end) in [
        (0, 4),
        (5, 7),
        (8, 10),
        (11, 13),
        (14, 16),
        (17, 19),
        (20, 23),
    ] {
        if !b[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let num = |range: std::ops::Range<usize>| value[range].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        num(0..4),
        num(5..7),
        num(8..10),
        num(11..13),
        num(14..16),
        num(17..19),
    ) else {
        return false;
    };
    if month == 0 || month > 12 || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day >= 1 && day <= days
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| LOWER_SHA256.as_bytes().contains(&c))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{verify_activation, verify_activation_with_key, verify_manifest_with_key};
    use base64::Engine as _;
    use serde::Deserialize;

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/taskmaster-knowledge/"
    );
    const FIXTURE_INDEX: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/taskmaster-knowledge/fixture-index.json"
    );
    const TEST_PUBLIC_KEY: &str =
        include_str!("../../../../tests/fixtures/taskmaster-knowledge/test-public-key.pem");

    #[derive(Deserialize)]
    struct FixtureIndex {
        groups: Vec<FixtureGroup>,
    }
    #[derive(Deserialize)]
    struct FixtureGroup {
        api: String,
        expected: String,
        files: Vec<String>,
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{FIXTURES}{name}")).expect("fixture exists")
    }

    fn indexed(api: &str, expected: &str) -> Vec<String> {
        let index: FixtureIndex = serde_json::from_str(
            &std::fs::read_to_string(FIXTURE_INDEX).expect("fixture index exists"),
        )
        .expect("fixture index is valid JSON");
        index
            .groups
            .into_iter()
            .filter(|group| group.api.contains(api) && group.expected == expected)
            .flat_map(|group| group.files)
            .collect()
    }

    #[test]
    fn shared_activation_corpus_matches_expected_outcomes() {
        for (expected, should_accept) in [("accept", true), ("reject", false)] {
            for name in indexed("verifyKnowledgeActivation", expected) {
                if name == "maximum-bundle-envelope.json"
                    || name == "maximum-manifest-envelope.json"
                {
                    continue;
                }
                let result = verify_activation_with_key(&fixture(&name), TEST_PUBLIC_KEY);
                assert_eq!(result.is_ok(), should_accept, "fixture {name}: {result:?}");
            }
        }
    }

    #[test]
    fn trusted_key_parser_rejects_non_ed25519_and_low_order_keys() {
        let mut ed25519_der = base64::engine::general_purpose::STANDARD
            .decode("MCowBQYDK2VwAyEA9P0SXFFvY2JxiNW0uW7IqAMierKBzhp5XqshXhZDDVg=")
            .unwrap();
        ed25519_der[8] = 0x6e;
        let x25519_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64::engine::general_purpose::STANDARD.encode(ed25519_der)
        );
        assert!(super::parse_public_key(&x25519_pem).is_err());

        let mut weak_der = base64::engine::general_purpose::STANDARD
            .decode("MCowBQYDK2VwAyEA9P0SXFFvY2JxiNW0uW7IqAMierKBzhp5XqshXhZDDVg=")
            .unwrap();
        weak_der[10..].fill(0);
        let weak_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64::engine::general_purpose::STANDARD.encode(weak_der)
        );
        assert!(super::parse_public_key(&weak_pem).is_err());
    }

    #[test]
    fn verified_identity_binds_exact_envelope_and_pinned_spki() {
        let bytes = fixture("valid-activation.json");
        let verified = verify_activation_with_key(&bytes, TEST_PUBLIC_KEY).unwrap();
        assert_eq!(
            verified.identity.bundle_sha256,
            std::fs::read_to_string(format!("{FIXTURES}valid-envelope.sha256"))
                .unwrap()
                .trim()
        );
        assert_eq!(
            verified.identity.public_key_sha256,
            std::fs::read_to_string(format!("{FIXTURES}test-public-key-spki.sha256"))
                .unwrap()
                .trim()
        );
        assert_eq!(verified.identity.version, "synthetic-1");
        assert_eq!(verified.identity.sequence, 1);
        assert_eq!(verified.activation_bytes(), bytes);
    }

    #[test]
    fn production_entrypoint_rejects_every_test_key_activation() {
        for name in indexed("verifyKnowledgeActivation", "accept") {
            if name == "maximum-bundle-envelope.json" || name == "maximum-manifest-envelope.json" {
                continue;
            }
            let result = verify_activation(&fixture(&name));
            assert!(
                result.is_err(),
                "production key accepted test fixture {name}"
            );
        }
    }

    #[test]
    fn manifest_corpus_matches_expected_outcomes() {
        for (expected, should_accept) in [("accept", true), ("reject", false)] {
            for name in indexed("verifyKnowledgeManifest", expected) {
                let result = verify_manifest_with_key(&fixture(&name), TEST_PUBLIC_KEY);
                assert_eq!(
                    result.is_ok(),
                    should_accept,
                    "manifest fixture {name}: {result:?}"
                );
            }
        }
    }

    #[test]
    fn manifest_selection_filters_policy_before_sequence_ranking() {
        let manifest =
            verify_manifest_with_key(&fixture("ranked-manifest-envelope.json"), TEST_PUBLIC_KEY)
                .unwrap();
        assert_eq!(
            super::select_manifest_entry(&manifest).unwrap().version,
            "supported-newer"
        );
    }

    #[test]
    fn strict_json_matches_shared_lexical_rules() {
        for json in [
            br#"{"a":1,"\u0061":2}"#.as_slice(),
            br#"{"n":9007199254740992}"#,
            br#"{"n":1.0}"#,
            br#"{"n":1e0}"#,
            br#"{"n":-0}"#,
            br#"{"text":"\ud800"}"#,
        ] {
            assert!(
                super::strict_json::parse::<serde_json::Value>(json, 1024).is_err(),
                "accepted {json:?}"
            );
        }
        assert!(super::strict_json::parse::<serde_json::Value>(
            &[0xef, 0xbb, 0xbf, b'{', b'}'],
            1024
        )
        .is_err());
        assert!(super::strict_json::parse::<serde_json::Value>(
            br#"{"text":"\uD83D\uDE00"}"#,
            1024
        )
        .is_ok());
        assert!(super::strict_json::parse::<serde_json::Value>(&[0xff], 1024).is_err());
        assert!(super::strict_json::parse::<serde_json::Value>(b"{} {}", 1024).is_err());
        assert!(super::strict_json::parse::<serde_json::Value>(b"{}", 1).is_err());
    }

    #[test]
    fn generated_activation_wrapper_obeys_combined_size_bounds() {
        let max_bundle = fixture("maximum-bundle-envelope.json");
        let max_manifest = fixture("maximum-manifest-envelope.json");
        assert_eq!(max_bundle.len(), 2 * 1024 * 1024);
        assert_eq!(max_manifest.len(), 2 * 1024 * 1024);
        let activation = format!("{{\"activationFormatVersion\":1,\"bundleEnvelopeBase64\":\"{}\",\"manifestEnvelopeBase64\":\"{}\"}}",
            base64::engine::general_purpose::STANDARD.encode(max_bundle),
            base64::engine::general_purpose::STANDARD.encode(max_manifest));
        let verified = verify_activation_with_key(activation.as_bytes(), TEST_PUBLIC_KEY)
            .expect("two maximum envelopes fit activation bound");
        assert_eq!(verified.bundle.version, "maximum-envelope");

        let valid = fixture("valid-activation.json");
        let mut activation_overflow = valid[..valid.len() - 1].to_vec();
        activation_overflow.extend(std::iter::repeat(b' ').take(6 * 1024 * 1024));
        activation_overflow.push(b'}');
        assert!(verify_activation_with_key(&activation_overflow, TEST_PUBLIC_KEY).is_err());

        let oversized_envelope = vec![b'x'; 2 * 1024 * 1024 + 1];
        let valid_manifest_b64 = serde_json::from_slice::<serde_json::Value>(&valid).unwrap()
            ["manifestEnvelopeBase64"]
            .as_str()
            .unwrap()
            .to_owned();
        let activation = serde_json::json!({
            "activationFormatVersion": 1,
            "bundleEnvelopeBase64": base64::engine::general_purpose::STANDARD.encode(oversized_envelope),
            "manifestEnvelopeBase64": valid_manifest_b64,
        }).to_string();
        assert!(verify_activation_with_key(activation.as_bytes(), TEST_PUBLIC_KEY).is_err());
    }

    #[test]
    fn provenance_urls_reject_empty_fragments_and_allow_encoded_hashes() {
        assert!(super::validate_url("https://example.test/source#").is_err());
        assert!(super::validate_url("https://example.test/source%23section").is_ok());
        assert!(
            super::validate_url("https://example.test/source@user?next=@other&escaped=%40").is_ok()
        );
        assert!(super::validate_url("https://@example.test/source").is_err());
        assert!(super::validate_url("HTTPS://@example.test/source").is_err());
    }

    #[test]
    fn timestamp_rules_reject_normalized_and_noncanonical_dates() {
        for timestamp in [
            "0000-02-29T00:00:00.000Z",
            "2000-02-29T12:34:56.789Z",
            "2024-02-29T23:59:59.999Z",
            "9999-12-31T23:59:59.999Z",
        ] {
            assert!(
                super::canonical_timestamp(timestamp),
                "rejected canonical timestamp {timestamp}"
            );
        }
        for timestamp in [
            "2024-02-30T00:00:00.000Z",
            "1900-02-29T00:00:00.000Z",
            "2024-00-01T00:00:00.000Z",
            "2024-13-01T00:00:00.000Z",
            "2024-01-00T00:00:00.000Z",
            "2024-01-32T00:00:00.000Z",
            "2024-01-01T24:00:00.000Z",
            "2024-01-01T00:60:00.000Z",
            "2024-01-01T00:00:60.000Z",
            "2024-01-01t00:00:00.000z",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00.0000Z",
            "2024-01-01T00:00:00.000+00:00",
            "+010000-02-29T00:00:00.000Z",
            "2024-01-01 00:00:00.000Z",
        ] {
            assert!(
                !super::canonical_timestamp(timestamp),
                "accepted invalid timestamp {timestamp}"
            );
        }
    }
}
