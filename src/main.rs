use ironflow_rs::application::{TaskProcessor, Worker};
use ironflow_rs::domain::repositories::{QueueRepository, SecretRepository};
use ironflow_rs::domain::services::{CommandExecutor, FileTransferService, FileWatcherService};
use ironflow_rs::domain::value_objects::{QueueConfig, SecretsConfig, WorkerConfig};
use ironflow_rs::infrastructure::logging::LogBuilder;
use ironflow_rs::infrastructure::queues::{RabbitMqQueueRepository, RedisQueueRepository};
use ironflow_rs::infrastructure::secrets::{AwsSecretRepository, AzureSecretRepository, VaultSecretRepository};
use ironflow_rs::infrastructure::services::{
    DefaultCommandExecutor, NotifyFileWatcherService, OpenDalFileTransferService,
};
use std::sync::Arc;

/// Main entry point for the IronFlow-Rs worker
///
/// This function initializes all components based on configuration and starts the worker.
/// The worker will process tasks from the configured queue until a shutdown signal is received.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let config = load_config()?;

    let log_builder = LogBuilder::new()
        .with_app_id(config.app.id.clone())
        .with_app_version(config.app.version.clone())
        .with_service_id(config.logging.service_id.clone())
        .with_service_version(config.logging.service_version.clone());

    log_builder
        .build(
            ironflow_rs::infrastructure::logging::LogLevel::Info,
            "Initializing IronFlow-Rs worker".to_string(),
        )
        .log();

    let queue_repository = create_queue_repository(&config).await?;
    let secret_repository = create_secret_repository(&config).await?;

    let command_executor = Arc::new(DefaultCommandExecutor::new()) as Arc<dyn CommandExecutor>;
    let file_transfer_service =
        Arc::new(OpenDalFileTransferService::new()) as Arc<dyn FileTransferService>;
    let file_watcher_service =
        Arc::new(NotifyFileWatcherService::new()) as Arc<dyn FileWatcherService>;

    let task_processor = Arc::new(TaskProcessor::new(
        command_executor,
        file_transfer_service,
        file_watcher_service,
        secret_repository,
        log_builder.clone(),
    ));

    let worker = Worker::new(
        config,
        queue_repository,
        task_processor,
        log_builder.clone(),
    );

    log_builder
        .build(
            ironflow_rs::infrastructure::logging::LogLevel::Info,
            "Worker started successfully".to_string(),
        )
        .log();

    worker.start().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}

/// Loads the worker configuration from environment variables and config files
///
/// # Returns
/// Result containing the WorkerConfig or an error
///
/// # Errors
/// Returns an error if the configuration cannot be loaded or is invalid
fn load_config() -> anyhow::Result<WorkerConfig> {
    let config_builder = config::Config::builder()
        .add_source(config::Environment::with_prefix("IRONFLOW"))
        .build()?;

    match config_builder.try_deserialize::<WorkerConfig>() {
        Ok(config) => Ok(config),
        Err(_) => {
            Ok(WorkerConfig::default())
        }
    }
}

/// Creates a queue repository based on the configuration
///
/// # Arguments
/// * `config` - The worker configuration
///
/// # Returns
/// Result containing the QueueRepository or an error
///
/// # Errors
/// Returns an error if the queue repository cannot be created
async fn create_queue_repository(
    config: &WorkerConfig,
) -> anyhow::Result<Arc<dyn QueueRepository>> {
    match &config.queue {
        QueueConfig::Redis { url, .. } => {
            let repo = RedisQueueRepository::new(url).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(Arc::new(repo) as Arc<dyn QueueRepository>)
        }
        QueueConfig::RabbitMq { url, prefetch_count, .. } => {
            let repo = RabbitMqQueueRepository::new(url, prefetch_count.unwrap_or(10)).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(Arc::new(repo) as Arc<dyn QueueRepository>)
        }
    }
}

/// Creates a secret repository based on the configuration
///
/// # Arguments
/// * `config` - The worker configuration
///
/// # Returns
/// Result containing an optional SecretRepository or an error
///
/// # Errors
/// Returns an error if the secret repository cannot be created
async fn create_secret_repository(
    config: &WorkerConfig,
) -> anyhow::Result<Option<Arc<dyn SecretRepository>>> {
    match &config.secrets {
        SecretsConfig::HashiCorpVault {
            url,
            token,
            mount_path,
            ..
        } => {
            let token = token.as_ref().ok_or_else(|| anyhow::anyhow!("Vault token is required"))?;
            let repo = VaultSecretRepository::new(url, token, mount_path).map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::AwsSecretsManager { region, prefix } => {
            let repo = AwsSecretRepository::new(region, prefix.clone()).await.map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::AzureKeyVault {
            vault_url,
            tenant_id,
            client_id,
            client_secret,
        } => {
            let secret = client_secret.as_ref().ok_or_else(|| anyhow::anyhow!("Azure client secret is required"))?;
            let repo = AzureSecretRepository::new(vault_url, tenant_id, client_id, secret).map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::None => Ok(None),
    }
}
