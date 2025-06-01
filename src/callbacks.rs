// src/callbacks.rs

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::LighthouseResult;
use crate::types::{ResourceMetrics, ScaleAction};

/// Context provided to callbacks with additional information
#[derive(Debug, Clone)]
pub struct CallbackContext {
    /// Current timestamp when callback is invoked
    pub timestamp: u64,
    /// Any additional metadata from the engine
    pub metadata: HashMap<String, String>,
}

/// Trait for handling metric collection and validation
/// 
/// Implement this to provide metrics to the lighthouse engine.
/// This allows you to integrate with any monitoring system.
#[async_trait]
pub trait MetricsProvider: Send + Sync {
    /// Fetch current metrics for a specific resource
    /// 
    /// # Arguments
    /// * `resource_id` - The resource to get metrics for
    /// * `context` - Additional context for the callback
    /// 
    /// # Returns
    /// * `Ok(Some(metrics))` - Successfully fetched metrics
    /// * `Ok(None)` - Resource exists but no metrics available
    /// * `Err(error)` - Failed to fetch metrics
    async fn get_metrics(
        &self,
        resource_id: &str,
        context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>>;

    /// Validate that metrics are reasonable/expected
    /// 
    /// This is called before metrics are processed by scaling policies.
    /// Use this to filter out bad data, apply smoothing, etc.
    /// 
    /// # Arguments
    /// * `metrics` - The raw metrics to validate
    /// * `context` - Additional context for the callback
    /// 
    /// # Returns
    /// * `Ok(Some(validated_metrics))` - Metrics are valid (possibly modified)
    /// * `Ok(None)` - Metrics should be ignored
    /// * `Err(error)` - Validation failed
    async fn validate_metrics(
        &self,
        metrics: &ResourceMetrics,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        // Default implementation: accept all metrics as-is
        Ok(Some(metrics.clone()))
    }
}

/// Trait for executing scaling actions
/// 
/// Implement this to actually perform scaling operations.
/// This is where you integrate with your infrastructure (K8s, AWS, etc.).
#[async_trait]
pub trait ScalingExecutor: Send + Sync {
    /// Execute a scaling action
    /// 
    /// # Arguments
    /// * `action` - The scaling action to perform
    /// * `context` - Additional context for the callback
    /// 
    /// # Returns
    /// * `Ok(true)` - Action was successfully executed
    /// * `Ok(false)` - Action was skipped (e.g., already at target scale)
    /// * `Err(error)` - Failed to execute action
    async fn execute_scale_action(
        &self,
        action: &ScaleAction,
        context: &CallbackContext,
    ) -> LighthouseResult<bool>;

    /// Check if a scaling action is safe to execute
    /// 
    /// This is called before `execute_scale_action` to provide a safety check.
    /// Use this to implement business rules, maintenance windows, etc.
    /// 
    /// # Arguments
    /// * `action` - The proposed scaling action
    /// * `context` - Additional context for the callback
    /// 
    /// # Returns
    /// * `Ok(true)` - Action is safe to execute
    /// * `Ok(false)` - Action should be skipped
    /// * `Err(error)` - Safety check failed
    async fn is_safe_to_scale(
        &self,
        _action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        // Default implementation: all actions are safe
        Ok(true)
    }

    /// Get current capacity/state of a resource
    /// 
    /// This helps the engine understand the current state before scaling.
    /// 
    /// # Arguments
    /// * `resource_id` - The resource to check
    /// * `context` - Additional context for the callback
    /// 
    /// # Returns
    /// * `Ok(Some(capacity))` - Current capacity (e.g., number of instances)
    /// * `Ok(None)` - Unable to determine current capacity
    /// * `Err(error)` - Failed to check capacity
    async fn get_current_capacity(
        &self,
        _resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<u32>> {
        // Default implementation: unknown capacity
        Ok(None)
    }
}

/// Trait for receiving scaling events and decisions
/// 
/// Implement this to get notified about scaling decisions.
/// Useful for logging, alerting, or custom business logic.
#[async_trait]
pub trait ScalingObserver: Send + Sync {
    /// Called when a scaling decision is made (before execution)
    async fn on_scaling_decision(
        &self,
        _action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        // Default implementation: do nothing
        Ok(())
    }

    /// Called after a scaling action is executed
    async fn on_scaling_executed(
        &self,
        _action: &ScaleAction,
        _success: bool,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        // Default implementation: do nothing
        Ok(())
    }

    /// Called when scaling is skipped (e.g., due to cooldown)
    async fn on_scaling_skipped(
        &self,
        _action: &ScaleAction,
        _reason: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        // Default implementation: do nothing
        Ok(())
    }

    /// Called when an error occurs during scaling
    async fn on_scaling_error(
        &self,
        _action: &ScaleAction,
        _error: &crate::error::LighthouseError,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        // Default implementation: do nothing
        Ok(())
    }
}

/// Combine all callbacks into a single struct for easier management
#[derive(Clone)]
pub struct LighthouseCallbacks {
    pub metrics_provider: std::sync::Arc<dyn MetricsProvider>,
    pub scaling_executor: std::sync::Arc<dyn ScalingExecutor>,
    pub observers: Vec<std::sync::Arc<dyn ScalingObserver>>,
}

impl LighthouseCallbacks {
    /// Create a new callback configuration
    pub fn new(
        metrics_provider: std::sync::Arc<dyn MetricsProvider>,
        scaling_executor: std::sync::Arc<dyn ScalingExecutor>,
    ) -> Self {
        Self {
            metrics_provider,
            scaling_executor,
            observers: Vec::new(),
        }
    }

    /// Add an observer to receive scaling events
    pub fn add_observer(mut self, observer: std::sync::Arc<dyn ScalingObserver>) -> Self {
        self.observers.push(observer);
        self
    }

    /// Add multiple observers at once
    pub fn add_observers(mut self, observers: Vec<std::sync::Arc<dyn ScalingObserver>>) -> Self {
        self.observers.extend(observers);
        self
    }
}