use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sprawl_core::Result;

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
    
    pub fn ingest(&mut self, event: Event) {
        // If event.paths is empty, skip
        if let Some(path) = event.paths.first() {
            // Very naive project root extraction for mockup (assume parent of the changed file)
            // In reality, we'd match against the known watched roots from the ledger
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

pub struct FilesystemWatcher {
    watcher: RecommendedWatcher,
    _rx: Receiver<std::result::Result<Event, notify::Error>>,
}

impl FilesystemWatcher {
    pub fn new(project_roots: &[PathBuf], config_paths: &[PathBuf]) -> Result<(Self, Receiver<std::result::Result<Event, notify::Error>>)> {
        let (tx, rx) = channel();
        
        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
            .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to initialize watcher: {}", e)))?;
            
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
        
        // We need to return rx for the event loop, but we also create a dummy rx for the struct just to satisfy types if needed, 
        // or just return it alongside.
        let (_, dummy_rx) = channel();
        
        Ok((Self { watcher, _rx: dummy_rx }, rx))
    }
}
