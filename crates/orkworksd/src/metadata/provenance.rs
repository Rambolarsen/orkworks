//! Independent provenance scopes in the canonical session metadata record.
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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

impl MetadataProvenance {
    /// Populate legacy authority in memory. `None` age is deliberately kept
    /// unknown so Peon cannot infer that a protected source is stale.
    pub(crate) fn hydrate_legacy(
        &mut self,
        legacy_source: &str,
        legacy_confidence: f64,
        modified: Option<SystemTime>,
    ) {
        let modified =
            modified.map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
        if self.attention_source.is_none() && self.attention_origin.is_none() {
            self.attention_source = Some(if legacy_source == "codex_hook" {
                "agent".to_string()
            } else {
                legacy_source.to_string()
            });
            self.attention_confidence = Some(legacy_confidence);
            self.attention_origin = Some("legacy_unknown".to_string());
        }
        if self.attention_updated_at.is_none()
            && self.attention_origin.as_deref() == Some("legacy_unknown")
        {
            self.attention_updated_at = modified.clone();
        }
        if self.work_metadata_updated_at.is_none() {
            self.work_metadata_updated_at = modified;
        }
    }

    pub(crate) fn attention_source_or<'a>(&'a self, legacy_source: &'a str) -> &'a str {
        self.attention_source.as_deref().unwrap_or(legacy_source)
    }

    pub(crate) fn attention_age_secs(&self) -> Option<u64> {
        Self::age_secs(self.attention_updated_at.as_deref())
    }

    pub(crate) fn work_age_secs(&self) -> Option<u64> {
        Self::age_secs(self.work_metadata_updated_at.as_deref())
    }

    fn age_secs(value: Option<&str>) -> Option<u64> {
        let then = chrono::DateTime::parse_from_rfc3339(value?)
            .ok()?
            .with_timezone(&chrono::Utc);
        let elapsed = chrono::Utc::now().signed_duration_since(then);
        u64::try_from(elapsed.num_seconds()).ok()
    }
}
