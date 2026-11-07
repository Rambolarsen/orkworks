//! One-shot, directory-scoped observations for retained async cleanup tests.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
type Hook = Box<dyn FnOnce() + Send>;
fn hooks() -> &'static Mutex<HashMap<PathBuf, Hook>> {
    static HOOKS: OnceLock<Mutex<HashMap<PathBuf, Hook>>> = OnceLock::new();
    HOOKS.get_or_init(Default::default)
}
pub(crate) fn on_recovered(directory: PathBuf, hook: impl FnOnce() + Send + 'static) {
    let directory = std::fs::canonicalize(directory).unwrap();
    assert!(hooks()
        .lock()
        .unwrap()
        .insert(directory, Box::new(hook))
        .is_none());
}
pub(super) fn recovered(directory: &Path) {
    let hook = hooks().lock().unwrap().remove(directory);
    if let Some(hook) = hook {
        hook();
    }
}
