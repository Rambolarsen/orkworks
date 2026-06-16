use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use tokio::sync::broadcast;

pub struct MetadataWatcher {
    tx: broadcast::Sender<String>,
}

impl MetadataWatcher {
    pub fn start(sessions_dir: &Path) -> Self {
        let (tx, _) = broadcast::channel::<String>(32);
        let tx_clone = tx.clone();
        let dir = sessions_dir.to_path_buf();

        std::thread::spawn(move || {
            let (watcher_tx, watcher_rx) = mpsc::channel::<Result<Event, notify::Error>>();
            let mut watcher = notify::recommended_watcher(move |res| {
                let _ = watcher_tx.send(res);
            })
            .unwrap();

            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);

            for res in watcher_rx {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in &event.paths {
                            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                                let _ = tx_clone.send(name.to_string());
                            }
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}
