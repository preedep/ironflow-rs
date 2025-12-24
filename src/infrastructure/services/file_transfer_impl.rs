use crate::domain::entities::task::{FileLocation, TransferOptions};
use crate::domain::services::{FileTransferService, TransferProgress};
use async_trait::async_trait;
use futures::StreamExt;
use opendal::{Operator, services};
use std::error::Error;
use std::path::Path;

/// File transfer service implementation using OpenDAL
/// Supports multiple storage backends including local, FTP, SFTP, S3, and Azure Blob
pub struct OpenDalFileTransferService;

impl OpenDalFileTransferService {
    /// Creates a new OpenDalFileTransferService
    ///
    /// # Returns
    /// A new OpenDalFileTransferService instance
    pub fn new() -> Self {
        Self
    }

    /// Creates an OpenDAL operator for the given file location
    ///
    /// # Arguments
    /// * `location` - The file location to create an operator for
    ///
    /// # Returns
    /// Result containing the Operator or an error
    ///
    /// # Errors
    /// Returns an error if the operator cannot be created
    async fn create_operator(
        &self,
        location: &FileLocation,
    ) -> Result<Operator, Box<dyn Error + Send + Sync>> {
        match location {
            FileLocation::Local { path } => {
                let builder = services::Fs::default().root(path);
                Ok(Operator::new(builder)?.finish())
            }
            FileLocation::Ftp {
                host: _,
                port: _,
                path: _,
                username: _,
                password_secret: _,
            } => {
                Err("FTP support is currently disabled due to dependency issues. Use SFTP instead.".into())
            }
            FileLocation::Sftp {
                host,
                port,
                path: _,
                username,
                key_secret: _,
            } => {
                let builder = services::Sftp::default()
                    .endpoint(&format!("{}:{}", host, port))
                    .user(username);
                Ok(Operator::new(builder)?.finish())
            }
            FileLocation::S3 {
                bucket,
                key: _,
                region,
                endpoint,
            } => {
                let mut builder = services::S3::default()
                    .bucket(bucket)
                    .region(region);
                if let Some(ep) = endpoint {
                    builder = builder.endpoint(ep);
                }
                Ok(Operator::new(builder)?.finish())
            }
            FileLocation::AzureBlob {
                account,
                container,
                blob: _,
            } => {
                let builder = services::Azblob::default()
                    .account_name(account)
                    .container(container);
                Ok(Operator::new(builder)?.finish())
            }
        }
    }

    /// Extracts the path from a file location
    ///
    /// # Arguments
    /// * `location` - The file location
    ///
    /// # Returns
    /// The path as a string
    fn get_path(location: &FileLocation) -> String {
        match location {
            FileLocation::Local { path } => path.clone(),
            FileLocation::Ftp { path, .. } => path.clone(),
            FileLocation::Sftp { path, .. } => path.clone(),
            FileLocation::S3 { key, .. } => key.clone(),
            FileLocation::AzureBlob { blob, .. } => blob.clone(),
        }
    }
}

impl Default for OpenDalFileTransferService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileTransferService for OpenDalFileTransferService {
    async fn transfer(
        &self,
        source: &FileLocation,
        destination: &FileLocation,
        options: &TransferOptions,
    ) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let source_op = self.create_operator(source).await?;
        let dest_op = self.create_operator(destination).await?;

        let source_path = Self::get_path(source);
        let dest_path = Self::get_path(destination);

        if !options.overwrite {
            if dest_op.exists(&dest_path).await? {
                return Err(format!("Destination file '{}' already exists", dest_path).into());
            }
        }

        if options.create_dirs {
            if let Some(parent) = Path::new(&dest_path).parent() {
                if let Some(parent_str) = parent.to_str() {
                    if !parent_str.is_empty() {
                        dest_op.create_dir(parent_str).await.ok();
                    }
                }
            }
        }

        let data = source_op.read(&source_path).await?;
        let bytes_transferred = data.len() as u64;

        dest_op.write(&dest_path, data).await?;

        Ok(bytes_transferred)
    }


    async fn list_files(
        &self,
        location: &FileLocation,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let op = self.create_operator(location).await?;
        let path = Self::get_path(location);

        let mut files = Vec::new();
        let mut lister = op.lister(&path).await?;

        while let Some(entry) = lister.next().await {
            match entry {
                Ok(entry) => files.push(entry.path().to_string()),
                Err(_) => continue,
            }
        }

        Ok(files)
    }

    async fn exists(&self, location: &FileLocation) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let op = self.create_operator(location).await?;
        let path = Self::get_path(location);
        Ok(op.exists(&path).await?)
    }

    async fn delete(&self, location: &FileLocation) -> Result<(), Box<dyn Error + Send + Sync>> {
        let op = self.create_operator(location).await?;
        let path = Self::get_path(location);
        op.delete(&path).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_path() {
        let local = FileLocation::Local {
            path: "/tmp/test.txt".to_string(),
        };
        assert_eq!(OpenDalFileTransferService::get_path(&local), "/tmp/test.txt");

        let s3 = FileLocation::S3 {
            bucket: "mybucket".to_string(),
            key: "path/to/file.txt".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
        };
        assert_eq!(
            OpenDalFileTransferService::get_path(&s3),
            "path/to/file.txt"
        );
    }

    #[tokio::test]
    async fn test_local_file_transfer() {
        use tempfile::tempdir;
        use std::fs;

        let service = OpenDalFileTransferService::new();
        let temp_dir = tempdir().unwrap();

        let source_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("dest.txt");

        fs::write(&source_path, b"test content").unwrap();

        let source = FileLocation::Local {
            path: source_path.to_str().unwrap().to_string(),
        };
        let dest = FileLocation::Local {
            path: dest_path.to_str().unwrap().to_string(),
        };

        let result = service
            .transfer(&source, &dest, &TransferOptions::default())
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12);
        assert!(dest_path.exists());
    }

    #[tokio::test]
    async fn test_file_exists() {
        use tempfile::tempdir;
        use std::fs;

        let service = OpenDalFileTransferService::new();
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, b"test").unwrap();

        let location = FileLocation::Local {
            path: file_path.to_str().unwrap().to_string(),
        };

        let exists = service.exists(&location).await.unwrap();
        assert!(exists);
    }
}
