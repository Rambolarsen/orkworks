use super::*;
use crate::metadata::internal_mutations::HookTransition;
use crate::session_store::worker::SessionStore;
use serde_json::json;
const CREATED: &str = "2026-01-01T00:00:00Z";
const AT: &str = "2026-01-03T00:00:00Z";
fn record() -> serde_json::Value {
    let mut value = crate::session_store::tests::document("hook-relay");
    value["harnessId"] = json!("codex");
    value["harnessSessionIdSource"] = json!("codex_hook");
    value["resume"] = json!({"state":"available","preferredStrategy":"exact","harnessSessionId":"root","latestFallback":false});
    value["lifecycle"] = json!("alive");
    value["lifecyclePhase"] = json!("active");
    value["observedStatus"] = json!("blocked");
    value["attention"] = json!("needs_you");
    value["attentionSource"] = json!("agent");
    value["needsUserInput"] = json!(true);
    value["detectedQuestion"] = json!("Previous question");
    value["suggestedOptions"] = json!(["A", "B"]);
    value["workMetadataUpdatedAt"] = json!("2026-01-02T00:00:00Z");
    value
}
#[tokio::test]
async fn hook_relay_transitions_preserve_work_and_write_the_exact_attention_tuple() {
    for (transition, status) in [
        (HookTransition::Activate, None),
        (HookTransition::Stop, Some("idle")),
        (HookTransition::UserPromptSubmit, Some("working")),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(record()).await.unwrap();
        let changed = store
            .mutate_internal(
                "hook-relay".into(),
                CREATED.into(),
                None,
                InternalMutation::HookTransition {
                    root: "root".into(),
                    transition,
                    timestamp: AT.into(),
                },
            )
            .await
            .unwrap();
        let doc = changed.document.unwrap();
        assert_eq!(doc["observedStatus"], json!(status));
        assert_eq!(doc["attention"], json!(status));
        assert_eq!(doc["needsUserInput"], serde_json::Value::Null);
        assert_eq!(doc["detectedQuestion"], serde_json::Value::Null);
        assert_eq!(doc["suggestedOptions"], serde_json::Value::Null);
        assert_eq!(doc["attentionSource"], "agent");
        assert_eq!(doc["attentionOrigin"], "harness_hook");
        assert_eq!(doc["attentionConfidence"], 1.0);
        assert_eq!(doc["lastActivity"], AT);
        assert_eq!(doc["workMetadataUpdatedAt"], "2026-01-02T00:00:00Z");
        assert_eq!(doc["metadataRevision"], "2");
        assert_eq!(doc["futureExtension"]["nested"][2], "keep");
        assert!(changed.attention_proof.is_some());
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn hook_relay_fallback_retains_activity_and_cannot_overwrite_a_concrete_prompt() {
    for protected in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut value = record();
        if !protected {
            value["needsUserInput"] = serde_json::Value::Null;
            value["detectedQuestion"] = serde_json::Value::Null;
            value["suggestedOptions"] = serde_json::Value::Null;
            value["observedStatus"] = json!("working");
            value["attention"] = json!("working");
        }
        store.create(value).await.unwrap();
        let before = store.get("hook-relay".into()).await.unwrap().unwrap();
        let changed = store
            .mutate_internal(
                "hook-relay".into(),
                CREATED.into(),
                None,
                InternalMutation::HookTransition {
                    root: "root".into(),
                    transition: HookTransition::FallbackPermission,
                    timestamp: AT.into(),
                },
            )
            .await
            .unwrap();
        if protected {
            assert!(changed.document.is_none());
            assert_eq!(
                store.get("hook-relay".into()).await.unwrap().unwrap(),
                before
            );
        } else {
            let doc = changed.document.unwrap();
            assert_eq!(doc["observedStatus"], "waiting_for_input");
            assert_eq!(doc["attention"], "needs_you");
            assert_eq!(doc["attentionOrigin"], "harness_hook");
            assert_eq!(doc["attentionSource"], "agent");
            assert_eq!(doc["attentionConfidence"], 1.0);
            assert_eq!(doc["lastActivity"], before["lastActivity"]);
            assert_eq!(doc["metadataRevision"], "2");
        }
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn hook_relay_transition_rejects_newer_user_or_native_identity_without_a_write() {
    for kind in ["user", "root", "source", "harness", "ended", "ending"] {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut value = record();
        match kind {
            "user" => value["attentionSource"] = json!("user"),
            "root" => value["resume"]["harnessSessionId"] = json!("replacement"),
            "source" => value["harnessSessionIdSource"] = json!("agent"),
            "harness" => {
                value["harnessId"] = json!("claude");
            }
            "ended" => {
                value["lifecycle"] = json!("dead");
                value["lifecyclePhase"] = json!("ended");
            }
            "ending" => value["lifecyclePhase"] = json!("ending"),
            _ => unreachable!(),
        }
        store.create(value).await.unwrap();
        let before = store.get("hook-relay".into()).await.unwrap().unwrap();
        assert!(
            store
                .mutate_internal(
                    "hook-relay".into(),
                    CREATED.into(),
                    None,
                    InternalMutation::HookTransition {
                        root: "root".into(),
                        transition: HookTransition::Stop,
                        timestamp: AT.into()
                    }
                )
                .await
                .unwrap()
                .document
                .is_none(),
            "{kind}"
        );
        assert_eq!(
            store.get("hook-relay".into()).await.unwrap().unwrap(),
            before,
            "{kind}"
        );
        store.close().await.unwrap();
    }
}
