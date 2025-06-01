// src/error.rs

use crate::types::ResourceId;

/// Result type used throughout the lighthouse library
pub type LighthouseResult<T> = Result<T, LighthouseError>;

/// All possible errors that can occur in the lighthouse library
#[derive(thiserror::Error, Debug)]
pub enum LighthouseError {
    /// Configuration-related errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Resource not found or invalid
    #[error("Resource '{resource_id}' not found or invalid")]
    ResourceNotFound { resource_id: ResourceId },

    /// Invalid or missing metrics
    #[error("Invalid metric data: {message}")]
    InvalidMetric { message: String },

    /// Callback execution failed
    #[error("Callback execution failed for '{operation}': {message}")]
    CallbackFailed { operation: String, message: String },

    /// Engine is not running or has stopped
    #[error("Lighthouse engine is not running: {message}")]
    EngineNotRunning { message: String },

    /// Policy evaluation failed
    #[error("Failed to evaluate scaling policy '{policy_name}': {message}")]
    PolicyEvaluation { policy_name: String, message: String },

    /// Channel communication error (internal)
    #[error("Internal channel error: {message}")]
    ChannelError { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },

    /// IO-related errors
    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// Generic error for unexpected situations
    #[error("Unexpected error: {message}")]
    Unexpected { message: String },
}

/// Helper methods for creating common errors
impl LighthouseError {
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn resource_not_found<S: Into<String>>(resource_id: S) -> Self {
        Self::ResourceNotFound {
            resource_id: resource_id.into(),
        }
    }

    pub fn invalid_metric<S: Into<String>>(message: S) -> Self {
        Self::InvalidMetric {
            message: message.into(),
        }
    }

    pub fn callback_failed<S: Into<String>>(operation: S, message: S) -> Self {
        Self::CallbackFailed {
            operation: operation.into(),
            message: message.into(),
        }
    }

    pub fn engine_not_running<S: Into<String>>(message: S) -> Self {
        Self::EngineNotRunning {
            message: message.into(),
        }
    }

    pub fn policy_evaluation<S: Into<String>>(policy_name: S, message: S) -> Self {
        Self::PolicyEvaluation {
            policy_name: policy_name.into(),
            message: message.into(),
        }
    }

    pub fn unexpected<S: Into<String>>(message: S) -> Self {
        Self::Unexpected {
            message: message.into(),
        }
    }
}

/// Convert from channel send errors
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for LighthouseError {
    fn from(error: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelError {
            message: format!("Failed to send on channel: {}", error),
        }
    }
}

/// Convert from channel receive errors  
impl From<tokio::sync::oneshot::error::RecvError> for LighthouseError {
    fn from(error: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::ChannelError {
            message: format!("Failed to receive on channel: {}", error),
        }
    }
}