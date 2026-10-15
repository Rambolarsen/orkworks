// Synthetic adapter fixtures; these do not establish live/version verification.
mod copilot_attention {
    use super::*;
    use axum::http::StatusCode;
    use chrono::{DateTime, Utc};

    struct Fixture {
        state: Arc<AppState>,
        id: String,
        _dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let state = test_app_state_with_workspace(dir.path());
            let id = format!("copilot-mapping-{}", uuid::Uuid::new_v4().simple());
            let mut session = test_session_metadata(
                &id,
                "Copilot",
                dir.path().display().to_string(),
                "running",
                "now",
                "now",
            );
            session.harness = "copilot".into();
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
            handle.info.harness = Some("copilot".into());
            handle.info.harness_id = Some("copilot".into());
            handle.info.lifecycle = "alive".into();
            handle.info.lifecycle_phase = "active".into();
            handle.info.observed_status = session.observed_status.clone();
            handle.info.attention = session.attention.clone();
            handle.info.metadata_source = Some("peon".into());
            state.sessions.lock().unwrap().insert(id.clone(), handle);
            crate::runtime::terminal_runtime::set_workflow_report_token(&id, "token".into());
            crate::runtime::prompt_authority::registry().issue_with_native_id(
                &id,
                "copilot",
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

        fn time(seconds: i64) -> String {
            DateTime::<Utc>::from_timestamp(1_800_000_000 + seconds, 0)
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
        }

        fn report(&self, event: &str, kind: Option<&str>, seconds: i64) -> serde_json::Value {
            let mut report = serde_json::json!({
                "status": if event == "userPromptSubmitted" { "working" } else { "waiting_for_input" },
                "source":"copilot_hook", "event":event,
                "harnessSessionId":"native", "promptHookGeneration":"generation",
                "observedAt":Self::time(seconds)
            });
            if let Some(kind) = kind {
                report["notificationType"] = kind.into();
            }
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

        async fn prompt(&self, kind: &str, seconds: i64) {
            assert_eq!(
                self.send(self.report("notification", Some(kind), seconds))
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
    async fn copilot_submitted_prompt_is_noop_before_activation_then_clears_wait() {
        let fixture = Fixture::new();
        let before = serde_json::to_value(fixture.saved()).unwrap();
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 0))
                .await,
            StatusCode::OK
        );
        assert_eq!(serde_json::to_value(fixture.saved()).unwrap(), before);
        assert!(!crate::runtime::prompt_authority::registry().is_active(&fixture.id));

        fixture.prompt("permission_prompt", 1).await;
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 2))
                .await,
            StatusCode::OK
        );
        let saved = fixture.saved();
        assert_eq!(saved.observed_status.as_deref(), Some("working"));
        assert_eq!(saved.attention.as_deref(), Some("working"));
        assert_eq!(saved.detected_question, None);
        assert_eq!(saved.needs_user_input, None);
        assert_eq!(saved.summary.as_deref(), Some("Independent summary"));
        assert_eq!(saved.work_phase, "review");
        let sessions = fixture.state.sessions.lock().unwrap();
        assert_eq!(
            sessions[&fixture.id].info.attention.as_deref(),
            Some("working")
        );
        assert_eq!(sessions[&fixture.id].runtime.active_prompt_kind, None);
    }

    #[tokio::test]
    async fn copilot_turn_and_prompt_reports_share_timestamp_ordering() {
        let fixture = Fixture::new();
        fixture.prompt("permission_prompt", 2).await;
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 1))
                .await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(fixture.saved().attention.as_deref(), Some("needs_you"));
        // Equal source times are ordered by the sidecar's receive sequence.
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 2))
                .await,
            StatusCode::OK
        );
        assert_eq!(fixture.saved().attention.as_deref(), Some("working"));
        fixture.prompt("elicitation_dialog", 2).await;
        assert_eq!(fixture.saved().attention.as_deref(), Some("needs_you"));
    }

    #[tokio::test]
    async fn copilot_turn_reports_require_valid_timestamp_identity_and_generation() {
        let fixture = Fixture::new();
        fixture.prompt("permission_prompt", 0).await;
        let before = serde_json::to_value(fixture.saved()).unwrap();
        for (field, value) in [
            ("observedAt", serde_json::Value::Null),
            ("observedAt", "invalid".into()),
            ("harnessSessionId", "other".into()),
            ("promptHookGeneration", "revoked".into()),
            ("status", "idle".into()),
        ] {
            let mut report = fixture.report("userPromptSubmitted", None, 1);
            report[field] = value;
            assert_eq!(
                fixture.send(report).await,
                StatusCode::BAD_REQUEST,
                "{field}"
            );
            assert_eq!(serde_json::to_value(fixture.saved()).unwrap(), before);
        }
    }

    #[tokio::test]
    async fn copilot_unmapped_events_never_clear_or_activate_attention() {
        let fixture = Fixture::new();
        for active in [false, true] {
            if active {
                fixture.prompt("permission_prompt", 0).await;
            }
            let before = serde_json::to_value(fixture.saved()).unwrap();
            for (event, kind) in [
                ("notification", Some("agent_idle")),
                ("notification", Some("agent_completed")),
                ("notification", Some("shell_completed")),
                ("notification", Some("shell_detached_completed")),
                ("notification", Some("unknown")),
                ("agentStop", None),
                ("postToolUse", None),
            ] {
                let _ = fixture.send(fixture.report(event, kind, 1)).await;
                assert_eq!(
                    serde_json::to_value(fixture.saved()).unwrap(),
                    before,
                    "{event}/{kind:?}"
                );
            }
            assert_eq!(
                crate::runtime::prompt_authority::registry().is_active(&fixture.id),
                active
            );
        }
    }

    #[tokio::test]
    async fn copilot_committed_input_fences_equal_time_reports() {
        let fixture = Fixture::new();
        fixture.prompt("permission_prompt", 0).await;
        fixture
            .state
            .sessions
            .lock()
            .unwrap()
            .get_mut(&fixture.id)
            .unwrap()
            .runtime
            .committed_input_at = Some(
            DateTime::parse_from_rfc3339(&Fixture::time(2))
                .unwrap()
                .with_timezone(&Utc),
        );
        for event in ["notification", "userPromptSubmitted"] {
            assert_eq!(
                fixture
                    .send(fixture.report(event, Some("permission_prompt"), 2))
                    .await,
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 3))
                .await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn copilot_permission_and_elicitation_keep_distinct_input_clearing() {
        for kind in ["permission_prompt", "elicitation_dialog"] {
            let fixture = Fixture::new();
            fixture.prompt(kind, 0).await;
            crate::runtime::terminal_runtime::record_terminal_input(
                &fixture.state,
                &fixture.id,
                "y",
            );
            if kind == "permission_prompt" {
                assert_eq!(fixture.saved().attention.as_deref(), Some("working"));
            } else {
                assert_eq!(fixture.saved().attention.as_deref(), Some("needs_you"));
                crate::runtime::terminal_runtime::record_terminal_input(
                    &fixture.state,
                    &fixture.id,
                    "\r",
                );
                assert_eq!(fixture.saved().attention.as_deref(), Some("working"));
            }
        }
    }

    #[tokio::test]
    async fn copilot_submission_cannot_restore_revoked_authority_or_override_user_attention() {
        let fixture = Fixture::new();
        fixture.prompt("permission_prompt", 0).await;
        {
            let workspace = fixture.state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut saved = store.read_session(&fixture.id).unwrap();
            saved.provenance.attention_source = Some("user".into());
            saved.detected_question = Some("User-owned question".into());
            store.write_session(&saved);
        }
        let before = serde_json::to_value(fixture.saved()).unwrap();
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 1))
                .await,
            StatusCode::OK
        );
        assert_eq!(serde_json::to_value(fixture.saved()).unwrap(), before);
        crate::runtime::prompt_authority::registry().revoke(&fixture.id, "generation");
        assert_eq!(
            fixture
                .send(fixture.report("userPromptSubmitted", None, 2))
                .await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(serde_json::to_value(fixture.saved()).unwrap(), before);
    }
}
