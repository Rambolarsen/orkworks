//! Scoped monotonic time for one isolated native SQL test process.
use std::sync::Mutex;
use std::time::{Duration, Instant};

static NOW: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn now() -> Instant {
    let sampled = *NOW.lock().unwrap();
    sampled.unwrap_or_else(Instant::now)
}

pub(crate) struct Clock {
    _private: (),
}

impl Clock {
    pub(crate) fn scoped() -> Self {
        let case = std::env::var("ORKWORKS_NATIVE_SQL_TEST_CASE")
            .expect("native fixture clock requires an isolated case");
        let args: Vec<_> = std::env::args().skip(1).collect();
        assert!(
            args.iter().any(|arg| arg == "--exact")
                && args.iter().any(|arg| arg == &case)
                && args
                    .iter()
                    .all(|arg| arg == "--exact" || arg == "--nocapture" || arg == &case),
            "native fixture clock requires one exact isolated case"
        );
        let mut now = NOW.lock().unwrap();
        if now.is_some() {
            drop(now);
            panic!("native fixture clock is already owned");
        }
        *now = Some(Instant::now());
        Self { _private: () }
    }

    pub(crate) fn now(&self) -> Instant {
        NOW.lock().unwrap().expect("native fixture clock is owned")
    }

    pub(crate) fn advance(&self, duration: Duration) {
        let mut now = NOW.lock().unwrap();
        *now.as_mut().expect("native fixture clock is owned") += duration;
    }
}

impl Drop for Clock {
    fn drop(&mut self) {
        NOW.lock().unwrap().take();
    }
}
