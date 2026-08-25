use crate::{session_types::SessionInfo, AppState};
use std::sync::Arc;

/// Coordinates session-list projections without owning session or metadata state.
pub(crate) struct SessionProjection {
    state: Arc<AppState>,
}

impl SessionProjection {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub(crate) fn list(&self) -> Vec<SessionInfo> {
        let _ = &self.state;
        Vec::new()
    }
}
