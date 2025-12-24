use crate::domain::entities::{FileEvent, FileEventKind};
use crate::domain::services::{FileWatcherService, WatcherHandle};
use async_trait::async_trait;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

/// File watcher service implementation using notify crate
/// Monitors file system changes and sends events through channels
pub struct NotifyFileWatcherService {
    watchers: Arc<Mutex<HashMap<String, RecommendedWatcher>>>,
}

impl NotifyFileWatcherService {
    /// Creates a new NotifyFileWatcherService
    ///
    /// # Returns
    /// A new NotifyFileWatcherService instance
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Converts a notify event to a FileEvent
    ///
    /// # Arguments
    /// * `event` - The notify event to convert
    ///
    /// # Returns
    /// Optional FileEvent if the conversion is successful
    fn convert_event(event: Event) -> Option<FileEvent> {
        let kind = match event.kind {
            EventKind::Create(_) => FileEventKind::Created,
            EventKind::Modify(_) => FileEventKind::Modified,
            EventKind::Remove(_) => FileEventKind::Deleted,
            _ => return None,
        };

        event.paths.first().map(|path| {
            let is_directory = path.is_dir();
            let file_size = if path.is_file() {
                std::fs::metadata(path).ok().map(|m| m.len())
            } else {
                None
            };

            FileEvent::new(kind, path.clone())
                .with_is_directory(is_directory)
                .with_file_size(file_size.unwrap_or(0))
        })
    }

    /// Checks if a path matches any of the given patterns
    ///
    /// # Arguments
    /// * `path` - The path to check
    /// * `patterns` - The patterns to match against
    ///
    /// # Returns
    /// true if the path matches any pattern, false otherwise
    fn matches_patterns(path: &Path, patterns: &[String]) -> bool {
        if patterns.is_empty() {
            return true;
        }

        let path_str = path.to_string_lossy();

        patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                let pattern_regex = pattern
                    .replace('.', r"\.")
                    .replace('*', ".*")
                    .replace('?', ".");
                if let Ok(re) = regex::Regex::new(&pattern_regex) {
                    return re.is_match(&path_str);
                }
            }
            path_str.ends_with(pattern)
        })
    }
}

impl Default for NotifyFileWatcherService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileWatcherService for NotifyFileWatcherService {
    async fn watch(
        &self,
        path: &Path,
        patterns: &[String],
        recursive: bool,
        event_sender: mpsc::UnboundedSender<FileEvent>,
    ) -> Result<WatcherHandle, Box<dyn Error + Send + Sync>> {
        let handle_id = Uuid::new_v4().to_string();
        let patterns_clone = patterns.to_vec();

        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if let Some(file_event) = Self::convert_event(event) {
                    if Self::matches_patterns(&file_event.path, &patterns_clone) {
                        let _ = tx.send(file_event);
                    }
                }
            }
        })?;

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(path, mode)?;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if event_sender.send(event).is_err() {
                    break;
                }
            }
        });

        let mut watchers = self.watchers.lock().await;
        watchers.insert(handle_id.clone(), watcher);

        Ok(WatcherHandle::new(handle_id))
    }

    async fn stop(&self, handle: WatcherHandle) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut watchers = self.watchers.lock().await;
        watchers.remove(&handle.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_matches_patterns() {
        let path = PathBuf::from("/test/file.txt");

        assert!(NotifyFileWatcherService::matches_patterns(&path, &[]));

        assert!(NotifyFileWatcherService::matches_patterns(
            &path,
            &["*.txt".to_string()]
        ));

        assert!(!NotifyFileWatcherService::matches_patterns(
            &path,
            &["*.log".to_string()]
        ));

        assert!(NotifyFileWatcherService::matches_patterns(
            &path,
            &["file.txt".to_string()]
        ));
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        use tempfile::tempdir;

        let service = NotifyFileWatcherService::new();
        let temp_dir = tempdir().unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();

        let result = service
            .watch(temp_dir.path(), &[], false, tx)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_file_watcher_stop() {
        use tempfile::tempdir;

        let service = NotifyFileWatcherService::new();
        let temp_dir = tempdir().unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();

        let handle = service
            .watch(temp_dir.path(), &[], false, tx)
            .await
            .unwrap();

        let result = service.stop(handle).await;
        assert!(result.is_ok());
    }
}
