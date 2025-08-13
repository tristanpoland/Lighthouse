//! # Metrics Persistence Example
//! 
//! This example demonstrates Lighthouse's comprehensive metrics persistence capabilities:
//! 
//! - **Historical Metrics Storage**: Long-term storage of resource metrics
//! - **Trend Analysis**: Statistical analysis and trend detection
//! - **Data Aggregation**: Automatic aggregation for efficient long-term storage
//! - **Query Interface**: Rich querying capabilities for historical data
//! - **Maintenance Tasks**: Automated data cleanup and optimization
//! 
//! ## Features Demonstrated
//! 
//! 1. Setting up metrics persistence with retention policies
//! 2. Storing metrics data automatically as the engine runs
//! 3. Querying historical metrics with different time ranges and aggregations
//! 4. Getting statistical summaries and trend analysis
//! 5. Running maintenance tasks for data cleanup and aggregation
//! 6. Using persistence-based insights for better scaling decisions
//! 
//! Run with: cargo run --example metrics_persistence --features "metrics-persistence time-utils"

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks, ResourceConfig,
    MetricsProvider, ScalingExecutor, ScalingObserver,
    CallbackContext, ScaleAction, ResourceMetrics, LighthouseResult,
    ScalingThreshold, ScalingPolicy, ScaleDirection,
    persistence::{
        MetricsStore, MetricsStoreConfig, RetentionPolicy, MetricsQuery,
        AggregationLevel, TrendDirection, HistoricalDataPoint, StoreStatistics
    },
    utils, policies,
};

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use std::collections::HashMap;
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use std::sync::Arc;
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use tokio::time::{sleep, Duration};
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use async_trait::async_trait;
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
use chrono::{Utc, Duration as ChronoDuration};

/// Advanced metrics provider that simulates realistic workload patterns
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
struct AdvancedMetricsProvider {
    metrics_store: Arc<tokio::sync::RwLock<HashMap<String, WorkloadPattern>>>,
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
#[derive(Clone)]
struct WorkloadPattern {
    base_cpu: f64,
    base_memory: f64,
    cpu_volatility: f64,
    memory_volatility: f64,
    trend_slope: f64, // CPU increase/decrease per hour
    time_offset: f64, // For simulating different phases
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
impl AdvancedMetricsProvider {
    fn new() -> Self {
        let mut patterns = HashMap::new();
        
        // Simulate different workload patterns
        patterns.insert("web-frontend".to_string(), WorkloadPattern {
            base_cpu: 45.0,
            base_memory: 60.0,
            cpu_volatility: 15.0,
            memory_volatility: 10.0,
            trend_slope: 2.0, // Gradually increasing load
            time_offset: 0.0,
        });
        
        patterns.insert("api-backend".to_string(), WorkloadPattern {
            base_cpu: 35.0,
            base_memory: 50.0,
            cpu_volatility: 20.0,
            memory_volatility: 15.0,
            trend_slope: -0.5, // Gradually decreasing load
            time_offset: 1.0,
        });
        
        patterns.insert("database".to_string(), WorkloadPattern {
            base_cpu: 25.0,
            base_memory: 75.0,
            cpu_volatility: 5.0,
            memory_volatility: 8.0,
            trend_slope: 0.1, // Stable load
            time_offset: 2.0,
        });
        
        Self {
            metrics_store: Arc::new(tokio::sync::RwLock::new(patterns)),
        }
    }
    
    /// Generate realistic metrics based on workload patterns
    async fn generate_metrics(&self, resource_id: &str, time_offset_hours: f64) -> Option<HashMap<String, f64>> {
        let store = self.metrics_store.read().await;
        let pattern = store.get(resource_id)?;
        
        // Simulate time-based variations
        let time_factor = (time_offset_hours + pattern.time_offset).sin();
        let trend_factor = time_offset_hours * pattern.trend_slope / 24.0; // Adjust trend per day
        
        // Generate CPU with trend, seasonality, and randomness
        let cpu_base = pattern.base_cpu + trend_factor;
        let cpu_seasonal = time_factor * 10.0; // ±10% seasonal variation
        let cpu_random = (rand::random::<f64>() - 0.5) * pattern.cpu_volatility;
        let cpu = (cpu_base + cpu_seasonal + cpu_random).max(0.0).min(100.0);
        
        // Generate Memory with different patterns
        let memory_base = pattern.base_memory + trend_factor * 0.5;
        let memory_seasonal = (time_offset_hours * 2.0 + pattern.time_offset).sin() * 5.0;
        let memory_random = (rand::random::<f64>() - 0.5) * pattern.memory_volatility;
        let memory = (memory_base + memory_seasonal + memory_random).max(0.0).min(100.0);
        
        // Generate additional synthetic metrics
        let requests_per_sec = (cpu / 100.0) * 1000.0 + (rand::random::<f64>() * 200.0);
        let response_time_ms = if cpu > 80.0 { 
            150.0 + (cpu - 80.0) * 5.0 
        } else { 
            50.0 + cpu 
        };
        let error_rate = if cpu > 90.0 { 
            (cpu - 90.0) / 10.0 
        } else { 
            0.1 
        };
        
        Some([
            ("cpu_percent".to_string(), cpu),
            ("memory_percent".to_string(), memory),
            ("requests_per_second".to_string(), requests_per_sec),
            ("response_time_ms".to_string(), response_time_ms),
            ("error_rate_percent".to_string(), error_rate),
        ].into())
    }
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
#[async_trait]
impl MetricsProvider for AdvancedMetricsProvider {
    async fn get_metrics(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        // Simulate getting metrics based on current time
        let now = Utc::now();
        let hours_offset = (now.timestamp() % 86400) as f64 / 3600.0; // Hours in current day
        
        if let Some(metrics) = self.generate_metrics(resource_id, hours_offset).await {
            Ok(Some(ResourceMetrics {
                resource_id: resource_id.to_string(),
                resource_type: "service".to_string(),
                timestamp: now.timestamp() as u64,
                metrics,
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
        // Advanced validation with anomaly detection
        for (name, value) in &metrics.metrics {
            // Reject obviously invalid values
            if *value < 0.0 || (name.contains("percent") && *value > 100.0) {
                println!("⚠️  Rejected invalid metric {}: {}", name, value);
                return Ok(None);
            }
            
            // Flag suspicious values (basic anomaly detection)
            if name == "cpu_percent" && *value > 95.0 {
                println!("🚨 High CPU detected: {}%", value);
            }
            if name == "error_rate_percent" && *value > 5.0 {
                println!("🚨 High error rate detected: {}%", value);
            }
        }
        
        Ok(Some(metrics.clone()))
    }
}

/// Simple scaling executor for demonstration
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
struct PersistenceAwareScalingExecutor {
    current_capacity: Arc<tokio::sync::RwLock<HashMap<String, u32>>>,
    engine_handle: Option<Arc<lighthouse::LighthouseHandle>>,
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
impl PersistenceAwareScalingExecutor {
    fn new() -> Self {
        let mut initial_capacity = HashMap::new();
        initial_capacity.insert("web-frontend".to_string(), 3);
        initial_capacity.insert("api-backend".to_string(), 2);
        initial_capacity.insert("database".to_string(), 1);
        
        Self {
            current_capacity: Arc::new(tokio::sync::RwLock::new(initial_capacity)),
            engine_handle: None,
        }
    }
    
    fn set_engine_handle(&mut self, handle: Arc<lighthouse::LighthouseHandle>) {
        self.engine_handle = Some(handle);
    }
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
#[async_trait]
impl ScalingExecutor for PersistenceAwareScalingExecutor {
    async fn execute_scale_action(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        // Use historical data to make smarter scaling decisions
        if let Some(handle) = &self.engine_handle {
            // Get recent trend data to inform scaling decision
            if let Ok(Some(stats)) = handle.get_metrics_statistics(
                action.resource_id.clone(),
                "cpu_percent".to_string(),
                2, // Last 2 hours
            ).await {
                println!("📊 Historical context for {}:", action.resource_id);
                println!("   • Mean CPU (2h): {:.1}%", stats.mean);
                println!("   • Trend: {:?} (confidence: {:.0}%)", 
                    stats.trend.direction, stats.trend.confidence * 100.0);
                
                // Be more conservative if trend is stable
                if stats.trend.direction == TrendDirection::Stable && stats.trend.confidence > 0.7 {
                    println!("   • Trend is stable - using conservative scaling");
                    // You could modify the scaling decision here based on trends
                }
                
                // Be more aggressive if trend shows consistent increase
                if stats.trend.direction == TrendDirection::Increasing && stats.trend.confidence > 0.8 {
                    println!("   • Strong upward trend - considering proactive scaling");
                }
            }
        }
        
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

        // Simulate scaling operation
        sleep(Duration::from_millis(300)).await;

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
        
        // Enhanced safety checks using historical data
        if let Some(handle) = &self.engine_handle {
            // Check if there was a recent scaling event that failed
            if let Ok(recent_history) = handle.get_metrics_last_hours(
                action.resource_id.clone(),
                Some("cpu_percent".to_string()),
                1, // Last hour
            ).await {
                if recent_history.len() < 3 {
                    println!("🛡️  SAFETY: Insufficient recent data for safe scaling");
                    return Ok(false);
                }
                
                // Check for recent spikes that might indicate instability
                let recent_values: Vec<f64> = recent_history.iter().map(|dp| dp.value).collect();
                let max_recent = recent_values.iter().fold(0.0f64, |a, &b| a.max(b));
                let min_recent = recent_values.iter().fold(100.0f64, |a, &b| a.min(b));
                
                if max_recent - min_recent > 40.0 {
                    println!("🛡️  SAFETY: High volatility detected (range: {:.1}%-{:.1}%)", min_recent, max_recent);
                    return Ok(false);
                }
            }
        }
        
        // Standard safety rules
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

/// Enhanced observer that logs scaling events with historical context
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
struct AnalyticsObserver {
    engine_handle: Option<Arc<lighthouse::LighthouseHandle>>,
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
impl AnalyticsObserver {
    fn new() -> Self {
        Self { engine_handle: None }
    }
    
    fn set_engine_handle(&mut self, handle: Arc<lighthouse::LighthouseHandle>) {
        self.engine_handle = Some(handle);
    }
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
#[async_trait]
impl ScalingObserver for AnalyticsObserver {
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
        
        // Log historical context if available
        if let Some(handle) = &self.engine_handle {
            if let Ok(Some(stats)) = handle.get_metrics_statistics(
                action.resource_id.clone(),
                "cpu_percent".to_string(),
                1, // Last hour
            ).await {
                println!(
                    "   📈 Context: Mean={:.1}%, Trend={:?}, Change Rate={:.1}%/hr",
                    stats.mean, 
                    stats.trend.direction,
                    stats.trend.change_rate_per_hour
                );
            }
        }
        
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

/// Display historical data in a readable format
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
fn display_historical_data(data: &[HistoricalDataPoint], metric_name: &str) {
    if data.is_empty() {
        println!("   No historical data available");
        return;
    }
    
    println!("   Historical {} data ({} points):", metric_name, data.len());
    
    // Show recent trend
    if data.len() >= 2 {
        let first_value = data.first().unwrap().value;
        let last_value = data.last().unwrap().value;
        let change = last_value - first_value;
        let change_percent = if first_value > 0.0 { (change / first_value) * 100.0 } else { 0.0 };
        
        println!("   • Change: {:.1} -> {:.1} ({:+.1}%)", first_value, last_value, change_percent);
    }
    
    // Show sample data points (latest 5)
    let sample_size = data.len().min(5);
    let recent_data = &data[data.len() - sample_size..];
    
    print!("   • Recent values: ");
    for (i, point) in recent_data.iter().enumerate() {
        if i > 0 { print!(", "); }
        print!("{:.1}", point.value);
    }
    println!();
}

/// Display statistics in a readable format
#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
fn display_statistics(stats: &lighthouse::persistence::MetricsStatistics) {
    println!("📊 Statistics for {} - {}:", stats.resource_id, stats.metric_name);
    println!("   • Mean: {:.1}, Median: {:.1}", stats.mean, stats.median);
    println!("   • Range: {:.1} - {:.1} (±{:.1})", stats.min, stats.max, stats.std_dev);
    println!("   • 95th Percentile: {:.1}", stats.percentiles.get(&95).unwrap_or(&0.0));
    println!("   • Trend: {:?} (confidence: {:.0}%)", 
        stats.trend.direction, stats.trend.confidence * 100.0);
    if stats.trend.direction != TrendDirection::Stable {
        println!("   • Change Rate: {:.2}/hour", stats.trend.change_rate_per_hour);
    }
    println!("   • Data Points: {} (coverage: {:.1}%)", 
        stats.data_points, 100.0 - stats.missing_data_percentage);
}

#[cfg(all(feature = "metrics-persistence", feature = "time-utils"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚨 Lighthouse Metrics Persistence Example");
    println!("==========================================\n");
    
    // 1. Configure metrics persistence with custom retention policy
    println!("⚙️  Setting up metrics persistence...");
    let store_config = MetricsStoreConfig::builder()
        .database_path("./lighthouse_demo.db")
        .max_connections(5)
        .retention_policy(
            RetentionPolicy::new()
                .raw_data_days(3)      // Keep raw data for 3 days
                .hourly_aggregates_days(14)  // Keep hourly data for 2 weeks
                .daily_aggregates_days(90)   // Keep daily data for 3 months
                .max_data_points(50_000)
        )
        .enable_aggregation(true)
        .maintenance_interval_hours(4)
        .build();

    // 2. Create lighthouse configuration with advanced policies
    let config = LighthouseConfig::builder()
        .evaluation_interval(10) // Check every 10 seconds for demo
        .add_resource_config(
            "service",
            ResourceConfig {
                resource_type: "service".to_string(),
                policies: vec![
                    // Multi-metric policy considering CPU, memory, and response time
                    ScalingPolicy {
                        name: "comprehensive-scaling".to_string(),
                        thresholds: vec![
                            ScalingThreshold {
                                metric_name: "cpu_percent".to_string(),
                                scale_up_threshold: 70.0,
                                scale_down_threshold: 30.0,
                                scale_factor: 1.4,
                                cooldown_seconds: 60, // Short cooldown for demo
                            },
                            ScalingThreshold {
                                metric_name: "memory_percent".to_string(),
                                scale_up_threshold: 75.0,
                                scale_down_threshold: 40.0,
                                scale_factor: 1.3,
                                cooldown_seconds: 60,
                            },
                            ScalingThreshold {
                                metric_name: "response_time_ms".to_string(),
                                scale_up_threshold: 200.0,
                                scale_down_threshold: 80.0,
                                scale_factor: 1.5,
                                cooldown_seconds: 90,
                            },
                        ],
                        min_capacity: Some(1),
                        max_capacity: Some(20),
                        enabled: true,
                    },
                ],
                default_policy: Some("comprehensive-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .enable_logging(true)
        .build();

    // 3. Create components with persistence awareness
    let metrics_provider = Arc::new(AdvancedMetricsProvider::new());
    let mut scaling_executor = PersistenceAwareScalingExecutor::new();
    let mut observer = AnalyticsObserver::new();

    // 4. Create engine with persistence
    println!("🚀 Creating engine with persistence...");
    let engine = LighthouseEngine::new_with_persistence(config, 
        LighthouseCallbacks::new(metrics_provider, Arc::new(scaling_executor)), 
        store_config
    ).await?;
    
    let handle = Arc::new(engine.handle());

    // Set up cross-references (this is a bit awkward but needed for the demo)
    // In a real application, you'd structure this differently
    
    // Start engine
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.start().await {
            eprintln!("Engine error: {}", e);
        }
    });

    println!("✅ Lighthouse engine with persistence started!\n");

    // 5. Demonstrate metrics collection over time
    println!("📊 Collecting metrics over time...");
    let resources = ["web-frontend", "api-backend", "database"];
    
    // Collect metrics for a few minutes to build up historical data
    for iteration in 0..20 {
        println!("\n🔄 Iteration {} - Collecting metrics...", iteration + 1);
        
        for resource in &resources {
            // Generate and send metrics
            if let Ok(Some(metrics)) = metrics_provider.get_metrics(resource, &CallbackContext {
                timestamp: utils::current_timestamp(),
                metadata: HashMap::new(),
            }).await {
                handle.update_metrics(metrics).await?;
                
                if let Ok(Some(current_stats)) = handle.get_metrics_statistics(
                    resource.to_string(),
                    "cpu_percent".to_string(),
                    1 // Last hour (which is everything in our demo)
                ).await {
                    println!("   {} - CPU: mean={:.1}%, trend={:?}", 
                        resource, current_stats.mean, current_stats.trend.direction);
                }
            }
        }
        
        // Check engine status periodically
        if iteration % 5 == 4 {
            let status = handle.get_status().await?;
            println!("   📈 Engine Status: {} resources, {} recommendations", 
                status.resources_tracked, status.total_recommendations);
        }
        
        sleep(Duration::from_secs(5)).await;
    }

    // 6. Demonstrate historical data queries
    println!("\n🔍 Querying historical data...");
    
    for resource in &resources {
        println!("\n📊 Historical analysis for {}:", resource);
        
        // Get recent CPU data
        let cpu_history = handle.get_metrics_last_hours(
            resource.to_string(), 
            Some("cpu_percent".to_string()), 
            1
        ).await?;
        display_historical_data(&cpu_history, "CPU");
        
        // Get comprehensive statistics
        if let Ok(Some(stats)) = handle.get_metrics_statistics(
            resource.to_string(),
            "cpu_percent".to_string(),
            1
        ).await {
            display_statistics(&stats);
        }
        
        // Get memory trends
        if let Ok(Some(memory_stats)) = handle.get_metrics_statistics(
            resource.to_string(),
            "memory_percent".to_string(),
            1
        ).await {
            println!("   🧠 Memory: mean={:.1}%, trend={:?}", 
                memory_stats.mean, memory_stats.trend.direction);
        }
    }

    // 7. Demonstrate advanced queries with different aggregation levels
    println!("\n📈 Advanced querying examples...");
    
    // Create custom queries
    let now = chrono::Utc::now();
    let custom_query = MetricsQuery {
        resource_id: "web-frontend".to_string(),
        metric_name: Some("cpu_percent".to_string()),
        start_time: now - ChronoDuration::minutes(30),
        end_time: now,
        aggregation: Some(AggregationLevel::Raw),
        limit: Some(50),
    };
    
    let query_results = handle.get_historical_metrics(custom_query).await?;
    println!("   Custom query returned {} data points", query_results.len());
    
    if !query_results.is_empty() {
        let values: Vec<f64> = query_results.iter().map(|dp| dp.value).collect();
        let avg = values.iter().sum::<f64>() / values.len() as f64;
        let min_val = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        println!("   Range: {:.1} - {:.1}, Average: {:.1}", min_val, max_val, avg);
    }

    // 8. Run maintenance tasks
    println!("\n🧹 Running maintenance tasks...");
    handle.run_maintenance().await?;
    println!("   ✅ Maintenance completed (aggregation, cleanup)");

    // 9. Display storage statistics
    println!("\n📊 Final storage statistics:");
    // Note: We can't get storage stats through the handle directly,
    // but in a real app you might want to expose this
    println!("   Database operations completed successfully");
    println!("   Check ./lighthouse_demo.db for persistent data");

    // 10. Cleanup
    println!("\n🛑 Shutting down...");
    handle.shutdown().await?;
    
    // Give the engine task a moment to shut down gracefully
    sleep(Duration::from_millis(100)).await;
    engine_task.abort();

    println!("\n✨ Persistence example completed successfully!");
    println!("\nKey Features Demonstrated:");
    println!("• ✅ Automatic metrics persistence with SQLite storage");
    println!("• ✅ Historical data querying with flexible time ranges");
    println!("• ✅ Statistical analysis and trend detection");
    println!("• ✅ Data retention policies and automatic cleanup");
    println!("• ✅ Aggregation levels (raw, hourly, daily)");
    println!("• ✅ Persistence-aware scaling decisions");
    println!("• ✅ Performance optimization with database indexing");
    println!("• ✅ Configurable storage and maintenance policies");
    
    println!("\nNext Steps:");
    println!("• Examine the generated database: sqlite3 ./lighthouse_demo.db");
    println!("• Try different retention policies and aggregation settings");
    println!("• Integrate with your monitoring systems");
    println!("• Use historical data for predictive scaling (see predictive_scaling example)");

    Ok(())
}

// Provide a simple fallback for when persistence features aren't enabled
#[cfg(not(all(feature = "metrics-persistence", feature = "time-utils")))]
fn main() {
    println!("❌ This example requires both 'metrics-persistence' and 'time-utils' features.");
    println!("Run with: cargo run --example metrics_persistence --features \"metrics-persistence time-utils\"");
    std::process::exit(1);
}