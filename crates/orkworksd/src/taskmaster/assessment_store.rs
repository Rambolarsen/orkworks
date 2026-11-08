//! Bounded atomic storage for the workspace-local assessment protocol.
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(crate) const MAX_ASSESSMENT_BYTES: usize = 64 * 1024;
pub(crate) struct AssessmentStore {
    path: PathBuf,
}
impl AssessmentStore {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            path: root.join("assessment.json"),
        }
    }
    /// Existing callers are legacy owners and cannot replay format4.
    pub(crate) fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, String> {
        let Some(bytes) = self.read_bytes()? else {
            return Ok(None);
        };
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Assessment report is invalid")?;
        if value
            .get("formatVersion")
            .and_then(serde_json::Value::as_u64)
            == Some(4)
        {
            return Err("SQL assessment requires a qualified reader".into());
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "Assessment report is invalid".into())
    }
    /// Parsed data only. The caller must separately validate SQL and ownership.
    #[cfg_attr(not(test), allow(dead_code))] // Data reader; no production SQL route yet.
    pub(crate) fn load_sql(
        &self,
    ) -> Result<Option<super::assessment_report::AssessmentRecord>, String> {
        let Some(bytes) = self.read_bytes()? else {
            return Ok(None);
        };
        let record: super::assessment_report::AssessmentRecord =
            serde_json::from_slice(&bytes).map_err(|_| "Assessment report is invalid")?;
        record.sources_for_sql_validation()?;
        Ok(Some(record))
    }
    fn read_bytes(&self) -> Result<Option<Vec<u8>>, String> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Assessment report is unavailable".into()),
        };
        let mut bytes = Vec::new();
        file.take((MAX_ASSESSMENT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "Assessment report could not be read")?;
        validate_bytes(&bytes)?;
        Ok(Some(bytes))
    }
    pub(crate) fn save<T: Serialize>(&self, value: &T) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|_| "Assessment report cannot be encoded")?;
        validate_bytes(&bytes)?;
        #[cfg(test)]
        inject_save_failure(&self.path)?;
        let parent = self
            .path
            .parent()
            .ok_or("Assessment metadata is unavailable")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "Assessment report could not be staged")?;
        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|_| "Assessment report could not be staged")?;
        crate::harness::integration::atomic_replace(
            temporary.path(),
            &self.path,
            self.path.exists(),
        )
        .map_err(|_| "Assessment report could not be committed")?;
        #[cfg(not(windows))]
        std::fs::File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| "Assessment report directory could not be synced")?;
        Ok(())
    }
}

fn validate_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ASSESSMENT_BYTES {
        return Err("Assessment report exceeds 64 KiB".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Assessment report is invalid")?;
    if value
        .get("formatVersion")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|v| (1..=3).contains(&v))
        && contains_sql_source_field(bytes)?
    {
        return Err("Legacy assessment contains SQL source fields".into());
    }
    match value
        .get("formatVersion")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => {
            if value.get("inputManifest").is_some() || value.get("phase").is_some() {
                return Err("Legacy assessment contains unsupported fields".into());
            }
        }
        Some(2 | 3 | 4) => {
            let record: super::assessment_report::AssessmentRecord = serde_json::from_slice(bytes)
                .map_err(|_| "Assessment record has an invalid shape")?;
            record.validate_format()?;
        }
        _ => return Err("Assessment report version is unsupported".into()),
    }
    Ok(())
}

// Walk the reserved paths in the original stream: Value would discard earlier
// duplicate parents, potentially hiding a forbidden field from a legacy reader.
fn contains_sql_source_field(bytes: &[u8]) -> Result<bool, String> {
    use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
    struct Probe<'a> {
        level: u8,
        found: &'a mut bool,
    }
    impl<'de> DeserializeSeed<'de> for Probe<'_> {
        type Value = ();
        fn deserialize<D: serde::Deserializer<'de>>(self, de: D) -> Result<(), D::Error> {
            de.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Probe<'_> {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("JSON data")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
            while let Some(key) = map.next_key::<String>()? {
                if matches!(self.level, 0 | 2) && key == "sqlSources" {
                    *self.found = true;
                    map.next_value::<IgnoredAny>()?;
                } else if (self.level == 0 && key == "pendingRecommendation")
                    || (self.level == 1 && key == "assessment")
                {
                    map.next_value_seed(Probe {
                        level: self.level + 1,
                        found: &mut *self.found,
                    })?;
                } else {
                    map.next_value::<IgnoredAny>()?;
                }
            }
            Ok(())
        }
        fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
            Ok(())
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
            Ok(())
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
            Ok(())
        }
    }
    let mut found = false;
    Probe {
        level: 0,
        found: &mut found,
    }
    .deserialize(&mut serde_json::Deserializer::from_slice(bytes))
    .map_err(|_| "Assessment report is invalid".to_string())?;
    Ok(found)
}

#[cfg(test)]
pub(crate) fn set_save_failures(
    root: &Path,
    successful_saves_before_failure: usize,
    consecutive_failures: usize,
) {
    let mut faults = SAVE_FAULTS.lock().expect("assessment fault lock poisoned");
    let path = root.join("assessment.json");
    if consecutive_failures == 0 {
        faults.remove(&path);
    } else {
        faults.insert(
            path,
            (successful_saves_before_failure, consecutive_failures),
        );
    }
}

#[cfg(test)]
static SAVE_FAULTS: std::sync::Mutex<std::collections::BTreeMap<PathBuf, (usize, usize)>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

#[cfg(test)]
fn inject_save_failure(path: &Path) -> Result<(), String> {
    let mut faults = SAVE_FAULTS.lock().expect("assessment fault lock poisoned");
    let Some((successes, failures)) = faults.get_mut(path) else {
        return Ok(());
    };
    if *successes > 0 {
        *successes -= 1;
        return Ok(());
    }
    *failures -= 1;
    if *failures == 0 {
        faults.remove(path);
    }
    Err("Assessment report could not be committed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Record {
        format_version: u8,
        summary: String,
    }
    fn record(summary: &str) -> Record {
        Record {
            format_version: 1,
            summary: summary.into(),
        }
    }

    #[test]
    fn assessment_store_round_trip_replacement_and_legacy_absence() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssessmentStore::new(dir.path());
        assert!(store.load::<Record>().unwrap().is_none());
        store.save(&record("first")).unwrap();
        assert_eq!(
            AssessmentStore::new(dir.path()).load::<Record>().unwrap(),
            Some(record("first"))
        );
        store.save(&record("second")).unwrap();
        assert_eq!(store.load::<Record>().unwrap(), Some(record("second")));
    }

    #[test]
    fn assessment_store_rejects_corruption_version_and_oversized_reads() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssessmentStore::new(dir.path());
        for bytes in [
            b"broken".to_vec(),
            br#"{"formatVersion":2,"summary":"unsupported"}"#.to_vec(),
            br#"{"formatVersion":1,"summary":"valid","extra":true}"#.to_vec(),
            vec![b' '; MAX_ASSESSMENT_BYTES + 1],
        ] {
            std::fs::write(dir.path().join("assessment.json"), bytes).unwrap();
            assert!(store.load::<Record>().is_err());
        }
    }

    #[test]
    fn assessment_store_oversized_replacement_preserves_previous_record() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssessmentStore::new(dir.path());
        store.save(&record("keep")).unwrap();
        let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
        assert!(store
            .save(&record(&"x".repeat(MAX_ASSESSMENT_BYTES)))
            .is_err());
        assert_eq!(
            std::fs::read(dir.path().join("assessment.json")).unwrap(),
            before
        );
        assert_eq!(store.load::<Record>().unwrap(), Some(record("keep")));
    }

    #[test]
    fn assessment_store_is_workspace_local_and_deleted_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        AssessmentStore::new(&a).save(&record("a")).unwrap();
        assert!(AssessmentStore::new(&b).load::<Record>().unwrap().is_none());
        std::fs::remove_dir_all(&a).unwrap();
        assert!(AssessmentStore::new(&a).load::<Record>().unwrap().is_none());
    }
    #[test]
    fn assessment_store_failed_atomic_save_preserves_prior_record_and_can_retry() {
        let directory = tempfile::tempdir().unwrap();
        let store = AssessmentStore::new(directory.path());
        store.save(&record("prior")).unwrap();
        let before = std::fs::read(directory.path().join("assessment.json")).unwrap();
        set_save_failures(directory.path(), 0, 1);
        assert!(store.save(&record("replacement")).is_err());
        assert_eq!(
            std::fs::read(directory.path().join("assessment.json")).unwrap(),
            before
        );
        store.save(&record("replacement")).unwrap();
        assert_eq!(
            store.load::<Record>().unwrap().unwrap().summary,
            "replacement"
        );
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;
    #[test]
    fn legacy_source_fields_are_rejected_before_null_collapses_or_bytes_change() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssessmentStore::new(dir.path());
        let (record, _) = crate::taskmaster::assessment_report::tests::proposal();
        let value = serde_json::to_value(record).unwrap();
        store.save(&value).unwrap();
        let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
        for nested in [false, true] {
            let mut invalid = value.clone();
            if nested {
                invalid["pendingRecommendation"]["assessment"]["sqlSources"] =
                    serde_json::Value::Null;
            } else {
                invalid["sqlSources"] = serde_json::Value::Null;
            }
            assert!(store.save(&invalid).is_err(), "nested={nested}");
            assert_eq!(
                std::fs::read(dir.path().join("assessment.json")).unwrap(),
                before
            );
        }
    }

    #[test]
    fn format_two_requires_a_bounded_manifest_and_legacy_rejects_new_fields() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("README.md"), "current workflow").unwrap();
        let context = crate::taskmaster::assessment_context::RepositoryContext::collect(
            workspace.path(),
            crate::taskmaster::runtime::ContextLevel::WorkflowContext,
            &[],
            "now",
        )
        .unwrap();
        let (record, _) = crate::taskmaster::assessment_report::tests::proposal();
        let mut value = serde_json::to_value(record).unwrap();
        value["formatVersion"] = serde_json::json!(2);
        value["state"] = serde_json::json!("pending_recommendation");
        let mut manifest = serde_json::to_value(context.manifest()).unwrap();
        manifest["discoveryDigest"] = serde_json::json!("a".repeat(64));
        value["inputManifest"] = manifest.clone();
        let store = AssessmentStore::new(dir.path());
        assert!(
            store.save(&value).is_ok(),
            "valid format two must be supported"
        );
        assert!(store
            .load::<crate::taskmaster::assessment_report::AssessmentRecord>()
            .unwrap()
            .is_some());
        value["formatVersion"] = serde_json::json!(1);
        assert!(
            store.save(&value).is_err(),
            "legacy must not silently accept new semantics"
        );
        value["formatVersion"] = serde_json::json!(2);
        value.as_object_mut().unwrap().remove("inputManifest");
        assert!(
            store.save(&value).is_err(),
            "completed format two requires manifest"
        );
        value["inputManifest"] = manifest;
        value["inputManifest"]["files"] =
            serde_json::json!(vec![value["inputManifest"]["files"][0].clone(); 33]);
        assert!(store.save(&value).is_err());
    }
}

#[cfg(test)]
pub(crate) mod sql_source_tests;
