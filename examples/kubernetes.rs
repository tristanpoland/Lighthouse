// examples/kubernetes.rs
//! Example showing how to use Lighthouse with Kubernetes

use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks, ResourceConfig,
    MetricsProvider, ScalingExecutor, ScalingObserver,
    CallbackContext, ScaleAction, ResourceMetrics, LighthouseResult,
    policies, utils,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use async_trait::async_trait;
use tracing::{info, warn, error};

/// Example metrics provider that simulates fetching metrics from Kubernetes
struct KubernetesMetricsProvider {
    // In real implementation, this would be a Kubernetes client
    simulated_metrics: Arc<tokio::sync::RwLock<HashMap<String, f64>>>,
}

impl KubernetesMetricsProvider {
    fn new() -> Self {
        Self {
            simulated_metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Simulate updating metrics (in real app, this would poll Kubernetes metrics)
    async fn simulate_metrics_update(&self, resource_id: &str, cpu: f64, memory: f64) {
        let mut metrics = self.simulated_metrics.write().await;
        metrics.insert(format!("{}_cpu", resource_id), cpu);
        metrics.insert(format!("{}_memory", resource_id), memory);
    }
}

#[async_trait]
impl MetricsProvider for KubernetesMetricsProvider {
    async fn get_metrics(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        let metrics = self.simulated_metrics.read().await;
        
        let cpu_key = format!("{}_cpu", resource_id);
        let memory_key = format!("{}_memory", resource_id);
        
        let cpu = metrics.get(&cpu_key).copied();
        let memory = metrics.get(&memory_key).copied();

        if let (Some(cpu_val), Some(memory_val)) = (cpu, memory) {
            Ok(Some(utils::multi_metrics(
                resource_id,
                "kubernetes-deployment",
                vec![
                    ("cpu_percent", cpu_val),
                    ("memory_percent", memory_val),
                ],
            )))
        } else {
            Ok(None) // No metrics available yet
        }
    }

    async fn validate_metrics(
        &self,
        metrics: &ResourceMetrics,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        // Example validation: reject unreasonable values
        for (name, value) in &metrics.metrics {
            if name.contains("percent") && (*value < 0.0 || *value > 100.0) {
                warn!("Invalid percentage metric {}: {}", name, value);
                return Ok(None); // Reject these metrics
            }
        }
        
        Ok(Some(metrics.clone()))
    }
}

/// Example scaling executor that simulates Kubernetes scaling operations
struct KubernetesScalingExecutor {
    // In real implementation, this would be a Kubernetes client
    current_replicas: Arc<tokio::sync::RwLock<HashMap<String, u32>>>,
}

impl KubernetesScalingExecutor {
    fn new() -> Self {
        let mut initial_replicas = HashMap::new();
        initial_replicas.insert("web-app".to_string(), 3);
        initial_replicas.insert("api-service".to_string(), 2);
        
        Self {
            current_replicas: Arc::new(tokio::sync::RwLock::new(initial_replicas)),
        }
    }
}

#[async_trait]
impl ScalingExecutor for KubernetesScalingExecutor {
    async fn execute_scale_action(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        let mut replicas = self.current_replicas.write().await;
        let current = replicas.get(&action.resource_id).unwrap_or(&1);
        
        let new_replicas = if let Some(target) = action.target_capacity {
            target
        } else if let Some(factor) = action.scale_factor {
            match action.direction {
                lighthouse::ScaleDirection::Up => {
                    ((*current as f64) * factor).round() as u32
                }
                lighthouse::ScaleDirection::Down => {
                    ((*current as f64) / factor).round().max(1.0) as u32
                }
                lighthouse::ScaleDirection::Maintain => *current,
            }
        } else {
            *current
        };

        // Simulate the scaling operation
        info!(
            "Scaling {} from {} to {} replicas ({})",
            action.resource_id, current, new_replicas, action.reason
        );

        // In real implementation, you would call:
        // kubectl_client.scale_deployment(&action.resource_id, new_replicas).await?;
        
        replicas.insert(action.resource_id.clone(), new_replicas);
        
        // Simulate operation taking some time
        sleep(Duration::from_millis(100)).await;
        
        Ok(true)
    }

    async fn is_safe_to_scale(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        // Example safety checks
        let replicas = self.current_replicas.read().await;
        let current = replicas.get(&action.resource_id).unwrap_or(&1);
        
        // Don't scale below 1 replica
        if action.direction == lighthouse::ScaleDirection::Down && *current <= 1 {
            info!("Skipping scale down - already at minimum replicas");
            return Ok(false);
        }
        
        // Don't scale above 10 replicas (example business rule)
        if action.direction == lighthouse::ScaleDirection::Up && *current >= 10 {
            info!("Skipping scale up - already at maximum replicas");
            return Ok(false);
        }
        
        // Example: Don't scale during maintenance window (9 PM - 6 AM)
        let hour = chrono::Utc::now().hour();
        if hour >= 21 || hour < 6 {
            info!("Skipping scaling during maintenance window");
            return Ok(false);
        }
        
        Ok(true)
    }

    async fn get_current_capacity(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<u32>> {
        let replicas = self.current_replicas.read().await;
        Ok(replicas.get(resource_id).copied())
    }
}

/// Example observer for logging scaling events
struct ScalingLogger;

#[async_trait]
impl ScalingObserver for ScalingLogger {
    async fn on_scaling_decision(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        info!(
            "SCALING DECISION: {} -> {:?} (confidence: {:.1}%, reason: {})",
            action.resource_id,
            action.direction,
            action.confidence * 100.0,
            action.reason
        );
        Ok(())
    }

    async fn on_scaling_executed(
        &self,
        action: &ScaleAction,
        success: bool,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        if success {
            info!("✅ Scaling executed successfully: {}", action.resource_id);
        } else {
            warn!("⚠️  Scaling was skipped: {}", action.resource_id);
        }
        Ok(())
    }

    async fn on_scaling_skipped(
        &self,
        action: &ScaleAction,
        reason: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        info!("⏭️  Scaling skipped for {}: {}", action.resource_id, reason);
        Ok(())
    }

    async fn on_scaling_error(
        &self,
        action: &ScaleAction,
        error: &lighthouse::LighthouseError,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        error!("❌ Scaling error for {}: {}", action.resource_id, error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚨 Starting Lighthouse Kubernetes Autoscaler Example");

    // Create metrics provider and scaling executor
    let metrics_provider = Arc::new(KubernetesMetricsProvider::new());
    let scaling_executor = Arc::new(KubernetesScalingExecutor::new());
    let observer = Arc::new(ScalingLogger);

    // Create lighthouse configuration
    let config = LighthouseConfig::builder()
        .evaluation_interval(10) // Check every 10 seconds for demo
        .add_resource_config(
            "kubernetes-deployment",
            ResourceConfig {
                resource_type: "kubernetes-deployment".to_string(),
                policies: vec![
                    // CPU-based scaling
                    policies::cpu_scaling_policy(
                        70.0, // Scale up at 70% CPU
                        30.0, // Scale down at 30% CPU
                        1.5,  // Scale by 50%
                        60,   // 1 minute cooldown
                    ),
                    // Memory-based scaling
                    policies::memory_scaling_policy(
                        80.0, // Scale up at 80% memory
                        40.0, // Scale down at 40% memory
                        1.3,  // Scale by 30%
                        90,   // 1.5 minute cooldown
                    ),
                ],
                default_policy: Some("cpu-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .global_setting("environment", "demo")
        .enable_logging(true)
        .build();

    // Create callbacks
    let callbacks = LighthouseCallbacks::new(metrics_provider.clone(), scaling_executor)
        .add_observer(observer);

    // Create and start the lighthouse engine
    let engine = LighthouseEngine::new(config, callbacks);
    let handle = engine.handle();

    // Start the engine in the background
    let engine_handle = tokio::spawn(async move {
        if let Err(e) = engine.start().await {
            error!("Engine error: {}", e);
        }
    });

    // Simulate metric updates and scaling scenarios
    info!("🎯 Starting simulation...");

    // Scenario 1: Normal load
    info!("📊 Scenario 1: Normal load");
    metrics_provider.simulate_metrics_update("web-app", 45.0, 60.0).await;
    metrics_provider.simulate_metrics_update("api-service", 35.0, 50.0).await;
    sleep(Duration::from_secs(15)).await;

    // Scenario 2: High CPU load (should trigger scale up)
    info!("📈 Scenario 2: High CPU load");
    metrics_provider.simulate_metrics_update("web-app", 85.0, 65.0).await;
    metrics_provider.simulate_metrics_update("api-service", 75.0, 55.0).await;
    sleep(Duration::from_secs(15)).await;

    // Scenario 3: High memory load (should trigger scale up)
    info!("🧠 Scenario 3: High memory load");
    metrics_provider.simulate_metrics_update("web-app", 60.0, 90.0).await;
    sleep(Duration::from_secs(15)).await;

    // Scenario 4: Low load (should trigger scale down after cooldown)
    info!("📉 Scenario 4: Low load");
    metrics_provider.simulate_metrics_update("web-app", 15.0, 25.0).await;
    metrics_provider.simulate_metrics_update("api-service", 20.0, 30.0).await;
    sleep(Duration::from_secs(15)).await;

    // Check current status
    let status = handle.get_status().await?;
    info!("📋 Engine Status: {:?}", status);

    // Update configuration live
    info!("⚙️  Updating configuration...");
    let new_config = LighthouseConfig::builder()
        .evaluation_interval(5) // More frequent checks
        .add_resource_config(
            "kubernetes-deployment", 
            ResourceConfig {
                resource_type: "kubernetes-deployment".to_string(),
                policies: vec![
                    policies::multi_metric_policy(
                        "aggressive-scaling",
                        (60.0, 20.0), // Lower CPU thresholds
                        (70.0, 30.0), // Lower memory thresholds
                        1.8,          // More aggressive scaling
                        30,           // Shorter cooldown
                    ),
                ],
                default_policy: Some("aggressive-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .build();

    handle.update_config(new_config).await?;
    info!("✅ Configuration updated successfully");

    // Test with new configuration
    info!("🔄 Testing with new configuration...");
    metrics_provider.simulate_metrics_update("web-app", 65.0, 75.0).await;
    sleep(Duration::from_secs(10)).await;

    // Cleanup
    info!("🛑 Shutting down...");
    handle.shutdown().await?;
    engine_handle.await?;

    info!("✨ Example completed successfully!");
    Ok(())
}

// Example of how to add additional dependencies to Cargo.toml:
/*
[dependencies]
lighthouse = { path = "../" }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
*/