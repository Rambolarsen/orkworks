//! Scoped ownership of the process-wide authorization token in async tests.
static TOKEN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) struct TokenGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    previous: Option<std::ffi::OsString>,
}
pub(crate) async fn set(token: &str) -> TokenGuard {
    let lock = TOKEN_LOCK.lock().await;
    let previous = std::env::var_os("ORKWORKS_OPEN_PLAN_TOKEN");
    std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", token);
    TokenGuard {
        _lock: lock,
        previous,
    }
}
impl Drop for TokenGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", value),
            None => std::env::remove_var("ORKWORKS_OPEN_PLAN_TOKEN"),
        }
    }
}
