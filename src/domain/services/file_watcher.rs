use crate::domain::entities::FileEvent;
use async_trait::async_trait;
use std::error::Error;
use std::path::Path;
use tokio::sync::mpsc;

/// Service trait for watching file system changes
/// This abstraction allows monitoring of file system events
/// and triggering actions based on those events
#[async_trait]
pub trait FileWatcherService: Send + Sync {
    /// Starts watching a path for changes
    ///
    /// # Arguments
    /// * `path` - The path to watch
    /// * `patterns` - File patterns to match (e.g., "*.txt", "*.log")
    /// * `recursive` - Whether to watch subdirectories recursively
    /// * `event_sender` - Channel to send file events to
    ///
    /// # Returns
    /// Result containing a handle to stop the watcher, or an error
    ///
    /// # Errors
    /// Returns an error if the watcher cannot be started
    async fn watch(
        &self,
        path: &Path,
        patterns: &[String],
        recursive: bool,
        event_sender: mpsc::UnboundedSender<FileEvent>,
    ) -> Result<WatcherHandle, Box<dyn Error + Send + Sync>>;

    /// Stops watching a path
    ///
    /// # Arguments
    /// * `handle` - The watcher handle to stop
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the watcher cannot be stopped
    async fn stop(&self, handle: WatcherHandle) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// Handle for a file watcher instance
/// Used to control and stop the watcher
#[derive(Debug, Clone)]
pub struct WatcherHandle {
    pub id: String,
}

impl WatcherHandle {
    /// Creates a new WatcherHandle
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the watcher
    ///
    /// # Returns
    /// A new WatcherHandle instance
    pub fn new(id: String) -> Self {
        Self { id }
    }
}
