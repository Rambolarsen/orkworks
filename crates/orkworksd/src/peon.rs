use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct PeonConfig {
    pub harness: String,
    pub harness_args: String,
    pub model: Option<String>,
    pub interval_secs: u64,
    pub max_lines: usize,
    pub timeout_secs: u64,
    pub enabled: bool,
}

impl PeonConfig {
    pub fn from_env() -> Self {
        Self {
            harness: std::env::var("PEON_HARNESS").unwrap_or_else(|_| "opencode".into()),
            harness_args: std::env::var("PEON_HARNESS_ARGS").unwrap_or_else(|_| "--print -p".into()),
            model: std::env::var("PEON_MODEL").ok(),
            interval_secs: match std::env::var("PEON_INTERVAL") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_INTERVAL is not a valid number, using default 5");
                        5
                    }
                },
                Err(_) => 5,
            },
            max_lines: match std::env::var("PEON_MAX_LINES") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_MAX_LINES is not a valid number, using default 200");
                        200
                    }
                },
                Err(_) => 200,
            },
            timeout_secs: match std::env::var("PEON_TIMEOUT") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_TIMEOUT is not a valid number, using default 30");
                        30
                    }
                },
                Err(_) => 30,
            },
            enabled: std::env::var("PEON_ENABLED")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RingBuffer {
    lines: VecDeque<String>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { lines: VecDeque::new(), capacity }
    }

    pub fn push(&mut self, line: String) {
        self.lines.push_back(line);
        while self.lines.len() > self.capacity {
            self.lines.pop_front();
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_ring_buffer_push_and_snapshot() {
        let mut buf = RingBuffer::new(3);
        buf.push("line1".into());
        buf.push("line2".into());
        let snapshot = buf.snapshot();
        assert_eq!(snapshot, vec!["line1", "line2"]);
    }

    #[test]
    fn test_ring_buffer_capacity_enforcement() {
        let mut buf = RingBuffer::new(2);
        buf.push("a".into());
        buf.push("b".into());
        buf.push("c".into());
        let snapshot = buf.snapshot();
        assert_eq!(snapshot, vec!["b", "c"]);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_ring_buffer_empty() {
        let buf = RingBuffer::new(5);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        let snapshot = buf.snapshot();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn test_peon_config_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("PEON_ENABLED");
        std::env::remove_var("PEON_HARNESS");
        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::remove_var("PEON_MODEL");
        std::env::remove_var("PEON_INTERVAL");
        std::env::remove_var("PEON_MAX_LINES");
        std::env::remove_var("PEON_TIMEOUT");

        let config = PeonConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.harness, "opencode");
        assert_eq!(config.harness_args, "--print -p");
        assert!(config.model.is_none());
        assert_eq!(config.interval_secs, 5);
        assert_eq!(config.max_lines, 200);
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_peon_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::set_var("PEON_ENABLED", "false");
        std::env::set_var("PEON_HARNESS", "claude");
        std::env::set_var("PEON_HARNESS_ARGS", "-p --print");
        std::env::set_var("PEON_MODEL", "haiku");
        std::env::set_var("PEON_INTERVAL", "10");
        std::env::set_var("PEON_MAX_LINES", "100");
        std::env::set_var("PEON_TIMEOUT", "60");

        let config = PeonConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.harness, "claude");
        assert_eq!(config.harness_args, "-p --print");
        assert_eq!(config.model, Some("haiku".into()));
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.max_lines, 100);
        assert_eq!(config.timeout_secs, 60);

        std::env::remove_var("PEON_ENABLED");
        std::env::remove_var("PEON_HARNESS");
        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::remove_var("PEON_MODEL");
        std::env::remove_var("PEON_INTERVAL");
        std::env::remove_var("PEON_MAX_LINES");
        std::env::remove_var("PEON_TIMEOUT");
    }
}
