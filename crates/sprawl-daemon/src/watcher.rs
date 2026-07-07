use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use sprawl_core::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub struct EventDeduplicator {
    pub pending: HashMap<PathBuf, Vec<Event>>,
    pub debounce_duration: Duration,
    pub last_flush: Instant,
}

impl EventDeduplicator {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            debounce_duration: Duration::from_secs(2), // 2-second debounce window
            last_flush: Instant::now(),
        }
    }
}

impl Default for EventDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDeduplicator {
    pub fn ingest(&mut self, event: Event) {
        if let Some(path) = event.paths.first() {
            if let Some(parent) = path.parent() {
                let root = parent.to_path_buf();
                self.pending.entry(root).or_default().push(event);
            }
        }
    }

    pub fn flush_if_ready(&mut self) -> Option<HashMap<PathBuf, Vec<Event>>> {
        if self.last_flush.elapsed() >= self.debounce_duration {
            let ready = std::mem::take(&mut self.pending);
            self.last_flush = Instant::now();
            if !ready.is_empty() {
                return Some(ready);
            }
        }
        None
    }
}

#[allow(dead_code)]
pub struct FilesystemWatcher {
    watcher: RecommendedWatcher,
    _rx: Receiver<std::result::Result<Event, notify::Error>>,
}

impl FilesystemWatcher {
    pub fn new(
        project_roots: &[PathBuf],
        config_paths: &[PathBuf],
    ) -> Result<(Self, Receiver<std::result::Result<Event, notify::Error>>)> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).map_err(|e| {
            sprawl_core::SprawlError::Other(format!("Failed to initialize watcher: {}", e))
        })?;

        for root in project_roots {
            if let Err(e) = watcher.watch(root, RecursiveMode::NonRecursive) {
                tracing::warn!("Watcher failed on root {}: {}", root.display(), e);
            }
        }

        for config in config_paths {
            if config.exists() {
                if let Err(e) = watcher.watch(config, RecursiveMode::NonRecursive) {
                    tracing::warn!("Watcher failed on config {}: {}", config.display(), e);
                }
            }
        }

        let (_, dummy_rx) = channel();
        Ok((
            Self {
                watcher,
                _rx: dummy_rx,
            },
            rx,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::EventKind;

    #[test]
    fn test_event_deduplication_reduces_noise() {
        let mut dedup = EventDeduplicator::new();
        // Artificially simulate 0 debounce duration for testing flush state
        dedup.debounce_duration = Duration::from_millis(0);

        let project_root = PathBuf::from("/mock/project");
        let file1 = project_root.join("package.json");
        let file2 = project_root.join("index.js");

        // Simulate a noisy `npm install` emitting 100 events
        for _ in 0..50 {
            dedup.ingest(Event::new(EventKind::Any).add_path(file1.clone()));
            dedup.ingest(Event::new(EventKind::Any).add_path(file2.clone()));
        }

        // Validate pending state
        assert_eq!(
            dedup.pending.len(),
            1,
            "Events should map to the single project root"
        );
        assert_eq!(
            dedup.pending.get(&project_root).unwrap().len(),
            100,
            "All 100 events captured"
        );

        // Flush
        let batches = dedup.flush_if_ready().unwrap();
        assert_eq!(
            batches.len(),
            1,
            "Deduped down to 1 batch mapped to the root"
        );

        // Subsequent flush is empty
        assert!(dedup.flush_if_ready().is_none());
    }
}
