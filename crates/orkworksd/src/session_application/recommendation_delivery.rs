//! Settle acknowledged delivery without releasing its in-flight protection on contention.
use super::*;

impl SessionApplication {
    pub(super) async fn settle_recommendation_delivery(
        &self,
        id: &str,
        title: &str,
        target: &RecommendationDeliveryTarget,
        delivered: bool,
    ) -> Result<Option<Recommendation>, RecommendationAcceptError> {
        loop {
            let wait_for = {
                let workspace_guard = self.state.workspace.lock().unwrap();
                let workspace = workspace_guard
                    .as_ref()
                    .filter(|workspace| target.workspace.matches(workspace))
                    .ok_or(RecommendationAcceptError::Conflict)?;
                let result = if delivered {
                    workspace
                        .recommendation_store
                        .complete_execution(id, iso_now())
                } else {
                    workspace
                        .recommendation_store
                        .cancel_execution(id, iso_now())
                };
                match result {
                    Err(crate::taskmaster::store::StoreError::Busy) => {
                        workspace.recommendation_store.clone()
                    }
                    result => {
                        let current = result.map_err(RecommendationAcceptError::Store)?;
                        if delivered {
                            let current = current
                                .as_ref()
                                .ok_or(RecommendationAcceptError::Conflict)?;
                            let summary = if current.assessment.is_some() {
                                "Taskmaster sent an approved assessment fix prompt.".to_string()
                            } else {
                                format!("Taskmaster sent a fix prompt for: {title}")
                            };
                            // Busy retries have no event effect. Once acceptance
                            // commits, append exactly one best-effort delivery event.
                            workspace.metadata.append_event(
                                &target.session_id,
                                &metadata::Event {
                                    event_type: "taskmaster_fix_requested".into(),
                                    timestamp: iso_now(),
                                    status: "working".into(),
                                    observed_status: Some("working".into()),
                                    confidence: None,
                                    summary: Some(summary),
                                    source: Some("user".into()),
                                    recommendation_id: Some(id.into()),
                                },
                            );
                        }
                        return Ok(current);
                    }
                }
            };
            // The caller retains its original delivery guard/target while this
            // yields. No app mutex or store permit crosses this await. Only
            // owner contention is retried; filesystem failures remain errors.
            wait_for.wait_until_available().await;
        }
    }
}
