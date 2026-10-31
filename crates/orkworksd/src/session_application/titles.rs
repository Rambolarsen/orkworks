//! User-owned title operations. Label epochs fence delayed automatic work.
use super::{SessionApplication, SessionError, SessionTransactionError, WriteScope};
use crate::{metadata::LabelSource, peon};

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TitleAction {
    Read,
    Set,
    Automatic,
}

#[derive(serde::Serialize)]
pub(crate) struct SessionTitle {
    pub(crate) title: String,
    pub(crate) automatic: bool,
}

impl SessionApplication {
    pub(crate) fn session_title(
        &self,
        id: &str,
        expected_workspace: &str,
        action: TitleAction,
        title: Option<String>,
    ) -> Result<SessionTitle, SessionError> {
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err(SessionError::BadRequest("Invalid session ID."));
        }
        let title = match (&action, title) {
            (TitleAction::Set, Some(title))
                if !title.chars().any(char::is_control)
                    && !title.trim().is_empty()
                    && title.trim().chars().count() <= 100 =>
            {
                Some(title.trim().to_owned())
            }
            (TitleAction::Read | TitleAction::Automatic, None) => None,
            _ => {
                return Err(SessionError::BadRequest(
                    "Title must contain 1–100 characters without control characters.",
                ))
            }
        };
        let changing = !matches!(action, TitleAction::Read);
        // Match the native gate -> epoch -> workspace -> sessions lock ordering.
        if changing {
            crate::codex_session_store::invalidate_label_refresh_generation(id);
        }
        let mut epochs = self.state.peon.label_epochs.write().unwrap();
        let ws = self.state.workspace.lock().unwrap();
        let ws = ws.as_ref().ok_or(SessionError::Conflict)?;
        if ws
            .path
            .canonicalize()
            .map_err(|_| SessionError::Conflict)?
            .to_string_lossy()
            != expected_workspace
        {
            return Err(SessionError::Conflict);
        }
        if !changing {
            let meta = ws.metadata.read_session(id).ok_or(SessionError::NotFound)?;
            return Ok(SessionTitle {
                title: meta.label,
                automatic: meta.label_source != LabelSource::User,
            });
        }
        let mut selected = None;
        let saved = ws
            .metadata
            .mutate_session_optional(id, WriteScope::NONE, |meta| {
                let (label, source) = if let Some(title) = &title {
                    (title.clone(), LabelSource::User)
                } else {
                    meta.last_user_input
                        .as_deref()
                        .filter(|input| peon::is_descriptive_input(input))
                        .map(|input| {
                            (
                                input.chars().take(100).collect(),
                                LabelSource::TerminalInput,
                            )
                        })
                        .unwrap_or_else(|| {
                            (
                                crate::session_types::placeholder_label(id),
                                LabelSource::Placeholder,
                            )
                        })
                };
                meta.label = label.clone();
                meta.label_source = source;
                meta.label_from_initial_prompt = false;
                meta.label_context_refined = false;
                selected = Some(SessionTitle {
                    title: label,
                    automatic: source != LabelSource::User,
                });
                Ok(true)
            })
            .map_err(|error| match error {
                SessionTransactionError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    SessionError::NotFound
                }
                _ => SessionError::Internal("Could not save session title."),
            })?;
        if saved.is_none() {
            return Err(SessionError::NotFound);
        }
        let selected = selected.ok_or(SessionError::Internal("Could not save session title."))?;
        let epoch = epochs.entry(id.to_owned()).or_default();
        *epoch = epoch.saturating_add(1);
        self.state.peon.label_hint.write().unwrap().remove(id);
        self.state.peon.label_pending.write().unwrap().remove(id);
        if let Some(handle) = self.state.sessions.lock().unwrap().get_mut(id) {
            handle.info.label = selected.title.clone();
            if selected.automatic && peon::is_descriptive_input(&selected.title) {
                self.state.peon.label_hint.write().unwrap().insert(
                    id.to_owned(),
                    crate::LabelHint {
                        text: selected.title.clone(),
                        epoch: *epoch,
                        from_initial_prompt: false,
                        context: Vec::new(),
                    },
                );
                self.state
                    .peon
                    .label_pending
                    .write()
                    .unwrap()
                    .insert(id.to_owned());
            }
        }
        self.state.updates.sessions_changed();
        Ok(selected)
    }

    pub(crate) fn queue_contextual_title(
        &self,
        attempt: &crate::runtime::peon_runtime::AttemptContextView,
        capture: &crate::runtime::peon_runtime::PeonOutputCaptureIdentity,
        expected_epoch: u64,
        output: &[String],
    ) -> bool {
        let id = attempt.session_id();
        let epochs = self.state.peon.label_epochs.read().unwrap();
        if epochs.get(id).copied().unwrap_or(0) != expected_epoch || output.is_empty() {
            return false;
        }
        let workspace = self.state.workspace.lock().unwrap();
        let sessions = self.state.sessions.lock().unwrap();
        if !sessions.get(id).is_some_and(|h| {
            attempt.is_current_with_session(h)
                && capture.is_current(
                    h.runtime.input_generation,
                    h.runtime.min_peon_output_revision,
                    &h.runtime.identity(),
                )
        }) {
            return false;
        };
        let Some(meta) = workspace
            .as_ref()
            .and_then(|ws| ws.metadata.read_session(id))
        else {
            return false;
        };
        if meta.label_context_refined
            || !meta.label_source.is_automatic()
            || !meta.label_source.accepts_peon()
        {
            return false;
        }
        let input = meta
            .last_user_input
            .as_deref()
            .filter(|s| peon::is_descriptive_input(s))
            .unwrap_or(&meta.label);
        if !peon::is_descriptive_input(input) || super::is_placeholder_label(input, id) {
            return false;
        }
        let context: Vec<String> = output
            .iter()
            .rev()
            .take(40)
            .map(|line| line.chars().take(500).collect())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        self.state.peon.label_hint.write().unwrap().insert(
            id.to_owned(),
            crate::LabelHint {
                text: input.to_owned(),
                epoch: expected_epoch,
                from_initial_prompt: meta.label_from_initial_prompt,
                context,
            },
        );
        self.state
            .peon
            .label_pending
            .write()
            .unwrap()
            .insert(id.to_owned());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_title_survives_conversation_reset_and_automatic_restores_input() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let mut meta = crate::test_support::test_session_metadata(
            "manual-title",
            "Login",
            dir.path().display().to_string(),
            "running",
            "now",
            "now",
        );
        meta.last_user_input = Some("repair the login redirect".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        let app = SessionApplication::new(state.clone());
        let workspace = dir.path().canonicalize().unwrap().display().to_string();
        app.session_title(
            "manual-title",
            &workspace,
            TitleAction::Set,
            Some("Auth repair".into()),
        )
        .unwrap();
        assert!(app.reset_session_topic("manual-title"));
        let selected = app
            .session_title("manual-title", &workspace, TitleAction::Read, None)
            .unwrap();
        assert_eq!(selected.title, "Auth repair");
        assert!(!selected.automatic);
        let selected = app
            .session_title("manual-title", &workspace, TitleAction::Automatic, None)
            .unwrap();
        assert_eq!(selected.title, "repair the login redirect");
        assert!(selected.automatic);
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session("manual-title")
                .unwrap()
                .last_activity,
            "now"
        );
    }
}
