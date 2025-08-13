// src/types.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Unique identifier for a resource (e.g., "web-servers", "database-pool")
pub type ResourceId = String;

/// A metric value (CPU %, memory usage, request count, etc.)
pub type MetricValue = f64;

/// Unix timestamp in seconds
pub type Timestamp = u64;

/// Represents metrics for a specific resource at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Unique identifier for this resource
    pub resource_id: ResourceId,
    /// Type of resource (e.g., "kubernetes-deployment", "ec2-asg", "database")
    pub resource_type: String,
    /// When these metrics were collected
    pub timestamp: Timestamp,
    /// Key-value pairs of metric names to values
    /// Examples: "cpu_percent" -> 75.5, "memory_mb" -> 2048.0
    pub metrics: HashMap<String, MetricValue>,
}

/// Direction to scale a resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScaleDirection {
    /// Scale up (add resources)
    Up,
    /// Scale down (remove resources)  
    Down,
    /// Keep current scale
    Maintain,
}

/// A scaling recommendation from the lighthouse engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleAction {
    /// Which resource to scale
    pub resource_id: ResourceId,
    /// Type of resource being scaled
    pub resource_type: String,
    /// Direction to scale
    pub direction: ScaleDirection,
    /// Specific target (e.g., number of instances)
    pub target_capacity: Option<u32>,
    /// Multiplier for scaling (e.g., 1.5x current capacity)
    pub scale_factor: Option<f64>,
    /// Human-readable explanation
    pub reason: String,
    /// How confident the engine is (0.0 = uncertain, 1.0 = very confident)
    pub confidence: f64,
    /// When this recommendation was generated
    pub timestamp: Timestamp,
    /// Optional metadata for the scaling action
    pub metadata: HashMap<String, String>,
}

/// Defines when and how to scale based on metric thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingThreshold {
    /// The metric to watch (e.g., "cpu_percent")
    pub metric_name: String,
    /// Scale up when metric goes above this value
    pub scale_up_threshold: MetricValue,
    /// Scale down when metric goes below this value  
    pub scale_down_threshold: MetricValue,
    /// How much to scale by (e.g., 1.5 = increase by 50%)
    pub scale_factor: f64,
    /// Minimum time between scaling actions (prevents flapping)
    pub cooldown_seconds: u64,
    /// Confidence level for scaling actions (0.0 to 1.0)
    pub confidence: Option<f64>,
}

/// A scaling policy that combines multiple thresholds and rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    /// Name of this policy (e.g., "aggressive-cpu-scaling")
    pub name: String,
    /// List of thresholds to evaluate
    pub thresholds: Vec<ScalingThreshold>,
    /// Minimum number of instances/capacity
    pub min_capacity: Option<u32>,
    /// Maximum number of instances/capacity  
    pub max_capacity: Option<u32>,
    /// Whether this policy is currently active
    pub enabled: bool,
}

/// Configuration for a specific resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Type identifier (e.g., "kubernetes-deployment")
    pub resource_type: String,
    /// Scaling policies for this resource type
    pub policies: Vec<ScalingPolicy>,
    /// Default policy to use if none specified
    pub default_policy: Option<String>,
    /// Custom settings specific to this resource type
    pub settings: HashMap<String, String>,
}

/// Main configuration for the lighthouse engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighthouseConfig {
    /// How often to evaluate scaling decisions (seconds)
    pub evaluation_interval_seconds: u64,
    /// Configuration for each resource type
    pub resource_configs: HashMap<String, ResourceConfig>,
    /// Global settings that apply to all resources
    pub global_settings: HashMap<String, String>,
    /// Whether to log scaling decisions
    pub enable_logging: bool,
}

impl Default for LighthouseConfig {
    fn default() -> Self {
        Self {
            evaluation_interval_seconds: 30,
            resource_configs: HashMap::new(),
            global_settings: HashMap::new(),
            enable_logging: true,
        }
    }
}

/// Builder pattern for easy configuration creation
impl LighthouseConfig {
    pub fn builder() -> LighthouseConfigBuilder {
        LighthouseConfigBuilder::new()
    }
}

/// Builder for creating lighthouse configurations easily
#[derive(Debug)]
pub struct LighthouseConfigBuilder {
    config: LighthouseConfig,
}

impl LighthouseConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: LighthouseConfig::default(),
        }
    }

    pub fn evaluation_interval(mut self, seconds: u64) -> Self {
        self.config.evaluation_interval_seconds = seconds;
        self
    }

    pub fn add_resource_config(mut self, resource_type: &str, config: ResourceConfig) -> Self {
        self.config.resource_configs.insert(resource_type.to_string(), config);
        self
    }

    pub fn global_setting(mut self, key: &str, value: &str) -> Self {
        self.config.global_settings.insert(key.to_string(), value.to_string());
        self
    }

    pub fn enable_logging(mut self, enabled: bool) -> Self {
        self.config.enable_logging = enabled;
        self
    }

    pub fn build(self) -> LighthouseConfig {
        self.config
    }
}