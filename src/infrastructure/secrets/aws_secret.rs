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
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = Client::new(&config);

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
        let secret_name = self.build_secret_name(key);

        let response = self
            .client
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await?;

        response
            .secret_string()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Secret '{}' has no string value", secret_name).into())
    }

    async fn set_secret(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let secret_name = self.build_secret_name(key);

        match self
            .client
            .update_secret()
            .secret_id(&secret_name)
            .secret_string(value)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => {
                self.client
                    .create_secret()
                    .name(&secret_name)
                    .secret_string(value)
                    .send()
                    .await?;
                Ok(())
            }
        }
    }

    async fn delete_secret(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let secret_name = self.build_secret_name(key);

        self.client
            .delete_secret()
            .secret_id(&secret_name)
            .force_delete_without_recovery(true)
            .send()
            .await?;

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        match self.client.list_secrets().max_results(1).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
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
