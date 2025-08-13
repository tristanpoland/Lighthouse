//! Convenience builder for creating common scaling policies

use crate::types::{ScalingPolicy, ScalingThreshold};

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
