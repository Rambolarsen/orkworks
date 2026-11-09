//! Private semantic policy only. No producer claim or OriginalInputs minting.
use super::{
    source_data_from_origins, validate_pairs, OriginEnvelope, OriginValue, SourceCreation,
    SourceError, SqlAssessmentSources, MAX_SOURCES,
};
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
const MAX_ENVELOPE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ClassificationError {
    Unqualified,
    Conflict,
    Invalid(&'static str),
    TooLarge,
}
impl From<SourceError> for ClassificationError {
    fn from(error: SourceError) -> Self {
        match error {
            SourceError::Invalid(reason) => Self::Invalid(reason),
            SourceError::TooLarge => Self::TooLarge,
        }
    }
}
#[derive(PartialEq, Eq)]
struct Scope {
    epoch: String,
    workspace: String,
}
enum SelectedScope {
    Unknown,
    Qualified(Scope),
}
struct Qualified {
    scope: Scope,
    sources: Vec<SourceCreation>,
}
enum Origin {
    Unknown,
    Qualified(Qualified),
}
struct Input {
    value: Value,
    origin: Origin,
}

/// Complete owned membership, not a filtered caller iterator. Only test
/// utilities can construct qualified inputs/scopes in this first slice.
pub(crate) struct CapturedInputSet {
    scope: SelectedScope,
    observations: Vec<Input>,
    recommendations: Vec<Input>,
    readiness: Option<Input>,
}
impl Scope {
    fn validate(&self, assessment_id: &str) -> Result<(), ClassificationError> {
        // Reject unbounded identity strings before copying into the existing
        // validator. Its canonical scope/identity rules remain authoritative.
        if self.epoch.len() != 36 || self.workspace.len() != 64 || !super::valid_id(assessment_id) {
            return Err(ClassificationError::Invalid(
                "invalid scope or assessment identity",
            ));
        }
        SqlAssessmentSources {
            schema_version: 1,
            database_epoch: self.epoch.clone(),
            workspace_sha256: self.workspace.clone(),
            assessment_id: assessment_id.into(),
            input_dependencies_sha256: "0".repeat(64),
            sources: vec![],
        }
        .validate()
        .map_err(Into::into)
    }
}
impl CapturedInputSet {
    pub(crate) fn classify(
        self,
        assessment_id: &str,
    ) -> Result<SqlAssessmentSources, ClassificationError> {
        let SelectedScope::Qualified(scope) = &self.scope else {
            return Err(ClassificationError::Unqualified);
        };
        scope.validate(assessment_id)?;
        if self
            .observations
            .iter()
            .chain(&self.recommendations)
            .chain(self.readiness.iter())
            .any(|input| matches!(input.origin, Origin::Unknown))
        {
            // Unknown has no accepted canonical envelope. Refuse before trying
            // to encode it as an empty source array or constructing any result.
            return Err(ClassificationError::Unqualified);
        }
        // Borrow exact accepted-envelope fields; no value clone, conversion or
        // source-union allocation occurs before the complete byte preflight.
        preflight(&EnvelopeView {
            schema_version: 1,
            database_epoch: &scope.epoch,
            workspace_sha256: &scope.workspace,
            observations: InputList(&self.observations),
            recommendations: InputList(&self.recommendations),
            readiness_focus: self.readiness.as_ref().map(InputView),
        })?;
        let mut union = BTreeMap::new();
        let mut convert = |input: Input| -> Result<OriginValue, ClassificationError> {
            let Origin::Qualified(claim) = input.origin else {
                return Err(ClassificationError::Unqualified);
            };
            if claim.scope != *scope {
                claim.scope.validate(assessment_id)?;
                return Err(ClassificationError::Conflict);
            }
            validate_pairs(&claim.sources)?;
            for pair in &claim.sources {
                if union
                    .insert(pair.session_id.clone(), pair.created_at.clone())
                    .is_some_and(|created| created != pair.created_at)
                {
                    return Err(ClassificationError::Conflict);
                }
                if union.len() > MAX_SOURCES {
                    return Err(ClassificationError::TooLarge);
                }
            }
            Ok(OriginValue {
                value: input.value,
                sources: claim.sources,
            })
        };
        let observations = self
            .observations
            .into_iter()
            .map(&mut convert)
            .collect::<Result<Vec<_>, _>>()?;
        let recommendations = self
            .recommendations
            .into_iter()
            .map(&mut convert)
            .collect::<Result<Vec<_>, _>>()?;
        let readiness_focus = self.readiness.map(&mut convert).transpose()?;
        // The existing codec's additional canonical tree/encoding is bounded
        // by the completed preflight. Return data, never OriginalInputs.
        source_data_from_origins(
            assessment_id,
            OriginEnvelope {
                schema_version: 1,
                database_epoch: scope.epoch.clone(),
                workspace_sha256: scope.workspace.clone(),
                observations,
                recommendations,
                readiness_focus,
            },
        )
        .map_err(Into::into)
    }
}

/// Borrowed byte-accounting views only; the opaque claims themselves have no
/// Serialize/Deserialize implementation and no production constructor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvelopeView<'a> {
    schema_version: u8,
    database_epoch: &'a str,
    workspace_sha256: &'a str,
    observations: InputList<'a>,
    recommendations: InputList<'a>,
    readiness_focus: Option<InputView<'a>>,
}
struct InputList<'a>(&'a [Input]);
struct InputView<'a>(&'a Input);
impl Serialize for InputList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for input in self.0 {
            seq.serialize_element(&InputView(input))?;
        }
        seq.end()
    }
}
impl Serialize for InputView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Origin::Qualified(claim) = &self.0.origin else {
            return Err(serde::ser::Error::custom("unqualified input"));
        };
        let mut object = serializer.serialize_struct("OriginValue", 2)?;
        object.serialize_field("value", &self.0.value)?;
        object.serialize_field("sources", &claim.sources)?;
        object.end()
    }
}
fn preflight(envelope: &EnvelopeView<'_>) -> Result<(), ClassificationError> {
    struct Counter {
        bytes: usize,
        overflow: bool,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let Some(next) = self
                .bytes
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_ENVELOPE_BYTES)
            else {
                self.overflow = true;
                return Err(std::io::Error::other(
                    "assessment input envelope exceeds bound",
                ));
            };
            self.bytes = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter {
        bytes: 0,
        overflow: false,
    };
    serde_json::to_writer(&mut counter, envelope).map_err(|_| {
        if counter.overflow {
            ClassificationError::TooLarge
        } else {
            ClassificationError::Invalid("invalid envelope encoding")
        }
    })
}
#[cfg(test)]
mod tests;
