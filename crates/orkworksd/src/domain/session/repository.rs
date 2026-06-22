use super::{entity::Session, events::DomainEvent, value_objects::*};

pub trait SessionRepository: Send + Sync {
    fn save(&self, session: &Session, events: Vec<DomainEvent>) -> Result<(), String>;
    fn load(&self, id: &SessionId) -> Result<Option<Session>, String>;
    fn delete(&self, id: &SessionId) -> Result<(), String>;
    fn list_by_workspace(&self, path: &WorkspacePath) -> Result<Vec<Session>, String>;
    fn append_terminal_output(&self, id: &SessionId, lines: Vec<String>) -> Result<(), String>;
}
