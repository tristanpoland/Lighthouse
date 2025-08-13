//! # Predictive Scaling Module
//! 
//! This module provides intelligent, forward-looking scaling capabilities by analyzing
//! historical metrics data to predict future resource needs. It transforms Lighthouse
//! from a reactive to a proactive autoscaling system.
//! 
//! ## Features
//! 
//! - **Time Series Forecasting**: Predict future metric values based on historical trends
//! - **Seasonal Pattern Detection**: Identify and account for recurring patterns (daily, weekly)
//! - **Anomaly Prediction**: Detect unusual patterns that may indicate future issues
//! - **Proactive Scaling**: Generate scaling recommendations before thresholds are reached
//! - **Confidence Scoring**: Provide confidence levels for predictions
//! - **Multiple Forecasting Models**: Support various algorithms from simple linear to advanced ML
//! 
//! ## Architecture
//! 
//! The predictive scaling system consists of several components:
//! 
//! 1. **Forecasting Engine**: Core prediction algorithms
//! 2. **Pattern Analyzer**: Seasonal and cyclical pattern detection
//! 3. **Anomaly Detector**: Unusual pattern identification
//! 4. **Prediction Cache**: Cached forecasts to avoid redundant calculations
//! 5. **Model Trainer**: Continuous learning from new data
//! 
//! ## Usage
//! 
//! ```rust,ignore
//! use lighthouse::predictive::{PredictiveScaler, PredictiveConfig, ForecastModel};
//! 
//! // Create predictive scaler
//! let config = PredictiveConfig::builder()
//!     .forecast_horizon_minutes(60)
//!     .model(ForecastModel::LinearRegression)
//!     .confidence_threshold(0.7)
//!     .build();
//! 
//! let predictor = PredictiveScaler::new(config, metrics_store).await?;
//! 
//! // Get predictions
//! let forecast = predictor.forecast_metric(
//!     "web-frontend", 
//!     "cpu_percent", 
//!     Duration::from_minutes(30)
//! ).await?;
//! 
//! // Check if proactive scaling is recommended
//! let recommendation = predictor.get_proactive_recommendation(
//!     "web-frontend",
//!     &current_policies
//! ).await?;
//! ```



#[cfg(feature = "predictive-scaling")]
#[cfg(feature = "time-utils")]
use chrono::{Utc, Duration as ChronoDuration};

#[cfg(feature = "predictive-scaling")]
use chrono::{Utc, Duration as ChronoDuration};

#[cfg(feature = "metrics-persistence")]
use crate::persistence::{MetricsStore, HistoricalDataPoint, MetricsQuery};

use std::sync::Arc;
#[allow(unused_imports)]
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use crate::types::{ResourceId, MetricValue, ScaleAction, ScaleDirection, ScalingPolicy};
use crate::error::{LighthouseError, LighthouseResult};

/// Configuration for predictive scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveConfig {
    /// How far into the future to forecast (in minutes)
    pub forecast_horizon_minutes: u64,
    
    /// Minimum amount of historical data required (in hours)
    pub minimum_history_hours: u64,
    
    /// Forecasting model to use
    pub model: ForecastModel,
    
    /// Minimum confidence level for predictions (0.0 to 1.0)
    pub confidence_threshold: f64,
    
    /// How often to update forecasts (in minutes)
    pub update_interval_minutes: u64,
    
    /// Enable seasonal pattern detection
    pub enable_seasonality: bool,
    
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
    
    /// Proactive scaling lead time (minutes before threshold breach)
    pub proactive_lead_time_minutes: u64,
    
    /// Cache forecasts to avoid redundant calculations
    pub cache_forecasts: bool,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            forecast_horizon_minutes: 60,
            minimum_history_hours: 4,
            model: ForecastModel::LinearRegression,
            confidence_threshold: 0.75,
            update_interval_minutes: 10,
            enable_seasonality: true,
            enable_anomaly_detection: true,
            proactive_lead_time_minutes: 15,
            cache_forecasts: true,
        }
    }
}

impl PredictiveConfig {
    /// Create a new config builder
    pub fn builder() -> PredictiveConfigBuilder {
        PredictiveConfigBuilder::default()
    }
}

/// Builder for PredictiveConfig
#[derive(Default)]
pub struct PredictiveConfigBuilder {
    forecast_horizon_minutes: Option<u64>,
    minimum_history_hours: Option<u64>,
    model: Option<ForecastModel>,
    confidence_threshold: Option<f64>,
    update_interval_minutes: Option<u64>,
    enable_seasonality: Option<bool>,
    enable_anomaly_detection: Option<bool>,
    proactive_lead_time_minutes: Option<u64>,
    cache_forecasts: Option<bool>,
}

impl PredictiveConfigBuilder {
    /// Set forecast horizon in minutes
    pub fn forecast_horizon_minutes(mut self, minutes: u64) -> Self {
        self.forecast_horizon_minutes = Some(minutes);
        self
    }
    
    /// Set minimum history requirement in hours
    pub fn minimum_history_hours(mut self, hours: u64) -> Self {
        self.minimum_history_hours = Some(hours);
        self
    }
    
    /// Set forecasting model
    pub fn model(mut self, model: ForecastModel) -> Self {
        self.model = Some(model);
        self
    }
    
    /// Set confidence threshold
    pub fn confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = Some(threshold.clamp(0.0, 1.0));
        self
    }
    
    /// Set update interval in minutes
    pub fn update_interval_minutes(mut self, minutes: u64) -> Self {
        self.update_interval_minutes = Some(minutes);
        self
    }
    
    /// Enable or disable seasonality detection
    pub fn enable_seasonality(mut self, enable: bool) -> Self {
        self.enable_seasonality = Some(enable);
        self
    }
    
    /// Enable or disable anomaly detection
    pub fn enable_anomaly_detection(mut self, enable: bool) -> Self {
        self.enable_anomaly_detection = Some(enable);
        self
    }
    
    /// Set proactive scaling lead time
    pub fn proactive_lead_time_minutes(mut self, minutes: u64) -> Self {
        self.proactive_lead_time_minutes = Some(minutes);
        self
    }
    
    /// Enable or disable forecast caching
    pub fn cache_forecasts(mut self, enable: bool) -> Self {
        self.cache_forecasts = Some(enable);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> PredictiveConfig {
        let default = PredictiveConfig::default();
        PredictiveConfig {
            forecast_horizon_minutes: self.forecast_horizon_minutes.unwrap_or(default.forecast_horizon_minutes),
            minimum_history_hours: self.minimum_history_hours.unwrap_or(default.minimum_history_hours),
            model: self.model.unwrap_or(default.model),
            confidence_threshold: self.confidence_threshold.unwrap_or(default.confidence_threshold),
            update_interval_minutes: self.update_interval_minutes.unwrap_or(default.update_interval_minutes),
            enable_seasonality: self.enable_seasonality.unwrap_or(default.enable_seasonality),
            enable_anomaly_detection: self.enable_anomaly_detection.unwrap_or(default.enable_anomaly_detection),
            proactive_lead_time_minutes: self.proactive_lead_time_minutes.unwrap_or(default.proactive_lead_time_minutes),
            cache_forecasts: self.cache_forecasts.unwrap_or(default.cache_forecasts),
        }
    }
}

/// Available forecasting models
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ForecastModel {
    /// Simple linear regression
    LinearRegression,
    /// Moving average with trend
    MovingAverageWithTrend,
    /// Exponential smoothing
    ExponentialSmoothing,
    /// Seasonal decomposition with linear trend
    SeasonalLinear,
    /// ARIMA-like model (simplified)
    AutoRegressive,
}

/// A forecast for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricForecast {
    /// Resource and metric this forecast applies to
    pub resource_id: ResourceId,
    pub metric_name: String,
    
    /// When this forecast was generated
    #[cfg(feature = "time-utils")]
    pub generated_at: chrono::DateTime<chrono::Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub generated_at: u64,
    
    /// Forecast horizon
    pub horizon_minutes: u64,
    
    /// Predicted values with timestamps
    pub predictions: Vec<ForecastPoint>,
    
    /// Overall confidence in this forecast
    pub confidence: f64,
    
    /// Model used for forecasting
    pub model: ForecastModel,
    
    /// Detected seasonal patterns
    pub seasonality: Option<SeasonalInfo>,
    
    /// Anomaly alerts
    pub anomalies: Vec<AnomalyAlert>,
}

/// A single prediction point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    /// When this prediction is for
    #[cfg(feature = "time-utils")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub timestamp: u64,
    
    /// Predicted value
    pub value: f64,
    
    /// Confidence interval bounds
    pub lower_bound: f64,
    pub upper_bound: f64,
    
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Information about detected seasonal patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalInfo {
    /// Dominant period detected (in minutes)
    pub period_minutes: u64,
    
    /// Strength of seasonal component (0.0 to 1.0)
    pub strength: f64,
    
    /// Phase offset (where in the cycle we are)
    pub phase_offset: f64,
    
    /// Peak times in the cycle
    pub peak_times: Vec<String>,
}

/// An anomaly detected in the forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    /// When the anomaly is predicted to occur
    #[cfg(feature = "time-utils")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub timestamp: u64,
    
    /// Severity of the anomaly (0.0 to 1.0)
    pub severity: f64,
    
    /// Expected vs predicted value
    pub expected_value: f64,
    pub predicted_value: f64,
    
    /// Description of the anomaly
    pub description: String,
}

/// A proactive scaling recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveRecommendation {
    /// Resource this recommendation is for
    pub resource_id: ResourceId,
    
    /// Recommended scaling action
    pub action: ScaleAction,
    
    /// When to execute the scaling (lead time before threshold breach)
    #[cfg(feature = "time-utils")]
    pub execute_at: chrono::DateTime<chrono::Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub execute_at: u64,
    
    /// Forecasts that led to this recommendation
    pub supporting_forecasts: Vec<String>, // metric names
    
    /// Overall confidence in this recommendation
    pub confidence: f64,
    
    /// Reason for the recommendation
    pub rationale: String,
}

/// Main predictive scaling engine
#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence"))]
pub struct PredictiveScaler {
    config: PredictiveConfig,
    metrics_store: Arc<MetricsStore>,
    forecast_cache: tokio::sync::RwLock<HashMap<String, CachedForecast>>,
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence"))]
struct CachedForecast {
    forecast: MetricForecast,
    #[cfg(feature = "time-utils")]
    cached_at: DateTime<Utc>,
    #[cfg(not(feature = "time-utils"))]
    cached_at: u64,
}

#[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence"))]
impl PredictiveScaler {
    /// Create a new predictive scaler
    pub async fn new(
        config: PredictiveConfig,
        metrics_store: Arc<MetricsStore>,
    ) -> LighthouseResult<Self> {
        Ok(Self {
            config,
            metrics_store,
            forecast_cache: tokio::sync::RwLock::new(HashMap::new()),
        })
    }
    
    /// Generate a forecast for a specific metric
    #[cfg(feature = "predictive-scaling")]
    pub async fn forecast_metric(
        &self,
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        // Check cache first
        let cache_key = format!("{}:{}", resource_id, metric_name);
        if self.config.cache_forecasts {
            let cache = self.forecast_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                #[cfg(feature = "time-utils")]
                {
                    let cache_age = Utc::now() - cached.cached_at;
                    if cache_age.num_minutes() < self.config.update_interval_minutes as i64 {
                        return Ok(cached.forecast.clone());
                    }
                }
                #[cfg(not(feature = "time-utils"))]
                {
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let cache_age_seconds = current_time - cached.cached_at;
                    if cache_age_seconds < (self.config.update_interval_minutes * 60) {
                        return Ok(cached.forecast.clone());
                    }
                }
            }
        }
        
        // Get historical data
        let history_hours = self.config.minimum_history_hours.max(
            (horizon.num_minutes() / 60 * 4) as u64 // At least 4x the forecast horizon
        );
        
        #[cfg(feature = "time-utils")]
        let query = MetricsQuery::last_hours(
            resource_id.to_string(),
            Some(metric_name.to_string()),
            history_hours as i64,
        );
        
        #[cfg(not(feature = "time-utils"))]
        let query = {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let start_time = current_time - (history_hours * 3600);
            MetricsQuery {
                resource_id: resource_id.to_string(),
                metric_name: Some(metric_name.to_string()),
                start_time,
                end_time: current_time,
                aggregation: None,
                limit: None,
            }
        };
        
        let historical_data = self.metrics_store.query_metrics(&query).await?;
        
        if historical_data.len() < 10 {
            return Err(LighthouseError::config("Insufficient historical data for forecasting"));
        }
        
        // Generate forecast based on selected model
        let forecast = match self.config.model {
            ForecastModel::LinearRegression => {
                self.linear_regression_forecast(&historical_data, resource_id, metric_name, horizon).await?
            }
            ForecastModel::MovingAverageWithTrend => {
                self.moving_average_forecast(&historical_data, resource_id, metric_name, horizon).await?
            }
            ForecastModel::ExponentialSmoothing => {
                self.exponential_smoothing_forecast(&historical_data, resource_id, metric_name, horizon).await?
            }
            ForecastModel::SeasonalLinear => {
                self.seasonal_linear_forecast(&historical_data, resource_id, metric_name, horizon).await?
            }
            ForecastModel::AutoRegressive => {
                self.auto_regressive_forecast(&historical_data, resource_id, metric_name, horizon).await?
            }
        };
        
        // Cache the forecast
        if self.config.cache_forecasts {
            let mut cache = self.forecast_cache.write().await;
            cache.insert(cache_key, CachedForecast {
                forecast: forecast.clone(),
                #[cfg(feature = "time-utils")]
                cached_at: Utc::now(),
                #[cfg(not(feature = "time-utils"))]
                cached_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }
        
        Ok(forecast)
    }
    
    /// Get proactive scaling recommendation
    pub async fn get_proactive_recommendation(
        &self,
        resource_id: &str,
        policies: &[ScalingPolicy],
    ) -> LighthouseResult<Option<ProactiveRecommendation>> {
        let mut supporting_forecasts = Vec::new();
        let mut max_confidence = 0.0;
        let mut recommended_action: Option<ScaleAction> = None;
        let mut rationale_parts = Vec::new();

        // Define current_time for both cfg branches
        #[cfg(feature = "time-utils")]
        let current_time = Utc::now();
        #[cfg(not(feature = "time-utils"))]
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Analyze forecasts for each metric used in policies
        for policy in policies {
            if !policy.enabled {
                continue;
            }
            
            for threshold in &policy.thresholds {
                let metric_name = &threshold.metric_name;
                
                // Get forecast for this metric
                let horizon = ChronoDuration::minutes(
                    self.config.forecast_horizon_minutes as i64
                );
                
                let forecast = match self.forecast_metric(resource_id, metric_name, horizon).await {
                    Ok(f) => f,
                    Err(_) => continue, // Skip if forecasting fails
                };
                
                if forecast.confidence < self.config.confidence_threshold {
                    continue; // Skip low-confidence forecasts
                }
                
                // Analyze predictions for threshold breaches
                #[cfg(feature = "time-utils")]
                let (action_window_start, breach_window_end) = {
                    let lead_time = ChronoDuration::minutes(self.config.proactive_lead_time_minutes as i64);
                    let breach_window_end = current_time + horizon;
                    let action_window_start = current_time + lead_time;
                    (action_window_start, breach_window_end)
                };
                #[cfg(not(feature = "time-utils"))]
                let (action_window_start, breach_window_end) = {
                    let lead_time_seconds = self.config.proactive_lead_time_minutes * 60;
                    let horizon_seconds = horizon.num_minutes() as u64 * 60;
                    let action_window_start = current_time + lead_time_seconds;
                    let breach_window_end = current_time + horizon_seconds;
                    (action_window_start, breach_window_end)
                };
                
                for prediction in &forecast.predictions {
                    if prediction.timestamp >= action_window_start && prediction.timestamp <= breach_window_end {
                        // Check for scale-up conditions
                        if prediction.value > threshold.scale_up_threshold {
                            let confidence = prediction.confidence * forecast.confidence;
                            if confidence > max_confidence {
                                max_confidence = confidence;
                                supporting_forecasts.clear();
                                supporting_forecasts.push(metric_name.clone());

                                // Format time string for reason
                                let time_str = {
                                    #[cfg(feature = "time-utils")]
                                    { prediction.timestamp.format("%H:%M").to_string() }
                                    #[cfg(not(feature = "time-utils"))]
                                    { format!("{}:{:02}", (prediction.timestamp / 3600) % 24, (prediction.timestamp / 60) % 60) }
                                };

                                // Calculate minutes until breach
                                let minutes_until_breach = {
                                    #[cfg(feature = "time-utils")]
                                    { (prediction.timestamp - current_time).num_minutes() }
                                    #[cfg(not(feature = "time-utils"))]
                                    { ((prediction.timestamp as i64 - current_time as i64) / 60) }
                                };

                                recommended_action = Some(ScaleAction {
                                    resource_id: resource_id.to_string(),
                                    resource_type: "predicted".to_string(),
                                    direction: ScaleDirection::Up,
                                    target_capacity: None,
                                    scale_factor: Some(threshold.scale_factor),
                                    reason: format!(
                                        "Proactive scaling: {} predicted to reach {:.1} (threshold: {:.1}) at {}",
                                        metric_name,
                                        prediction.value,
                                        threshold.scale_up_threshold,
                                        time_str
                                    ),
                                    confidence,
                                    timestamp: {
                                        #[cfg(feature = "time-utils")]
                                        { current_time.timestamp() as u64 }
                                        #[cfg(not(feature = "time-utils"))]
                                        { current_time }
                                    },
                                    metadata: HashMap::new(),
                                });

                                rationale_parts.clear();
                                rationale_parts.push(format!(
                                    "{} forecast shows breach of {:.1} threshold in {} minutes",
                                    metric_name,
                                    threshold.scale_up_threshold,
                                    minutes_until_breach
                                ));
                            } else if confidence > self.config.confidence_threshold {
                                supporting_forecasts.push(metric_name.clone());
                                rationale_parts.push(format!(
                                    "{} also predicted to breach threshold",
                                    metric_name
                                ));
                            }
                        }

                        // Check for scale-down conditions (less urgent, so higher confidence required)
                        else if prediction.value < threshold.scale_down_threshold {
                            let confidence = prediction.confidence * forecast.confidence;
                            if confidence > self.config.confidence_threshold + 0.1 && confidence > max_confidence {
                                max_confidence = confidence;
                                supporting_forecasts.clear();
                                supporting_forecasts.push(metric_name.clone());

                                recommended_action = Some(ScaleAction {
                                    resource_id: resource_id.to_string(),
                                    resource_type: "predicted".to_string(),
                                    direction: ScaleDirection::Down,
                                    target_capacity: None,
                                    scale_factor: Some(1.0 / threshold.scale_factor),
                                    reason: format!(
                                        "Proactive scale-down: {} predicted to drop to {:.1} (threshold: {:.1})",
                                        metric_name,
                                        prediction.value,
                                        threshold.scale_down_threshold
                                    ),
                                    confidence,
                                    timestamp: {
                                        #[cfg(feature = "time-utils")]
                                        { current_time.timestamp() as u64 }
                                        #[cfg(not(feature = "time-utils"))]
                                        { current_time }
                                    },
                                    metadata: HashMap::new(),
                                });

                                rationale_parts.clear();
                                rationale_parts.push(format!(
                                    "{} forecast shows sustained low utilization below {:.1}",
                                    metric_name,
                                    threshold.scale_down_threshold
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        if let Some(action) = recommended_action {
            Ok(Some(ProactiveRecommendation {
                resource_id: resource_id.to_string(),
                action,
                #[cfg(feature = "time-utils")]
                execute_at: Utc::now() + ChronoDuration::minutes(self.config.proactive_lead_time_minutes as i64),
                #[cfg(not(feature = "time-utils"))]
                execute_at: (Utc::now() + ChronoDuration::minutes(self.config.proactive_lead_time_minutes as i64)).timestamp() as u64,
                supporting_forecasts,
                confidence: max_confidence,
                rationale: rationale_parts.join("; "),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Linear regression forecasting
    async fn linear_regression_forecast(
        &self,
        data: &[HistoricalDataPoint],
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        let values: Vec<f64> = data.iter().map(|dp| dp.value).collect();
        let n = values.len() as f64;
        
        // Simple linear regression
        let x_values: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
        let x_mean = x_values.iter().sum::<f64>() / n;
        let y_mean = values.iter().sum::<f64>() / n;
        
        let numerator: f64 = x_values.iter().zip(&values)
            .map(|(x, y)| (x - x_mean) * (y - y_mean))
            .sum();
        
        let denominator: f64 = x_values.iter()
            .map(|x| (x - x_mean).powi(2))
            .sum();
        
        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        let intercept = y_mean - slope * x_mean;
        
        // Calculate R-squared for confidence
        let predicted: Vec<f64> = x_values.iter()
            .map(|x| intercept + slope * x)
            .collect();
        
        let ss_res: f64 = values.iter().zip(&predicted)
            .map(|(y, pred)| (y - pred).powi(2))
            .sum();
        
        let ss_tot: f64 = values.iter()
            .map(|y| (y - y_mean).powi(2))
            .sum();
        
        let r_squared = if ss_tot != 0.0 { 1.0 - (ss_res / ss_tot) } else { 0.0 };
        
        // Calculate prediction error for confidence intervals
        let mse = ss_res / n;
        let std_error = mse.sqrt();
        
        // Generate predictions
        let mut predictions = Vec::new();
        let forecast_points = (horizon.num_minutes() / 5).max(1) as usize; // One prediction every 5 minutes
        let time_step = horizon.num_minutes() as f64 / forecast_points as f64;
        
        let current_time = Utc::now();
        let start_x = data.len() as f64; // Continue from where data ends
        
        for i in 0..forecast_points {
            let x = start_x + (i as f64 * time_step / 5.0); // Adjust for time scale
            let predicted_value = intercept + slope * x;
            
            // Confidence interval (simplified)
            let confidence = (r_squared * 0.9).max(0.1).min(0.95);
            let z_score = 1.96; // 95% confidence interval
            let margin = z_score * std_error;
            
            predictions.push(ForecastPoint {
                #[cfg(feature = "time-utils")]
                timestamp: current_time + ChronoDuration::minutes((i as f64 * time_step) as i64),
                #[cfg(not(feature = "time-utils"))]
                timestamp: (current_time + ChronoDuration::minutes((i as f64 * time_step) as i64)).timestamp() as u64,
                value: predicted_value.max(0.0), // Don't predict negative values
                lower_bound: (predicted_value - margin).max(0.0),
                upper_bound: predicted_value + margin,
                confidence,
            });
        }
        
        // Detect anomalies (values outside prediction bounds)
        let anomalies = self.detect_anomalies(&predictions, std_error);
        
        Ok(MetricForecast {
            resource_id: resource_id.to_string(),
            metric_name: metric_name.to_string(),
            #[cfg(feature = "time-utils")]
            generated_at: current_time,
            #[cfg(not(feature = "time-utils"))]
            generated_at: current_time.timestamp() as u64,
            horizon_minutes: horizon.num_minutes() as u64,
            predictions,
            confidence: r_squared.max(0.1).min(0.95),
            model: ForecastModel::LinearRegression,
            seasonality: None, // Linear regression doesn't detect seasonality
            anomalies,
        })
    }
    
    /// Moving average with trend forecasting
    async fn moving_average_forecast(
        &self,
        data: &[HistoricalDataPoint],
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        let values: Vec<f64> = data.iter().map(|dp| dp.value).collect();
        let window_size = (data.len() / 4).max(3).min(12); // Adaptive window size
        
        // Calculate moving averages and trends
        let mut moving_averages = Vec::new();
        let mut trends = Vec::new();
        
        for i in window_size..values.len() {
            let window = &values[i-window_size..i];
            let avg = window.iter().sum::<f64>() / window.len() as f64;
            moving_averages.push(avg);
            
            // Calculate trend as difference between current and previous average
            if i > window_size {
                let prev_avg = moving_averages[moving_averages.len() - 2];
                trends.push(avg - prev_avg);
            }
        }
        
        // Get recent trend
        let recent_trend = if trends.is_empty() {
            0.0
        } else {
            // Average of last few trends, weighted toward recent
            let trend_window = trends.len().min(5);
            let recent_trends = &trends[trends.len() - trend_window..];
            recent_trends.iter().enumerate()
                .map(|(i, t)| t * (i + 1) as f64) // Weight recent trends more
                .sum::<f64>() / (1..=trend_window).sum::<usize>() as f64
        };
        
        let last_value = values.last().copied().unwrap_or(0.0);
        
        // Generate predictions
        let mut predictions = Vec::new();
        let forecast_points = (horizon.num_minutes() / 5).max(1) as usize;
        let time_step = horizon.num_minutes() as f64 / forecast_points as f64;
        
        let current_time = Utc::now();
        
        // Calculate confidence based on trend stability
        let trend_variance = if trends.len() > 1 {
            let trend_mean = trends.iter().sum::<f64>() / trends.len() as f64;
            trends.iter().map(|t| (t - trend_mean).powi(2)).sum::<f64>() / trends.len() as f64
        } else {
            0.0
        };
        
        let base_confidence = 1.0 / (1.0 + trend_variance * 0.1);
        
        // Calculate variance once outside the loop
        let variance = if values.len() > 1 {
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
        } else {
            1.0
        };
        let std_dev = variance.sqrt();
        
        for i in 0..forecast_points {
            let time_offset = (i + 1) as f64;
            let predicted_value = last_value + recent_trend * time_offset * 0.1; // Dampen trend over time
            
            // Confidence decreases with time
            let time_decay = (-time_offset * 0.05).exp();
            let confidence = (base_confidence * time_decay).max(0.1).min(0.95);
            
            // Simple confidence interval based on historical variance
            let margin = 1.96 * std_dev * (1.0 - confidence + 0.1);
            
            predictions.push(ForecastPoint {
                #[cfg(feature = "time-utils")]
                timestamp: current_time + ChronoDuration::minutes((i as f64 * time_step) as i64),
                #[cfg(not(feature = "time-utils"))]
                timestamp: (current_time + ChronoDuration::minutes((i as f64 * time_step) as i64)).timestamp() as u64,
                value: predicted_value.max(0.0),
                lower_bound: (predicted_value - margin).max(0.0),
                upper_bound: predicted_value + margin,
                confidence,
            });
        }
        
        let anomalies = self.detect_anomalies(&predictions, std_dev);
        
        Ok(MetricForecast {
            resource_id: resource_id.to_string(),
            metric_name: metric_name.to_string(),
            #[cfg(feature = "time-utils")]
            generated_at: current_time,
            #[cfg(not(feature = "time-utils"))]
            generated_at: current_time.timestamp() as u64,
            horizon_minutes: horizon.num_minutes() as u64,
            predictions,
            confidence: base_confidence.max(0.1).min(0.95),
            model: ForecastModel::MovingAverageWithTrend,
            seasonality: None,
            anomalies,
        })
    }
    
    /// Exponential smoothing forecasting
    async fn exponential_smoothing_forecast(
        &self,
        data: &[HistoricalDataPoint],
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        let values: Vec<f64> = data.iter().map(|dp| dp.value).collect();
        
        // Double exponential smoothing (Holt's method)
        let alpha = 0.3; // Smoothing parameter for level
        let beta = 0.1;  // Smoothing parameter for trend
        
        let mut level = values[0];
        let mut trend = if values.len() > 1 { values[1] - values[0] } else { 0.0 };
        let mut smoothed_values = vec![level];
        
        for &value in &values[1..] {
            let prev_level = level;
            level = alpha * value + (1.0 - alpha) * (level + trend);
            trend = beta * (level - prev_level) + (1.0 - beta) * trend;
            smoothed_values.push(level);
        }
        
        // Calculate forecast accuracy
        let errors: Vec<f64> = values.iter().zip(&smoothed_values)
            .map(|(actual, predicted)| (actual - predicted).abs())
            .collect();
        
        let mae = errors.iter().sum::<f64>() / errors.len() as f64;
        let confidence = (1.0 / (1.0 + mae * 0.01)).max(0.1).min(0.95);
        
        // Generate predictions
        let mut predictions = Vec::new();
        let forecast_points = (horizon.num_minutes() / 5).max(1) as usize;
        let time_step = horizon.num_minutes() as f64 / forecast_points as f64;
        
        let current_time = Utc::now();
        
        for i in 0..forecast_points {
            let steps_ahead = i + 1;
            let predicted_value = level + trend * steps_ahead as f64;
            
            // Confidence decreases with forecast horizon
            let time_decay = (-(steps_ahead as f64) * 0.1).exp();
            let point_confidence = (confidence * time_decay).max(0.1);
            
            // Confidence interval based on historical errors
            let margin = 1.96 * mae * (1.0 + steps_ahead as f64 * 0.1);
            
            predictions.push(ForecastPoint {
                #[cfg(feature = "time-utils")]
                timestamp: current_time + ChronoDuration::minutes((i as f64 * time_step) as i64),
                #[cfg(not(feature = "time-utils"))]
                timestamp: (current_time + ChronoDuration::minutes((i as f64 * time_step) as i64)).timestamp() as u64,
                value: predicted_value.max(0.0),
                lower_bound: (predicted_value - margin).max(0.0),
                upper_bound: predicted_value + margin,
                confidence: point_confidence,
            });
        }
        
        let anomalies = self.detect_anomalies(&predictions, mae);
        
        Ok(MetricForecast {
            resource_id: resource_id.to_string(),
            metric_name: metric_name.to_string(),
            #[cfg(feature = "time-utils")]
            generated_at: current_time,
            #[cfg(not(feature = "time-utils"))]
            generated_at: current_time.timestamp() as u64,
            horizon_minutes: horizon.num_minutes() as u64,
            predictions,
            confidence,
            model: ForecastModel::ExponentialSmoothing,
            seasonality: None,
            anomalies,
        })
    }
    
    /// Seasonal linear forecasting (placeholder - would implement proper seasonal decomposition)
    async fn seasonal_linear_forecast(
        &self,
        data: &[HistoricalDataPoint],
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        // For now, fall back to linear regression with seasonal detection
        let mut forecast = self.linear_regression_forecast(data, resource_id, metric_name, horizon).await?;
        
        // Simple seasonality detection
        if self.config.enable_seasonality {
            forecast.seasonality = self.detect_seasonality(data).await;
        }
        
        forecast.model = ForecastModel::SeasonalLinear;
        Ok(forecast)
    }
    
    /// Auto-regressive forecasting (simplified AR model)
    async fn auto_regressive_forecast(
        &self,
        data: &[HistoricalDataPoint],
        resource_id: &str,
        metric_name: &str,
        horizon: ChronoDuration,
    ) -> LighthouseResult<MetricForecast> {
        // For now, use moving average with trend as a simplified AR model
        let mut forecast = self.moving_average_forecast(data, resource_id, metric_name, horizon).await?;
        forecast.model = ForecastModel::AutoRegressive;
        Ok(forecast)
    }
    
    /// Detect anomalies in predictions
    fn detect_anomalies(&self, predictions: &[ForecastPoint], std_error: f64) -> Vec<AnomalyAlert> {
        if !self.config.enable_anomaly_detection {
            return Vec::new();
        }
        
        let mut anomalies = Vec::new();
        
        for prediction in predictions {
            // Check if prediction is outside reasonable bounds
            let z_score = (prediction.value - prediction.lower_bound).abs() / std_error;
            
            if z_score > 3.0 { // 3-sigma rule
                anomalies.push(AnomalyAlert {
                    timestamp: prediction.timestamp,
                    severity: (z_score / 5.0).min(1.0),
                    expected_value: (prediction.lower_bound + prediction.upper_bound) / 2.0,
                    predicted_value: prediction.value,
                    description: format!("Unusual value predicted: {:.1} (z-score: {:.1})", 
                        prediction.value, z_score),
                });
            }
        }
        
        anomalies
    }
    
    /// Detect seasonal patterns (simplified implementation)
    async fn detect_seasonality(&self, data: &[HistoricalDataPoint]) -> Option<SeasonalInfo> {
        if data.len() < 24 { // Need at least 24 data points
            return None;
        }
        
        // Simple daily pattern detection
        let values: Vec<f64> = data.iter().map(|dp| dp.value).collect();
        
        // Check for daily patterns (assuming data is collected every hour)
        let daily_period = 24.min(data.len() / 2);
        let mut daily_correlation = 0.0;
        let mut count = 0;
        
        for i in daily_period..values.len() {
            let current = values[i];
            let daily_ago = values[i - daily_period];
            daily_correlation += current * daily_ago;
            count += 1;
        }
        
        if count > 0 {
            daily_correlation /= count as f64;
            
            // Normalize correlation
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
            
            if variance > 0.0 {
                let normalized_correlation = (daily_correlation - mean.powi(2)) / variance;
                
                if normalized_correlation > 0.3 { // Threshold for significant correlation
                    return Some(SeasonalInfo {
                        period_minutes: (daily_period * 60) as u64, // Assuming hourly data
                        strength: normalized_correlation.min(1.0),
                        phase_offset: 0.0, // Simplified
                        peak_times: vec!["Morning".to_string(), "Evening".to_string()],
                    });
                }
            }
        }
        
        None
    }
}

// Placeholder implementations for when predictive scaling is not enabled
#[cfg(not(all(feature = "predictive-scaling", feature = "metrics-persistence")))]
pub struct PredictiveScaler;

#[cfg(not(all(feature = "predictive-scaling", feature = "metrics-persistence")))]
impl PredictiveScaler {
    pub async fn new<T>(
        _config: PredictiveConfig,
        _metrics_store: T,
    ) -> LighthouseResult<Self> {
        Err(LighthouseError::config("Predictive scaling requires 'predictive-scaling' and 'metrics-persistence' features"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_predictive_config_builder() {
        let config = PredictiveConfig::builder()
            .forecast_horizon_minutes(120)
            .minimum_history_hours(8)
            .model(ForecastModel::ExponentialSmoothing)
            .confidence_threshold(0.8)
            .enable_seasonality(false)
            .proactive_lead_time_minutes(20)
            .build();
        
        assert_eq!(config.forecast_horizon_minutes, 120);
        assert_eq!(config.minimum_history_hours, 8);
        assert_eq!(config.model, ForecastModel::ExponentialSmoothing);
        assert_eq!(config.confidence_threshold, 0.8);
        assert_eq!(config.enable_seasonality, false);
        assert_eq!(config.proactive_lead_time_minutes, 20);
    }
    
    #[test]
    fn test_forecast_models() {
        let models = [
            ForecastModel::LinearRegression,
            ForecastModel::MovingAverageWithTrend,
            ForecastModel::ExponentialSmoothing,
            ForecastModel::SeasonalLinear,
            ForecastModel::AutoRegressive,
        ];
        
        // Test serialization
        for model in &models {
            let serialized = serde_json::to_string(model).unwrap();
            let deserialized: ForecastModel = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*model, deserialized);
        }
    }
    
    #[cfg(all(feature = "predictive-scaling", feature = "metrics-persistence", feature = "time-utils"))]
    #[tokio::test]
    async fn test_predictive_scaler_creation() {
        use crate::persistence::{MetricsStore, MetricsStoreConfig};
        
        let store_config = MetricsStoreConfig::builder()
            .database_path(":memory:")
            .build();
        
        let store = Arc::new(MetricsStore::new(store_config).await.unwrap());
        
        let config = PredictiveConfig::builder()
            .forecast_horizon_minutes(60)
            .build();
        
        let predictor = PredictiveScaler::new(config, store).await;
        assert!(predictor.is_ok());
    }
}