use super::*;
use crate::metadata::{
    internal_mutations::{Decision, InternalMutation},
    session_transactions::MetadataRevision,
};
#[derive(Debug, serde::Serialize)]
pub(crate) struct InternalMutationResult {
    pub(crate) document: Option<serde_json::Value>,
    pub(crate) decision: Decision,
}
impl Database {
    pub(super) fn mutate_internal(
        &mut self,
        id: &str,
        created: &str,
        expected: Option<&MetadataRevision>,
        intent: &InternalMutation,
    ) -> Result<InternalMutationResult> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let original = read_document(&tx, id, false)?.ok_or(StoreError::Missing)?;
        let mut record: crate::metadata::session_transactions::RevisionedSession =
            serde_json::from_value(original.clone())?;
        if record.metadata.created_at != created
            || expected.is_some_and(|expected| expected != &record.revision)
        {
            return Err(StoreError::Conflict);
        }
        let old = serde_json::to_value(&record)?;
        let old_workspace = record.metadata.workspace.clone();
        let decision = intent.apply(&mut record.metadata).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidInput {
                StoreError::Invalid(e.to_string())
            } else {
                StoreError::Io(e)
            }
        })?;
        let Some(scope) = decision.scope else {
            return Ok(InternalMutationResult {
                document: None,
                decision,
            });
        };
        if record.metadata.id != id
            || record.metadata.created_at != created
            || record.metadata.workspace != old_workspace
        {
            return Err(StoreError::Conflict);
        }
        let now = crate::workspace_runtime::iso_now();
        if scope.attention {
            record.metadata.provenance.attention_updated_at = Some(now.clone());
        }
        if scope.work {
            record.metadata.provenance.work_metadata_updated_at = Some(now);
        }
        record.revision = record.revision.next()?;
        let document = merge_known(original, &old, serde_json::to_value(&record)?)?;
        check_size(&document)?;
        let historical: bool = tx.query_row(
            "SELECT state!='active' FROM sessions WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        persist(&tx, &document, &record)?;
        if historical {
            bump_generation(&tx)?;
        }
        tx.commit()?;
        Ok(InternalMutationResult {
            document: Some(document),
            decision,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::super::worker::SessionStore;
    use super::*;
    use crate::metadata::{
        internal_mutations::{Disposition, PeonPolicy},
        HarnessSessionMergeResult, HarnessSessionReport,
    };
    use serde_json::json;
    fn inference() -> crate::peon::PeonInference {
        serde_json::from_value(json!({"observedStatus":"blocked","phase":"review","summary":null,"nextAction":"inferred next","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":null,"blockerDescription":"inferred blocker","failedCommand":null,"failedTest":null,"capacityHints":null,"confidence":0.8})).unwrap()
    }
    fn peon() -> InternalMutation {
        InternalMutation::Peon {
            inference: inference(),
            timestamp: "2026-01-03T00:00:00Z".into(),
            provider: None,
            history_summary: None,
            policy: PeonPolicy::Infer,
        }
    }
    fn base(id: &str) -> serde_json::Value {
        let mut d = super::super::tests::document(id);
        d["attentionSource"] = json!("agent");
        d["attentionConfidence"] = json!(1.0);
        d["attentionOrigin"] = json!("direct_agent");
        d["attentionUpdatedAt"] = json!("2000-01-01T00:00:00Z");
        d["workMetadataUpdatedAt"] = json!("2020-01-01T00:00:00Z");
        d
    }
    #[tokio::test]
    async fn delayed_peon_uses_latest_agent_source_instead_of_its_captured_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("latest");
        d["metadataSource"] = json!("peon");
        d["attentionSource"] = json!("user");
        store.create(d).await.unwrap();
        let newer=store.patch("latest".into(),json!({"metadataRevision":"1","patch":{"agentMetadata":{"workFields":{"task":"Latest agent task","nextAction":"Latest next","workPhase":"implementation","blockerDescription":null,"failedCommand":null,"failedTest":null}}}})).await.unwrap();
        let result = store
            .mutate_internal("latest".into(), "2026-01-01T00:00:00Z".into(), None, peon())
            .await
            .unwrap();
        assert!(result.document.is_none());
        assert_eq!(store.get("latest".into()).await.unwrap().unwrap(), newer);
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn protected_hook_policy_keeps_attention_while_merging_work_diagnostics() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("hook");
        d["metadataSource"] = json!("peon");
        d["observedStatus"] = json!("waiting_for_input");
        d["attention"] = json!("needs_you");
        d["attentionOrigin"] = json!("harness_hook");
        d["needsUserInput"] = json!(true);
        d["detectedQuestion"] = json!("Permission?");
        store.create(d).await.unwrap();
        let mut intent = peon();
        if let InternalMutation::Peon { policy, .. } = &mut intent {
            *policy = PeonPolicy::PreserveHook {
                status: "waiting_for_input".into(),
                confidence: 1.0,
                source: "codex_hook".into(),
            };
        }
        let result = store
            .mutate_internal("hook".into(), "2026-01-01T00:00:00Z".into(), None, intent)
            .await
            .unwrap();
        let scope = result.decision.scope.unwrap();
        assert!(!scope.attention);
        assert!(scope.work);
        let d = result.document.unwrap();
        assert_eq!(d["observedStatus"], "waiting_for_input");
        assert_eq!(d["detectedQuestion"], "Permission?");
        assert_eq!(d["blockerDescription"], "inferred blocker");
        assert_eq!(d["attentionUpdatedAt"], "2000-01-01T00:00:00Z");
        store.close().await.unwrap();
    }
    #[test]
    fn noops_survive_revision_exhaustion_but_mutations_fail_without_changing_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        let mut d = base("max");
        d["metadataRevision"] = json!("18446744073709551615");
        d["metadataSource"] = json!("user");
        d["attentionSource"] = json!("user");
        db.insert(d).unwrap();
        let before = db.get("max").unwrap();
        let noop = db
            .mutate_internal("max", "2026-01-01T00:00:00Z", None, &peon())
            .unwrap();
        assert!(noop.document.is_none());
        assert!(db
            .mutate_internal(
                "max",
                "2026-01-01T00:00:00Z",
                None,
                &InternalMutation::UserTitle {
                    title: Some("new title".into())
                }
            )
            .is_err());
        assert_eq!(db.get("max").unwrap(), before);
    }
    #[tokio::test]
    async fn invalid_and_oversized_mutations_cannot_publish_a_changed_record() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("bounded");
        d["futureExtension"] = json!("x".repeat(800_000));
        store.create(d).await.unwrap();
        let before = store.get("bounded".into()).await.unwrap();
        for title in ["", "bad\nname", &"x".repeat(101)] {
            assert!(matches!(
                store
                    .mutate_internal(
                        "bounded".into(),
                        "2026-01-01T00:00:00Z".into(),
                        None,
                        InternalMutation::UserTitle {
                            title: Some(title.into())
                        }
                    )
                    .await,
                Err(StoreError::Invalid(_))
            ));
        }
        let mut invalid = peon();
        if let InternalMutation::Peon { inference, .. } = &mut invalid {
            inference.confidence = f64::NAN;
        }
        assert!(matches!(
            store
                .mutate_internal(
                    "bounded".into(),
                    "2026-01-01T00:00:00Z".into(),
                    None,
                    invalid
                )
                .await,
            Err(StoreError::Invalid(_))
        ));
        let provider = crate::providers::ProviderObservation {
            provider_id: "p".into(),
            provider_label: "P".into(),
            provider_model: Some("x".repeat(400_000)),
            provider_state: "ready".into(),
        };
        assert!(matches!(
            store
                .mutate_internal(
                    "bounded".into(),
                    "2026-01-01T00:00:00Z".into(),
                    None,
                    InternalMutation::ProviderContext(provider)
                )
                .await,
            Err(StoreError::TooLarge)
        ));
        assert_eq!(store.get("bounded".into()).await.unwrap(), before);
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn file_and_sql_peon_policies_produce_the_same_committed_metadata() {
        use crate::metadata::{MetadataStore, PeonAttentionPolicy};
        for owned in [
            PeonPolicy::Infer,
            PeonPolicy::NonPrompt,
            PeonPolicy::PreserveAttention,
            PeonPolicy::PreserveHook {
                status: "waiting_for_input".into(),
                confidence: 1.0,
                source: "agent".into(),
            },
        ] {
            let dir = tempfile::tempdir().unwrap();
            let files = MetadataStore::new(&dir.path().join("legacy"));
            let sql = SessionStore::initialize(dir.path().join("sql"), "/workspace".into())
                .await
                .unwrap();
            let mut d = base("parity");
            d.as_object_mut().unwrap().remove("futureExtension");
            d["metadataSource"] = json!("peon");
            d["observedStatus"] = json!("waiting_for_input");
            d["attention"] = json!("needs_you");
            d["attentionOrigin"] = json!("harness_hook");
            files
                .try_write_session(&serde_json::from_value(d.clone()).unwrap())
                .unwrap();
            sql.create(d).await.unwrap();
            let borrowed = match &owned {
                PeonPolicy::Infer => PeonAttentionPolicy::Infer,
                PeonPolicy::NonPrompt => PeonAttentionPolicy::NonPrompt,
                PeonPolicy::PreserveAttention => PeonAttentionPolicy::PreserveAttention,
                PeonPolicy::PreserveHook {
                    status,
                    confidence,
                    source,
                } => PeonAttentionPolicy::PreserveHook {
                    status,
                    confidence: *confidence,
                    source,
                },
            };
            let (outcome, attention) = files
                .merge_peon_inference_with_history_policy_scoped(
                    "parity",
                    &inference(),
                    "2026-01-03T00:00:00Z",
                    None,
                    Some("classified summary"),
                    borrowed,
                )
                .unwrap();
            assert_eq!(outcome, crate::metadata::PeonMergeOutcome::Applied);
            let result = sql
                .mutate_internal(
                    "parity".into(),
                    "2026-01-01T00:00:00Z".into(),
                    None,
                    InternalMutation::Peon {
                        inference: inference(),
                        timestamp: "2026-01-03T00:00:00Z".into(),
                        provider: None,
                        history_summary: Some("classified summary".into()),
                        policy: owned,
                    },
                )
                .await
                .unwrap();
            assert_eq!(result.decision.scope.unwrap().attention, attention);
            let mut actual = result.document.unwrap();
            let mut legacy = serde_json::to_value(
                files
                    .try_read_revisioned_session("parity")
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            for field in ["attentionUpdatedAt", "workMetadataUpdatedAt"] {
                actual.as_object_mut().unwrap().remove(field);
                legacy.as_object_mut().unwrap().remove(field);
            }
            assert_eq!(actual, legacy);
            assert!(files
                .read_events("parity")
                .iter()
                .any(|e| e.event_type == "peon.inference"));
            sql.close().await.unwrap();
        }
    }
    #[tokio::test]
    async fn peon_arbitrates_latest_scopes_independently_without_overwriting_user_work() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("scoped");
        d["metadataSource"] = json!("user");
        d["task"] = json!("User task");
        d["summary"] = json!("Agent summary");
        d["summarySource"] = json!("agent");
        store.create(d).await.unwrap();
        let result = store
            .mutate_internal("scoped".into(), "2026-01-01T00:00:00Z".into(), None, peon())
            .await
            .unwrap();
        let scope = result.decision.scope.unwrap();
        assert!(scope.attention);
        assert!(!scope.work);
        let d = result.document.unwrap();
        assert_eq!(d["task"], "User task");
        assert_eq!(d["metadataSource"], "user");
        assert_eq!(d["observedStatus"], "blocked");
        assert_eq!(d["attentionSource"], "peon");
        assert_eq!(d["summary"], "Agent summary");
        assert_eq!(d["workMetadataUpdatedAt"], "2020-01-01T00:00:00Z");
        assert_eq!(d["metadataRevision"], "2");
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn internal_noop_and_stale_identity_revision_leave_bytes_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("protected");
        d["metadataSource"] = json!("user");
        d["attentionSource"] = json!("user");
        d["summarySource"] = json!("user");
        store.create(d).await.unwrap();
        let before = store.get("protected".into()).await.unwrap();
        let result = store
            .mutate_internal(
                "protected".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                peon(),
            )
            .await
            .unwrap();
        assert!(result.document.is_none());
        assert!(matches!(
            result.decision.disposition,
            Disposition::PeonSkipped {
                permanent_hold: true
            }
        ));
        assert_eq!(store.get("protected".into()).await.unwrap(), before);
        assert!(matches!(
            store
                .mutate_internal("protected".into(), "other".into(), None, peon())
                .await,
            Err(StoreError::Conflict)
        ));
        let stale = MetadataRevision::try_from("0".to_owned()).unwrap();
        assert!(matches!(
            store
                .mutate_internal(
                    "protected".into(),
                    "2026-01-01T00:00:00Z".into(),
                    Some(stale),
                    peon()
                )
                .await,
            Err(StoreError::Conflict)
        ));
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn harness_reports_preserve_exact_codex_identity_and_seen_only_revision_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = base("native");
        d["harnessId"] = json!("codex");
        store.create(d).await.unwrap();
        let report = |id: &str, time: &str| InternalMutation::HarnessReport {
            report: HarnessSessionReport {
                harness_session_id: id.into(),
                source: "codex_hook".into(),
                confidence: 1.0,
            },
            timestamp: time.into(),
            allow_codex_identity_replacement: false,
        };
        let first = store
            .mutate_internal(
                "native".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                report("native-one", "t1"),
            )
            .await
            .unwrap();
        assert!(matches!(
            first.decision.disposition,
            Disposition::HarnessReport(HarnessSessionMergeResult::Accepted)
        ));
        let denied = store
            .mutate_internal(
                "native".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                report("native-two", "t2"),
            )
            .await
            .unwrap();
        assert!(denied.document.is_none());
        assert!(matches!(
            denied.decision.disposition,
            Disposition::HarnessReport(HarnessSessionMergeResult::IgnoredIdentityChange)
        ));
        let seen = store
            .mutate_internal(
                "native".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                report("native-one", "t3"),
            )
            .await
            .unwrap();
        assert!(matches!(
            seen.decision.disposition,
            Disposition::HarnessReport(HarnessSessionMergeResult::IgnoredUnchanged)
        ));
        let d = seen.document.unwrap();
        assert_eq!(d["metadataRevision"], "3");
        assert_eq!(d["resume"]["lastSeenAt"], "t3");
        assert_eq!(d["attentionUpdatedAt"], "2000-01-01T00:00:00Z");
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn titles_and_provider_context_preserve_attention_and_update_historical_search() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut d = super::super::tests::closed_document("title");
        d["lastUserInput"] = json!("repair authentication redirect");
        d["attentionUpdatedAt"] = json!("2000-01-01T00:00:00Z");
        store.create(d).await.unwrap();
        let before = store
            .history(super::super::queries::HistoryQuery::default())
            .await
            .unwrap()
            .generation;
        let titled = store
            .mutate_internal(
                "title".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                InternalMutation::UserTitle {
                    title: Some("  Auth repair  ".into()),
                },
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(titled["label"], "Auth repair");
        assert_eq!(titled["labelSource"], "user");
        assert_eq!(titled["attentionUpdatedAt"], "2000-01-01T00:00:00Z");
        let page = store
            .history(super::super::queries::HistoryQuery {
                text: "Auth repair".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.rows.len(), 1);
        assert!(page.generation > before);
        let restored = store
            .mutate_internal(
                "title".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                InternalMutation::UserTitle { title: None },
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(restored["label"], "repair authentication redirect");
        assert_eq!(restored["labelSource"], "terminal_input");
        let provider = crate::providers::ProviderObservation {
            provider_id: "provider".into(),
            provider_label: "Provider".into(),
            provider_model: Some("model".into()),
            provider_state: "ready".into(),
        };
        let d = store
            .mutate_internal(
                "title".into(),
                "2026-01-01T00:00:00Z".into(),
                None,
                InternalMutation::ProviderContext(provider),
            )
            .await
            .unwrap()
            .document
            .unwrap();
        assert_eq!(d["modelProviderId"], "provider");
        assert_eq!(d["attentionUpdatedAt"], "2000-01-01T00:00:00Z");
        store.close().await.unwrap();
    }
}
