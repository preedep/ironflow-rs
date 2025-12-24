use serde::{Deserialize, Serialize};

/// Configuration for the IronFlow worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Worker identification
    pub worker_id: String,
    /// Application information
    pub app: AppConfig,
    /// Queue configuration
    pub queue: QueueConfig,
    /// Secret management configuration
    pub secrets: SecretsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Worker behavior configuration
    pub worker: WorkerBehaviorConfig,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub id: String,
    pub version: String,
    pub address: Option<String>,
    pub geo_location: Option<String>,
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueConfig {
    Redis {
        url: String,
        task_queue: String,
        result_queue: String,
        max_connections: Option<u32>,
    },
    RabbitMq {
        url: String,
        task_queue: String,
        result_queue: String,
        exchange: Option<String>,
        prefetch_count: Option<u16>,
    },
}

/// Secret management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SecretsConfig {
    HashiCorpVault {
        url: String,
        token: Option<String>,
        namespace: Option<String>,
        mount_path: String,
    },
    AwsSecretsManager {
        region: String,
        prefix: Option<String>,
    },
    AzureKeyVault {
        vault_url: String,
        tenant_id: String,
        client_id: String,
        client_secret: Option<String>,
    },
    None,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: debug, info, warn, error
    pub level: String,
    /// Log format: json, text
    pub format: String,
    /// Service identification
    pub service_id: String,
    pub service_version: String,
    pub service_pod_name: Option<String>,
}

/// Worker behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerBehaviorConfig {
    /// Number of concurrent tasks to process
    pub concurrency: usize,
    /// Timeout for receiving tasks from queue (seconds)
    pub receive_timeout_secs: u64,
    /// Maximum task execution time (seconds)
    pub max_task_timeout_secs: u64,
    /// Graceful shutdown timeout (seconds)
    pub shutdown_timeout_secs: u64,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_id: uuid::Uuid::new_v4().to_string(),
            app: AppConfig {
                id: "ironflow-rs".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                address: None,
                geo_location: None,
            },
            queue: QueueConfig::Redis {
                url: "redis://localhost:6379".to_string(),
                task_queue: "ironflow:tasks".to_string(),
                result_queue: "ironflow:results".to_string(),
                max_connections: Some(10),
            },
            secrets: SecretsConfig::None,
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                service_id: "ironflow-worker".to_string(),
                service_version: env!("CARGO_PKG_VERSION").to_string(),
                service_pod_name: None,
            },
            worker: WorkerBehaviorConfig {
                concurrency: 4,
                receive_timeout_secs: 30,
                max_task_timeout_secs: 3600,
                shutdown_timeout_secs: 30,
                health_check_interval_secs: 60,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorkerConfig::default();
        assert_eq!(config.app.id, "ironflow-rs");
        assert_eq!(config.worker.concurrency, 4);
    }

    #[test]
    fn test_config_serialization() {
        let config = WorkerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WorkerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.app.id, deserialized.app.id);
    }
}
