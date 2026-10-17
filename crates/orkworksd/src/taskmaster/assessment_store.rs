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
    pub(crate) fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, String> {
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
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "Assessment report is invalid".into())
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
        != Some(1)
    {
        return Err("Assessment report version is unsupported".into());
    }
    Ok(())
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
