use crate::domain::repositories::SecretRepository;
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

/// HashiCorp Vault secret repository implementation
/// Provides secret management operations using HashiCorp Vault
pub struct VaultSecretRepository {
    client: VaultClient,
    mount_path: String,
}

impl VaultSecretRepository {
    /// Creates a new VaultSecretRepository
    ///
    /// # Arguments
    /// * `url` - The Vault server URL (e.g., "http://localhost:8200")
    /// * `token` - The Vault authentication token
    /// * `mount_path` - The KV mount path (e.g., "secret")
    ///
    /// # Returns
    /// Result containing the new repository instance or an error
    ///
    /// # Errors
    /// Returns an error if the connection to Vault fails
    pub fn new(url: &str, token: &str, mount_path: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        log::debug!("Creating VaultSecretRepository: url={}, mount_path={}", url, mount_path);
        
        let settings = VaultClientSettingsBuilder::default()
            .address(url)
            .token(token)
            .build()?;

        let client = VaultClient::new(settings)?;
        
        log::info!("✓ VaultSecretRepository initialized successfully");

        Ok(Self {
            client,
            mount_path: mount_path.to_string(),
        })
    }

    /// Parses a key path into mount and secret path components
    ///
    /// # Arguments
    /// * `key` - The full key path (e.g., "app/database/password")
    ///
    /// # Returns
    /// A tuple of (path, secret_key) where path is the directory and secret_key is the final component
    fn parse_key_path(&self, key: &str) -> (String, String) {
        let parts: Vec<&str> = key.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            (parts[1].to_string(), parts[0].to_string())
        } else {
            ("default".to_string(), key.to_string())
        }
    }
}

#[async_trait]
impl SecretRepository for VaultSecretRepository {
    async fn get_secret(&self, key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        log::debug!("Getting secret: key={}", key);
        let (path, secret_key) = self.parse_key_path(key);
        log::trace!("Parsed key path: path={}, secret_key={}", path, secret_key);

        let secret: HashMap<String, String> = kv2::read(&self.client, &self.mount_path, &path).await
            .map_err(|e| {
                log::error!("Failed to read secret from Vault: path={}, error={}", path, e);
                e
            })?;

        secret
            .get(&secret_key)
            .cloned()
            .ok_or_else(|| {
                log::warn!("Secret key '{}' not found in path '{}'", secret_key, path);
                format!("Secret key '{}' not found in path '{}'", secret_key, path).into()
            })
            .map(|value| {
                log::debug!("✓ Secret retrieved successfully: key={}", key);
                value
            })
    }

    async fn set_secret(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Setting secret: key={}", key);
        let (path, secret_key) = self.parse_key_path(key);
        log::trace!("Parsed key path: path={}, secret_key={}", path, secret_key);

        let mut existing: HashMap<String, String> = kv2::read(&self.client, &self.mount_path, &path)
            .await
            .unwrap_or_else(|e| {
                log::debug!("No existing secrets at path '{}': {}, creating new", path, e);
                HashMap::new()
            });

        existing.insert(secret_key, value.to_string());

        kv2::set(&self.client, &self.mount_path, &path, &existing).await
            .map_err(|e| {
                log::error!("Failed to set secret in Vault: path={}, error={}", path, e);
                e
            })?;
        
        log::info!("✓ Secret set successfully: key={}", key);

        Ok(())
    }

    async fn delete_secret(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Deleting secret: key={}", key);
        let (path, _secret_key) = self.parse_key_path(key);
        log::trace!("Deleting from path: {}", path);

        kv2::delete_latest(&self.client, &self.mount_path, &path).await
            .map_err(|e| {
                log::error!("Failed to delete secret from Vault: path={}, error={}", path, e);
                e
            })?;
        
        log::info!("✓ Secret deleted successfully: key={}", key);

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        log::trace!("Performing Vault health check");
        match vaultrs::sys::health(&self.client).await {
            Ok(_) => {
                log::trace!("✓ Vault health check passed");
                Ok(true)
            },
            Err(e) => {
                log::warn!("Vault health check failed: {}", e);
                Ok(false)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_path() {
        let repo = VaultSecretRepository::new("http://localhost:8200", "token", "secret")
            .unwrap();

        let (path, key) = repo.parse_key_path("app/database/password");
        assert_eq!(path, "app/database");
        assert_eq!(key, "password");

        let (path, key) = repo.parse_key_path("simple_key");
        assert_eq!(path, "default");
        assert_eq!(key, "simple_key");
    }

    #[tokio::test]
    #[ignore]
    async fn test_vault_connection() {
        let repo = VaultSecretRepository::new(
            "http://localhost:8200",
            "dev-token",
            "secret",
        );
        assert!(repo.is_ok());
    }
}
