use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindResult {
    Bound,
    Unchanged,
    Held,
    Rejected,
}

#[derive(Default)]
struct ResetReservation {
    acknowledged: bool,
    candidate_native_session_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    queued_prompt_wait: Option<QueuedPromptWait>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QueuedPromptWait {
    pub(crate) notification_type: String,
    pub(crate) observed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) cwd: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResetCommit {
    pub(crate) native_session_id: String,
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
                entry.native_session_id = Some(native_session_id.to_string());
                BindResult::Bound
            }
            Some(current) if current == native_session_id && entry.reset.is_none() => {
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
        if entry.revoked
            || entry.harness_id != harness_id
            || entry.native_session_id.is_none()
            || !accepted_command
            || entry.reset.as_ref().is_some_and(|reset| {
                !reset.acknowledged || reset.candidate_native_session_id.is_some()
            })
        {
            return false;
        }
        entry.reset = Some(ResetReservation {
            created_at: chrono::Utc::now(),
            ..ResetReservation::default()
        });
        true
    }

    pub(crate) fn acknowledge_reset(&self, session_id: &str) -> Option<String> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session_id)?;
        let reset = entry.reset.as_mut()?;
        reset.acknowledged = true;
        reset.candidate_native_session_id.clone()
    }

    pub(crate) fn complete_reset(&self, session_id: &str) -> Option<ResetCommit> {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(session_id)?;
        let reset = entry.reset.as_mut()?;
        if !reset.acknowledged {
            return None;
        }
        let candidate = reset.candidate_native_session_id.take()?;
        let queued_prompt_wait = reset.queued_prompt_wait.take();
        entry.native_session_id = Some(candidate.clone());
        entry.epoch = entry.epoch.saturating_add(1);
        entry.reset = None;
        entry.active = false;
        Some(ResetCommit {
            native_session_id: candidate,
            queued_prompt_wait,
        })
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
                (Some(previous), Some(current)) if current > previous => {}
                (None, None) => {}
                _ => return false,
            }
        }
        reset.queued_prompt_wait = Some(QueuedPromptWait {
            notification_type: notification_type.to_string(),
            observed_at,
            cwd: cwd.filter(|cwd| !cwd.is_empty()).map(str::to_owned),
        });
        true
    }

    pub(crate) fn cancel_reset(&self, session_id: &str) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(session_id) {
            entry.reset = None;
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
                    && entry.reset.is_none()
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

    pub(crate) fn revoke(&self, session_id: &str, generation: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(entry) = sessions.get_mut(session_id) {
            if entry.generation == generation {
                entry.revoked = true;
                entry.active = false;
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
