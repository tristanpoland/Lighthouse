//! Utility functions for common operations

use crate::types::{MetricValue, ResourceMetrics};
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
