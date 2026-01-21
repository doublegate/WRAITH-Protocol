//! File System Watcher Module
//!
//! Monitors file system changes with debouncing for efficient sync triggering.

// Allow dead_code for event channels used by future implementations
#![allow(dead_code)]

use crate::error::{SyncError, SyncResult};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

/// File change event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

/// A file change event
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub is_directory: bool,
    pub timestamp: Instant,
}

/// Configuration for the file watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration in milliseconds
    pub debounce_ms: u64,
    /// Patterns to ignore (glob patterns)
    pub ignored_patterns: Vec<String>,
    /// Maximum number of pending events before forcing flush
    pub max_pending_events: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            ignored_patterns: vec![
                "**/.git/**".to_string(),
                "**/.svn/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/__pycache__/**".to_string(),
                "**/.DS_Store".to_string(),
                "**/Thumbs.db".to_string(),
                "**/*.tmp".to_string(),
                "**/*.swp".to_string(),
                "**/*~".to_string(),
                "**/.wraith-sync/**".to_string(),
            ],
            max_pending_events: 1000,
        }
    }
}

/// File system watcher with debouncing
pub struct FileSystemWatcher {
    watcher: RecommendedWatcher,
    watched_paths: Arc<RwLock<HashSet<PathBuf>>>,
    pending_events: Arc<RwLock<HashMap<PathBuf, FileChange>>>,
    config: WatcherConfig,
    event_tx: Sender<FileChange>,
    event_rx: Arc<parking_lot::Mutex<Receiver<FileChange>>>,
    running: Arc<RwLock<bool>>,
}

impl FileSystemWatcher {
    /// Create a new file system watcher
    pub fn new(config: WatcherConfig) -> SyncResult<Self> {
        let (event_tx, event_rx) = channel();
        let pending_events = Arc::new(RwLock::new(HashMap::new()));
        let watched_paths = Arc::new(RwLock::new(HashSet::new()));

        let pending_clone = pending_events.clone();
        let config_clone = config.clone();

        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| match result {
                Ok(event) => {
                    Self::process_event(event, &pending_clone, &config_clone);
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )?;

        Ok(Self {
            watcher,
            watched_paths,
            pending_events,
            config,
            event_tx,
            event_rx: Arc::new(parking_lot::Mutex::new(event_rx)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start watching a path
    pub fn watch_path(&mut self, path: &Path) -> SyncResult<()> {
        if !path.exists() {
            return Err(SyncError::FileNotFound(path.display().to_string()));
        }

        let canonical = path
            .canonicalize()
            .map_err(|e| SyncError::FileSystem(format!("Failed to canonicalize path: {}", e)))?;

        {
            let mut paths = self.watched_paths.write();
            if paths.contains(&canonical) {
                debug!("Path already being watched: {:?}", canonical);
                return Ok(());
            }
            paths.insert(canonical.clone());
        }

        self.watcher.watch(&canonical, RecursiveMode::Recursive)?;
        info!("Started watching: {:?}", canonical);

        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch_path(&mut self, path: &Path) -> SyncResult<()> {
        let canonical = path
            .canonicalize()
            .map_err(|e| SyncError::FileSystem(format!("Failed to canonicalize path: {}", e)))?;

        {
            let mut paths = self.watched_paths.write();
            paths.remove(&canonical);
        }

        self.watcher.unwatch(&canonical)?;
        info!("Stopped watching: {:?}", canonical);

        Ok(())
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().iter().cloned().collect()
    }

    /// Check if a path is being watched
    pub fn is_watching(&self, path: &Path) -> bool {
        if let Ok(canonical) = path.canonicalize() {
            self.watched_paths.read().contains(&canonical)
        } else {
            false
        }
    }

    /// Process a raw notify event
    fn process_event(
        event: Event,
        pending: &Arc<RwLock<HashMap<PathBuf, FileChange>>>,
        config: &WatcherConfig,
    ) {
        let change_type = match event.kind {
            EventKind::Create(_) => FileChangeType::Created,
            EventKind::Modify(_) => FileChangeType::Modified,
            EventKind::Remove(_) => FileChangeType::Deleted,
            _ => return, // Ignore other events
        };

        for path in event.paths {
            // Check if path should be ignored
            if Self::should_ignore(&path, &config.ignored_patterns) {
                debug!("Ignoring path: {:?}", path);
                continue;
            }

            let is_directory = path.is_dir();
            let change = FileChange {
                path: path.clone(),
                change_type: change_type.clone(),
                is_directory,
                timestamp: Instant::now(),
            };

            let mut events = pending.write();
            events.insert(path, change);
        }
    }

    /// Check if a path matches any ignored patterns
    fn should_ignore(path: &Path, patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in patterns {
            // Simple glob matching
            if Self::matches_glob(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple glob pattern matching
    fn matches_glob(path: &str, pattern: &str) -> bool {
        // Handle **/ prefix and /** suffix patterns (e.g., **/.git/**)
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                // Both empty means match anything
                if prefix.is_empty() && suffix.is_empty() {
                    return true;
                }

                // Handle suffix with single wildcard (e.g., **/*.tmp)
                if !suffix.is_empty() && suffix.contains('*') && prefix.is_empty() {
                    // Pattern like **/*.tmp - the suffix is *.tmp
                    let ext_parts: Vec<&str> = suffix.split('*').collect();
                    if ext_parts.len() == 2 && ext_parts[0].is_empty() {
                        // Suffix is *.ext - check if filename ends with .ext
                        let ext = ext_parts[1];
                        // Get filename from path
                        let filename = path.rsplit('/').next().unwrap_or(path);
                        return filename.ends_with(ext);
                    }
                }

                // Check if the middle part exists in the path
                // For **/.git/** we want to match paths containing .git/
                if !prefix.is_empty() && !suffix.is_empty() {
                    // Pattern like "foo/**/bar" - path must contain both
                    return path.contains(prefix) && path.contains(suffix);
                }

                if !prefix.is_empty() {
                    // Pattern starts with something before **
                    return path.starts_with(prefix) || path.contains(&format!("/{}", prefix));
                }

                if !suffix.is_empty() {
                    // Pattern like **/.git/** - check if path starts with suffix or contains /suffix
                    return path.starts_with(suffix)
                        || path.starts_with(&format!("{}/", suffix))
                        || path.contains(&format!("/{}/", suffix))
                        || path.contains(&format!("/{}", suffix));
                }
            } else if parts.len() == 3 {
                // Pattern like **/foo/** - middle part must be in path
                let middle = parts[1].trim_matches('/');
                if !middle.is_empty() {
                    return path.starts_with(&format!("{}/", middle))
                        || path.contains(&format!("/{}/", middle))
                        || path.contains(&format!("/{}", middle))
                        || path == middle;
                }
            }
        }

        // Handle single * wildcards
        if pattern.contains('*') && !pattern.contains("**") {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];

                if prefix.is_empty() {
                    return path.ends_with(suffix);
                }
                if suffix.is_empty() {
                    return path.starts_with(prefix);
                }
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }

        // Exact match or simple contains
        path == pattern || path.contains(pattern)
    }

    /// Flush pending events (called after debounce period)
    pub fn flush_pending(&self) -> Vec<FileChange> {
        let debounce = Duration::from_millis(self.config.debounce_ms);
        let now = Instant::now();

        let mut pending = self.pending_events.write();
        let mut flushed = Vec::new();

        // Collect events that have been pending long enough
        pending.retain(|_, change| {
            if now.duration_since(change.timestamp) >= debounce {
                flushed.push(change.clone());
                false
            } else {
                true
            }
        });

        flushed
    }

    /// Get count of pending events
    pub fn pending_count(&self) -> usize {
        self.pending_events.read().len()
    }

    /// Start the debounce flush loop
    pub fn start_flush_loop(
        &self,
        callback: impl Fn(Vec<FileChange>) + Send + 'static,
    ) -> std::thread::JoinHandle<()> {
        let pending = self.pending_events.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        *running.write() = true;

        std::thread::spawn(move || {
            let debounce = Duration::from_millis(config.debounce_ms);
            let check_interval = Duration::from_millis(config.debounce_ms / 2);

            while *running.read() {
                std::thread::sleep(check_interval);

                let now = Instant::now();
                let mut flushed = Vec::new();

                {
                    let mut events = pending.write();
                    events.retain(|_, change| {
                        if now.duration_since(change.timestamp) >= debounce {
                            flushed.push(change.clone());
                            false
                        } else {
                            true
                        }
                    });
                }

                if !flushed.is_empty() {
                    callback(flushed);
                }
            }

            info!("Flush loop stopped");
        })
    }

    /// Stop the flush loop
    pub fn stop(&self) {
        *self.running.write() = false;
    }

    /// Update ignored patterns
    pub fn set_ignored_patterns(&mut self, patterns: Vec<String>) {
        self.config.ignored_patterns = patterns;
    }

    /// Add an ignored pattern
    pub fn add_ignored_pattern(&mut self, pattern: String) {
        if !self.config.ignored_patterns.contains(&pattern) {
            self.config.ignored_patterns.push(pattern);
        }
    }
}

impl Drop for FileSystemWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_glob_matching() {
        assert!(FileSystemWatcher::matches_glob(
            "/foo/.git/config",
            "**/.git/**"
        ));
        assert!(FileSystemWatcher::matches_glob(
            "/project/node_modules/foo",
            "**/node_modules/**"
        ));
        assert!(FileSystemWatcher::matches_glob(
            "/path/to/file.tmp",
            "**/*.tmp"
        ));
        assert!(!FileSystemWatcher::matches_glob(
            "/path/to/file.txt",
            "**/*.tmp"
        ));
    }

    #[test]
    fn test_watcher_creation() {
        let config = WatcherConfig::default();
        let watcher = FileSystemWatcher::new(config);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_path() {
        let dir = tempdir().unwrap();
        let config = WatcherConfig::default();
        let mut watcher = FileSystemWatcher::new(config).unwrap();

        watcher.watch_path(dir.path()).unwrap();
        assert!(watcher.is_watching(dir.path()));

        watcher.unwatch_path(dir.path()).unwrap();
        assert!(!watcher.is_watching(dir.path()));
    }

    #[test]
    fn test_debounce() {
        let config = WatcherConfig {
            debounce_ms: 50,
            ..Default::default()
        };
        let watcher = FileSystemWatcher::new(config).unwrap();

        // Simulate adding events
        {
            let mut pending = watcher.pending_events.write();
            pending.insert(
                PathBuf::from("/test/file.txt"),
                FileChange {
                    path: PathBuf::from("/test/file.txt"),
                    change_type: FileChangeType::Modified,
                    is_directory: false,
                    timestamp: Instant::now() - Duration::from_millis(100),
                },
            );
        }

        let flushed = watcher.flush_pending();
        assert_eq!(flushed.len(), 1);
    }
}
