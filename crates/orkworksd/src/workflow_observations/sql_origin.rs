//! Strict immutable provenance data. No constructor for original claims or I/O grants.
use super::*;
use crate::session_store::StoreError;
use serde::{Deserialize, Deserializer, Serialize};
pub(crate) const MAX_ORIGIN_BYTES: usize = 2 * 1024;
pub(crate) const MAX_LINE_BYTES: usize = 64 * 1024;
type Result<T> = std::result::Result<T, StoreError>;
fn corrupt<T>() -> Result<T> {
    Err(StoreError::Corrupt)
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OriginWire {
    schema_version: u8,
    database_epoch: String,
    workspace_sha256: String,
    session_id: String,
    created_at: String,
    observation_sha256: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct OriginData(OriginWire);
impl<'de> Deserialize<'de> for OriginData {
    fn deserialize<D: Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        let value = Self(OriginWire::deserialize(de)?);
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}
impl OriginData {
    fn validate(&self) -> Result<()> {
        let v = &self.0;
        if v.schema_version != 1
            || uuid::Uuid::parse_str(&v.database_epoch)
                .map(|id| id.to_string() != v.database_epoch)
                .unwrap_or(true)
            || !hex_digest(&v.workspace_sha256)
            || !hex_digest(&v.observation_sha256)
            || !valid_id(&v.session_id)
            || v.created_at.is_empty()
            || v.created_at.len() > 256
        {
            return corrupt();
        }
        if serde_json::to_vec(self)?.len() > MAX_ORIGIN_BYTES {
            return Err(StoreError::TooLarge);
        }
        Ok(())
    }
    pub(crate) fn session_id(&self) -> &str {
        &self.0.session_id
    }
    pub(crate) fn created_at(&self) -> &str {
        &self.0.created_at
    }
    pub(crate) fn database_epoch(&self) -> &str {
        &self.0.database_epoch
    }
    pub(crate) fn workspace_sha256(&self) -> &str {
        &self.0.workspace_sha256
    }
}
// Remote derive makes only the new record strict; ordinary legacy decoding is unchanged.
#[derive(Serialize, Deserialize)]
#[serde(
    remote = "WorkflowObservation",
    rename_all = "camelCase",
    deny_unknown_fields
)]
struct StrictObservation {
    id: String,
    sequence: u64,
    session_id: String,
    observed_at: String,
    kind: ObservationKind,
    description: String,
    evidence: String,
    #[serde(default)]
    problem_area: Option<String>,
    reported_impact: Impact,
    source: ObservationSource,
    confidence: f64,
    fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ObservationData {
    #[serde(with = "StrictObservation")]
    observation: WorkflowObservation,
    origin: OriginData,
    idempotency_key_hash: String,
    payload_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TombstoneData {
    idempotency_key_hash: String,
    payload_hash: String,
    observation_id: String,
    sequence: u64,
    accepted_at: String,
    origin: OriginData,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "record")]
enum Record {
    #[serde(rename = "sql_observation_v1")]
    Observation(ObservationData),
    #[serde(rename = "sql_tombstone_v1")]
    Tombstone(TombstoneData),
    #[serde(rename = "observation")]
    LegacyObservation(StoredObservation),
    #[serde(rename = "tombstone")]
    LegacyTombstone(Tombstone),
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(transparent)]
pub(crate) struct RecordData(Record);
impl RecordData {
    pub(crate) fn origin(&self) -> Option<&OriginData> {
        match &self.0 {
            Record::Observation(v) => Some(&v.origin),
            Record::Tombstone(v) => Some(&v.origin),
            _ => None,
        }
    }
    pub(crate) fn observation(&self) -> Option<&WorkflowObservation> {
        match &self.0 {
            Record::Observation(v) => Some(&v.observation),
            Record::LegacyObservation(v) => Some(&v.observation),
            _ => None,
        }
    }
    pub(super) fn identity(&self) -> (&str, u64) {
        match &self.0 {
            Record::Observation(v) => (&v.observation.id, v.observation.sequence),
            Record::LegacyObservation(v) => (&v.observation.id, v.observation.sequence),
            Record::Tombstone(v) => (&v.observation_id, v.sequence),
            Record::LegacyTombstone(v) => (&v.observation_id, v.sequence),
        }
    }
    pub(crate) fn sequence(&self) -> u64 {
        match &self.0 {
            Record::Observation(v) => v.observation.sequence,
            Record::LegacyObservation(v) => v.observation.sequence,
            Record::Tombstone(v) => v.sequence,
            Record::LegacyTombstone(v) => v.sequence,
        }
    }
    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() > MAX_LINE_BYTES {
            return Err(StoreError::TooLarge);
        }
        Ok(bytes)
    }
    fn validate(&self, session: &str) -> Result<()> {
        if !valid_id(session) || self.sequence() == 0 {
            return corrupt();
        }
        if let Some(origin) = self.origin() {
            origin.validate()?;
            if origin.session_id() != session {
                return corrupt();
            }
        }
        if let Some(observation) = self.observation() {
            if observation.session_id != session {
                return corrupt();
            }
            validate_observation(observation)?;
        }
        let (key, payload) = match &self.0 {
            Record::Observation(v) => {
                if value_digest(&v.observation)? != v.origin.0.observation_sha256 {
                    return corrupt();
                }
                (&v.idempotency_key_hash, &v.payload_hash)
            }
            Record::LegacyObservation(v) => (&v.idempotency_key_hash, &v.payload_hash),
            Record::Tombstone(v) => {
                validate_tombstone(&v.observation_id, &v.accepted_at)?;
                (&v.idempotency_key_hash, &v.payload_hash)
            }
            Record::LegacyTombstone(v) => {
                validate_tombstone(&v.observation_id, &v.accepted_at)?;
                (&v.idempotency_key_hash, &v.payload_hash)
            }
        };
        if !hex_digest(key) || !hex_digest(payload) {
            return corrupt();
        }
        if let Some(v) = self.observation() {
            if hash_payload(
                v.kind,
                &v.description,
                &v.evidence,
                v.reported_impact,
                v.problem_area.as_deref(),
            ) != *payload
            {
                return corrupt();
            }
        }
        Ok(())
    }
}
fn validate_tombstone(id: &str, accepted: &str) -> Result<()> {
    if uuid::Uuid::parse_str(id).is_err() || DateTime::parse_from_rfc3339(accepted).is_err() {
        return corrupt();
    }
    Ok(())
}
fn validate_observation(v: &WorkflowObservation) -> Result<()> {
    validate_tombstone(&v.id, &v.observed_at)?;
    let candidate = ObservationCandidate {
        kind: v.kind,
        description: v.description.clone(),
        evidence: v.evidence.clone(),
        problem_area: v.problem_area.clone(),
        reported_impact: v.reported_impact,
        confidence: Some(v.confidence),
    };
    let origin = if v.source == ObservationSource::Agent {
        ObservationOrigin::Agent
    } else {
        ObservationOrigin::Peon
    };
    let (source, confidence) =
        validate_candidate(origin, &candidate).map_err(|_| StoreError::Corrupt)?;
    if source != v.source || confidence != v.confidence {
        return corrupt();
    }
    let expected = if let Some(area) = &v.problem_area {
        if canonicalize_problem_area(area).map_err(|_| StoreError::Corrupt)? != *area {
            return corrupt();
        }
        fingerprint_v2(v.kind, area)
    } else {
        format!(
            "v1:{}:{}",
            v.kind.as_str(),
            normalize_description(&v.description)
        )
    };
    if v.fingerprint != expected {
        return corrupt();
    }
    Ok(())
}
pub(crate) fn decode_line(bytes: &[u8], session: &str) -> Result<RecordData> {
    if bytes.len() > MAX_LINE_BYTES {
        return Err(StoreError::TooLarge);
    }
    // Size-only probe. Strict decoding below still consumes the original bytes,
    // so duplicate/unknown protocol fields cannot disappear through Value.
    #[derive(Deserialize)]
    struct SizeProbe {
        #[serde(default)]
        origin: Option<serde_json::Value>,
    }
    let probe: SizeProbe = serde_json::from_slice(bytes).map_err(|_| StoreError::Corrupt)?;
    if let Some(origin) = probe.origin {
        if serde_json::to_vec(&origin)?.len() > MAX_ORIGIN_BYTES {
            return Err(StoreError::TooLarge);
        }
    }
    let value: RecordData =
        RecordData(serde_json::from_slice(bytes).map_err(|_| StoreError::Corrupt)?);
    value.validate(session)?;
    Ok(value)
}
pub(crate) fn value_digest(observation: &WorkflowObservation) -> Result<String> {
    validate_observation(observation)?;
    fn canonical(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, canonical(v)))
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(canonical).collect())
            }
            v => v,
        }
    }
    let mut hash = Sha256::new();
    hash.update(b"orkworks/sql-observation-value/v1\0");
    hash.update(serde_json::to_vec(&canonical(serde_json::to_value(
        observation,
    )?))?);
    Ok(hex::encode(hash.finalize()))
}
mod reader;
#[allow(unused_imports)] // Staged reader API until the checked SQL owner is integrated.
pub(crate) use reader::{ScanRequest, Snapshot};
#[cfg(test)]
mod tests;
