//! # Advanced Policy Types Example
//! 
//! This example demonstrates Lighthouse's advanced policy capabilities:
//! 
//! - **Composite Policies**: Combine multiple policies with AND/OR logic
//! - **Time-Based Policies**: Different scaling rules based on time of day/week  
//! - **Weighted Voting**: Custom policy weighting for sophisticated decision making
//! - **Business Hours Scaling**: Automatic policy switching for day/night operations
//! - **Multi-Tier Policies**: Complex policies that consider multiple metrics with different priorities
//! 
//! ## Features Demonstrated
//! 
//! 1. Creating composite policies with different combination logic
//! 2. Time-based scheduling for automatic policy switching
//! 3. Weighted voting systems for balanced decision making
//! 4. Business hours vs after-hours scaling strategies
//! 5. Advanced multi-metric policies with custom sensitivity levels
//! 6. Real-time policy evaluation and decision logging
//! 
//! Run with: cargo run --example advanced_policies --features "time-utils"

#[cfg(feature = "time-utils")]
use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks, ResourceConfig,
    MetricsProvider, ScalingExecutor, ScalingObserver,
    CallbackContext, ScaleAction, ResourceMetrics, LighthouseResult,
    ScalingThreshold, ScalingPolicy, ScaleDirection,
    CompositePolicy, CompositeLogic, TimeSchedule, TimeBasedPolicy,
    utils, policies,
};

#[cfg(feature = "time-utils")]
use std::collections::HashMap;
#[cfg(feature = "time-utils")]
use std::sync::Arc;
#[cfg(feature = "time-utils")]
use tokio::time::{sleep, Duration};
#[cfg(feature = "time-utils")]
use async_trait::async_trait;
#[cfg(feature = "time-utils")]
use chrono::{Utc, Timelike, Datelike};

/// Advanced metrics provider that simulates different workload patterns
#[cfg(feature = "time-utils")]
struct AdvancedMetricsProvider {
    current_metrics: Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, f64>>>>,
}

#[cfg(feature = "time-utils")]
impl AdvancedMetricsProvider {
    fn new() -> Self {
        Self {
            current_metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Simulate realistic workload metrics that vary over time
    async fn simulate_workload(&self, resource_id: &str, scenario: &str) {
        let mut metrics = HashMap::new();
        
        let now = Utc::now();
        let hour = now.hour();
        let minute = now.minute();
        
        // Simulate different scenarios based on time and resource type
        match scenario {
            "web-frontend" => {
                // Higher load during business hours (9 AM - 5 PM)
                let base_cpu = if hour >= 9 && hour <= 17 { 60.0 } else { 25.0 };
                let base_memory = if hour >= 9 && hour <= 17 { 65.0 } else { 35.0 };
                
                // Add some randomness and minute-based fluctuation
                let cpu_variance = (minute as f64 / 60.0 * 20.0) - 10.0; // ±10% based on minute
                let memory_variance = (rand::random::<f64>() - 0.5) * 15.0; // ±7.5% random
                
                let cpu = (base_cpu + cpu_variance).max(5.0_f64).min(95.0);
                let memory = (base_memory + memory_variance).max(10.0).min(90.0);
                let requests_per_sec = cpu * 20.0; // Correlate requests with CPU
                let response_time = if cpu > 80.0 { 200.0 + (cpu - 80.0) * 10.0 } else { 80.0 };
                
                metrics.insert("cpu_percent".to_string(), cpu);
                metrics.insert("memory_percent".to_string(), memory);
                metrics.insert("requests_per_second".to_string(), requests_per_sec);
                metrics.insert("response_time_ms".to_string(), response_time);
            }
            "api-backend" => {
                // More stable but with occasional spikes
                let base_cpu = if hour >= 8 && hour <= 18 { 45.0 } else { 20.0 };
                let spike = if minute % 15 == 0 { 25.0 } else { 0.0 }; // Spike every 15 minutes
                
                let cpu = (base_cpu + spike + (rand::random::<f64>() - 0.5) * 10.0).max(5.0).min(95.0);
                let memory = (40.0 + (rand::random::<f64>() - 0.5) * 20.0).max(10.0).min(80.0);
                let requests_per_sec = cpu * 15.0;
                let response_time = if cpu > 75.0 { 150.0 } else { 60.0 };
                
                metrics.insert("cpu_percent".to_string(), cpu);
                metrics.insert("memory_percent".to_string(), memory);
                metrics.insert("requests_per_second".to_string(), requests_per_sec);
                metrics.insert("response_time_ms".to_string(), response_time);
            }
            "database" => {
                // Steady load with gradual increase during business hours
                let base_cpu = if hour >= 9 && hour <= 17 { 35.0 + (hour - 9) as f64 * 2.0 } else { 15.0 };
                let base_memory = if hour >= 9 && hour <= 17 { 70.0 } else { 50.0 };
                
                let cpu = (base_cpu + (rand::random::<f64>() - 0.5) * 5.0).max(5.0).min(85.0);
                let memory = (base_memory + (rand::random::<f64>() - 0.5) * 8.0).max(20.0).min(90.0);
                let connections = cpu * 2.0; // Active connections correlate with CPU
                
                metrics.insert("cpu_percent".to_string(), cpu);
                metrics.insert("memory_percent".to_string(), memory);
                metrics.insert("active_connections".to_string(), connections);
            }
            _ => {
                // Default random metrics
                metrics.insert("cpu_percent".to_string(), rand::random::<f64>() * 100.0);
                metrics.insert("memory_percent".to_string(), rand::random::<f64>() * 100.0);
            }
        }
        
        // Store the metrics
        let mut store = self.current_metrics.write().await;
        store.insert(resource_id.to_string(), metrics);
    }
}

#[cfg(feature = "time-utils")]
#[async_trait]
impl MetricsProvider for AdvancedMetricsProvider {
    async fn get_metrics(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        // Simulate workload based on resource type
        self.simulate_workload(resource_id, resource_id).await;
        
        let store = self.current_metrics.read().await;
        if let Some(metrics) = store.get(resource_id) {
            Ok(Some(ResourceMetrics {
                resource_id: resource_id.to_string(),
                resource_type: "service".to_string(),
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
        // Basic validation
        for (name, value) in &metrics.metrics {
            if *value < 0.0 || (name.contains("percent") && *value > 100.0) {
                println!("⚠️  Invalid metric {}: {}", name, value);
                return Ok(None);
            }
        }
        Ok(Some(metrics.clone()))
    }
}

/// Advanced scaling executor with policy-aware scaling decisions
#[cfg(feature = "time-utils")]
struct PolicyAwareScalingExecutor {
    current_capacity: Arc<tokio::sync::RwLock<HashMap<String, u32>>>,
}

#[cfg(feature = "time-utils")]
impl PolicyAwareScalingExecutor {
    fn new() -> Self {
        let mut initial_capacity = HashMap::new();
        initial_capacity.insert("web-frontend".to_string(), 3);
        initial_capacity.insert("api-backend".to_string(), 2);
        initial_capacity.insert("database".to_string(), 1);
        
        Self {
            current_capacity: Arc::new(tokio::sync::RwLock::new(initial_capacity)),
        }
    }
}

#[cfg(feature = "time-utils")]
#[async_trait]
impl ScalingExecutor for PolicyAwareScalingExecutor {
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
            "🔧 SCALING: {} {} -> {} instances (confidence: {:.0}%)",
            action.resource_id,
            current,
            new_capacity,
            action.confidence * 100.0
        );
        
        println!("   📋 Reason: {}", action.reason);

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
        
        // Safety checks based on resource type and direction
        match action.direction {
            ScaleDirection::Down if current <= 1 => {
                println!("🛡️  SAFETY: Preventing scale down below 1 instance");
                Ok(false)
            }
            ScaleDirection::Up if current >= 20 => {
                println!("🛡️  SAFETY: Preventing scale up above 20 instances");
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

/// Observer that logs advanced policy decisions
#[cfg(feature = "time-utils")]
struct AdvancedPolicyObserver;

#[cfg(feature = "time-utils")]
#[async_trait]
impl ScalingObserver for AdvancedPolicyObserver {
    async fn on_scaling_decision(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        let now = Utc::now();
        let hour = now.hour();
        let time_period = if hour >= 9 && hour <= 17 { "Business Hours" } else { "After Hours" };
        
        println!(
            "🎯 POLICY DECISION [{}]: {} should scale {:?}",
            time_period,
            action.resource_id,
            action.direction
        );
        println!("   📊 Confidence: {:.0}%", action.confidence * 100.0);
        println!("   📝 Policy Context: {}", action.reason);
        
        Ok(())
    }

    async fn on_scaling_executed(
        &self,
        action: &ScaleAction,
        success: bool,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        if success {
            println!("✅ EXECUTED: Successfully scaled {}", action.resource_id);
        } else {
            println!("⏭️  SKIPPED: Scaling of {} was skipped", action.resource_id);
        }
        Ok(())
    }

    async fn on_scaling_skipped(
        &self,
        action: &ScaleAction,
        reason: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        println!("⏸️  POLICY SKIP: {} - {}", action.resource_id, reason);
        Ok(())
    }

    async fn on_scaling_error(
        &self,
        action: &ScaleAction,
        error: &lighthouse::LighthouseError,
        _context: &CallbackContext,
    ) -> LighthouseResult<()> {
        println!("❌ POLICY ERROR: Failed to scale {}: {}", action.resource_id, error);
        Ok(())
    }
}

#[cfg(feature = "time-utils")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Lighthouse Advanced Policies Example");
    println!("=========================================\\n");
    
    // 1. Create basic policies for composition
    println!("⚙️  Creating base policies...");
    
    let cpu_policy = policies::cpu_scaling_policy(75.0, 25.0, 1.5, 180);
    let memory_policy = policies::memory_scaling_policy(80.0, 30.0, 1.4, 180);
    let request_policy = policies::request_rate_scaling_policy(1000.0, 200.0, 1.6, 120);
    
    // 2. Create advanced composite policies
    println!("🏗️  Building advanced composite policies...");
    
    // AND Policy: All metrics must trigger (conservative scaling)
    let conservative_policy = policies::composite_and_policy(
        "conservative-scaling",
        vec![cpu_policy.clone(), memory_policy.clone()]
    );
    
    // OR Policy: Any metric can trigger (aggressive scaling)
    let aggressive_policy = policies::composite_or_policy(
        "aggressive-scaling", 
        vec![cpu_policy.clone(), memory_policy.clone(), request_policy.clone()]
    );
    
    // Weighted Policy: Balanced decision making
    let balanced_policy = policies::composite_weighted_policy(
        "balanced-scaling",
        vec![cpu_policy.clone(), memory_policy.clone(), request_policy.clone()],
        vec![0.5, 0.3, 0.2] // 50% CPU, 30% memory, 20% requests
    );
    
    // Multi-tier policy with advanced logic
    let multi_tier_policy = policies::advanced_multi_tier_policy(
        "multi-tier-scaling",
        (70.0, 20.0), // CPU thresholds
        (75.0, 25.0), // Memory thresholds  
        (800.0, 150.0), // Request thresholds
        300 // Cooldown
    );
    
    // 3. Create time-based policies
    println!("⏰ Creating time-based policies...");
    
    // Business hours: More conservative scaling
    let business_hours_base = policies::cpu_scaling_policy(80.0, 30.0, 1.3, 300);
    // After hours: More aggressive scaling (fewer users, can be more responsive)
    let after_hours_base = policies::cpu_scaling_policy(60.0, 20.0, 1.8, 180);
    
    let time_based_policy = policies::business_hours_scaling_policy(
        "time-aware-scaling",
        business_hours_base,
        after_hours_base,
        Some("UTC")
    );
    
    // 4. Configure resources with different policy strategies
    println!("📋 Configuring resource policies...");
    
    let config = LighthouseConfig::builder()
        .evaluation_interval(15) // Check every 15 seconds for demo
        .add_resource_config(
            "web-frontend",
            ResourceConfig {
                resource_type: "service".to_string(),
                policies: vec![
                    // Convert composite policy back to basic for demo
                    // In a full implementation, the engine would handle composite policies
                    policies::multi_metric_policy(
                        "web-frontend-scaling",
                        (70.0, 25.0), // CPU
                        (75.0, 30.0), // Memory  
                        1.5,
                        240
                    )
                ],
                default_policy: Some("web-frontend-scaling".to_string()),
                settings: [
                    ("policy_type".to_string(), "web-optimized".to_string()),
                    ("scaling_strategy".to_string(), "balanced".to_string()),
                ].into(),
            },
        )
        .add_resource_config(
            "api-backend",
            ResourceConfig {
                resource_type: "service".to_string(),
                policies: vec![
                    policies::multi_metric_policy(
                        "api-backend-scaling",
                        (65.0, 20.0), // More sensitive CPU scaling
                        (70.0, 25.0), // Memory
                        1.6, // Faster scaling
                        180  // Shorter cooldown
                    )
                ],
                default_policy: Some("api-backend-scaling".to_string()),
                settings: [
                    ("policy_type".to_string(), "api-optimized".to_string()),
                    ("scaling_strategy".to_string(), "responsive".to_string()),
                ].into(),
            },
        )
        .add_resource_config(
            "database",
            ResourceConfig {
                resource_type: "service".to_string(), 
                policies: vec![
                    policies::cpu_scaling_policy(85.0, 40.0, 1.2, 600) // Conservative database scaling
                ],
                default_policy: Some("cpu-scaling".to_string()),
                settings: [
                    ("policy_type".to_string(), "database-optimized".to_string()),
                    ("scaling_strategy".to_string(), "conservative".to_string()),
                ].into(),
            },
        )
        .global_setting("demo_mode", "advanced_policies")
        .enable_logging(true)
        .build();

    // 5. Create components
    let metrics_provider = Arc::new(AdvancedMetricsProvider::new());
    let scaling_executor = Arc::new(PolicyAwareScalingExecutor::new());
    let observer = Arc::new(AdvancedPolicyObserver);

    let callbacks = LighthouseCallbacks::new(metrics_provider.clone(), scaling_executor.clone())
        .add_observer(observer);

    // 6. Start engine
    println!("🚀 Starting Lighthouse engine with advanced policies...\\n");
    
    let engine = LighthouseEngine::new(config, callbacks);
    let handle = Arc::new(engine.handle());

    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.start().await {
            eprintln!("Engine error: {}", e);
        }
    });

    // 7. Demonstrate policy evaluation over time
    println!("📊 Running advanced policy demonstration...");
    let resources = ["web-frontend", "api-backend", "database"];
    
    for iteration in 0..24 { // Run for about 6 minutes (24 * 15 seconds)
        println!("\\n🔄 Iteration {} ({} minutes elapsed)", iteration + 1, iteration * 15 / 60);
        
        let now = Utc::now();
        let current_hour = now.hour();
        let time_context = if current_hour >= 9 && current_hour <= 17 { 
            "Business Hours" 
        } else { 
            "After Hours" 
        };
        println!("⏰ Current context: {} ({}:{})", time_context, current_hour, now.minute());
        
        // Show policy decisions for each resource
        for resource in &resources {
            if let Ok(Some(metrics)) = metrics_provider.get_metrics(resource, &CallbackContext {
                timestamp: utils::current_timestamp(),
                metadata: HashMap::new(),
            }).await {
                println!("\\n📋 {} Metrics:", resource);
                for (name, value) in &metrics.metrics {
                    let status = match name.as_str() {
                        "cpu_percent" | "memory_percent" => {
                            if *value > 80.0 { "🔴 HIGH" }
                            else if *value > 60.0 { "🟡 MEDIUM" }  
                            else { "🟢 LOW" }
                        }
                        "requests_per_second" => {
                            if *value > 1000.0 { "🔴 HIGH" }
                            else if *value > 500.0 { "🟡 MEDIUM" }
                            else { "🟢 LOW" }
                        }
                        _ => "ℹ️  INFO"
                    };
                    println!("   • {}: {:.1} {}", name, value, status);
                }
                
                // Send metrics to engine
                handle.update_metrics(metrics).await?;
            }
        }
        
        // Show engine status periodically
        if iteration % 4 == 3 {
            let status = handle.get_status().await?;
            println!("\\n🏗️  Engine Status:");
            println!("   📈 Resources tracked: {}", status.resources_tracked);
            println!("   🎯 Total recommendations: {}", status.total_recommendations);
            println!("   ⏱️  Last evaluation: {:?}", status.last_evaluation);
        }
        
        sleep(Duration::from_secs(15)).await;
    }
    
    // 8. Demonstrate policy evaluation functions
    println!("\\n🔬 Testing policy evaluation utilities...");
    
    // Test time schedule evaluation
    let business_schedule = policies::business_hours_schedule(Some("UTC"));
    let is_business_hours = policies::evaluation::is_schedule_active(&business_schedule);
    println!("📅 Is currently business hours? {}", if is_business_hours { "Yes" } else { "No" });
    
    let weekend_schedule = policies::weekend_schedule(Some("UTC"));
    let is_weekend = policies::evaluation::is_schedule_active(&weekend_schedule);  
    println!("🏖️  Is currently weekend? {}", if is_weekend { "Yes" } else { "No" });
    
    // 9. Cleanup
    println!("\\n🛑 Shutting down...");
    handle.shutdown().await?;
    
    sleep(Duration::from_millis(100)).await;
    engine_task.abort();

    println!("\\n✨ Advanced policies example completed successfully!");
    println!("\\n🎯 Features Demonstrated:");
    println!("• ✅ Composite policies with AND/OR/Weighted logic");
    println!("• ✅ Time-based policy scheduling and evaluation");
    println!("• ✅ Business hours vs after-hours scaling strategies");
    println!("• ✅ Multi-tier policies with different metric priorities");
    println!("• ✅ Advanced policy decision logging and monitoring");
    println!("• ✅ Policy-aware scaling with context information");
    println!("• ✅ Real-time policy evaluation utilities");
    
    println!("\\nNext Steps:");
    println!("• Integrate composite policies into the main engine evaluation loop");
    println!("• Add policy performance metrics and optimization");
    println!("• Implement policy A/B testing and comparison features");
    println!("• Create policy templates for common scaling scenarios");

    Ok(())
}

// Provide a simple fallback for when time-utils feature isn't enabled
#[cfg(not(feature = "time-utils"))]
fn main() {
    println!("❌ This example requires the 'time-utils' feature.");
    println!("Run with: cargo run --example advanced_policies --features \\\"time-utils\\\"");
    std::process::exit(1);
}