//! Process-local conditional write proofs. Never persisted or trusted as runtime authority.
use super::*;
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct AttentionProof {
    session: String,
    created: String,
    #[serde(skip)]
    write: crate::runtime::observed_status::AttentionWriteToken,
}
impl Database {
    pub(super) fn accepted_attention(
        &mut self,
        meta: &crate::metadata::SessionMetadata,
    ) -> Option<AttentionProof> {
        if meta.lifecycle != "alive" || meta.lifecycle_phase != "active" {
            self.attention_owners.remove(&meta.id);
            return None;
        }
        self.attention_owners
            .entry(meta.id.clone())
            .or_default()
            .accepted_write()
            .map(|write| AttentionProof {
                session: meta.id.clone(),
                created: meta.created_at.clone(),
                write,
            })
    }
    pub(super) fn owns_attention(&self, id: &str, created: &str, proof: &AttentionProof) -> bool {
        proof.session == id
            && proof.created == created
            && self
                .attention_owners
                .get(id)
                .is_some_and(|owner| owner.owns(&proof.write))
    }
    pub(super) fn read_owned_attention(
        &self,
        id: &str,
        created: &str,
    ) -> Result<Option<(serde_json::Value, AttentionProof)>> {
        let document = read_document(&self.conn, id, false)?.ok_or(StoreError::Missing)?;
        let (document, record) = canonical(document)?;
        if record.metadata.id != id || record.metadata.created_at != created {
            return Err(StoreError::Conflict);
        }
        let proof = self
            .attention_owners
            .get(id)
            .and_then(|owner| owner.snapshot())
            .map(|write| AttentionProof {
                session: id.into(),
                created: created.into(),
                write,
            });
        Ok(proof.map(|proof| (document, proof)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{
        internal_mutations::InternalMutation, lifecycle_mutations::LifecycleMutation,
    };
    use serde_json::json;
    const CREATED: &str = "2026-01-01T00:00:00Z";
    fn active(id: &str) -> serde_json::Value {
        let mut d = super::super::tests::document(id);
        d["harnessId"] = json!("codex");
        d["resume"] = json!({"state":"available","preferredStrategy":"exact","harnessSessionId":"root-one","latestFallback":false});
        d["observedStatus"] = json!("working");
        d["attention"] = json!("working");
        d["attentionSource"] = json!("agent");
        d["attentionOrigin"] = json!("harness_hook");
        d["attentionConfidence"] = json!(1.0);
        d
    }
    fn hook(root: &str, waiting: bool) -> InternalMutation {
        InternalMutation::HookCompletion {
            root: root.into(),
            waiting,
            timestamp: "2026-01-05T00:00:00Z".into(),
        }
    }
    fn transition(status: &str, live_phase: Option<&str>) -> InternalMutation {
        InternalMutation::Lifecycle(LifecycleMutation::Transition {
            status: status.into(),
            had_live_handle: live_phase.is_some(),
            already_ending: false,
            live_phase: live_phase.map(str::to_owned),
            resume: None,
            resumed_from: None,
            timestamp: "2026-01-04T00:00:00Z".into(),
        })
    }
    #[tokio::test]
    async fn attention_proof_cannot_be_used_for_an_unrelated_mutation_intent() {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(active("limited")).await.unwrap();
        let (_, proof) = store
            .read_owned_attention("limited".into(), CREATED.into())
            .await
            .unwrap()
            .unwrap();
        let before = store.get("limited".into()).await.unwrap();
        assert!(matches!(
            store
                .mutate_owned_attention(
                    "limited".into(),
                    CREATED.into(),
                    proof.clone(),
                    InternalMutation::UserTitle {
                        title: Some("unrelated title".into())
                    },
                )
                .await,
            Err(StoreError::Conflict)
        ));
        assert_eq!(store.get("limited".into()).await.unwrap(), before);
        assert_eq!(
            store
                .mutate_owned_attention(
                    "limited".into(),
                    CREATED.into(),
                    proof,
                    hook("root-one", true)
                )
                .await
                .unwrap()
                .document
                .unwrap()["attention"],
            "needs_you"
        );
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn invalid_lifecycle_status_does_not_publish_a_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(active("invalid")).await.unwrap();
        let before = store.get("invalid".into()).await.unwrap();
        assert!(matches!(
            store
                .mutate_internal(
                    "invalid".into(),
                    CREATED.into(),
                    None,
                    transition("bogus", Some("active"))
                )
                .await,
            Err(StoreError::Invalid(_))
        ));
        assert_eq!(store.get("invalid".into()).await.unwrap(), before);
        store.close().await.unwrap();
    }
    #[test]
    fn failed_conditional_commit_keeps_the_record_and_its_write_proof() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        db.insert(active("rollback")).unwrap();
        let (_, proof) = db
            .read_owned_attention("rollback", CREATED)
            .unwrap()
            .unwrap();
        let before = db.get("rollback").unwrap();
        db.conn.execute_batch("CREATE TRIGGER fail_write BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").unwrap();
        assert!(db
            .mutate_internal_owned(
                "rollback",
                CREATED,
                None,
                Some(&proof),
                &hook("root-one", true)
            )
            .is_err());
        assert_eq!(db.get("rollback").unwrap(), before);
        assert!(db.owns_attention("rollback", CREATED, &proof));
        db.conn.execute_batch("DROP TRIGGER fail_write;").unwrap();
        let accepted = db
            .mutate_internal_owned(
                "rollback",
                CREATED,
                None,
                Some(&proof),
                &hook("root-one", true),
            )
            .unwrap();
        assert_eq!(accepted.document.unwrap()["attention"], "needs_you");
        assert!(!db.owns_attention("rollback", CREATED, &proof));
        assert!(accepted.attention_proof.is_some());
    }
    #[tokio::test]
    async fn ending_and_finalization_preserve_snapshot_and_publish_closed_history_once() {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(active("lifecycle")).await.unwrap();
        let before = store
            .history(queries::HistoryQuery::default())
            .await
            .unwrap()
            .generation;
        let ending = store
            .mutate_internal(
                "lifecycle".into(),
                CREATED.into(),
                None,
                transition("killed", Some("ending")),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(ending["status"], "running");
        assert_eq!(ending["lifecyclePhase"], "ending");
        assert_eq!(ending["pendingTerminalStatus"], "killed");
        assert_eq!(ending["endingObservedStatusSnapshot"]["value"], "working");
        assert!(ending
            .get("observedStatus")
            .is_none_or(serde_json::Value::is_null));
        assert!(store
            .history(queries::HistoryQuery::default())
            .await
            .unwrap()
            .rows
            .is_empty());
        let snapshot = crate::metadata::ObservedStatusSnapshotMetadata {
            value: Some("blocked".into()),
            source: "agent".into(),
            confidence: Some(1.0),
            observed_at: Some("final".into()),
            attention_origin: Some("harness_hook".into()),
        };
        let finalize = InternalMutation::Lifecycle(LifecycleMutation::Finalize {
            snapshot,
            fallback_status: "error".into(),
            timestamp: "2026-01-05T00:00:00Z".into(),
        });
        let ended = store
            .mutate_internal("lifecycle".into(), CREATED.into(), None, finalize.clone())
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(ended["status"], "killed");
        assert_eq!(ended["terminalOutcome"], "killed");
        assert_eq!(ended["finalObservedStatusSnapshot"]["value"], "blocked");
        assert_eq!(ended["futureExtension"]["nested"][2], "keep");
        let page = store
            .history(queries::HistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(page.rows.len(), 1);
        assert!(page.generation > before);
        let noop = store
            .mutate_internal("lifecycle".into(), CREATED.into(), None, finalize)
            .await
            .unwrap();
        assert!(noop.document.is_none());
        assert_eq!(store.get("lifecycle".into()).await.unwrap().unwrap(), ended);
        assert_eq!(
            store
                .history(queries::HistoryQuery::default())
                .await
                .unwrap()
                .generation,
            page.generation
        );
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn resumed_closed_choice_follows_active_lifecycle_and_stale_live_transition_cannot_reopen_ending(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store
            .create(super::super::tests::closed_document("resume"))
            .await
            .unwrap();
        store
            .organize("resume".into(), CREATED.into(), "0".into(), "closed".into())
            .await
            .unwrap();
        let resumed = store
            .mutate_internal(
                "resume".into(),
                CREATED.into(),
                None,
                transition("running", Some("active")),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(resumed["lifecycle"], "alive");
        assert!(store
            .history(queries::HistoryQuery::default())
            .await
            .unwrap()
            .rows
            .is_empty());
        store
            .mutate_internal(
                "resume".into(),
                CREATED.into(),
                None,
                transition("ended", Some("ending")),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        let before = store.get("resume".into()).await.unwrap();
        assert!(store
            .mutate_internal(
                "resume".into(),
                CREATED.into(),
                None,
                transition("running", Some("ending"))
            )
            .await
            .unwrap()
            .document
            .is_none());
        assert_eq!(store.get("resume".into()).await.unwrap(), before);
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn conditional_hook_preserves_new_work_and_identical_attention_revokes_ownership() {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(active("owned")).await.unwrap();
        let (_, proof) = store
            .read_owned_attention("owned".into(), CREATED.into())
            .await
            .unwrap()
            .unwrap();
        let waiting = store
            .mutate_owned_attention(
                "owned".into(),
                CREATED.into(),
                proof,
                hook("root-one", true),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(waiting["attention"], "needs_you");
        let (_, proof) = store
            .read_owned_attention("owned".into(), CREATED.into())
            .await
            .unwrap()
            .unwrap();
        store.patch("owned".into(),json!({"metadataRevision":waiting["metadataRevision"],"patch":{"agentMetadata":{"workFields":{"task":"Newest work","nextAction":null,"workPhase":"review","blockerDescription":null,"failedCommand":null,"failedTest":null}}}})).await.unwrap();
        let working = store
            .mutate_owned_attention(
                "owned".into(),
                CREATED.into(),
                proof,
                hook("root-one", false),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(working["task"], "Newest work");
        assert_eq!(working["attention"], "working");
        let (_, proof) = store
            .read_owned_attention("owned".into(), CREATED.into())
            .await
            .unwrap()
            .unwrap();
        let same=store.patch("owned".into(),json!({"metadataRevision":working["metadataRevision"],"patch":{"attentionState":{"observedStatus":"working","needsUserInput":null,"detectedQuestion":null,"suggestedOptions":null}}})).await.unwrap();
        assert!(matches!(
            store
                .mutate_owned_attention(
                    "owned".into(),
                    CREATED.into(),
                    proof,
                    hook("root-one", true)
                )
                .await,
            Err(StoreError::Conflict)
        ));
        assert_eq!(store.get("owned".into()).await.unwrap().unwrap(), same);
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn missing_proof_wrong_root_and_reopened_store_cannot_inherit_conditional_authority() {
        let dir = tempfile::tempdir().unwrap();
        let store = worker::SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(active("fenced")).await.unwrap();
        let (_, proof) = store
            .read_owned_attention("fenced".into(), CREATED.into())
            .await
            .unwrap()
            .unwrap();
        let before = store.get("fenced".into()).await.unwrap();
        assert!(matches!(
            store
                .mutate_internal(
                    "fenced".into(),
                    CREATED.into(),
                    None,
                    hook("root-one", true)
                )
                .await,
            Err(StoreError::Conflict)
        ));
        assert!(store
            .mutate_owned_attention(
                "fenced".into(),
                CREATED.into(),
                proof.clone(),
                hook("other-root", true)
            )
            .await
            .unwrap()
            .document
            .is_none());
        assert_eq!(store.get("fenced".into()).await.unwrap(), before);
        store.close().await.unwrap();
        let reopened = worker::SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert!(reopened
            .read_owned_attention("fenced".into(), CREATED.into())
            .await
            .unwrap()
            .is_none());
        assert!(matches!(
            reopened
                .mutate_owned_attention(
                    "fenced".into(),
                    CREATED.into(),
                    proof,
                    hook("root-one", true)
                )
                .await,
            Err(StoreError::Conflict)
        ));
        assert_eq!(reopened.get("fenced".into()).await.unwrap(), before);
        reopened.close().await.unwrap();
    }
}
