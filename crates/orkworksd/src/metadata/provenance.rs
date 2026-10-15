//! Independent provenance scopes in the canonical session metadata record.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_metadata_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_observed_at: Option<String>,
}
