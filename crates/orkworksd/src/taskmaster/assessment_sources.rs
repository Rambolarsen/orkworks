//! Immutable source data. Parsing or validation never grants publication authority.
#![cfg_attr(not(test), allow(dead_code))]
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const MAX_SOURCES: usize = 128;
pub(crate) const MAX_SOURCE_BYTES: usize = 16 * 1024;
const INPUT_DOMAIN: &[u8] = b"orkworks/sql-assessment-inputs/v1\0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceError {
    Invalid(&'static str),
    TooLarge,
}
impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "assessment sources: {message}"),
            Self::TooLarge => write!(f, "assessment sources exceed their bound"),
        }
    }
}
impl std::error::Error for SourceError {}
type Result<T> = std::result::Result<T, SourceError>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceCreation {
    session_id: String,
    created_at: String,
}
impl SourceCreation {
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SqlAssessmentSources {
    schema_version: u8,
    database_epoch: String,
    workspace_sha256: String,
    assessment_id: String,
    input_dependencies_sha256: String,
    sources: Vec<SourceCreation>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Wire {
    schema_version: u8,
    database_epoch: String,
    workspace_sha256: String,
    assessment_id: String,
    input_dependencies_sha256: String,
    sources: Vec<SourceCreation>,
}
impl<'de> Deserialize<'de> for SqlAssessmentSources {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let wire = Wire::deserialize(d)?;
        let value = Self {
            schema_version: wire.schema_version,
            database_epoch: wire.database_epoch,
            workspace_sha256: wire.workspace_sha256,
            assessment_id: wire.assessment_id,
            input_dependencies_sha256: wire.input_dependencies_sha256,
            sources: wire.sources,
        };
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}
impl SqlAssessmentSources {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(SourceError::Invalid("unsupported schema"));
        }
        if self.database_epoch.len() != 36
            || uuid::Uuid::parse_str(&self.database_epoch)
                .map_err(|_| SourceError::Invalid("invalid epoch"))?
                .to_string()
                != self.database_epoch
        {
            return Err(SourceError::Invalid("noncanonical epoch"));
        }
        if !hex_digest(&self.workspace_sha256)
            || !hex_digest(&self.input_dependencies_sha256)
            || !valid_id(&self.assessment_id)
        {
            return Err(SourceError::Invalid("invalid identity"));
        }
        validate_pairs(&self.sources)?;
        if serde_json::to_vec(self)
            .map_err(|_| SourceError::Invalid("encoding failed"))?
            .len()
            > MAX_SOURCE_BYTES
        {
            return Err(SourceError::TooLarge);
        }
        Ok(())
    }
    pub(crate) fn database_epoch(&self) -> &str {
        &self.database_epoch
    }
    pub(crate) fn workspace_sha256(&self) -> &str {
        &self.workspace_sha256
    }
    pub(crate) fn assessment_id(&self) -> &str {
        &self.assessment_id
    }
    pub(crate) fn input_digest(&self) -> &str {
        &self.input_dependencies_sha256
    }
    /// New SQL input identity domain, data only; existing legacy keys are unchanged.
    pub(crate) fn input_fingerprint(&self, inputs: &serde_json::Value) -> Result<String> {
        self.validate()?;
        let material = canonical(serde_json::json!({"inputs": inputs, "sqlSources": self}));
        let bytes = serde_json::to_vec(&material)
            .map_err(|_| SourceError::Invalid("invalid input identity"))?;
        let mut digest = Sha256::new();
        digest.update(b"orkworks/sql-assessment-fingerprint/v1\0");
        digest.update(bytes);
        Ok(hex::encode(digest.finalize()))
    }
    pub(crate) fn sources(&self) -> &[SourceCreation] {
        &self.sources
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn validate_pairs(pairs: &[SourceCreation]) -> Result<()> {
    if pairs.len() > MAX_SOURCES {
        return Err(SourceError::TooLarge);
    }
    let mut last = None;
    for pair in pairs {
        if !valid_id(&pair.session_id) || pair.created_at.is_empty() || pair.created_at.len() > 256
        {
            return Err(SourceError::Invalid("invalid source creation"));
        }
        if last.is_some_and(|id: &str| id >= pair.session_id.as_str()) {
            return Err(SourceError::Invalid("sources are not sorted unique"));
        }
        last = Some(pair.session_id.as_str());
    }
    Ok(())
}

/// No production constructor or Deserialize implementation. A stored packet
/// cannot promote itself into a newly captured origin claim.
#[derive(Serialize)]
pub(crate) struct OriginalInputs {
    packet: SqlAssessmentSources,
}
impl OriginalInputs {
    pub(crate) fn sources(&self) -> &SqlAssessmentSources {
        &self.packet
    }
    pub(crate) fn into_sources(self) -> SqlAssessmentSources {
        self.packet
    }

    /// Trusted copied-fixture origins only, never a legacy input upgrade.
    #[cfg(test)]
    pub(crate) fn fixture(assessment_id: &str, envelope: serde_json::Value) -> Result<Self> {
        let envelope: OriginEnvelope = serde_json::from_value(envelope)
            .map_err(|_| SourceError::Invalid("invalid origin envelope"))?;
        Ok(Self {
            packet: source_data_from_origins(assessment_id, envelope)?,
        })
    }
}
fn source_data_from_origins(
    assessment_id: &str,
    envelope: OriginEnvelope,
) -> Result<SqlAssessmentSources> {
    if envelope.schema_version != 1 {
        return Err(SourceError::Invalid("unsupported origin schema"));
    }
    let mut pairs = BTreeMap::new();
    for input in envelope
        .observations
        .iter()
        .chain(&envelope.recommendations)
        .chain(envelope.readiness_focus.iter())
    {
        validate_pairs(&input.sources)?;
        for pair in &input.sources {
            if pairs
                .insert(pair.session_id.clone(), pair.created_at.clone())
                .is_some_and(|created| created != pair.created_at)
            {
                return Err(SourceError::Invalid("conflicting source creations"));
            }
        }
    }
    let mut packet = SqlAssessmentSources {
        schema_version: 1,
        database_epoch: envelope.database_epoch.clone(),
        workspace_sha256: envelope.workspace_sha256.clone(),
        assessment_id: assessment_id.into(),
        input_dependencies_sha256: "0".repeat(64),
        sources: pairs
            .into_iter()
            .map(|(session_id, created_at)| SourceCreation {
                session_id,
                created_at,
            })
            .collect(),
    };
    packet.validate()?;
    let value = serde_json::to_value(envelope)
        .map_err(|_| SourceError::Invalid("invalid origin values"))?;
    let bytes = serde_json::to_vec(&canonical(value))
        .map_err(|_| SourceError::Invalid("origin encoding failed"))?;
    let mut digest = Sha256::new();
    digest.update(INPUT_DOMAIN);
    digest.update(bytes);
    packet.input_dependencies_sha256 = hex::encode(digest.finalize());
    packet.validate()?;
    Ok(packet)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OriginEnvelope {
    schema_version: u8,
    database_epoch: String,
    workspace_sha256: String,
    observations: Vec<OriginValue>,
    recommendations: Vec<OriginValue>,
    readiness_focus: Option<OriginValue>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginValue {
    value: serde_json::Value,
    sources: Vec<SourceCreation>,
}
fn canonical(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(values) => serde_json::Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, canonical(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonical).collect())
        }
        value => value,
    }
}
#[cfg(test)]
pub(crate) mod tests;

pub(crate) mod classification;
