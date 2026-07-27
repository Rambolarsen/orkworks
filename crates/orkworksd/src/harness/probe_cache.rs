use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_CACHE_ENTRIES: usize = 64;

#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct VersionProbeCacheKey {
    pub harness_id: String,
    pub launch_command: String,
    pub executable: PathBuf,
}

pub(crate) struct VersionProbeCache {
    generation: AtomicU64,
    entries: Mutex<HashMap<VersionProbeCacheKey, VersionProbeCacheEntry>>,
}

#[derive(Clone)]
struct VersionProbeCacheEntry {
    generation: u64,
    expires_at: Instant,
    version_output: Option<String>,
}

impl VersionProbeCache {
    pub(crate) fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) async fn probe_or_get<F, Fut>(
        &self,
        key: VersionProbeCacheKey,
        now: Instant,
        positive_ttl: Duration,
        negative_ttl: Duration,
        probe: F,
    ) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<String>>,
    {
        let generation = self.generation.load(Ordering::SeqCst);
        {
            let mut entries = self.entries.lock().unwrap();
            prune_entries(&mut entries, generation, now);
            if let Some(entry) = entries.get(&key) {
                if entry.generation == generation && entry.expires_at > now {
                    return entry.version_output.clone();
                }
            }
        }

        let version_output = probe().await;
        let ttl = if version_output.is_some() {
            positive_ttl
        } else {
            negative_ttl
        };
        let expires_at = now + ttl;

        if self.generation.load(Ordering::SeqCst) == generation {
            let mut entries = self.entries.lock().unwrap();
            prune_entries(&mut entries, generation, now);
            entries.insert(
                key,
                VersionProbeCacheEntry {
                    generation,
                    expires_at,
                    version_output: version_output.clone(),
                },
            );
            prune_to_capacity(&mut entries);
        }

        version_output
    }
}

fn prune_entries(
    entries: &mut HashMap<VersionProbeCacheKey, VersionProbeCacheEntry>,
    generation: u64,
    now: Instant,
) {
    entries.retain(|_, entry| entry.generation == generation && entry.expires_at > now);
}

fn prune_to_capacity(entries: &mut HashMap<VersionProbeCacheKey, VersionProbeCacheEntry>) {
    if entries.len() <= MAX_CACHE_ENTRIES {
        return;
    }

    let mut evictions: Vec<_> = entries
        .iter()
        .map(|(key, entry)| (key.clone(), entry.expires_at))
        .collect();
    evictions.sort_by_key(|(_, expires_at)| *expires_at);

    for (key, _) in evictions
        .into_iter()
        .take(entries.len() - MAX_CACHE_ENTRIES)
    {
        entries.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn key() -> VersionProbeCacheKey {
        VersionProbeCacheKey {
            harness_id: "copilot".into(),
            launch_command: "copilot".into(),
            executable: PathBuf::from("/tmp/copilot"),
        }
    }

    #[tokio::test]
    async fn reuses_a_positive_probe_until_ttl_expires() {
        let cache = VersionProbeCache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let now = Instant::now();

        let first = cache
            .probe_or_get(
                key(),
                now,
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("copilot-cli 1.2.3".into())
                    }
                },
            )
            .await;
        let second = cache
            .probe_or_get(
                key(),
                now + Duration::from_secs(1),
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("copilot-cli 1.2.3".into())
                    }
                },
            )
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.as_deref(), Some("copilot-cli 1.2.3"));
        assert_eq!(second.as_deref(), Some("copilot-cli 1.2.3"));
    }

    #[tokio::test]
    async fn reuses_a_negative_probe_for_the_short_negative_ttl() {
        let cache = VersionProbeCache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let now = Instant::now();

        let first = cache
            .probe_or_get(
                key(),
                now,
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        None
                    }
                },
            )
            .await;
        let second = cache
            .probe_or_get(
                key(),
                now + Duration::from_secs(1),
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("ignored".into())
                    }
                },
            )
            .await;
        let third = cache
            .probe_or_get(
                key(),
                now + Duration::from_secs(6),
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("copilot-cli 1.2.3".into())
                    }
                },
            )
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(first, None);
        assert_eq!(second, None);
        assert_eq!(third.as_deref(), Some("copilot-cli 1.2.3"));
    }

    #[tokio::test]
    async fn bump_generation_invalidates_an_entry_before_ttl_expires() {
        let cache = VersionProbeCache::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let now = Instant::now();

        let first = cache
            .probe_or_get(
                key(),
                now,
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("copilot-cli 1.2.3".into())
                    }
                },
            )
            .await;
        cache.bump_generation();
        let second = cache
            .probe_or_get(
                key(),
                now + Duration::from_secs(1),
                Duration::from_secs(30),
                Duration::from_secs(5),
                {
                    let calls = calls.clone();
                    move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Some("copilot-cli 1.2.4".into())
                    }
                },
            )
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(first.as_deref(), Some("copilot-cli 1.2.3"));
        assert_eq!(second.as_deref(), Some("copilot-cli 1.2.4"));
    }
}
