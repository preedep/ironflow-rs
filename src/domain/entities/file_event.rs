use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the type of file system event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Represents a file system event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// Type of the event
    pub kind: FileEventKind,
    /// Path to the file or directory
    pub path: PathBuf,
    /// Old path (for rename events)
    pub old_path: Option<PathBuf>,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Size of the file (if available)
    pub file_size: Option<u64>,
    /// Whether this is a directory
    pub is_directory: bool,
}

impl FileEvent {
    /// Creates a new FileEvent
    ///
    /// # Arguments
    /// * `kind` - The type of file event
    /// * `path` - The path to the file or directory
    ///
    /// # Returns
    /// A new FileEvent instance
    pub fn new(kind: FileEventKind, path: PathBuf) -> Self {
        Self {
            kind,
            path,
            old_path: None,
            timestamp: Utc::now(),
            file_size: None,
            is_directory: false,
        }
    }

    /// Sets the old path for rename events
    ///
    /// # Arguments
    /// * `old_path` - The old path before rename
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_old_path(mut self, old_path: PathBuf) -> Self {
        self.old_path = Some(old_path);
        self
    }

    /// Sets the file size
    ///
    /// # Arguments
    /// * `size` - The size of the file in bytes
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size = Some(size);
        self
    }

    /// Sets whether this is a directory
    ///
    /// # Arguments
    /// * `is_dir` - true if this is a directory, false otherwise
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_is_directory(mut self, is_dir: bool) -> Self {
        self.is_directory = is_dir;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_event_creation() {
        let path = PathBuf::from("/test/file.txt");
        let event = FileEvent::new(FileEventKind::Created, path.clone());

        assert_eq!(event.kind, FileEventKind::Created);
        assert_eq!(event.path, path);
        assert_eq!(event.old_path, None);
        assert!(!event.is_directory);
    }

    #[test]
    fn test_file_event_with_metadata() {
        let path = PathBuf::from("/test/file.txt");
        let event = FileEvent::new(FileEventKind::Modified, path.clone())
            .with_file_size(1024)
            .with_is_directory(false);

        assert_eq!(event.file_size, Some(1024));
        assert!(!event.is_directory);
    }

    #[test]
    fn test_file_event_rename() {
        let old_path = PathBuf::from("/test/old.txt");
        let new_path = PathBuf::from("/test/new.txt");
        let event = FileEvent::new(FileEventKind::Renamed, new_path.clone())
            .with_old_path(old_path.clone());

        assert_eq!(event.kind, FileEventKind::Renamed);
        assert_eq!(event.path, new_path);
        assert_eq!(event.old_path, Some(old_path));
    }
}
