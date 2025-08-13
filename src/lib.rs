//! # Lighthouse - Intelligent Autoscaling Library
//! 
//! Lighthouse is a flexible, generic autoscaling library for Rust that can work with any
//! infrastructure or resource type. It provides type-safe callbacks, streaming metrics
//! processing, advanced policy composition, predictive scaling, and comprehensive
//! historical data analysis to make intelligent scaling decisions.
//! 
//! ## 🎯 Core Philosophy
//! 
//! Lighthouse is designed to be **infrastructure-agnostic** and **policy-flexible**:
//! - **Generic by Design**: Works with any platform through trait implementations
//! - **Type Safety First**: Compile-time guarantees for scaling logic
//! - **Production Ready**: Built for reliability, observability, and performance
//! - **Extensible**: Advanced features through optional feature flags
//! 
//! ## 📊 Architecture Overview
//! 
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           Lighthouse Engine                                 │
//! ├─────────────────┬─────────────────┬─────────────────┬───────────────────────┤
//! │  Policy Engine  │ Metrics Storage │  Predictive AI  │   Observer System     │
//! │                 │                 │                 │                       │
//! │ • Basic Policies│ • SQLite Backend│ • Forecasting   │ • Event Logging       │
//! │ • Composite     │ • Retention     │ • Trend Analysis│ • Custom Hooks        │  
//! │ • Time-Based    │ • Aggregation   │ • Seasonality   │ • Monitoring          │
//! │ • Weighted      │ • Statistics    │ • Anomalies     │ • Alerting            │
//! └─────────────────┴─────────────────┴─────────────────┴───────────────────────┘
//!                                        │
//!                              ┌─────────▼─────────┐
//!                              │   Your Callbacks  │
//!                              │                   │
//!                              │ • MetricsProvider │
//!                              │ • ScalingExecutor │ 
//!                              │ • ScalingObserver │
//!                              └─────────┬─────────┘
//!                                        │
//!                     ┌──────────────────▼──────────────────┐
//!                     │          Your Infrastructure        │
//!                     │                                     │
//!                     │ Kubernetes • AWS • Docker • GCP     │
//!                     │ Bare Metal • Custom APIs • More     │
//!                     └─────────────────────────────────────┘
//! ```
//! 
//! ## 🚀 Feature Matrix
//! 
//! | Feature | Basic | With Persistence | With Predictive | Full Features |
//! |---------|-------|------------------|-----------------|---------------|
//! | Core Scaling | ✅ | ✅ | ✅ | ✅ |
//! | Policy Composition | ✅ | ✅ | ✅ | ✅ |
//! | Time-Based Policies | 🟡¹ | ✅ | ✅ | ✅ |
//! | Historical Storage | ❌ | ✅ | ✅ | ✅ |
//! | Trend Analysis | ❌ | ✅ | ✅ | ✅ |
//! | Predictive Scaling | ❌ | ❌ | ✅ | ✅ |
//! | Anomaly Detection | ❌ | ❌ | ✅ | ✅ |
//! | Statistical Reports | ❌ | ✅ | ✅ | ✅ |
//! 
//! ¹ Basic time-based policies require `time-utils` feature
//! 
//! ## 🎛️ Usage Patterns
//! 
//! ### Basic Autoscaling
//! Perfect for simple CPU/memory-based scaling:
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
//!                     confidence: None,
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

pub mod error;
pub mod utils;
pub mod types;
pub mod tests;
pub mod engine;
pub mod policies;
pub mod callbacks;
#[cfg(feature = "predictive-scaling")]
pub mod predictive;
#[cfg(feature = "metrics-persistence")]
pub mod persistence;

// Re-export common types for convenience
pub use types::{
    LighthouseConfig, LighthouseConfigBuilder,
    ResourceMetrics, ScaleAction, ScaleDirection,
    ScalingThreshold, ScalingPolicy, ResourceConfig,
    ResourceId, MetricValue, Timestamp,
    CompositePolicy, CompositeLogic, CustomPolicyFn,
};

#[cfg(feature = "time-utils")]
pub use types::{TimeSchedule, TimeBasedPolicy};

pub use error::{LighthouseError, LighthouseResult};

pub use callbacks::{
    CallbackContext, LighthouseCallbacks,
    MetricsProvider, ScalingExecutor, ScalingObserver,
};

pub use engine::{
    LighthouseEngine, LighthouseHandle, EngineStatus,
};

#[cfg(feature = "metrics-persistence")]
pub use persistence::{
    MetricsStore, MetricsStoreConfig, MetricsStoreConfigBuilder,
    RetentionPolicy, HistoricalDataPoint, MetricsStatistics,
    TrendAnalysis, TrendDirection, AggregationLevel, MetricsQuery,
    SeasonalPattern, StoreStatistics,
};

#[cfg(feature = "predictive-scaling")]
pub use predictive::{
    PredictiveScaler, PredictiveConfig, PredictiveConfigBuilder,
    ForecastModel, MetricForecast, ForecastPoint, ProactiveRecommendation,
    SeasonalInfo, AnomalyAlert,
};