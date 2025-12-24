use crate::domain::entities::task::{FileLocation, TransferOptions};
use async_trait::async_trait;
use std::error::Error;

/// Progress information for file transfers
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
    pub percentage: Option<f64>,
}

/// Service trait for file transfer operations
/// This abstraction supports multiple protocols and storage types
/// including FTP, SFTP, and object storage (S3, Azure Blob, etc.)
#[async_trait]
pub trait FileTransferService: Send + Sync {
    /// Transfers a file from source to destination
    ///
    /// # Arguments
    /// * `source` - The source file location
    /// * `destination` - The destination file location
    /// * `options` - Transfer options (overwrite, create dirs, etc.)
    ///
    /// # Returns
    /// Result containing the number of bytes transferred, or an error
    ///
    /// # Errors
    /// Returns an error if the transfer fails
    async fn transfer(
        &self,
        source: &FileLocation,
        destination: &FileLocation,
        options: &TransferOptions,
    ) -> Result<u64, Box<dyn Error + Send + Sync>>;


    /// Lists files at a location
    ///
    /// # Arguments
    /// * `location` - The location to list files from
    ///
    /// # Returns
    /// Result containing a vector of file paths, or an error
    ///
    /// # Errors
    /// Returns an error if listing fails
    async fn list_files(
        &self,
        location: &FileLocation,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>>;

    /// Checks if a file exists at the given location
    ///
    /// # Arguments
    /// * `location` - The location to check
    ///
    /// # Returns
    /// Result containing true if the file exists, false otherwise
    ///
    /// # Errors
    /// Returns an error if the check fails
    async fn exists(&self, location: &FileLocation) -> Result<bool, Box<dyn Error + Send + Sync>>;

    /// Deletes a file at the given location
    ///
    /// # Arguments
    /// * `location` - The location of the file to delete
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the deletion fails
    async fn delete(&self, location: &FileLocation) -> Result<(), Box<dyn Error + Send + Sync>>;
}
