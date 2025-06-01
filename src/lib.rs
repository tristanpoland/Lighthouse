//! # Lighthouse - Intelligent Autoscaling Library
//! 
//! Lighthouse is a flexible, generic autoscaling library for Rust that can work with any
//! infrastructure or resource type. It provides type-safe callbacks and streaming metrics
//! processing to make scaling decisions.
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use lighthouse::{
//!     LighthouseEngine, LighthouseConfig, LighthouseCallbacks,
//!     ScalingThreshold, ScalingPolicy, ResourceConfig,
//!     MetricsProvider, ScalingExecutor, ResourceMetrics, CallbackContext, ScaleAction, LighthouseResult
//! };
//! use std::sync::Arc;
//! use std::collections::HashMap;
//! 
//! // Placeholder implementations for the example
//! struct MyMetricsProvider;
//! #[async_trait::async_trait]
//! impl MetricsProvider for MyMetricsProvider {
//!     async fn get_metrics(
//!         &self,
//!         _resource_id: &str,
//!         _context: &CallbackContext,
//!     ) -> LighthouseResult<Option<ResourceMetrics>> {
//!         Ok(None)
//!     }
//! }
//! struct MyScalingExecutor;
//! #[async_trait::async_trait]
//! impl ScalingExecutor for MyScalingExecutor {
//!     async fn execute_scale_action(
//!         &self,
//!         _action: &ScaleAction,
//!         _context: &CallbackContext,
//!     ) -> LighthouseResult<bool> {
//!         Ok(true)
//!     }
//! }
//! 
//! #[tokio::main]
//! async fn main() {
//!     // 1. Create configuration
//!     let config = LighthouseConfig::builder()
//!         .evaluation_interval(30) // Check every 30 seconds
//!         .add_resource_config("web-servers", ResourceConfig {
//!             resource_type: "web-servers".to_string(),
//!             policies: vec![ScalingPolicy {
//!                 name: "cpu-scaling".to_string(),
//!                 thresholds: vec![ScalingThreshold {
//!                     metric_name: "cpu_percent".to_string(),
//!                     scale_up_threshold: 80.0,
//!                     scale_down_threshold: 20.0,
//!                     scale_factor: 1.5,
//!                     cooldown_seconds: 300,
//!                 }],
//!                 min_capacity: Some(2),
//!                 max_capacity: Some(20),
//!                 enabled: true,
//!             }],
//!             default_policy: Some("cpu-scaling".to_string()),
//!             settings: HashMap::new(),
//!         })
//!         .build();
//! 
//!     // 2. Implement callbacks (your infrastructure integration)
//!     let metrics_provider = Arc::new(MyMetricsProvider);
//!     let scaling_executor = Arc::new(MyScalingExecutor);
//! 
//!     let callbacks = LighthouseCallbacks::new(metrics_provider, scaling_executor);
//! 
//!     // 3. Start the engine
//!     let engine = LighthouseEngine::new(config, callbacks);
//!     let handle = engine.handle();
//! 
//!     // Start engine (runs forever)
//!     tokio::spawn(async move {
//!         engine.start().await.unwrap();
//!     });
//! 
//!     // 4. Send metrics
//!     handle.update_metrics(ResourceMetrics {
//!         resource_id: "my-web-servers".to_string(),
//!         resource_type: "web-servers".to_string(),
//!         timestamp: 1234567890,
//!         metrics: [("cpu_percent".to_string(), 85.0)].into(),
//!     }).await.unwrap();
//! }
//! ```
//! 
//! ## Features
//! 
//! - **Generic**: Works with any infrastructure (Kubernetes, AWS, bare metal, etc.)
//! - **Type-safe**: Compile-time guarantees for your scaling logic
//! - **Live updates**: Change configuration without restarting
//! - **Cooldown handling**: Prevents scaling flapping
//! - **Observability**: Built-in hooks for logging and monitoring
//! - **Async**: Fully async/await compatible

pub mod types;
pub mod error;
pub mod callbacks;
pub mod engine;

// Re-export common types for convenience
pub use types::{
    LighthouseConfig, LighthouseConfigBuilder,
    ResourceMetrics, ScaleAction, ScaleDirection,
    ScalingThreshold, ScalingPolicy, ResourceConfig,
    ResourceId, MetricValue, Timestamp,
};

pub use error::{LighthouseError, LighthouseResult};

pub use callbacks::{
    CallbackContext, LighthouseCallbacks,
    MetricsProvider, ScalingExecutor, ScalingObserver,
};

pub use engine::{
    LighthouseEngine, LighthouseHandle, EngineStatus,
};

/// Convenience builder for creating common scaling policies
pub mod policies {
    use crate::types::{ScalingThreshold, ScalingPolicy};

    /// Create a simple CPU-based scaling policy
    pub fn cpu_scaling_policy(
        scale_up_threshold: f64,
        scale_down_threshold: f64,
        scale_factor: f64,
        cooldown_seconds: u64,
    ) -> ScalingPolicy {
        ScalingPolicy {
            name: "cpu-scaling".to_string(),
            thresholds: vec![ScalingThreshold {
                metric_name: "cpu_percent".to_string(),
                scale_up_threshold,
                scale_down_threshold,
                scale_factor,
                cooldown_seconds,
            }],
            min_capacity: None,
            max_capacity: None,
            enabled: true,
        }
    }

    /// Create a memory-based scaling policy
    pub fn memory_scaling_policy(
        scale_up_threshold: f64,
        scale_down_threshold: f64,
        scale_factor: f64,
        cooldown_seconds: u64,
    ) -> ScalingPolicy {
        ScalingPolicy {
            name: "memory-scaling".to_string(),
            thresholds: vec![ScalingThreshold {
                metric_name: "memory_percent".to_string(),
                scale_up_threshold,
                scale_down_threshold,
                scale_factor,
                cooldown_seconds,
            }],
            min_capacity: None,
            max_capacity: None,
            enabled: true,
        }
    }

    /// Create a request-rate based scaling policy
    pub fn request_rate_scaling_policy(
        scale_up_threshold: f64,
        scale_down_threshold: f64,
        scale_factor: f64,
        cooldown_seconds: u64,
    ) -> ScalingPolicy {
        ScalingPolicy {
            name: "request-rate-scaling".to_string(),
            thresholds: vec![ScalingThreshold {
                metric_name: "requests_per_second".to_string(),
                scale_up_threshold,
                scale_down_threshold,
                scale_factor,
                cooldown_seconds,
            }],
            min_capacity: None,
            max_capacity: None,
            enabled: true,
        }
    }

    /// Create a multi-metric scaling policy
    pub fn multi_metric_policy(
        name: &str,
        cpu_threshold: (f64, f64),
        memory_threshold: (f64, f64),
        scale_factor: f64,
        cooldown_seconds: u64,
    ) -> ScalingPolicy {
        ScalingPolicy {
            name: name.to_string(),
            thresholds: vec![
                ScalingThreshold {
                    metric_name: "cpu_percent".to_string(),
                    scale_up_threshold: cpu_threshold.0,
                    scale_down_threshold: cpu_threshold.1,
                    scale_factor,
                    cooldown_seconds,
                },
                ScalingThreshold {
                    metric_name: "memory_percent".to_string(),
                    scale_up_threshold: memory_threshold.0,
                    scale_down_threshold: memory_threshold.1,
                    scale_factor,
                    cooldown_seconds,
                },
            ],
            min_capacity: None,
            max_capacity: None,
            enabled: true,
        }
    }
}

/// Utility functions for common operations
pub mod utils {
    use crate::types::{ResourceMetrics, MetricValue};
    use std::collections::HashMap;

    /// Create ResourceMetrics with a single metric
    pub fn single_metric(
        resource_id: &str,
        resource_type: &str,
        metric_name: &str,
        value: MetricValue,
    ) -> ResourceMetrics {
        let mut metrics = HashMap::new();
        metrics.insert(metric_name.to_string(), value);
        
        ResourceMetrics {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            timestamp: current_timestamp(),
            metrics,
        }
    }

    /// Create ResourceMetrics with multiple metrics
    pub fn multi_metrics(
        resource_id: &str,
        resource_type: &str,
        metrics: Vec<(&str, MetricValue)>,
    ) -> ResourceMetrics {
        let metrics_map: HashMap<String, MetricValue> = metrics
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        ResourceMetrics {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            timestamp: current_timestamp(),
            metrics: metrics_map,
        }
    }

    /// Get current Unix timestamp
    pub fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    // Mock implementations for testing
    struct MockMetricsProvider {
        metrics: Arc<Mutex<HashMap<String, ResourceMetrics>>>,
    }

    #[async_trait::async_trait]
    impl MetricsProvider for MockMetricsProvider {
        async fn get_metrics(
            &self,
            resource_id: &str,
            _context: &CallbackContext,
        ) -> LighthouseResult<Option<ResourceMetrics>> {
            let metrics = self.metrics.lock().await;
            Ok(metrics.get(resource_id).cloned())
        }
    }

    struct MockScalingExecutor;

    #[async_trait::async_trait]
    impl ScalingExecutor for MockScalingExecutor {
        async fn execute_scale_action(
            &self,
            _action: &ScaleAction,
            _context: &CallbackContext,
        ) -> LighthouseResult<bool> {
            Ok(true) // Always succeed
        }
    }

    #[tokio::test]
    async fn test_basic_engine_creation() {
        let config = LighthouseConfig::default();
        let callbacks = LighthouseCallbacks::new(
            Arc::new(MockMetricsProvider {
                metrics: Arc::new(Mutex::new(HashMap::new())),
            }),
            Arc::new(MockScalingExecutor),
        );

        let engine = LighthouseEngine::new(config, callbacks);
        let handle = engine.handle();

        // Test that we can get status
        tokio::spawn(async move {
            let _ = engine.start().await;
        });

        // Give engine time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let status = handle.get_status().await.unwrap();
        assert!(status.is_running);
        
        handle.shutdown().await.unwrap();
    }
}