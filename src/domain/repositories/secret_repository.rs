use async_trait::async_trait;
use std::error::Error;

/// Repository trait for secret management operations
/// This abstraction allows different secret management systems
/// (HashiCorp Vault, AWS Secrets Manager, Azure Key Vault, etc.)
/// to be used interchangeably
#[async_trait]
pub trait SecretRepository: Send + Sync {
    /// Retrieves a secret value by key
    ///
    /// # Arguments
    /// * `key` - The key/path of the secret to retrieve
    ///
    /// # Returns
    /// Result containing the secret value as a string, or an error
    ///
    /// # Errors
    /// Returns an error if the secret cannot be retrieved or doesn't exist
    async fn get_secret(&self, key: &str) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// Retrieves multiple secrets by keys
    ///
    /// # Arguments
    /// * `keys` - A slice of keys/paths to retrieve
    ///
    /// # Returns
    /// Result containing a HashMap of key-value pairs, or an error
    ///
    /// # Errors
    /// Returns an error if any secret cannot be retrieved
    async fn get_secrets(
        &self,
        keys: &[&str],
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let mut results = std::collections::HashMap::new();

        for key in keys {
            let value = self.get_secret(key).await?;
            results.insert(key.to_string(), value);
        }

        Ok(results)
    }

    /// Stores a secret value
    ///
    /// # Arguments
    /// * `key` - The key/path where the secret should be stored
    /// * `value` - The secret value to store
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the secret cannot be stored
    async fn set_secret(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Deletes a secret
    ///
    /// # Arguments
    /// * `key` - The key/path of the secret to delete
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// Returns an error if the secret cannot be deleted
    async fn delete_secret(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Checks if the secret repository connection is healthy
    ///
    /// # Returns
    /// Result containing true if healthy, false otherwise
    ///
    /// # Errors
    /// Returns an error if the health check fails
    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>>;
}
