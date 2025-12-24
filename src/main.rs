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
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    
    log::info!("🚀 IronFlow-Rs starting up...");

    log::debug!("Loading configuration from environment variables");
    let config = load_config()?;
    log::info!("✓ Configuration loaded successfully");
    log::debug!("Worker ID: {}, Concurrency: {}", config.worker_id, config.worker.concurrency);

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

    log::debug!("Initializing queue repository");
    let queue_repository = create_queue_repository(&config).await?;
    log::info!("✓ Queue repository initialized");
    
    log::debug!("Initializing secret repository");
    let secret_repository = create_secret_repository(&config).await?;
    if secret_repository.is_some() {
        log::info!("✓ Secret repository initialized");
    } else {
        log::warn!("⚠ No secret repository configured");
    }

    log::debug!("Initializing core services");
    let command_executor = Arc::new(DefaultCommandExecutor::new()) as Arc<dyn CommandExecutor>;
    let file_transfer_service =
        Arc::new(OpenDalFileTransferService::new()) as Arc<dyn FileTransferService>;
    let file_watcher_service =
        Arc::new(NotifyFileWatcherService::new()) as Arc<dyn FileWatcherService>;
    log::info!("✓ Core services initialized");

    log::debug!("Creating task processor");
    let task_processor = Arc::new(TaskProcessor::new(
        command_executor,
        file_transfer_service,
        file_watcher_service,
        secret_repository,
        log_builder.clone(),
    ));
    log::info!("✓ Task processor created");

    log::debug!("Creating worker instance");
    let worker = Worker::new(
        config,
        queue_repository,
        task_processor,
        log_builder.clone(),
    );
    log::info!("✓ Worker instance created");

    log_builder
        .build(
            ironflow_rs::infrastructure::logging::LogLevel::Info,
            "Worker started successfully".to_string(),
        )
        .log();

    log::info!("🎯 Starting worker main loop");
    worker.start().await.map_err(|e| {
        log::error!("❌ Worker failed: {}", e);
        anyhow::anyhow!("{}", e)
    })?;

    log::info!("👋 IronFlow-Rs shutting down gracefully");
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
    log::debug!("Building configuration from environment");
    let config_builder = config::Config::builder()
        .add_source(config::Environment::with_prefix("IRONFLOW"))
        .build()?;

    match config_builder.try_deserialize::<WorkerConfig>() {
        Ok(config) => {
            log::debug!("Configuration deserialized successfully");
            Ok(config)
        },
        Err(e) => {
            log::warn!("Failed to deserialize config: {}, using defaults", e);
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
            log::debug!("Connecting to Redis queue at: {}", url);
            let repo = RedisQueueRepository::new(url).await.map_err(|e| {
                log::error!("Failed to connect to Redis: {}", e);
                anyhow::anyhow!("{}", e)
            })?;
            log::info!("✓ Connected to Redis queue");
            Ok(Arc::new(repo) as Arc<dyn QueueRepository>)
        }
        QueueConfig::RabbitMq { url, prefetch_count, .. } => {
            let prefetch = prefetch_count.unwrap_or(10);
            log::debug!("Connecting to RabbitMQ queue at: {} (prefetch: {})", url, prefetch);
            let repo = RabbitMqQueueRepository::new(url, prefetch).await.map_err(|e| {
                log::error!("Failed to connect to RabbitMQ: {}", e);
                anyhow::anyhow!("{}", e)
            })?;
            log::info!("✓ Connected to RabbitMQ queue");
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
            log::debug!("Connecting to HashiCorp Vault at: {} (mount: {})", url, mount_path);
            let token = token.as_ref().ok_or_else(|| anyhow::anyhow!("Vault token is required"))?;
            let repo = VaultSecretRepository::new(url, token, mount_path).map_err(|e| {
                log::error!("Failed to connect to Vault: {}", e);
                anyhow::anyhow!("{}", e)
            })?;
            log::info!("✓ Connected to HashiCorp Vault");
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::AwsSecretsManager { region, prefix } => {
            log::debug!("Connecting to AWS Secrets Manager in region: {}", region);
            let repo = AwsSecretRepository::new(region, prefix.clone()).await.map_err(|e| {
                log::error!("Failed to connect to AWS Secrets Manager: {}", e);
                anyhow::anyhow!("{}", e)
            })?;
            log::info!("✓ Connected to AWS Secrets Manager");
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::AzureKeyVault {
            vault_url,
            tenant_id,
            client_id,
            client_secret,
        } => {
            log::debug!("Connecting to Azure Key Vault at: {}", vault_url);
            let secret = client_secret.as_ref().ok_or_else(|| anyhow::anyhow!("Azure client secret is required"))?;
            let repo = AzureSecretRepository::new(vault_url, tenant_id, client_id, secret).map_err(|e| {
                log::error!("Failed to connect to Azure Key Vault: {}", e);
                anyhow::anyhow!("{}", e)
            })?;
            log::info!("✓ Connected to Azure Key Vault");
            Ok(Some(Arc::new(repo) as Arc<dyn SecretRepository>))
        }
        SecretsConfig::None => {
            log::debug!("No secret repository configured");
            Ok(None)
        },
    }
}
