use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Log type enumeration based on standard-app-log specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogType {
    AppLog,
    ReqLog,
    ReqExLog,
    ResLog,
    ResExLog,
    PiiLog,
}

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Standard application log structure
/// Based on https://github.com/preedep/standard-app-log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdAppLog {
    pub event_date_time: DateTime<Utc>,
    pub log_type: LogType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_pod_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_channel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    pub level: LogLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<u32>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Request>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_log: Option<PIILog>,
}

/// Request structure for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub host: String,
    pub headers: HashMap<String, String>,
    pub url: String,
    pub method: String,
    pub body: Value,
}

/// Response structure for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u32,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// PII Log structure for logging personally identifiable information events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIILog {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_criteria: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<PIILogUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_values: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_key_values: Option<HashMap<String, String>>,
}

/// PII Log update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIILogUpdate {
    pub previous_values: HashMap<String, String>,
    pub new_values: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_values: Option<HashMap<String, String>>,
}

impl StdAppLog {
    /// Creates a new application log entry
    ///
    /// # Arguments
    /// * `level` - The log level
    /// * `message` - The log message
    ///
    /// # Returns
    /// A new StdAppLog instance
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            event_date_time: Utc::now(),
            log_type: LogType::AppLog,
            app_id: None,
            app_version: None,
            app_address: None,
            geo_location: None,
            service_id: None,
            service_version: None,
            service_pod_name: None,
            code_location: None,
            caller_channel_name: None,
            caller_user: None,
            caller_address: None,
            correlation_id: None,
            request_id: None,
            trace_id: None,
            span_id: None,
            level,
            execution_time: None,
            message,
            request: None,
            response: None,
            pii_log: None,
        }
    }

    /// Sets the correlation ID
    ///
    /// # Arguments
    /// * `correlation_id` - The correlation ID to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Sets the code location
    ///
    /// # Arguments
    /// * `location` - The code location to set
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_code_location(mut self, location: String) -> Self {
        self.code_location = Some(location);
        self
    }

    /// Sets the execution time
    ///
    /// # Arguments
    /// * `time_ms` - The execution time in milliseconds
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_execution_time(mut self, time_ms: u32) -> Self {
        self.execution_time = Some(time_ms);
        self
    }

    /// Logs the entry as JSON to stdout
    pub fn log(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{}", json);
        }
    }
}

/// Builder for creating StdAppLog instances with application context
#[derive(Clone)]
pub struct LogBuilder {
    app_id: Option<String>,
    app_version: Option<String>,
    app_address: Option<String>,
    geo_location: Option<String>,
    service_id: Option<String>,
    service_version: Option<String>,
    service_pod_name: Option<String>,
}

impl LogBuilder {
    /// Creates a new LogBuilder
    ///
    /// # Returns
    /// A new LogBuilder instance
    pub fn new() -> Self {
        Self {
            app_id: None,
            app_version: None,
            app_address: None,
            geo_location: None,
            service_id: None,
            service_version: None,
            service_pod_name: None,
        }
    }

    /// Sets the application ID
    ///
    /// # Arguments
    /// * `app_id` - The application ID
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_app_id(mut self, app_id: String) -> Self {
        self.app_id = Some(app_id);
        self
    }

    /// Sets the application version
    ///
    /// # Arguments
    /// * `version` - The application version
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_app_version(mut self, version: String) -> Self {
        self.app_version = Some(version);
        self
    }

    /// Sets the service ID
    ///
    /// # Arguments
    /// * `service_id` - The service ID
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_service_id(mut self, service_id: String) -> Self {
        self.service_id = Some(service_id);
        self
    }

    /// Sets the service version
    ///
    /// # Arguments
    /// * `version` - The service version
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_service_version(mut self, version: String) -> Self {
        self.service_version = Some(version);
        self
    }

    /// Builds a log entry with the configured context
    ///
    /// # Arguments
    /// * `level` - The log level
    /// * `message` - The log message
    ///
    /// # Returns
    /// A StdAppLog instance with the configured context
    pub fn build(&self, level: LogLevel, message: String) -> StdAppLog {
        let mut log = StdAppLog::new(level, message);
        log.app_id = self.app_id.clone();
        log.app_version = self.app_version.clone();
        log.app_address = self.app_address.clone();
        log.geo_location = self.geo_location.clone();
        log.service_id = self.service_id.clone();
        log.service_version = self.service_version.clone();
        log.service_pod_name = self.service_pod_name.clone();
        log
    }
}

impl Default for LogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_creation() {
        let log = StdAppLog::new(LogLevel::Info, "Test message".to_string());
        assert_eq!(log.level, LogLevel::Info);
        assert_eq!(log.message, "Test message");
        assert_eq!(log.log_type, LogType::AppLog);
    }

    #[test]
    fn test_log_with_correlation_id() {
        let log = StdAppLog::new(LogLevel::Info, "Test".to_string())
            .with_correlation_id("test-id".to_string());
        assert_eq!(log.correlation_id, Some("test-id".to_string()));
    }

    #[test]
    fn test_log_builder() {
        let builder = LogBuilder::new()
            .with_app_id("test-app".to_string())
            .with_service_id("test-service".to_string());

        let log = builder.build(LogLevel::Info, "Test message".to_string());
        assert_eq!(log.app_id, Some("test-app".to_string()));
        assert_eq!(log.service_id, Some("test-service".to_string()));
    }

    #[test]
    fn test_log_serialization() {
        let log = StdAppLog::new(LogLevel::Info, "Test".to_string());
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"message\":\"Test\""));
    }
}
