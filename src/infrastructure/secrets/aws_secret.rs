use crate::domain::repositories::SecretRepository;
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::Client;
use std::error::Error;

/// AWS Secrets Manager secret repository implementation
/// Provides secret management operations using AWS Secrets Manager
pub struct AwsSecretRepository {
    client: Client,
    prefix: Option<String>,
}

impl AwsSecretRepository {
    /// Creates a new AwsSecretRepository
    ///
    /// # Arguments
    /// * `region` - The AWS region (e.g., "us-east-1")
    /// * `prefix` - Optional prefix for secret names
    ///
    /// # Returns
    /// Result containing the new repository instance or an error
    ///
    /// # Errors
    /// Returns an error if the AWS SDK initialization fails
    pub async fn new(
        region: &str,
        prefix: Option<String>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        log::debug!("Creating AwsSecretRepository: region={}, prefix={:?}", region, prefix);
        
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = Client::new(&config);
        
        log::info!("✓ AwsSecretRepository initialized successfully");

        Ok(Self { client, prefix })
    }

    /// Builds the full secret name with prefix
    ///
    /// # Arguments
    /// * `key` - The secret key
    ///
    /// # Returns
    /// The full secret name with prefix applied
    fn build_secret_name(&self, key: &str) -> String {
        if let Some(prefix) = &self.prefix {
            format!("{}/{}", prefix, key)
        } else {
            key.to_string()
        }
    }
}

#[async_trait]
impl SecretRepository for AwsSecretRepository {
    async fn get_secret(&self, key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        log::debug!("Getting secret from AWS: key={}", key);
        let secret_name = self.build_secret_name(key);
        log::trace!("Full secret name: {}", secret_name);

        let response = self
            .client
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await
            .map_err(|e| {
                log::error!("Failed to get secret from AWS: secret_name={}, error={}", secret_name, e);
                e
            })?;

        response
            .secret_string()
            .map(|s| {
                log::debug!("✓ Secret retrieved successfully from AWS: key={}", key);
                s.to_string()
            })
            .ok_or_else(|| {
                log::warn!("Secret '{}' has no string value", secret_name);
                format!("Secret '{}' has no string value", secret_name).into()
            })
    }

    async fn set_secret(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Setting secret in AWS: key={}", key);
        let secret_name = self.build_secret_name(key);
        log::trace!("Full secret name: {}", secret_name);

        match self
            .client
            .update_secret()
            .secret_id(&secret_name)
            .secret_string(value)
            .send()
            .await
        {
            Ok(_) => {
                log::info!("✓ Secret updated successfully in AWS: key={}", key);
                Ok(())
            },
            Err(e) => {
                log::debug!("Secret doesn't exist, creating new: {} (error: {})", secret_name, e);
                self.client
                    .create_secret()
                    .name(&secret_name)
                    .secret_string(value)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!("Failed to create secret in AWS: secret_name={}, error={}", secret_name, e);
                        e
                    })?;
                log::info!("✓ Secret created successfully in AWS: key={}", key);
                Ok(())
            }
        }
    }

    async fn delete_secret(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Deleting secret from AWS: key={}", key);
        let secret_name = self.build_secret_name(key);
        log::trace!("Full secret name: {}", secret_name);

        self.client
            .delete_secret()
            .secret_id(&secret_name)
            .force_delete_without_recovery(true)
            .send()
            .await
            .map_err(|e| {
                log::error!("Failed to delete secret from AWS: secret_name={}, error={}", secret_name, e);
                e
            })?;
        
        log::info!("✓ Secret deleted successfully from AWS: key={}", key);

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        log::trace!("Performing AWS Secrets Manager health check");
        match self.client.list_secrets().max_results(1).send().await {
            Ok(_) => {
                log::trace!("✓ AWS Secrets Manager health check passed");
                Ok(true)
            },
            Err(e) => {
                log::warn!("AWS Secrets Manager health check failed: {}", e);
                Ok(false)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_secret_name() {
        let repo_with_prefix = AwsSecretRepository {
            client: Client::from_conf(aws_sdk_secretsmanager::Config::builder().build()),
            prefix: Some("myapp".to_string()),
        };

        assert_eq!(
            repo_with_prefix.build_secret_name("database/password"),
            "myapp/database/password"
        );

        let repo_without_prefix = AwsSecretRepository {
            client: Client::from_conf(aws_sdk_secretsmanager::Config::builder().build()),
            prefix: None,
        };

        assert_eq!(
            repo_without_prefix.build_secret_name("database/password"),
            "database/password"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_aws_secrets_manager_connection() {
        let repo = AwsSecretRepository::new("us-east-1", None).await;
        assert!(repo.is_ok());
    }
}
