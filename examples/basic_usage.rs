// examples/basic_usage.rs
//! Basic usage example showing core Lighthouse functionality
//! 
//! This example demonstrates:
//! - Setting up a simple autoscaler
//! - Implementing basic callbacks
//! - Configuring scaling policies
//! - Streaming metrics and getting recommendations
//! 
//! Run with: cargo run --example basic_usage

use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks, ResourceConfig,
    MetricsProvider, ScalingExecutor, ScalingObserver,
    CallbackContext, ScaleAction, ResourceMetrics, LighthouseResult,
    ScalingThreshold, ScalingPolicy, ScaleDirection,
    utils,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use async_trait::async_trait;

/// Simple in-memory metrics provider
/// In a real application, this would connect to your monitoring system
struct SimpleMetricsProvider {
    // Simulate a metrics database
    metrics_store: Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, f64>>>>,
}

impl SimpleMetricsProvider {
    fn new() -> Self {
        Self {
            metrics_store: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Simulate adding metrics (in real app, this would come from your monitoring)
    async fn add_metrics(&self, resource_id: &str, metrics: HashMap<String, f64>) {
        let mut store = self.metrics_store.write().await;
        store.insert(resource_id.to_string(), metrics);
        println!("📊 Added metrics for {}: {:?}", resource_id, store.get(resource_id).unwrap());
    }
}

#[async_trait]
impl MetricsProvider for SimpleMetricsProvider {
    async fn get_metrics(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        let store = self.metrics_store.read().await;
        
        if let Some(metrics) = store.get(resource_id) {
            Ok(Some(ResourceMetrics {
                resource_id: resource_id.to_string(),
                resource_type: "web-service".to_string(),
                timestamp: utils::current_timestamp(),
                metrics: metrics.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn validate_metrics(
        &self,
        metrics: &ResourceMetrics,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        // Simple validation: reject negative values
        for (name, value) in &metrics.metrics {
            if *value < 0.0 {
                println!("⚠️  Rejecting invalid metric {}: {}", name, value);
                return Ok(None);
            }
        }
        Ok(Some(metrics.clone()))
    }
}

/// Simple scaling executor that simulates scaling operations
/// In a real application, this would integrate with your infrastructure
struct SimpleScalingExecutor {
    // Track current capacity for each resource
    current_capacity: Arc<tokio::sync::RwLock<HashMap<String, u32>>>,
}

impl SimpleScalingExecutor {
    fn new() -> Self {
        let mut initial_capacity = HashMap::new();
        initial_capacity.insert("web-frontend".to_string(), 3);
        initial_capacity.insert("api-backend".to_string(), 2);
        
        Self {
            current_capacity: Arc::new(tokio::sync::RwLock::new(initial_capacity)),
        }
    }
}

#[async_trait]
impl ScalingExecutor for SimpleScalingExecutor {
    async fn execute_scale_action(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        let mut capacity = self.current_capacity.write().await;
        let current = capacity.get(&action.resource_id).copied().unwrap_or(1);
        
        let new_capacity = if let Some(target) = action.target_capacity {
            target
        } else if let Some(factor) = action.scale_factor {
            match action.direction {
                ScaleDirection::Up => ((current as f64) * factor).round() as u32,
                ScaleDirection::Down => ((current as f64) / factor).round().max(1.0) as u32,
                ScaleDirection::Maintain => current,
            }
        } else {
            current
        };

        println!(
            "🔧 SCALING: {} {} -> {} instances ({})",
            action.resource_id,
            current,
            new_capacity,
            action.reason
        );

        // Simulate the scaling operation taking some time
        sleep(Duration::from_millis(500)).await;

        capacity.insert(action.resource_id.clone(), new_capacity);
        
        Ok(true)
    }

    async fn is_safe_to_scale(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        let capacity = self.current_capacity.read().await;
        let current = capacity.get(&action.resource_id).copied().unwrap_or(1);
        
        // Simple safety rules
        match action.direction {
            ScaleDirection::Down if current <= 1 => {
                println!("🛡️  SAFETY: Preventing scale down below 1 instance");
                Ok(false)
            }
            ScaleDirection::Up if current >= 10 => {
                println!("🛡️  SAFETY: Preventing scale up above 10 instances");
                Ok(false)
            }
            _ => Ok(true),
        }
    }

    async fn get_current_capacity(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<u32>> {
        let capacity = self.current_capacity.read().await;
        Ok(capacity.get(resource_id).copied())
    }
}

/// Simple observer that logs scaling events
struct ConsoleObserver;

#[async_trait]
impl ScalingObserver for ConsoleObserver {
    async fn on_scaling_decision(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        println!(
            "🎯 DECISION: {} should scale {:?} (confidence: {:.0}%)",
            action.resource_id,
            action.direction,
            action.confidence * 100.0
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
            println!("✅ SUCCESS: Scaled {}", action.resource_id);
        } else {
            println!("⏭️  SKIPPED: Scaling of {}", action.resource_id);
        }
        Ok(())
    }

    async fn on_scaling_skipped(
        &self,
        action: &ScaleAction,
        reason: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        println!("⏸️  SKIPPED: {} - {}", action.resource_id, reason);
        Ok(())
    }

    async fn on_scaling_error(
        &self,
        action: &ScaleAction,
        error: &lighthouse::LighthouseError,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        println!("❌ ERROR: Failed to scale {}: {}", action.resource_id, error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚨 Lighthouse Basic Usage Example");
    println!("==================================\n");

    // 1. Create our implementations
    let metrics_provider = Arc::new(SimpleMetricsProvider::new());
    let scaling_executor = Arc::new(SimpleScalingExecutor::new());
    let observer = Arc::new(ConsoleObserver);

    // 2. Configure lighthouse
    let config = LighthouseConfig::builder()
        .evaluation_interval(5) // Check every 5 seconds
        .add_resource_config(
            "web-service",
            ResourceConfig {
                resource_type: "web-service".to_string(),
                policies: vec![
                    ScalingPolicy {
                        name: "cpu-based-scaling".to_string(),
                        thresholds: vec![
                            ScalingThreshold {
                                metric_name: "cpu_percent".to_string(),
                                scale_up_threshold: 75.0,   // Scale up at 75% CPU
                                scale_down_threshold: 25.0, // Scale down at 25% CPU
                                scale_factor: 1.5,          // Scale by 50%
                                cooldown_seconds: 30,       // Wait 30s between scaling
                                confidence: None,            // Use default confidence
                            },
                            ScalingThreshold {
                                metric_name: "memory_percent".to_string(),
                                scale_up_threshold: 80.0,   // Scale up at 80% memory
                                scale_down_threshold: 30.0, // Scale down at 30% memory
                                scale_factor: 1.3,          // Scale by 30%
                                cooldown_seconds: 45,       // Wait 45s between scaling
                                confidence: None,            // Use default confidence
                            },
                        ],
                        min_capacity: Some(1),
                        max_capacity: Some(10),
                        enabled: true,
                    },
                ],
                default_policy: Some("cpu-based-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .enable_logging(true)
        .build();

    // 3. Create callbacks
    let callbacks = LighthouseCallbacks::new(metrics_provider.clone(), scaling_executor)
        .add_observer(observer);

    // 4. Create and start the engine
    let engine = LighthouseEngine::new(config, callbacks);
    let handle = engine.handle();

    // Start engine in background
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.start().await {
            eprintln!("Engine error: {}", e);
        }
    });

    println!("🚀 Lighthouse engine started!\n");

    // 5. Simulate different scenarios
    
    // Scenario 1: Normal load
    println!("📈 Scenario 1: Normal load");
    metrics_provider.add_metrics("web-frontend", [
        ("cpu_percent".to_string(), 45.0),
        ("memory_percent".to_string(), 55.0),
    ].into()).await;
    
    metrics_provider.add_metrics("api-backend", [
        ("cpu_percent".to_string(), 40.0),
        ("memory_percent".to_string(), 50.0),
    ].into()).await;
    
    sleep(Duration::from_secs(8)).await;

    // Scenario 2: High CPU load (should trigger scale up)
    println!("\n📈 Scenario 2: High CPU load - should scale up!");
    metrics_provider.add_metrics("web-frontend", [
        ("cpu_percent".to_string(), 85.0),
        ("memory_percent".to_string(), 60.0),
    ].into()).await;
    
    sleep(Duration::from_secs(8)).await;

    // Scenario 3: High memory load
    println!("\n🧠 Scenario 3: High memory load - should scale up!");
    metrics_provider.add_metrics("api-backend", [
        ("cpu_percent".to_string(), 50.0),
        ("memory_percent".to_string(), 90.0),
    ].into()).await;
    
    sleep(Duration::from_secs(8)).await;

    // Scenario 4: Low load (should trigger scale down after cooldown)
    println!("\n📉 Scenario 4: Low load - should scale down!");
    metrics_provider.add_metrics("web-frontend", [
        ("cpu_percent".to_string(), 15.0),
        ("memory_percent".to_string(), 20.0),
    ].into()).await;
    
    metrics_provider.add_metrics("api-backend", [
        ("cpu_percent".to_string(), 10.0),
        ("memory_percent".to_string(), 25.0),
    ].into()).await;
    
    sleep(Duration::from_secs(8)).await;

    // 6. Test manual recommendations
    println!("\n🔍 Testing manual recommendations:");
    
    let recommendation = handle.get_recommendation("web-frontend".to_string()).await?;
    match recommendation {
        Some(action) => println!("💡 Recommendation: {:?} - {}", action.direction, action.reason),
        None => println!("💡 No scaling recommendation"),
    }

    // 7. Show engine status
    let status = handle.get_status().await?;
    println!("\n📊 Engine Status:");
    println!("   - Running: {}", status.is_running);
    println!("   - Resources tracked: {}", status.resources_tracked);
    println!("   - Total recommendations: {}", status.total_recommendations);

    // 8. Test live config updates
    println!("\n⚙️  Testing live configuration update...");
    let new_config = LighthouseConfig::builder()
        .evaluation_interval(3) // More frequent evaluation
        .add_resource_config(
            "web-service",
            ResourceConfig {
                resource_type: "web-service".to_string(),
                policies: vec![
                    ScalingPolicy {
                        name: "aggressive-scaling".to_string(),
                        thresholds: vec![
                            ScalingThreshold {
                                metric_name: "cpu_percent".to_string(),
                                scale_up_threshold: 60.0,   // Lower threshold
                                scale_down_threshold: 20.0,
                                scale_factor: 2.0,          // More aggressive scaling
                                cooldown_seconds: 15,       // Shorter cooldown
                                confidence: None,            // Use default confidence
                            },
                        ],
                        min_capacity: Some(1),
                        max_capacity: Some(8),
                        enabled: true,
                    },
                ],
                default_policy: Some("aggressive-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .build();

    handle.update_config(new_config).await?;
    println!("✅ Configuration updated successfully!");

    // Test with new config
    metrics_provider.add_metrics("web-frontend", [
        ("cpu_percent".to_string(), 65.0), // Should trigger with new lower threshold
        ("memory_percent".to_string(), 45.0),
    ].into()).await;
    
    sleep(Duration::from_secs(5)).await;

    // 9. Cleanup
    println!("\n🛑 Shutting down engine...");
    handle.shutdown().await?;
    engine_task.await?;

    println!("✨ Example completed successfully!");
    println!("\nKey takeaways:");
    println!("• Lighthouse automatically evaluates scaling policies");
    println!("• Cooldowns prevent scaling flapping");
    println!("• Safety checks prevent dangerous scaling operations");  
    println!("• Configuration can be updated live");
    println!("• All operations are async and non-blocking");

    Ok(())
}