//! Copied fixture composition only: parsed data never mints a production claim.
use super::sql_observation_origin_tests::ending;
use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::lifecycle::SqlCandidateResult;
use crate::session_application::sql_writes::observation_opening::SqlObservationWorkspace;
use crate::session_store::{SessionStore, StoreError};
use crate::taskmaster::assessment_sources::{OriginalInputs, SqlAssessmentSources};
use crate::workflow_observations::{sql_origin::RecordData, RecordOutcome};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
const CREATED: &str = "2026-01-01T00:00:00Z";

struct Copied {
    _directory: tempfile::TempDir,
    root: PathBuf,
    workspace: String,
    epoch: String,
    workspace_hash: String,
    value: Value,
}
impl Copied {
    async fn produced() -> Self {
        let (_original_directory, state, store, writes, _, _, _, _) = fixture().await;
        let original = ending(&state, &writes)
            .await
            .capture_sql_observation_origin()
            .await
            .unwrap();
        let epoch = original.source().database_epoch().to_owned();
        let workspace_hash = original.source().workspace_sha256().to_owned();
        let candidate = serde_json::from_value(json!({"kind":"obstacle","description":"blocked","evidence":"real error","problemArea":"build","reportedImpact":"low","confidence":0.8})).unwrap();
        let batch = original
            .persist_workflow_observations(7, vec![candidate], vec!["real error".into()])
            .await
            .unwrap();
        let value = match &batch.candidates[0] {
            SqlCandidateResult::Recorded(Ok(RecordOutcome::Accepted(observation))) => {
                serde_json::to_value(observation).unwrap()
            }
            other => panic!("expected actual qualified producer receipt: {other:?}"),
        };
        let source_root = store.location().0.to_owned();
        let workspace = store.location().1.to_owned();
        drop(original);
        store.close().await.unwrap();
        drop(writes);
        drop(store);
        state.workspace.lock().unwrap().take();
        state.sessions.lock().unwrap().clear();
        // Only durable database/evidence files are copied, never live owners,
        // leases, SQLite WAL state, or an application context.
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("copied");
        for name in [
            "session-store/v1.sqlite",
            "workflow-observations/sql-runtime.ndjson",
            "workflow-observations/sequence",
        ] {
            let destination = root.join(name);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(source_root.join(name), destination).unwrap();
        }
        Self {
            _directory: directory,
            root,
            workspace,
            epoch,
            workspace_hash,
            value,
        }
    }
    async fn read(&self) -> Vec<RecordData> {
        let lease =
            Arc::new(crate::workspace_runtime::WorkspaceLease::acquire(&self.root).unwrap());
        // No workspace is adopted. Use the existing checked owner-loss opener.
        let state = crate::test_support::test_app_state_with_workspace(self._directory.path());
        state.workspace.lock().unwrap().take();
        let opened =
            SqlObservationWorkspace::open(state, self.root.clone(), self.workspace.clone(), lease)
                .await
                .unwrap();
        let records = opened.snapshot().records().to_vec();
        opened.close().await.unwrap();
        records
    }
    fn evidence_bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        std::fs::read_dir(self.root.join("workflow-observations"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }
    fn envelope(&self, records: &[RecordData]) -> Value {
        // Trusted test-only representation of already-qualified fixture data.
        // This is not a production conversion, mixed-input admission gate, or
        // proof that parsed data supplies original authority.
        let observations: Vec<_> = records
            .iter()
            .map(|record| {
                let origin = record.origin().expect("qualified fixture input only");
                let sources = vec![json!({
                    "sessionId":origin.session_id(), "createdAt":origin.created_at()
                })];
                json!({"value":record.observation().unwrap(), "sources":sources})
            })
            .collect();
        json!({"schemaVersion":1,"databaseEpoch":self.epoch,"workspaceSha256":self.workspace_hash,
            "observations":observations,"recommendations":[],"readinessFocus":null})
    }
    async fn store(&self) -> SessionStore {
        SessionStore::open(self.root.clone(), self.workspace.clone())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn produced_observation_survives_copied_reopen_into_exact_assessment_envelope() {
    let _serial = diagnostic_test_guard();
    let copied = Copied::produced().await;
    let before = copied.evidence_bytes();
    let records = copied.read().await;
    assert_eq!(records.len(), 1);
    assert_eq!(
        serde_json::to_value(records[0].observation().unwrap()).unwrap(),
        copied.value
    );
    let envelope = copied.envelope(&records);
    assert_eq!(
        envelope["observations"],
        json!([{"value":copied.value,
        "sources":[{"sessionId":"sql-runtime","createdAt":"2026-01-01T00:00:00Z"}]}])
    );
    let original = OriginalInputs::fixture("assessment-copied", envelope.clone()).unwrap();
    let store = copied.store().await;
    let packet = store.capture_assessment_sources(original).await.unwrap();
    assert_eq!(packet.database_epoch(), copied.epoch);
    assert_eq!(packet.workspace_sha256(), copied.workspace_hash);
    assert_eq!(
        serde_json::to_value(packet.sources()).unwrap(),
        json!([
            {"sessionId":"sql-runtime","createdAt":"2026-01-01T00:00:00Z"}
        ])
    );
    // JSON maps use sorted keys; this expectation hashes the specified envelope
    // directly, without invoking the codec's canonicalization/digest helpers.
    let mut digest = Sha256::new();
    digest.update(b"orkworks/sql-assessment-inputs/v1\0");
    digest.update(serde_json::to_vec(&envelope).unwrap());
    assert_eq!(packet.input_digest(), hex::encode(digest.finalize()));
    let encoded = serde_json::to_vec(&packet).unwrap();
    assert_eq!(
        serde_json::from_slice::<SqlAssessmentSources>(&encoded).unwrap(),
        packet
    );
    let mut changed = envelope;
    changed["observations"][0]["value"]["evidence"] = json!("a different retained value");
    let changed = OriginalInputs::fixture("assessment-copied", changed)
        .unwrap()
        .into_sources();
    assert_ne!(packet.input_digest(), changed.input_digest());
    store
        .validate_assessment_sources(
            packet.clone(),
            "assessment-copied".into(),
            packet.input_digest().into(),
        )
        .await
        .unwrap();
    store.close().await.unwrap();
    assert_eq!(copied.evidence_bytes(), before);
    assert!(!copied.root.join("assessment.json").exists());
    assert!(!copied.root.join("recommendations").exists());
}

#[tokio::test]
async fn copied_observation_sources_refuse_stale_sql_without_refresh_or_file_effects() {
    let _serial = diagnostic_test_guard();
    for change in ["creation", "deleted", "missing", "epoch"] {
        let copied = Copied::produced().await;
        let records = copied.read().await;
        let envelope = copied.envelope(&records);
        let original = OriginalInputs::fixture("assessment-copied", envelope.clone()).unwrap();
        let before = copied.evidence_bytes();
        let conn = rusqlite::Connection::open(copied.root.join("session-store/v1.sqlite")).unwrap();
        match change {
            "creation" => {
                conn.execute("UPDATE sessions SET created_at='replacement'", [])
                    .unwrap();
            }
            "deleted" => {
                conn.execute("UPDATE sessions SET deleted=1", []).unwrap();
            }
            "missing" => {
                conn.execute("DELETE FROM sessions", []).unwrap();
            }
            "epoch" => {
                conn.execute(
                    "UPDATE store_header SET epoch='00000000-0000-4000-8000-000000000001'",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        drop(conn);
        let store = copied.store().await;
        assert!(
            matches!(
                store.capture_assessment_sources(original).await,
                Err(StoreError::Conflict)
            ),
            "{change}"
        );
        assert_eq!(
            envelope["observations"][0]["sources"][0]["createdAt"],
            CREATED
        );
        assert_eq!(copied.evidence_bytes(), before, "{change}");
        assert!(!copied.root.join("assessment.json").exists());
        assert!(!copied.root.join("recommendations").exists());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn copied_strict_inventory_retains_legacy_data_without_origin() {
    let _serial = diagnostic_test_guard();
    let copied = Copied::produced().await;
    let store = copied.store().await;
    store
        .create(
            serde_json::to_value(crate::test_support::test_session_metadata(
                "legacy",
                "Legacy",
                &copied.workspace,
                "closed",
                CREATED,
                CREATED,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(store.get("legacy".into()).await.unwrap().is_some());
    store.close().await.unwrap();
    // Add a valid ordinary legacy observation with no origin. Keep the complete
    // retained inventory; its matching current session cannot supply provenance.
    let mut legacy = copied.value.clone();
    legacy["id"] = json!(uuid::Uuid::new_v4().to_string());
    legacy["sequence"] = json!(2);
    legacy["sessionId"] = json!("legacy");
    let raw: Value = serde_json::from_slice(
        &std::fs::read(copied.root.join("workflow-observations/sql-runtime.ndjson")).unwrap(),
    )
    .unwrap();
    let mut line = legacy.clone();
    line["record"] = json!("observation");
    line["idempotencyKeyHash"] = raw["idempotencyKeyHash"].clone();
    line["payloadHash"] = raw["payloadHash"].clone();
    let mut bytes = serde_json::to_vec(&line).unwrap();
    bytes.push(b'\n');
    std::fs::write(
        copied.root.join("workflow-observations/legacy.ndjson"),
        bytes,
    )
    .unwrap();
    std::fs::write(copied.root.join("workflow-observations/sequence"), b"2").unwrap();
    let before = copied.evidence_bytes();
    let records = copied.read().await;
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .iter()
            .filter(|record| record.origin().is_none())
            .count(),
        1
    );
    let retained = records
        .iter()
        .find(|record| record.origin().is_none())
        .unwrap();
    assert_eq!(
        serde_json::to_value(retained.observation().unwrap()).unwrap(),
        legacy
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record.origin().is_some())
            .count(),
        1
    );
    // The strict reader preserves unknown provenance as data; there is no
    // production OriginalInputs constructor to qualify this mixed inventory.
    assert_eq!(copied.evidence_bytes(), before);
    assert!(!copied.root.join("assessment.json").exists());
    assert!(!copied.root.join("recommendations").exists());
}
