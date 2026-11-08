//! Dormant supplied-byte retirement snapshot codec. Hashes do not establish
//! provenance authenticity, retirement state, selection eligibility or authority.
#![allow(dead_code)]
use super::root_bootstrap::{source::native_path, wire::*, RootBootstrapError};
use serde_json::Value;

const PAYLOAD_BYTES: usize = 1024 * 1024;
const CONTENT_BYTES: usize = 256 * 1024;
const DOMAIN: &[u8] = b"orkworks.retirement-artifact-snapshot.v1\n";
type Result<T> = std::result::Result<T, RootBootstrapError>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetirementArtifactError(pub &'static str);

/// Catalog identity is independent of the digest and changing source revision.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RetirementArtifactKey {
    RoleTemplate {
        source_identity: String,
        artifact_id: String,
        version: u32,
    },
    Skill {
        source_identity: String,
        artifact_id: String,
        version: String,
    },
}
impl RetirementArtifactKey {
    pub(crate) fn source_identity(&self) -> &str {
        match self {
            Self::RoleTemplate {
                source_identity, ..
            }
            | Self::Skill {
                source_identity, ..
            } => source_identity,
        }
    }
    pub(crate) fn artifact_id(&self) -> &str {
        match self {
            Self::RoleTemplate { artifact_id, .. } | Self::Skill { artifact_id, .. } => artifact_id,
        }
    }
}
#[derive(Debug)]
pub(crate) struct ValidatedRetirementArtifact {
    canonical_bytes: Vec<u8>,
    snapshot_digest: String,
    key: RetirementArtifactKey,
}
impl ValidatedRetirementArtifact {
    pub(crate) fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub(crate) fn snapshot_digest(&self) -> &str {
        &self.snapshot_digest
    }
    pub(crate) fn key(&self) -> &RetirementArtifactKey {
        &self.key
    }
}

pub(crate) fn validate_retirement_artifact_v1(
    bytes: &[u8],
) -> std::result::Result<ValidatedRetirementArtifact, RetirementArtifactError> {
    validate(bytes).map_err(|e| RetirementArtifactError(e.0))
}
fn provenance(x: &Value) -> Result<&str> {
    tags(&x["kind"], &["local", "nonlocal"])?;
    exact(
        x,
        if x["kind"] == "local" {
            "schemaVersion kind sourceKind sourceIdentity sourceRevision sourcePath"
        } else {
            "schemaVersion kind sourceKind sourceIdentity sourceRevision"
        },
    )?;
    number(&x["schemaVersion"], 1, 1)?;
    tags(
        &x["sourceKind"],
        &["repository", "bundled", "user-approved"],
    )?;
    let identity = text(&x["sourceIdentity"], 512, 1)?;
    text(&x["sourceRevision"], 512, 1)?;
    if x["kind"] == "local" {
        native_path(&x["sourcePath"], 2048)?;
    } else {
        ensure(x["sourceKind"] != "repository", "repository provenance")?;
    }
    Ok(identity)
}
fn content(x: &Value, field: &str, digest_field: &str, total: &mut usize) -> Result<()> {
    let content = text(&x[field], CONTENT_BYTES, 1)?;
    *total = total
        .checked_add(content.len())
        .filter(|n| *n <= CONTENT_BYTES)
        .ok_or(RootBootstrapError("aggregate content bytes"))?;
    ensure(hash(content) == digest(&x[digest_field])?, "content digest")
}
fn validate(bytes: &[u8]) -> Result<ValidatedRetirementArtifact> {
    // decode checks the byte cap before parsing; roundtrip equality also rejects
    // duplicate keys, alternate escapes, floats and noncanonical object order.
    let x = decode(bytes, PAYLOAD_BYTES)?;
    exact(&x, "artifactKind snapshot resources")?;
    tags(&x["artifactKind"], &["role-template", "skill"])?;
    let snapshot = &x["snapshot"];
    let resources = list(&x["resources"], 32, 0)?;
    let mut total = 0;
    let key = if x["artifactKind"] == "role-template" {
        exact(
            snapshot,
            "id version role instructions instructionsDigest contentDigest provenance",
        )?;
        let artifact_id = id(&snapshot["id"])?;
        let version = number(&snapshot["version"], 1, i32::MAX as u64)? as u32;
        tags(
            &snapshot["role"],
            &[
                "orchestrator",
                "research",
                "implementation",
                "review",
                "verification",
                "remediation",
            ],
        )?;
        ensure(resources.is_empty(), "role resources")?;
        content(snapshot, "instructions", "instructionsDigest", &mut total)?;
        let source_identity = provenance(&snapshot["provenance"])?;
        let mut body = snapshot.clone();
        body.as_object_mut()
            .ok_or(RootBootstrapError("closed fields"))?
            .remove("contentDigest");
        ensure(
            raw(&body)? == digest(&snapshot["contentDigest"])?,
            "role descriptor digest",
        )?;
        RetirementArtifactKey::RoleTemplate {
            source_identity: source_identity.into(),
            artifact_id: artifact_id.into(),
            version,
        }
    } else {
        exact(
            snapshot,
            "id version content contentDigest resourceIds provenance",
        )?;
        let artifact_id = id(&snapshot["id"])?;
        let version = text(&snapshot["version"], 128, 1)?;
        let source_identity = provenance(&snapshot["provenance"])?;
        content(snapshot, "content", "contentDigest", &mut total)?;
        let ids = list(&snapshot["resourceIds"], 32, 0)?;
        sorted(ids, None)?;
        for rid in ids {
            id(rid)?;
        }
        sorted(resources, Some("id"))?;
        ensure(ids.len() == resources.len(), "resource closure")?;
        for (rid, resource) in ids.iter().zip(resources) {
            exact(
                resource,
                "id skillId sourceReference content contentDigest provenance",
            )?;
            id(&resource["id"])?;
            ensure(
                resource["id"] == *rid && id(&resource["skillId"])? == artifact_id,
                "resource closure",
            )?;
            text(&resource["sourceReference"], 2048, 1)?;
            provenance(&resource["provenance"])?;
            content(resource, "content", "contentDigest", &mut total)?;
        }
        RetirementArtifactKey::Skill {
            source_identity: source_identity.into(),
            artifact_id: artifact_id.into(),
            version: version.into(),
        }
    };
    let mut digest_input = Vec::with_capacity(DOMAIN.len() + bytes.len());
    digest_input.extend_from_slice(DOMAIN);
    digest_input.extend_from_slice(bytes);
    Ok(ValidatedRetirementArtifact {
        canonical_bytes: bytes.to_vec(),
        snapshot_digest: hash(digest_input),
        key,
    })
}
