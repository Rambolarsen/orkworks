use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindResult {
    Bound,
    Unchanged,
    Held,
    Rejected,
}

#[derive(Clone, Default)]
struct ResetReservation {
    acknowledged: bool,
    epoch_committed: bool,
    candidate_native_session_id: Option<String>,
    retired_native_session_id: Option<String>,
    superseded: Option<Box<ResetReservation>>,
    created_at: chrono::DateTime<chrono::Utc>,
    queued_prompt_wait: Option<QueuedPromptWait>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QueuedPromptWait {
    pub(crate) notification_type: String,
    pub(crate) observed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) receipt_sequence: u64,
    pub(crate) committed_input_sequence: u64,
    pub(crate) cwd: Option<String>,
}

impl QueuedPromptWait {
    pub(crate) fn superseded_by_committed_input(&self, current_sequence: u64) -> bool {
        current_sequence != self.committed_input_sequence
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResetCommit {
    pub(crate) native_session_id: Option<String>,
    pub(crate) queued_prompt_wait: Option<QueuedPromptWait>,
}

#[derive(Default)]
struct SessionAuthority {
    harness_id: String,
    generation: String,
    native_session_id: Option<String>,
    revoked: bool,
    active: bool,
    epoch: u64,
    reset: Option<ResetReservation>,
    reset_retry_scheduled: bool,
}

/// Process-local prompt authority for Claude Code and GitHub Copilot CLI.
///
/// Authority is deliberately not persisted. A sidecar restart therefore
/// cannot restore a generation or the native identity that was bound to it.
#[derive(Default)]
pub(crate) struct PromptAuthorityRegistry {
    sessions: Mutex<HashMap<String, SessionAuthority>>,
}

impl PromptAuthorityRegistry {
    #[cfg(test)]
    pub(crate) fn issue(&self, session_id: &str, harness_id: &str, generation: &str) {
        self.issue_with_native_id(session_id, harness_id, generation, None);
    }

    pub(crate) fn issue_with_native_id(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: Option<&str>,
    ) {
        self.sessions.lock().unwrap().insert(
            session_id.to_string(),
            SessionAuthority {
                harness_id: harness_id.to_string(),
                generation: generation.to_string(),
                native_session_id: native_session_id.map(str::to_owned),
                ..SessionAuthority::default()
            },
        );
    }

    pub(crate) fn generation_matches(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
    ) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| {
                !entry.revoked && entry.harness_id == harness_id && entry.generation == generation
            })
    }

    pub(crate) fn generation_for_launch(&self, session_id: &str) -> Option<(String, String)> {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .and_then(|entry| {
                (!entry.revoked).then(|| (entry.harness_id.clone(), entry.generation.clone()))
            })
    }

    pub(crate) fn register_native_id(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: &str,
        lifecycle_reset: bool,
        observed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BindResult {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return BindResult::Rejected;
        };
        if entry.revoked
            || entry.harness_id != harness_id
            || entry.generation != generation
            || native_session_id.is_empty()
        {
            return BindResult::Rejected;
        }
        match entry.native_session_id.as_deref() {
            None => {
                if entry.reset.is_none() {
                    entry.native_session_id = Some(native_session_id.to_string());
                    return BindResult::Bound;
                }
                let Some(reset) = entry.reset.as_mut() else {
                    unreachable!();
                };
                if reset.retired_native_session_id.as_deref() == Some(native_session_id) {
                    return BindResult::Rejected;
                }
                if reset.candidate_native_session_id.as_deref() == Some(native_session_id) {
                    return BindResult::Held;
                }
                if !lifecycle_reset
                    || (entry.harness_id == "copilot"
                        && observed_at.is_none_or(|timestamp| timestamp <= reset.created_at))
                {
                    return BindResult::Rejected;
                }
                match reset.candidate_native_session_id.as_deref() {
                    Some(candidate) if candidate != native_session_id => BindResult::Rejected,
                    _ => {
                        reset.candidate_native_session_id = Some(native_session_id.to_string());
                        BindResult::Held
                    }
                }
            }
            Some(current) if current == native_session_id && entry.reset.is_none() => {
                BindResult::Unchanged
            }
            Some(current)
                if current == native_session_id
                    && entry.reset.as_ref().is_some_and(|reset| {
                        reset.epoch_committed
                            && reset.candidate_native_session_id.as_deref()
                                == Some(native_session_id)
                    }) =>
            {
                BindResult::Unchanged
            }
            Some(current) if current == native_session_id => {
                if entry.reset.as_ref().is_some_and(|reset| {
                    !reset.acknowledged
                        && reset.candidate_native_session_id.as_deref() == Some(native_session_id)
                }) {
                    BindResult::Held
                } else {
                    BindResult::Rejected
                }
            }
            Some(_)
                if entry.reset.as_ref().is_some_and(|reset| {
                    reset.candidate_native_session_id.as_deref() == Some(native_session_id)
                }) =>
            {
                BindResult::Held
            }
            Some(_) if lifecycle_reset => {
                let Some(reset) = entry.reset.as_mut() else {
                    return BindResult::Rejected;
                };
                if entry.harness_id == "copilot"
                    && observed_at.is_none_or(|timestamp| timestamp <= reset.created_at)
                {
                    return BindResult::Rejected;
                }
                match reset.candidate_native_session_id.as_deref() {
                    Some(candidate) if candidate != native_session_id => {
                        return BindResult::Rejected
                    }
                    None => reset.candidate_native_session_id = Some(native_session_id.to_string()),
                    _ => {}
                }
                BindResult::Held
            }
            Some(_) => BindResult::Rejected,
        }
    }

    pub(crate) fn reserve_reset(&self, session_id: &str, harness_id: &str, command: &str) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return false;
        };
        let accepted_command = match harness_id {
            "claude-code" => command == "/clear",
            "copilot" => matches!(command, "/clear" | "/new"),
            _ => false,
        };
        let superseded = entry.reset.as_ref().filter(|reset| {
            reset.acknowledged
                && reset.epoch_committed
                && reset.candidate_native_session_id.is_none()
        });
        if entry.revoked
            || entry.harness_id != harness_id
            || (entry.native_session_id.is_none() && superseded.is_none())
            || !accepted_command
            || entry.reset.as_ref().is_some_and(|reset| {
                !(reset.acknowledged
                    && reset.epoch_committed
                    && reset.candidate_native_session_id.is_none())
            })
        {
            return false;
        }
        let retired_native_session_id = entry
            .native_session_id
            .clone()
            .or_else(|| superseded.and_then(|reset| reset.retired_native_session_id.clone()));
        let previous = superseded.cloned().map(Box::new);
        entry.reset = Some(ResetReservation {
            retired_native_session_id,
            superseded: previous,
            created_at: chrono::Utc::now(),
            ..ResetReservation::default()
        });
        true
    }

    pub(crate) fn acknowledge_reset(&self, session_id: &str) -> Option<Option<String>> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session_id)?;
        if entry.revoked {
            return None;
        }
        let reset = entry.reset.as_mut()?;
        reset.acknowledged = true;
        Some(reset.candidate_native_session_id.clone())
    }

    pub(crate) fn complete_reset(&self, session_id: &str) -> Option<ResetCommit> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session_id)?;
        let reset = entry.reset.as_mut()?;
        if !reset.acknowledged {
            return None;
        }
        if reset.candidate_native_session_id.is_none() {
            if reset.epoch_committed {
                return Some(ResetCommit {
                    native_session_id: None,
                    queued_prompt_wait: None,
                });
            }
            reset.epoch_committed = true;
            entry.epoch = entry.epoch.saturating_add(1);
            entry.native_session_id = None;
            entry.active = false;
            return Some(ResetCommit {
                native_session_id: None,
                queued_prompt_wait: None,
            });
        }
        let candidate = reset.candidate_native_session_id.clone()?;
        let queued_prompt_wait = reset.queued_prompt_wait.clone();
        entry.native_session_id = Some(candidate.clone());
        if !reset.epoch_committed {
            entry.epoch = entry.epoch.saturating_add(1);
        }
        reset.epoch_committed = true;
        if queued_prompt_wait.is_none() {
            entry.reset = None;
        }
        entry.active = false;
        Some(ResetCommit {
            native_session_id: Some(candidate),
            queued_prompt_wait,
        })
    }

    pub(crate) fn reset_acknowledged(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| entry.reset.as_ref().is_some_and(|reset| reset.acknowledged))
    }

    pub(crate) fn deactivate_for_reset(&self, session_id: &str) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(session_id) {
            if !entry.revoked && entry.reset.is_some() {
                entry.active = false;
            }
        }
    }

    pub(crate) fn finish_queued_prompt_wait(
        &self,
        session_id: &str,
        receipt_sequence: u64,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return false;
        };
        let Some(reset) = entry.reset.as_ref() else {
            return false;
        };
        if !reset.epoch_committed
            || !reset
                .queued_prompt_wait
                .as_ref()
                .is_some_and(|queued| queued.receipt_sequence == receipt_sequence)
        {
            return false;
        }
        entry.reset = None;
        true
    }

    pub(crate) fn acknowledged_candidate_matches(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: &str,
    ) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| {
                !entry.revoked
                    && entry.harness_id == harness_id
                    && entry.generation == generation
                    && entry.reset.as_ref().is_some_and(|reset| {
                        reset.acknowledged
                            && reset.candidate_native_session_id.as_deref()
                                == Some(native_session_id)
                    })
            })
    }

    pub(crate) fn queue_prompt_wait(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: &str,
        notification_type: &str,
        observed_at: Option<chrono::DateTime<chrono::Utc>>,
        receipt_sequence: u64,
        committed_input_sequence: u64,
        cwd: Option<&str>,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return false;
        };
        if entry.revoked || entry.harness_id != harness_id || entry.generation != generation {
            return false;
        }
        let Some(reset) = entry.reset.as_mut() else {
            return false;
        };
        if reset.candidate_native_session_id.as_deref() != Some(native_session_id) {
            return false;
        }
        if let Some(previous) = reset.queued_prompt_wait.as_ref() {
            match (previous.observed_at, observed_at) {
                (Some(previous), Some(current))
                    if current > previous
                        || (current == previous
                            && receipt_sequence
                                > reset
                                    .queued_prompt_wait
                                    .as_ref()
                                    .expect("queued prompt exists")
                                    .receipt_sequence) => {}
                (None, None)
                    if reset
                        .queued_prompt_wait
                        .as_ref()
                        .is_some_and(|previous| receipt_sequence > previous.receipt_sequence) => {}
                _ => return false,
            }
        }
        reset.queued_prompt_wait = Some(QueuedPromptWait {
            notification_type: notification_type.to_string(),
            observed_at,
            receipt_sequence,
            committed_input_sequence,
            cwd: cwd.filter(|cwd| !cwd.is_empty()).map(str::to_owned),
        });
        true
    }

    pub(crate) fn cancel_reset(&self, session_id: &str) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(session_id) {
            entry.reset = entry
                .reset
                .take()
                .and_then(|reset| reset.superseded.map(|old| *old));
        }
    }

    pub(crate) fn native_id_matches(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: &str,
    ) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| {
                !entry.revoked
                    && entry.harness_id == harness_id
                    && entry.generation == generation
                    && (entry.reset.is_none()
                        || entry.reset.as_ref().is_some_and(|reset| {
                            reset.epoch_committed
                                && reset.candidate_native_session_id.as_deref()
                                    == Some(native_session_id)
                        }))
                    && entry.native_session_id.as_deref() == Some(native_session_id)
            })
    }

    pub(crate) fn activate(
        &self,
        session_id: &str,
        harness_id: &str,
        generation: &str,
        native_session_id: &str,
    ) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return false;
        };
        if entry.revoked
            || entry.harness_id != harness_id
            || entry.generation != generation
            || entry.native_session_id.as_deref() != Some(native_session_id)
        {
            return false;
        }
        entry.active = true;
        true
    }

    pub(crate) fn is_active(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| entry.active && !entry.revoked)
    }

    pub(crate) fn identity_reset_pending(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| entry.reset.is_some())
    }

    pub(crate) fn reset_commit_pending(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .is_some_and(|entry| {
                entry.reset.as_ref().is_some_and(|reset| {
                    reset.acknowledged
                        && (!reset.epoch_committed || reset.queued_prompt_wait.is_some())
                })
            })
    }

    pub(crate) fn schedule_reset_retry(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(session_id) else {
            return false;
        };
        let pending = entry.reset.as_ref().is_some_and(|reset| {
            reset.acknowledged && (!reset.epoch_committed || reset.queued_prompt_wait.is_some())
        });
        if !pending || entry.reset_retry_scheduled {
            return false;
        }
        entry.reset_retry_scheduled = true;
        true
    }

    pub(crate) fn finish_reset_retry(&self, session_id: &str) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(session_id) {
            entry.reset_retry_scheduled = false;
        }
    }

    pub(crate) fn revoke(&self, session_id: &str, generation: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(entry) = sessions.get_mut(session_id) {
            if entry.generation == generation {
                entry.revoked = true;
                entry.active = false;
                entry.reset = None;
            }
        }
    }

    pub(crate) fn remove(&self, session_id: &str) {
        self.sessions.lock().unwrap().remove(session_id);
    }

    pub(crate) fn remove_if_generation(&self, session_id: &str, generation: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions
            .get(session_id)
            .is_some_and(|entry| entry.generation == generation)
        {
            sessions.remove(session_id);
        }
    }

    pub(crate) fn epoch(&self, session_id: &str) -> Option<u64> {
        self.sessions
            .lock()
            .unwrap()
            .get(session_id)
            .map(|entry| entry.epoch)
    }
}

pub(crate) fn registry() -> &'static PromptAuthorityRegistry {
    static REGISTRY: std::sync::OnceLock<PromptAuthorityRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(PromptAuthorityRegistry::default)
}

pub(crate) fn transition_lock() -> &'static Mutex<()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::{BindResult, PromptAuthorityRegistry};

    #[test]
    fn acknowledged_reset_without_candidate_commits_and_accepts_late_lifecycle_id() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));

        assert!(registry.acknowledge_reset("session").is_some());
        assert!(registry.complete_reset("session").is_some());
        assert_eq!(registry.epoch("session"), Some(1));
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "old", true, None,),
            BindResult::Rejected,
            "a delayed lifecycle event cannot restore the retired identity"
        );

        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None,),
            BindResult::Held,
            "the acknowledged reset must retain a slot for its matching lifecycle report"
        );
        assert!(registry.acknowledged_candidate_matches(
            "session",
            "claude-code",
            "generation",
            "new"
        ));
        assert!(registry.complete_reset("session").is_some());
        assert_eq!(registry.epoch("session"), Some(1));
        assert!(registry.native_id_matches("session", "claude-code", "generation", "new"));
    }

    #[test]
    fn candidate_free_reset_can_be_superseded_and_cancel_restores_reservation() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert!(registry.acknowledge_reset("session").is_some());
        assert!(registry.complete_reset("session").is_some());

        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        registry.cancel_reset("session");
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "old", true, None),
            BindResult::Rejected,
            "cancel restores the already-committed candidate-free reset"
        );
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None),
            BindResult::Held
        );
    }

    #[test]
    fn second_reset_reservation_cannot_restore_retired_id_or_bind_before_ack() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert!(registry.acknowledge_reset("session").is_some());
        assert!(registry.complete_reset("session").is_some());

        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert_eq!(
            registry
                .register_native_id("session", "claude-code", "generation", "old", false, None,),
            BindResult::Rejected,
            "ordinary registration cannot restore an identity retired by the first reset"
        );
        assert_eq!(
            registry.register_native_id(
                "session",
                "claude-code",
                "generation",
                "replacement",
                true,
                None,
            ),
            BindResult::Held,
            "a lifecycle candidate must stay held until the second reset is acknowledged"
        );
    }

    #[test]
    fn ordinary_registration_preserves_an_already_held_reset_candidate() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert!(registry.acknowledge_reset("session").is_some());
        assert!(registry.complete_reset("session").is_some());
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None),
            BindResult::Held
        );

        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", false, None),
            BindResult::Held,
            "the same held candidate may be re-reported without lifecycle fields"
        );
        assert_eq!(
            registry.register_native_id(
                "session",
                "claude-code",
                "generation",
                "other",
                false,
                None
            ),
            BindResult::Rejected
        );
    }

    #[test]
    fn revocation_invalidates_acknowledged_and_in_flight_reset_candidates() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None,),
            BindResult::Held
        );

        registry.revoke("session", "generation");

        assert!(registry.acknowledge_reset("session").is_none());
        assert!(registry.complete_reset("session").is_none());
        assert!(!registry.acknowledged_candidate_matches(
            "session",
            "claude-code",
            "generation",
            "new"
        ));
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None,),
            BindResult::Rejected
        );
    }

    #[test]
    fn committed_reset_retains_queued_prompt_until_persistence_is_confirmed() {
        let registry = PromptAuthorityRegistry::default();
        registry.issue_with_native_id("session", "claude-code", "generation", Some("old"));
        assert!(registry.reserve_reset("session", "claude-code", "/clear"));
        assert_eq!(
            registry.register_native_id("session", "claude-code", "generation", "new", true, None,),
            BindResult::Held
        );
        assert!(registry.acknowledge_reset("session").is_some());
        assert!(registry.queue_prompt_wait(
            "session",
            "claude-code",
            "generation",
            "new",
            "permission_prompt",
            None,
            1,
            0,
            None,
        ));

        let first = registry.complete_reset("session").unwrap();
        let retry = registry.complete_reset("session").unwrap();

        assert_eq!(first, retry);
        assert_eq!(
            retry.queued_prompt_wait.unwrap().notification_type,
            "permission_prompt"
        );
        assert!(registry.native_id_matches("session", "claude-code", "generation", "new"));
        assert!(registry.finish_queued_prompt_wait("session", 1));
        assert!(!registry.identity_reset_pending("session"));
    }

    #[test]
    fn queued_prompt_wait_detects_user_input_after_receipt() {
        let queued = super::QueuedPromptWait {
            notification_type: "permission_prompt".into(),
            observed_at: None,
            receipt_sequence: 1,
            committed_input_sequence: 4,
            cwd: None,
        };
        assert!(!queued.superseded_by_committed_input(4));
        assert!(queued.superseded_by_committed_input(5));
    }
}
