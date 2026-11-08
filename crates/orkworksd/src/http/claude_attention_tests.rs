// Synthetic adapter fixtures; these do not establish live/version verification.
mod claude_attention {
    use super::*;
    use axum::http::StatusCode;

    struct Fixture {
        state: Arc<AppState>,
        id: String,
        _dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let state = test_app_state_with_workspace(dir.path());
            let id = format!("claude-mapping-{}", uuid::Uuid::new_v4().simple());
            let mut session = test_session_metadata(
                &id,
                "Claude",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            session.harness = "claude-code".into();
            session.lifecycle = "alive".into();
            session.lifecycle_phase = "active".into();
            session.metadata_source = "peon".into();
            session.observed_status = Some("waiting_for_input".into());
            session.attention = Some("needs_you".into());
            session.needs_user_input = Some(true);
            session.detected_question = Some("Peon question".into());
            session.summary = Some("Independent summary".into());
            session.work_phase = "review".into();
            session.resume = Some(harness::ResumeMemory {
                state: harness::ResumeState::Available,
                preferred_strategy: harness::ResumeStrategy::Exact,
                harness_session_id: Some("native".into()),
                latest_fallback: false,
                last_seen_at: Some("now".into()),
            });
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .write_session(&session);
            let mut handle = attention_test_handle(&id, dir.path());
            handle.info.harness = Some("claude-code".into());
            handle.info.harness_id = Some("claude-code".into());
            handle.info.lifecycle = "alive".into();
            handle.info.lifecycle_phase = "active".into();
            handle.info.observed_status = session.observed_status.clone();
            handle.info.attention = session.attention.clone();
            handle.info.metadata_source = Some("peon".into());
            state.sessions.lock().unwrap().insert(id.clone(), handle);
            crate::runtime::terminal_runtime::set_workflow_report_token(&id, "token".into());
            crate::runtime::prompt_authority::registry().issue_with_native_id(
                &id,
                "claude-code",
                "generation",
                Some("native"),
            );
            Self {
                state,
                id,
                _dir: dir,
            }
        }

        fn saved(&self) -> metadata::SessionMetadata {
            self.state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session(&self.id)
                .unwrap()
        }

        fn report(&self, event: &str, kind: Option<&str>) -> serde_json::Value {
            let status = match (event, kind) {
                ("UserPromptSubmit", _) | (_, Some("elicitation_response" | "elicitation_complete")) => "working",
                (_, Some("idle_prompt")) => "idle",
                _ => "waiting_for_input",
            };
            let mut report = serde_json::json!({
                "status":status, "source":"claude_hook", "event":event,
                "harnessSessionId":"native", "promptHookGeneration":"generation"
            });
            if let Some(kind) = kind { report["notificationType"] = kind.into(); }
            report
        }

        async fn send(&self, report: serde_json::Value) -> StatusCode {
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::AUTHORIZATION,
                "Bearer token".parse().unwrap(),
            );
            report_attention_route(
                State(self.state.clone()),
                Path(self.id.clone()),
                headers,
                Json(report),
            )
            .await
            .status()
        }

        async fn prompt(&self, kind: &str) {
            assert_eq!(
                self.send(self.report("Notification", Some(kind)))
                    .await,
                StatusCode::OK
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            crate::runtime::prompt_authority::registry().remove(&self.id);
            crate::runtime::terminal_runtime::clear_workflow_report_token(&self.id);
        }
    }


    #[tokio::test]
    async fn claude_submission_is_noop_before_activation_then_clears_wait() {
        let f = Fixture::new();
        let before = serde_json::to_value(f.saved()).unwrap();
        assert_eq!(f.send(f.report("UserPromptSubmit", None)).await, StatusCode::OK);
        assert_eq!(serde_json::to_value(f.saved()).unwrap(), before);
        assert!(!crate::runtime::prompt_authority::registry().is_active(&f.id));
        f.prompt("permission_prompt").await;
        assert_eq!(f.send(f.report("UserPromptSubmit", None)).await, StatusCode::OK);
        let saved = f.saved();
        assert_eq!(saved.attention.as_deref(), Some("working"));
        assert_eq!(saved.summary.as_deref(), Some("Independent summary"));
        assert_eq!(saved.work_phase, "review");
        assert!(saved.needs_user_input.is_none() && saved.detected_question.is_none());
        assert!(f.state.sessions.lock().unwrap()[&f.id].runtime.active_prompt_kind.is_none());
    }

    #[tokio::test]
    async fn claude_idle_and_clear_notifications_do_not_activate_authority() {
        for kind in ["idle_prompt", "elicitation_response", "elicitation_complete"] {
            let f = Fixture::new();
            let before = serde_json::to_value(f.saved()).unwrap();
            assert_eq!(f.send(f.report("Notification", Some(kind))).await, StatusCode::OK);
            assert_eq!(serde_json::to_value(f.saved()).unwrap(), before);
            assert!(!crate::runtime::prompt_authority::registry().is_active(&f.id));
        }
    }

    #[tokio::test]
    async fn claude_idle_clears_permission_but_preserves_elicitation() {
        for kind in ["permission_prompt", "elicitation_dialog", "elicitation_url_dialog"] {
            let f = Fixture::new(); f.prompt(kind).await;
            assert_eq!(f.send(f.report("Notification", Some("idle_prompt"))).await, StatusCode::OK);
            assert_eq!(f.saved().attention.as_deref(), Some(if kind == "permission_prompt" { "idle" } else { "needs_you" }));
        }
    }

    #[tokio::test]
    async fn claude_clear_requires_one_matching_elicitation() {
        for (open, clear, wrong) in [
            ("elicitation_dialog", "elicitation_response", "elicitation_complete"),
            ("elicitation_url_dialog", "elicitation_complete", "elicitation_response")
        ] {
            let f = Fixture::new(); f.prompt(open).await;
            assert_eq!(f.send(f.report("Notification", Some(wrong))).await, StatusCode::OK);
            assert_eq!(f.saved().attention.as_deref(), Some("needs_you"));
            assert_eq!(f.send(f.report("Notification", Some(clear))).await, StatusCode::OK);
            assert_eq!(f.saved().attention.as_deref(), Some("working"));
            let before = serde_json::to_value(f.saved()).unwrap();
            assert_eq!(f.send(f.report("Notification", Some(clear))).await, StatusCode::OK);
            assert_eq!(serde_json::to_value(f.saved()).unwrap(), before, "zero pending elicitations is a no-op");
        }
    }

    #[tokio::test]
    async fn claude_parallel_and_mixed_prompts_remain_waiting_until_submission() {
        for opens in [
            ["elicitation_dialog", "elicitation_dialog"],
            ["elicitation_url_dialog", "elicitation_url_dialog"],
            ["elicitation_dialog", "elicitation_url_dialog"],
            ["permission_prompt", "elicitation_dialog"],
            ["elicitation_dialog", "permission_prompt"],
        ] {
            let f = Fixture::new();
            for open in opens { f.prompt(open).await; }
            for clear in ["elicitation_response", "elicitation_complete", "idle_prompt"] {
                assert_eq!(f.send(f.report("Notification", Some(clear))).await, StatusCode::OK);
                assert_eq!(f.saved().attention.as_deref(), Some("needs_you"), "{opens:?}/{clear}");
            }
            crate::runtime::terminal_runtime::record_terminal_input(&f.state, &f.id, "y");
            assert_eq!(f.saved().attention.as_deref(), Some("needs_you"));
            assert_eq!(f.send(f.report("UserPromptSubmit", None)).await, StatusCode::OK);
            assert_eq!(f.saved().attention.as_deref(), Some("working"));
            f.prompt("elicitation_dialog").await;
            assert_eq!(f.send(f.report("Notification", Some("elicitation_response"))).await, StatusCode::OK);
            assert_eq!(f.saved().attention.as_deref(), Some("working"), "submission resets ambiguity");
        }
    }

    #[tokio::test]
    async fn claude_unmapped_and_unauthorized_events_never_clear_or_activate() {
        let f = Fixture::new();
        for active in [false, true] {
            if active { f.prompt("permission_prompt").await; }
            let before = serde_json::to_value(f.saved()).unwrap();
            for (event, kind) in [
                ("Notification", Some("agent_needs_input")), ("Notification", Some("unknown")),
                ("Notification", Some("auth_success")), ("Notification", Some("quota")),
                ("Stop", None), ("PreToolUse", None), ("PostToolUse", None),
            ] {
                assert_eq!(f.send(f.report(event, kind)).await, StatusCode::BAD_REQUEST);
                assert_eq!(serde_json::to_value(f.saved()).unwrap(), before);
            }
            assert_eq!(crate::runtime::prompt_authority::registry().is_active(&f.id), active);
        }
        for (field, value) in [("harnessSessionId", "other"), ("promptHookGeneration", "revoked")] {
            let mut r=f.report("UserPromptSubmit",None); r[field]=value.into();
            let before=serde_json::to_value(f.saved()).unwrap();
            assert_eq!(f.send(r).await, StatusCode::BAD_REQUEST);
            assert_eq!(serde_json::to_value(f.saved()).unwrap(),before);
        }
    }

    #[tokio::test]
    async fn claude_accepted_input_preserves_permission_and_elicitation_semantics() {
        for kind in ["permission_prompt", "elicitation_dialog", "elicitation_url_dialog"] {
            let f=Fixture::new(); f.prompt(kind).await;
            crate::runtime::terminal_runtime::record_terminal_input(&f.state,&f.id,"y");
            if kind == "permission_prompt" { assert_eq!(f.saved().attention.as_deref(),Some("working")); }
            else {
                assert_eq!(f.saved().attention.as_deref(),Some("needs_you"));
                crate::runtime::terminal_runtime::record_terminal_input(&f.state,&f.id,"\r");
                assert_eq!(f.saved().attention.as_deref(),Some("working"));
            }
        }
    }
    #[tokio::test]
    async fn claude_failed_clear_persistence_preserves_the_live_prompt_for_retry() {
        let f=Fixture::new(); f.prompt("elicitation_dialog").await;
        let before=serde_json::to_value(f.saved()).unwrap();
        let kind=f.state.sessions.lock().unwrap()[&f.id].runtime.active_prompt_kind;
        let blocked=f.state.workspace.lock().unwrap().as_ref().unwrap().metadata.sessions_dir().join(format!("{}.json.tmp",f.id));
        std::fs::create_dir(&blocked).unwrap();
        assert_eq!(f.send(f.report("Notification",Some("elicitation_response"))).await,StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(serde_json::to_value(f.saved()).unwrap(),before);
        assert_eq!(f.state.sessions.lock().unwrap()[&f.id].runtime.active_prompt_kind,kind);
        std::fs::remove_dir(&blocked).unwrap();
        assert_eq!(f.send(f.report("Notification",Some("elicitation_response"))).await,StatusCode::OK);
        assert_eq!(f.saved().attention.as_deref(),Some("working"));
    }

    #[tokio::test]
    async fn claude_turns_preserve_user_owned_attention_and_reject_revoked_generation() {
        let f=Fixture::new(); f.prompt("elicitation_dialog").await;
        {
            let workspace=f.state.workspace.lock().unwrap();
            let store=&workspace.as_ref().unwrap().metadata;
            let mut saved=store.read_session(&f.id).unwrap();
            saved.provenance.attention_source=Some("user".into());
            saved.detected_question=Some("User-owned question".into());
            store.write_session(&saved);
        }
        let before=serde_json::to_value(f.saved()).unwrap();
        for (event, kind) in [("UserPromptSubmit",None),("Notification",Some("elicitation_response")),("Notification",Some("idle_prompt"))] {
            assert_eq!(f.send(f.report(event,kind)).await,StatusCode::OK);
            assert_eq!(serde_json::to_value(f.saved()).unwrap(),before);
        }
        crate::runtime::prompt_authority::registry().revoke(&f.id,"generation");
        assert_eq!(f.send(f.report("UserPromptSubmit",None)).await,StatusCode::BAD_REQUEST);
        assert_eq!(serde_json::to_value(f.saved()).unwrap(),before);
    }

}
