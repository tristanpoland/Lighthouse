//! # Predictive Scaling Example
//! 
//! This example demonstrates Lighthouse's intelligent predictive scaling capabilities:
//! 
//! - **Time Series Forecasting**: Predict future metric values using multiple algorithms
//! - **Proactive Scaling**: Scale resources before thresholds are breached
//! - **Trend Analysis**: Detect and extrapolate trends from historical data
//! - **Seasonal Pattern Detection**: Account for recurring daily/weekly patterns
//! - **Anomaly Detection**: Identify unusual patterns that may indicate issues
//! - **Multiple Forecasting Models**: Compare different prediction algorithms
//! 
//! ## Features Demonstrated
//! 
//! 1. Setting up predictive scaling with various forecasting models
//! 2. Generating realistic workload patterns with trends and seasonality
//! 3. Creating forecasts using different algorithms (linear, exponential smoothing, etc.)
//! 4. Getting proactive scaling recommendations with confidence levels
//! 5. Comparing forecasting accuracy across models
//! 6. Handling edge cases and insufficient data scenarios
//! 
//! Run with: cargo run --example predictive_scaling --features "predictive-scaling metrics-persistence time-utils"

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks, ResourceConfig,
    MetricsProvider, ScalingExecutor, ScalingObserver,
    CallbackContext, ScaleAction, ResourceMetrics, LighthouseResult,
    ScalingThreshold, ScalingPolicy, ScaleDirection,
    persistence::{MetricsStore, MetricsStoreConfig, RetentionPolicy},
    predictive::{
        PredictiveScaler, PredictiveConfig, ForecastModel, 
        MetricForecast, ProactiveRecommendation, AnomalyAlert
    },
    utils, policies,
};

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use std::collections::HashMap;
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use std::sync::Arc;
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use tokio::time::{sleep, Duration};
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use async_trait::async_trait;
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
use chrono::{Utc, Duration as ChronoDuration};

/// Sophisticated metrics provider that generates realistic workload patterns
/// with trends, seasonality, and controlled anomalies
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
struct PredictiveMetricsProvider {
    workloads: Arc<tokio::sync::RwLock<HashMap<String, WorkloadSimulator>>>,
    start_time: chrono::DateTime<Utc>,
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
#[derive(Clone)]
struct WorkloadSimulator {
    // Base load characteristics
    base_cpu: f64,
    base_memory: f64,
    
    // Trend parameters (change over time)
    cpu_trend_slope: f64,    // % change per hour
    memory_trend_slope: f64,
    
    // Seasonal parameters
    has_daily_pattern: bool,
    daily_peak_hour: f64,    // Hour of day for peak load (0-24)
    daily_amplitude: f64,    // Size of daily variation
    
    has_weekly_pattern: bool,
    weekly_peak_day: f64,    // Day of week for peak load (0-7)
    weekly_amplitude: f64,   // Size of weekly variation
    
    // Noise and anomaly parameters
    noise_level: f64,        // Random variation
    anomaly_probability: f64, // Chance of anomaly per measurement
    anomaly_magnitude: f64,   // Size of anomalies
    
    // State tracking
    last_anomaly_time: Option<chrono::DateTime<Utc>>,
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
impl PredictiveMetricsProvider {
    fn new() -> Self {
        let mut workloads = HashMap::new();
        let start_time = Utc::now();
        
        // E-commerce frontend: Strong daily patterns, growing trend
        workloads.insert("ecommerce-frontend".to_string(), WorkloadSimulator {
            base_cpu: 35.0,
            base_memory: 55.0,
            cpu_trend_slope: 1.5,    // Growing 1.5% per hour
            memory_trend_slope: 0.8,
            has_daily_pattern: true,
            daily_peak_hour: 14.0,   // 2 PM peak
            daily_amplitude: 20.0,   // ±20% daily variation
            has_weekly_pattern: true,
            weekly_peak_day: 1.0,    // Monday peak
            weekly_amplitude: 10.0,  // ±10% weekly variation
            noise_level: 5.0,        // ±5% random noise
            anomaly_probability: 0.02, // 2% chance per measurement
            anomaly_magnitude: 25.0, // ±25% anomalies
            last_anomaly_time: None,
        });
        
        // API backend: Moderate patterns, stable trend
        workloads.insert("api-backend".to_string(), WorkloadSimulator {
            base_cpu: 45.0,
            base_memory: 60.0,
            cpu_trend_slope: 0.2,    // Slightly growing
            memory_trend_slope: -0.1, // Memory optimizing over time
            has_daily_pattern: true,
            daily_peak_hour: 10.0,   // 10 AM peak (business hours)
            daily_amplitude: 15.0,
            has_weekly_pattern: false,
            weekly_peak_day: 0.0,
            weekly_amplitude: 0.0,
            noise_level: 8.0,
            anomaly_probability: 0.01,
            anomaly_magnitude: 30.0,
            last_anomaly_time: None,
        });
        
        // Database: Steady load with weekly backup patterns
        workloads.insert("database".to_string(), WorkloadSimulator {
            base_cpu: 25.0,
            base_memory: 70.0,
            cpu_trend_slope: 0.1,
            memory_trend_slope: 0.3,  // Memory grows with data
            has_daily_pattern: false,
            daily_peak_hour: 0.0,
            daily_amplitude: 0.0,
            has_weekly_pattern: true,
            weekly_peak_day: 0.0,     // Sunday backup
            weekly_amplitude: 25.0,   // Large weekly backup spike
            noise_level: 3.0,         // Very stable
            anomaly_probability: 0.005, // Rare anomalies
            anomaly_magnitude: 40.0,  // But significant when they occur
            last_anomaly_time: None,
        });
        
        Self {
            workloads: Arc::new(tokio::sync::RwLock::new(workloads)),
            start_time,
        }
    }
    
    /// Generate metrics with realistic patterns
    async fn generate_metrics(&self, resource_id: &str) -> Option<HashMap<String, f64>> {
        let workloads = self.workloads.read().await;
        let mut simulator = workloads.get(resource_id)?.clone();
        
        let now = Utc::now();
        let hours_elapsed = (now - self.start_time).num_minutes() as f64 / 60.0;
        
        // Calculate trend component
        let cpu_trend = simulator.base_cpu + simulator.cpu_trend_slope * hours_elapsed;
        let memory_trend = simulator.base_memory + simulator.memory_trend_slope * hours_elapsed;
        
        // Calculate seasonal components
        let mut cpu_seasonal = 0.0;
        let mut memory_seasonal = 0.0;
        
        if simulator.has_daily_pattern {
            let hour_of_day = (now.timestamp() % 86400) as f64 / 3600.0;
            let daily_phase = (hour_of_day - simulator.daily_peak_hour) * 2.0 * std::f64::consts::PI / 24.0;
            let daily_factor = daily_phase.cos() * simulator.daily_amplitude / 100.0;
            
            cpu_seasonal += cpu_trend * daily_factor;
            memory_seasonal += memory_trend * daily_factor * 0.5; // Memory less seasonal
        }
        
        if simulator.has_weekly_pattern {
            let day_of_week = ((now.timestamp() / 86400) % 7) as f64;
            let weekly_phase = (day_of_week - simulator.weekly_peak_day) * 2.0 * std::f64::consts::PI / 7.0;
            let weekly_factor = weekly_phase.cos() * simulator.weekly_amplitude / 100.0;
            
            cpu_seasonal += cpu_trend * weekly_factor;
            memory_seasonal += memory_trend * weekly_factor * 0.3;
        }
        
        // Add noise
        let cpu_noise = (rand::random::<f64>() - 0.5) * simulator.noise_level;
        let memory_noise = (rand::random::<f64>() - 0.5) * simulator.noise_level;
        
        // Check for anomalies
        let mut cpu_anomaly = 0.0;
        let mut memory_anomaly = 0.0;
        
        if rand::random::<f64>() < simulator.anomaly_probability {
            // Don't generate anomalies too frequently
            if simulator.last_anomaly_time.map_or(true, |last| 
                (now - last).num_minutes() > 30
            ) {
                let anomaly_direction = if rand::random::<bool>() { 1.0 } else { -1.0 };
                cpu_anomaly = anomaly_direction * simulator.anomaly_magnitude;
                memory_anomaly = anomaly_direction * simulator.anomaly_magnitude * 0.7;
                
                simulator.last_anomaly_time = Some(now);
                println!("🚨 Anomaly generated for {}: CPU{:+.1}%, Memory{:+.1}%", 
                    resource_id, cpu_anomaly, memory_anomaly);
            }
        }
        
        // Combine all components
        let cpu = (cpu_trend + cpu_seasonal + cpu_noise + cpu_anomaly).max(0.0).min(100.0);
        let memory = (memory_trend + memory_seasonal + memory_noise + memory_anomaly).max(0.0).min(100.0);
        
        // Generate derived metrics
        let requests_per_sec = (cpu / 100.0) * 1500.0 + (rand::random::<f64>() * 300.0);
        let response_time_ms = if cpu > 80.0 { 
            100.0 + (cpu - 80.0) * 8.0 
        } else { 
            30.0 + cpu * 0.8
        };
        let error_rate = if cpu > 90.0 { 
            (cpu - 90.0) * 0.5 
        } else if cpu > 95.0 {
            (cpu - 95.0) * 2.0  // Exponential increase at very high CPU
        } else { 
            0.05
        };
        
        // Update the simulator state
        drop(workloads);
        let mut workloads_mut = self.workloads.write().await;
        if let Some(sim) = workloads_mut.get_mut(resource_id) {
            *sim = simulator;
        }
        
        Some([
            ("cpu_percent".to_string(), cpu),
            ("memory_percent".to_string(), memory),
            ("requests_per_second".to_string(), requests_per_sec),
            ("response_time_ms".to_string(), response_time_ms),
            ("error_rate_percent".to_string(), error_rate),
        ].into())
    }
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
#[async_trait]
impl MetricsProvider for PredictiveMetricsProvider {
    async fn get_metrics(
        &self,
        resource_id: &str,
        _context: &CallbackContext,
    ) -> LighthouseResult<Option<ResourceMetrics>> {
        if let Some(metrics) = self.generate_metrics(resource_id).await {
            Ok(Some(ResourceMetrics {
                resource_id: resource_id.to_string(),
                resource_type: "service".to_string(),
                timestamp: Utc::now().timestamp() as u64,
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
        // Enhanced validation with anomaly flagging
        for (name, value) in &metrics.metrics {
            if *value < 0.0 || (name.contains("percent") && *value > 100.0) {
                return Ok(None);
            }
            
            // Flag extreme values but don't reject (they might be real anomalies)
            if name == "cpu_percent" && *value > 98.0 {
                println!("🔥 Extreme CPU detected: {:.1}%", value);
            }
            if name == "error_rate_percent" && *value > 10.0 {
                println!("💥 High error rate detected: {:.1}%", value);
            }
        }
        
        Ok(Some(metrics.clone()))
    }
}

/// Predictive-aware scaling executor
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
struct IntelligentScalingExecutor {
    current_capacity: Arc<tokio::sync::RwLock<HashMap<String, u32>>>,
    predictor: Option<Arc<PredictiveScaler>>,
    last_predictions: Arc<tokio::sync::RwLock<HashMap<String, MetricForecast>>>,
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
impl IntelligentScalingExecutor {
    fn new() -> Self {
        let mut initial_capacity = HashMap::new();
        initial_capacity.insert("ecommerce-frontend".to_string(), 4);
        initial_capacity.insert("api-backend".to_string(), 3);
        initial_capacity.insert("database".to_string(), 2);
        
        Self {
            current_capacity: Arc::new(tokio::sync::RwLock::new(initial_capacity)),
            predictor: None,
            last_predictions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    fn set_predictor(&mut self, predictor: Arc<PredictiveScaler>) {
        self.predictor = Some(predictor);
    }
    
    async fn get_prediction_context(&self, resource_id: &str) -> String {
        if let Some(predictor) = &self.predictor {
            // Get recent forecast for context
            match predictor.forecast_metric(
                resource_id,
                "cpu_percent",
                ChronoDuration::minutes(30),
            ).await {
                Ok(forecast) => {
                    // Cache the forecast for display
                    let mut predictions = self.last_predictions.write().await;
                    predictions.insert(resource_id.to_string(), forecast.clone());
                    
                    let next_30_min = forecast.predictions.iter()
                        .take(6) // Next 30 minutes (5-minute intervals)
                        .map(|p| format!("{:.1}", p.value))
                        .collect::<Vec<_>>()
                        .join(", ");
                    
                    format!(
                        "Forecast ({}): [{}] (confidence: {:.0}%)",
                        forecast.model,
                        next_30_min,
                        forecast.confidence * 100.0
                    )
                }
                Err(_) => "No forecast available".to_string()
            }
        } else {
            "Predictor not available".to_string()
        }
    }
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
#[async_trait]
impl ScalingExecutor for IntelligentScalingExecutor {
    async fn execute_scale_action(
        &self,
        action: &ScaleAction,
        _context: &CallbackContext,
    ) -> LighthouseResult<bool> {
        let forecast_context = self.get_prediction_context(&action.resource_id).await;
        
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
            "🔧 INTELLIGENT SCALING: {} {} -> {} instances",
            action.resource_id, current, new_capacity
        );
        println!("   Reason: {}", action.reason);
        println!("   Confidence: {:.0}%", action.confidence * 100.0);
        println!("   {}", forecast_context);

        // Simulate scaling operation
        sleep(Duration::from_millis(200)).await;

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
        
        // Enhanced safety checks using predictions
        if let Some(predictor) = &self.predictor {
            // Check for conflicting predictions
            if let Ok(Some(recommendation)) = predictor.get_proactive_recommendation(
                &action.resource_id,
                &[],  // Would normally pass actual policies
            ).await {
                if recommendation.action.direction != action.direction {
                    println!("🛡️  SAFETY: Conflicting prediction suggests {:?} instead of {:?}", 
                        recommendation.action.direction, action.direction);
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
            ScaleDirection::Up if current >= 25 => {
                println!("🛡️  SAFETY: Preventing scale up above 25 instances");
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

/// Display forecast in a readable format
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
fn display_forecast(forecast: &MetricForecast) {
    println!("🔮 Forecast for {} - {}:", forecast.resource_id, forecast.metric_name);
    println!("   Model: {:?}", forecast.model);
    println!("   Confidence: {:.0}%", forecast.confidence * 100.0);
    println!("   Horizon: {} minutes", forecast.horizon_minutes);
    
    if let Some(ref seasonality) = forecast.seasonality {
        println!("   Seasonality: {:.0} minute period (strength: {:.0}%)", 
            seasonality.period_minutes, seasonality.strength * 100.0);
    }
    
    if !forecast.anomalies.is_empty() {
        println!("   ⚠️  {} anomalies detected", forecast.anomalies.len());
    }
    
    // Show next few predictions
    let sample_size = forecast.predictions.len().min(8);
    println!("   Next {} predictions:", sample_size);
    for (i, pred) in forecast.predictions.iter().take(sample_size).enumerate() {
        let time_from_now = i * 5; // Assuming 5-minute intervals
        println!("     +{}min: {:.1} (±{:.1}, confidence: {:.0}%)", 
            time_from_now,
            pred.value, 
            (pred.upper_bound - pred.lower_bound) / 2.0,
            pred.confidence * 100.0
        );
    }
}

/// Display proactive recommendation
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
fn display_recommendation(rec: &ProactiveRecommendation) {
    println!("🎯 PROACTIVE RECOMMENDATION for {}:", rec.resource_id);
    println!("   Action: Scale {:?} by {:.1}x", 
        rec.action.direction, 
        rec.action.scale_factor.unwrap_or(1.0)
    );
    println!("   Confidence: {:.0}%", rec.confidence * 100.0);
    println!("   Execute at: {}", rec.execute_at.format("%H:%M:%S"));
    println!("   Rationale: {}", rec.rationale);
    println!("   Supporting forecasts: {:?}", rec.supporting_forecasts);
}

/// Compare multiple forecasting models
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
async fn compare_forecasting_models(
    metrics_store: Arc<MetricsStore>,
    resource_id: &str,
    metric_name: &str,
) -> LighthouseResult<()> {
    println!("\n🔬 Comparing Forecasting Models for {} - {}", resource_id, metric_name);
    println!("=" .repeat(60));
    
    let models = [
        ForecastModel::LinearRegression,
        ForecastModel::MovingAverageWithTrend,
        ForecastModel::ExponentialSmoothing,
        ForecastModel::SeasonalLinear,
    ];
    
    let horizon = ChronoDuration::minutes(60);
    
    for model in &models {
        let config = PredictiveConfig::builder()
            .model(*model)
            .forecast_horizon_minutes(60)
            .confidence_threshold(0.6)
            .build();
        
        match PredictiveScaler::new(config, metrics_store.clone()).await {
            Ok(predictor) => {
                match predictor.forecast_metric(resource_id, metric_name, horizon).await {
                    Ok(forecast) => {
                        println!("\n📈 {:?}:", model);
                        println!("   Confidence: {:.0}%", forecast.confidence * 100.0);
                        
                        if !forecast.predictions.is_empty() {
                            let first = &forecast.predictions[0];
                            let last = &forecast.predictions.last().unwrap();
                            let change = last.value - first.value;
                            
                            println!("   Predicted change: {:+.1} over {} minutes", 
                                change, forecast.horizon_minutes);
                            println!("   Range: {:.1} - {:.1}", 
                                forecast.predictions.iter().map(|p| p.value).fold(f64::INFINITY, f64::min),
                                forecast.predictions.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max)
                            );
                        }
                        
                        if !forecast.anomalies.is_empty() {
                            println!("   Anomalies: {} detected", forecast.anomalies.len());
                        }
                    }
                    Err(e) => println!("   Error: {}", e),
                }
            }
            Err(e) => println!("   Failed to create predictor: {}", e),
        }
    }
    
    Ok(())
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔮 Lighthouse Predictive Scaling Example");
    println!("=========================================\n");
    
    // 1. Set up persistence with longer retention for better forecasting
    println!("⚙️  Setting up metrics persistence for forecasting...");
    let store_config = MetricsStoreConfig::builder()
        .database_path("./lighthouse_predictive.db")
        .retention_policy(
            RetentionPolicy::new()
                .raw_data_days(7)      // Keep raw data for a week
                .hourly_aggregates_days(30)
                .daily_aggregates_days(180)
        )
        .enable_aggregation(true)
        .build();

    let metrics_store = Arc::new(MetricsStore::new(store_config).await?);
    
    // 2. Create sophisticated metrics provider with realistic patterns
    let metrics_provider = Arc::new(PredictiveMetricsProvider::new());
    
    // 3. Set up lighthouse configuration optimized for prediction
    let config = LighthouseConfig::builder()
        .evaluation_interval(15) // Check every 15 seconds
        .add_resource_config(
            "service",
            ResourceConfig {
                resource_type: "service".to_string(),
                policies: vec![
                    ScalingPolicy {
                        name: "predictive-cpu-scaling".to_string(),
                        thresholds: vec![
                            ScalingThreshold {
                                metric_name: "cpu_percent".to_string(),
                                scale_up_threshold: 75.0,   // Conservative thresholds
                                scale_down_threshold: 35.0, // for predictive scaling
                                scale_factor: 1.3,
                                cooldown_seconds: 120,      // Longer cooldown
                                confidence: None,
                            },
                            ScalingThreshold {
                                metric_name: "response_time_ms".to_string(),
                                scale_up_threshold: 150.0,
                                scale_down_threshold: 60.0,
                                scale_factor: 1.4,
                                cooldown_seconds: 180,
                                confidence: None,
                            },
                        ],
                        min_capacity: Some(1),
                        max_capacity: Some(25),
                        enabled: true,
                    },
                ],
                default_policy: Some("predictive-cpu-scaling".to_string()),
                settings: HashMap::new(),
            },
        )
        .build();

    // 4. Create intelligent scaling executor
    let mut scaling_executor = IntelligentScalingExecutor::new();

    // 5. Create engine with persistence
    let engine = LighthouseEngine::new_with_persistence(
        config.clone(),
        LighthouseCallbacks::new(metrics_provider, Arc::new(scaling_executor)),
        store_config.clone()
    ).await?;
    
    let handle = Arc::new(engine.handle());
    
    // Start engine
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.start().await {
            eprintln!("Engine error: {}", e);
        }
    });

    println!("✅ Lighthouse engine started with predictive capabilities!\n");

    // 6. Collect initial data to train forecasting models
    println!("📊 Collecting initial data for forecasting models...");
    let resources = ["ecommerce-frontend", "api-backend", "database"];
    
    // Collect data for 2 minutes to build up some history
    for iteration in 0..24 { // 24 iterations * 5 seconds = 2 minutes
        for resource in &resources {
            if let Ok(Some(metrics)) = metrics_provider.get_metrics(resource, &CallbackContext {
                timestamp: utils::current_timestamp(),
                metadata: HashMap::new(),
            }).await {
                handle.update_metrics(metrics).await?;
            }
        }
        
        if iteration % 6 == 5 { // Every 30 seconds
            println!("   Collected {} minutes of data...", (iteration + 1) / 6);
        }
        
        sleep(Duration::from_secs(5)).await;
    }
    
    // 7. Set up predictive scaling with different configurations
    println!("\n🔮 Setting up predictive scaling...");
    
    let predictive_configs = [
        ("Linear Regression", PredictiveConfig::builder()
            .model(ForecastModel::LinearRegression)
            .forecast_horizon_minutes(60)
            .confidence_threshold(0.7)
            .proactive_lead_time_minutes(10)
            .enable_seasonality(true)
            .build()),
        
        ("Exponential Smoothing", PredictiveConfig::builder()
            .model(ForecastModel::ExponentialSmoothing)
            .forecast_horizon_minutes(45)
            .confidence_threshold(0.75)
            .proactive_lead_time_minutes(15)
            .build()),
        
        ("Moving Average with Trend", PredictiveConfig::builder()
            .model(ForecastModel::MovingAverageWithTrend)
            .forecast_horizon_minutes(30)
            .confidence_threshold(0.8)
            .proactive_lead_time_minutes(8)
            .build()),
    ];
    
    // 8. Demonstrate forecasting with different models
    for (name, config) in &predictive_configs {
        println!("\n🎯 Testing {} Model:", name);
        println!("-" .repeat(40));
        
        let predictor = PredictiveScaler::new(config.clone(), metrics_store.clone()).await?;
        
        for resource in ["ecommerce-frontend", "api-backend"].iter() {
            // Generate forecast
            match predictor.forecast_metric(
                resource,
                "cpu_percent",
                ChronoDuration::minutes(config.forecast_horizon_minutes as i64),
            ).await {
                Ok(forecast) => {
                    display_forecast(&forecast);
                    
                    // Check for proactive recommendations
                    let policies = config.default_policies_for_resource(resource);
                    match predictor.get_proactive_recommendation(resource, &policies).await {
                        Ok(Some(recommendation)) => {
                            println!();
                            display_recommendation(&recommendation);
                        }
                        Ok(None) => {
                            println!("   No proactive action recommended");
                        }
                        Err(e) => {
                            println!("   Error getting recommendation: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("   Forecasting failed for {}: {}", resource, e);
                }
            }
            println!();
        }
    }
    
    // 9. Model comparison analysis
    println!("\n🔬 Forecasting Model Analysis:");
    println!("=" .repeat(50));
    
    for resource in &resources {
        compare_forecasting_models(
            metrics_store.clone(),
            resource,
            "cpu_percent"
        ).await?;
        
        sleep(Duration::from_secs(1)).await; // Brief pause between comparisons
    }
    
    // 10. Demonstrate real-time predictive scaling
    println!("\n🚀 Real-time Predictive Scaling Demo:");
    println!("=" .repeat(40));
    
    let main_predictor = Arc::new(PredictiveScaler::new(
        PredictiveConfig::builder()
            .model(ForecastModel::ExponentialSmoothing)
            .forecast_horizon_minutes(45)
            .confidence_threshold(0.7)
            .proactive_lead_time_minutes(12)
            .enable_seasonality(true)
            .enable_anomaly_detection(true)
            .build(),
        metrics_store.clone()
    ).await?);
    
    // Continue collecting metrics and making predictions
    for iteration in 0..20 { // Run for ~2 more minutes
        println!("\n⏰ Iteration {} (Real-time prediction):", iteration + 1);
        
        // Collect new metrics
        for resource in &resources {
            if let Ok(Some(metrics)) = metrics_provider.get_metrics(resource, &CallbackContext {
                timestamp: utils::current_timestamp(),
                metadata: HashMap::new(),
            }).await {
                handle.update_metrics(metrics).await?;
                
                // Get current metric values for context
                if let Some(cpu) = metrics.metrics.get("cpu_percent") {
                    println!("   {} current CPU: {:.1}%", resource, cpu);
                }
            }
        }
        
        // Make predictions and check for proactive recommendations
        for resource in &resources[0..2] { // Focus on first two resources
            match main_predictor.forecast_metric(
                resource,
                "cpu_percent",
                ChronoDuration::minutes(30),
            ).await {
                Ok(forecast) => {
                    if forecast.confidence > 0.6 {
                        let next_prediction = &forecast.predictions[0];
                        println!("   {} forecast (+5min): {:.1}% (confidence: {:.0}%)", 
                            resource, next_prediction.value, next_prediction.confidence * 100.0);
                        
                        // Check for anomalies
                        if !forecast.anomalies.is_empty() {
                            let anomaly = &forecast.anomalies[0];
                            println!("   ⚠️  Anomaly predicted: {:.1}% severity", anomaly.severity * 100.0);
                        }
                    }
                }
                Err(_) => {} // Ignore prediction errors for cleaner output
            }
        }
        
        // Check engine status occasionally
        if iteration % 5 == 4 {
            let status = handle.get_status().await?;
            println!("   📊 Engine: {} resources, {} recommendations made", 
                status.resources_tracked, status.total_recommendations);
        }
        
        sleep(Duration::from_secs(6)).await;
    }
    
    // 11. Final analysis and cleanup
    println!("\n📈 Final Predictive Scaling Analysis:");
    println!("=" .repeat(45));
    
    // Show final forecasts for all resources
    for resource in &resources {
        println!("\nFinal forecast for {}:", resource);
        match main_predictor.forecast_metric(
            resource,
            "cpu_percent",
            ChronoDuration::minutes(60),
        ).await {
            Ok(forecast) => {
                println!("  Model: {:?}", forecast.model);
                println!("  Confidence: {:.0}%", forecast.confidence * 100.0);
                
                if let Some(seasonality) = &forecast.seasonality {
                    println!("  Seasonal pattern: {} min period, {:.0}% strength", 
                        seasonality.period_minutes, seasonality.strength * 100.0);
                }
                
                let trend = if forecast.predictions.len() >= 2 {
                    let first = forecast.predictions.first().unwrap().value;
                    let last = forecast.predictions.last().unwrap().value;
                    last - first
                } else {
                    0.0
                };
                
                println!("  Predicted trend: {:+.1}% over {} minutes", 
                    trend, forecast.horizon_minutes);
                
                if !forecast.anomalies.is_empty() {
                    println!("  Anomalies detected: {}", forecast.anomalies.len());
                }
            }
            Err(e) => println!("  Forecast error: {}", e),
        }
    }
    
    // 12. Cleanup
    println!("\n🛑 Shutting down predictive scaling demo...");
    handle.shutdown().await?;
    
    sleep(Duration::from_millis(100)).await;
    engine_task.abort();

    println!("\n✨ Predictive scaling demo completed successfully!");
    println!("\n🎯 Key Features Demonstrated:");
    println!("• ✅ Multiple forecasting algorithms (Linear, Exponential, Moving Average)");
    println!("• ✅ Proactive scaling recommendations with confidence levels");
    println!("• ✅ Trend analysis and seasonal pattern detection");
    println!("• ✅ Anomaly detection in forecasts");
    println!("• ✅ Model comparison and accuracy analysis");
    println!("• ✅ Real-time predictive scaling with lead time");
    println!("• ✅ Integration with historical metrics storage");
    println!("• ✅ Intelligent scaling decisions based on forecasts");
    
    println!("\n🔮 Predictive Scaling Benefits:");
    println!("• Reduces reactive scaling latency by predicting threshold breaches");
    println!("• Prevents performance degradation through proactive scaling");
    println!("• Accounts for seasonal patterns and trends");
    println!("• Provides confidence levels for scaling decisions");
    println!("• Detects anomalies that may indicate system issues");
    println!("• Optimizes resource utilization through intelligent forecasting");
    
    println!("\nNext Steps:");
    println!("• Tune forecasting parameters for your specific workloads");
    println!("• Integrate with your monitoring and alerting systems");
    println!("• Experiment with different models for different metrics");
    println!("• Set up automated model selection based on accuracy");

    Ok(())
}

// Add helper method to config for demo
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
impl PredictiveConfig {
    fn default_policies_for_resource(&self, _resource: &str) -> Vec<ScalingPolicy> {
        vec![ScalingPolicy {
            name: "demo-policy".to_string(),
            thresholds: vec![ScalingThreshold {
                metric_name: "cpu_percent".to_string(),
                scale_up_threshold: 75.0,
                scale_down_threshold: 35.0,
                scale_factor: 1.3,
                cooldown_seconds: 120,
                confidence: None,
            }],
            min_capacity: Some(1),
            max_capacity: Some(20),
            enabled: true,
        }]
    }
}

// Provide fallback for when features aren't enabled
#[cfg(not(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils")))]
fn main() {
    println!("❌ This example requires 'predictive-scaling', 'metrics-persistence', and 'time-utils' features.");
    println!("Run with: cargo run --example predictive_scaling --features \"predictive-scaling metrics-persistence time-utils\"");
    std::process::exit(1);
}