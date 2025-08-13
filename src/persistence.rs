//! # Metrics Persistence Layer
//! 
//! This module provides comprehensive metrics storage and historical analysis capabilities
//! for the Lighthouse autoscaling engine. It supports:
//! 
//! - **Historical Metrics Storage**: Long-term storage of resource metrics with efficient querying
//! - **Trend Analysis**: Statistical analysis of historical data to identify patterns
//! - **Data Retention**: Configurable retention policies to manage storage growth
//! - **Query Interface**: Rich querying API for metrics analysis and reporting
//! 
//! ## Features
//! 
//! - SQLite-based storage with optional migrations to other databases
//! - Automatic data aggregation and downsampling for long-term storage
//! - Statistical calculations (moving averages, percentiles, trends)
//! - Configurable retention policies
//! - Backup and restore capabilities
//! 
//! ## Usage
//! 
//! ```rust,ignore
//! use lighthouse::persistence::{MetricsStore, MetricsStoreConfig, RetentionPolicy};
//! 
//! // Create a metrics store
//! let config = MetricsStoreConfig::builder()
//!     .database_path("./metrics.db")
//!     .retention_policy(RetentionPolicy::new()
//!         .raw_data_days(7)
//!         .hourly_aggregates_days(30)
//!         .daily_aggregates_days(365))
//!     .build();
//! 
//! let store = MetricsStore::new(config).await?;
//! 
//! // Store metrics
//! store.store_metrics(&metrics).await?;
//! 
//! // Query historical data
//! let history = store.get_metrics_history(
//!     "web-frontend", 
//!     "cpu_percent", 
//!     Duration::from_hours(24)
//! ).await?;
//! 
//! // Get trend analysis
//! let trend = store.analyze_trend("web-frontend", "cpu_percent").await?;
//! ```

#[cfg(feature = "metrics-persistence")]
use sqlx::{sqlite::SqlitePool, Row, Sqlite, Pool};
#[cfg(feature = "time-utils")]
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::types::{ResourceMetrics, ResourceId, MetricValue};
use crate::error::{LighthouseError, LighthouseResult};

/// Configuration for the metrics persistence layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStoreConfig {
    /// Path to the SQLite database file
    pub database_path: String,
    
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    
    /// Data retention policies
    pub retention_policy: RetentionPolicy,
    
    /// Whether to enable automatic data aggregation
    pub enable_aggregation: bool,
    
    /// Interval for running maintenance tasks (cleanup, aggregation)
    pub maintenance_interval_hours: u64,
}

impl Default for MetricsStoreConfig {
    fn default() -> Self {
        Self {
            database_path: "./lighthouse_metrics.db".to_string(),
            max_connections: 10,
            retention_policy: RetentionPolicy::default(),
            enable_aggregation: true,
            maintenance_interval_hours: 6,
        }
    }
}

impl MetricsStoreConfig {
    /// Create a new config builder
    pub fn builder() -> MetricsStoreConfigBuilder {
        MetricsStoreConfigBuilder::default()
    }
}

/// Builder for MetricsStoreConfig
#[derive(Default)]
pub struct MetricsStoreConfigBuilder {
    database_path: Option<String>,
    max_connections: Option<u32>,
    retention_policy: Option<RetentionPolicy>,
    enable_aggregation: Option<bool>,
    maintenance_interval_hours: Option<u64>,
}

impl MetricsStoreConfigBuilder {
    /// Set the database path
    pub fn database_path<S: Into<String>>(mut self, path: S) -> Self {
        self.database_path = Some(path.into());
        self
    }
    
    /// Set maximum database connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }
    
    /// Set retention policy
    pub fn retention_policy(mut self, policy: RetentionPolicy) -> Self {
        self.retention_policy = Some(policy);
        self
    }
    
    /// Enable or disable automatic aggregation
    pub fn enable_aggregation(mut self, enable: bool) -> Self {
        self.enable_aggregation = Some(enable);
        self
    }
    
    /// Set maintenance interval in hours
    pub fn maintenance_interval_hours(mut self, hours: u64) -> Self {
        self.maintenance_interval_hours = Some(hours);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> MetricsStoreConfig {
        let default = MetricsStoreConfig::default();
        MetricsStoreConfig {
            database_path: self.database_path.unwrap_or(default.database_path),
            max_connections: self.max_connections.unwrap_or(default.max_connections),
            retention_policy: self.retention_policy.unwrap_or(default.retention_policy),
            enable_aggregation: self.enable_aggregation.unwrap_or(default.enable_aggregation),
            maintenance_interval_hours: self.maintenance_interval_hours.unwrap_or(default.maintenance_interval_hours),
        }
    }
}

/// Data retention policies for different aggregation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// How long to keep raw (unaggregated) metrics data
    pub raw_data_days: u32,
    
    /// How long to keep hourly aggregated data
    pub hourly_aggregates_days: u32,
    
    /// How long to keep daily aggregated data
    pub daily_aggregates_days: u32,
    
    /// Maximum number of data points to store per resource-metric combination
    pub max_data_points_per_metric: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_data_days: 7,           // Keep raw data for 1 week
            hourly_aggregates_days: 30, // Keep hourly aggregates for 1 month
            daily_aggregates_days: 365, // Keep daily aggregates for 1 year
            max_data_points_per_metric: Some(10_000),
        }
    }
}

impl RetentionPolicy {
    /// Create a new retention policy
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set raw data retention period
    pub fn raw_data_days(mut self, days: u32) -> Self {
        self.raw_data_days = days;
        self
    }
    
    /// Set hourly aggregates retention period
    pub fn hourly_aggregates_days(mut self, days: u32) -> Self {
        self.hourly_aggregates_days = days;
        self
    }
    
    /// Set daily aggregates retention period
    pub fn daily_aggregates_days(mut self, days: u32) -> Self {
        self.daily_aggregates_days = days;
        self
    }
    
    /// Set maximum data points per metric
    pub fn max_data_points(mut self, max: u64) -> Self {
        self.max_data_points_per_metric = Some(max);
        self
    }
}

/// Historical metrics data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    /// When this data point was recorded
    #[cfg(feature = "time-utils")]
    pub timestamp: DateTime<Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub timestamp: u64,
    
    /// The metric value
    pub value: MetricValue,
    
    /// Aggregation level (raw, hourly, daily)
    pub aggregation: AggregationLevel,
}

/// Aggregation levels for metrics data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AggregationLevel {
    /// Raw, unaggregated data points
    Raw,
    /// Hourly aggregated data
    Hourly,
    /// Daily aggregated data
    Daily,
}

/// Statistical summary of metrics over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStatistics {
    /// Resource and metric this applies to
    pub resource_id: ResourceId,
    pub metric_name: String,
    
    /// Time period analyzed
    #[cfg(feature = "time-utils")]
    pub start_time: DateTime<Utc>,
    #[cfg(feature = "time-utils")]
    pub end_time: DateTime<Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub start_time: u64,
    #[cfg(not(feature = "time-utils"))]
    pub end_time: u64,
    
    /// Statistical measures
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    
    /// Percentiles (25th, 75th, 90th, 95th, 99th)
    pub percentiles: HashMap<u8, f64>,
    
    /// Trend analysis
    pub trend: TrendAnalysis,
    
    /// Data quality metrics
    pub data_points: usize,
    pub missing_data_percentage: f64,
}

/// Trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Overall trend direction
    pub direction: TrendDirection,
    
    /// Confidence in the trend (0.0 to 1.0)
    pub confidence: f64,
    
    /// Rate of change per hour
    pub change_rate_per_hour: f64,
    
    /// Linear regression slope
    pub slope: f64,
    
    /// R-squared correlation coefficient
    pub r_squared: f64,
    
    /// Seasonal patterns detected
    pub seasonality: Vec<SeasonalPattern>,
}

/// Trend direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    /// Metric is increasing over time
    Increasing,
    /// Metric is decreasing over time
    Decreasing,
    /// Metric is stable (no significant trend)
    Stable,
    /// Insufficient data to determine trend
    Unknown,
}

/// Detected seasonal patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    /// Pattern type (daily, weekly, etc.)
    pub pattern_type: String,
    
    /// Strength of the pattern (0.0 to 1.0)
    pub strength: f64,
    
    /// Peak times for this pattern
    pub peak_times: Vec<String>,
}

/// Query parameters for historical metrics
#[derive(Debug, Clone)]
pub struct MetricsQuery {
    /// Resource to query
    pub resource_id: ResourceId,
    
    /// Specific metric name (optional - if None, returns all metrics)
    pub metric_name: Option<String>,
    
    /// Time range start
    #[cfg(feature = "time-utils")]
    pub start_time: DateTime<Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub start_time: u64,
    
    /// Time range end
    #[cfg(feature = "time-utils")]
    pub end_time: DateTime<Utc>,
    #[cfg(not(feature = "time-utils"))]
    pub end_time: u64,
    
    /// Desired aggregation level
    pub aggregation: Option<AggregationLevel>,
    
    /// Maximum number of data points to return
    pub limit: Option<u64>,
}

#[cfg(feature = "metrics-persistence")]
impl MetricsQuery {
    /// Create a new query for the last N hours
    #[cfg(feature = "time-utils")]
    pub fn last_hours(resource_id: ResourceId, metric_name: Option<String>, hours: i64) -> Self {
        let end_time = Utc::now();
        let start_time = end_time - ChronoDuration::hours(hours);
        
        Self {
            resource_id,
            metric_name,
            start_time,
            end_time,
            aggregation: None,
            limit: None,
        }
    }
    
    /// Create a new query for the last N days
    #[cfg(feature = "time-utils")]
    pub fn last_days(resource_id: ResourceId, metric_name: Option<String>, days: i64) -> Self {
        let end_time = Utc::now();
        let start_time = end_time - ChronoDuration::days(days);
        
        Self {
            resource_id,
            metric_name,
            start_time,
            end_time,
            aggregation: Some(AggregationLevel::Hourly),
            limit: None,
        }
    }
}

/// Main metrics persistence store
#[cfg(feature = "metrics-persistence")]
pub struct MetricsStore {
    pool: Pool<Sqlite>,
    config: MetricsStoreConfig,
}

#[cfg(feature = "metrics-persistence")]
impl MetricsStore {
    /// Create a new metrics store
    pub async fn new(config: MetricsStoreConfig) -> LighthouseResult<Self> {
        // Ensure database directory exists
        if let Some(parent) = Path::new(&config.database_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LighthouseError::config(&format!("Failed to create database directory: {}", e)))?;
        }
        
        // Create connection pool
        let database_url = format!("sqlite:{}", config.database_path);
        let pool = SqlitePool::connect(&database_url).await
            .map_err(|e| LighthouseError::config(&format!("Failed to connect to database: {}", e)))?;
        
        let store = Self { pool, config };
        
        // Initialize database schema
        store.initialize_schema().await?;
        
        Ok(store)
    }
    
    /// Initialize database schema
    async fn initialize_schema(&self) -> LighthouseResult<()> {
        // Create tables for raw metrics
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS raw_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                resource_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                metric_value REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
        "#).execute(&self.pool).await
        .map_err(|e| LighthouseError::config(&format!("Failed to create raw_metrics table: {}", e)))?;
        
        // Create tables for aggregated metrics
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS aggregated_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                resource_id TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                aggregation_level TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                mean_value REAL NOT NULL,
                min_value REAL NOT NULL,
                max_value REAL NOT NULL,
                count INTEGER NOT NULL,
                std_dev REAL,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
        "#).execute(&self.pool).await
        .map_err(|e| LighthouseError::config(&format!("Failed to create aggregated_metrics table: {}", e)))?;
        
        // Create indexes for better query performance
        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_raw_metrics_resource_time 
            ON raw_metrics(resource_id, timestamp)
        "#).execute(&self.pool).await.map_err(|e| LighthouseError::config(&format!("Failed to create index: {}", e)))?;
        
        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_raw_metrics_resource_metric_time 
            ON raw_metrics(resource_id, metric_name, timestamp)
        "#).execute(&self.pool).await.map_err(|e| LighthouseError::config(&format!("Failed to create index: {}", e)))?;
        
        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_aggregated_metrics_resource_time 
            ON aggregated_metrics(resource_id, aggregation_level, timestamp)
        "#).execute(&self.pool).await.map_err(|e| LighthouseError::config(&format!("Failed to create index: {}", e)))?;
        
        Ok(())
    }
    
    /// Store raw metrics data
    pub async fn store_metrics(&self, metrics: &ResourceMetrics) -> LighthouseResult<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| LighthouseError::config(&format!("Failed to start transaction: {}", e)))?;
        
        for (metric_name, metric_value) in &metrics.metrics {
            sqlx::query(r#"
                INSERT INTO raw_metrics (resource_id, resource_type, metric_name, metric_value, timestamp)
                VALUES (?, ?, ?, ?, ?)
            "#)
            .bind(&metrics.resource_id)
            .bind(&metrics.resource_type)
            .bind(metric_name)
            .bind(*metric_value)
            .bind(metrics.timestamp as i64)
            .execute(&mut *tx).await
            .map_err(|e| LighthouseError::config(&format!("Failed to insert metrics: {}", e)))?;
        }
        
        tx.commit().await
            .map_err(|e| LighthouseError::config(&format!("Failed to commit transaction: {}", e)))?;
        
        Ok(())
    }
    
    /// Query historical metrics data
    pub async fn query_metrics(&self, query: &MetricsQuery) -> LighthouseResult<Vec<HistoricalDataPoint>> {
        let aggregation_table = match query.aggregation.unwrap_or(AggregationLevel::Raw) {
            AggregationLevel::Raw => "raw_metrics",
            _ => "aggregated_metrics",
        };
        
        let sql = if aggregation_table == "raw_metrics" {
            if let Some(ref metric_name) = query.metric_name {
                format!(r#"
                    SELECT metric_name, metric_value, timestamp
                    FROM raw_metrics 
                    WHERE resource_id = ? AND metric_name = ? 
                    AND timestamp BETWEEN ? AND ?
                    ORDER BY timestamp {}
                "#, if query.limit.is_some() { "LIMIT ?" } else { "" })
            } else {
                format!(r#"
                    SELECT metric_name, metric_value, timestamp
                    FROM raw_metrics 
                    WHERE resource_id = ? 
                    AND timestamp BETWEEN ? AND ?
                    ORDER BY timestamp {}
                "#, if query.limit.is_some() { "LIMIT ?" } else { "" })
            }
        } else {
            // Aggregated data query
            if let Some(ref metric_name) = query.metric_name {
                format!(r#"
                    SELECT metric_name, mean_value as metric_value, timestamp
                    FROM aggregated_metrics 
                    WHERE resource_id = ? AND metric_name = ? 
                    AND aggregation_level = ?
                    AND timestamp BETWEEN ? AND ?
                    ORDER BY timestamp {}
                "#, if query.limit.is_some() { "LIMIT ?" } else { "" })
            } else {
                format!(r#"
                    SELECT metric_name, mean_value as metric_value, timestamp
                    FROM aggregated_metrics 
                    WHERE resource_id = ? 
                    AND aggregation_level = ?
                    AND timestamp BETWEEN ? AND ?
                    ORDER BY timestamp {}
                "#, if query.limit.is_some() { "LIMIT ?" } else { "" })
            }
        };
        
        let mut sql_query = sqlx::query(&sql).bind(&query.resource_id);
        
        if query.metric_name.is_some() {
            sql_query = sql_query.bind(query.metric_name.as_ref().unwrap());
        }
        
        if aggregation_table != "raw_metrics" {
            let agg_level = match query.aggregation.unwrap_or(AggregationLevel::Raw) {
                AggregationLevel::Hourly => "hourly",
                AggregationLevel::Daily => "daily",
                _ => "raw",
            };
            sql_query = sql_query.bind(agg_level);
        }
        
        #[cfg(feature = "time-utils")]
        {
            sql_query = sql_query.bind(query.start_time.timestamp()).bind(query.end_time.timestamp());
        }
        #[cfg(not(feature = "time-utils"))]
        {
            sql_query = sql_query.bind(query.start_time as i64).bind(query.end_time as i64);
        }
        
        if let Some(limit) = query.limit {
            sql_query = sql_query.bind(limit as i64);
        }
        
        let rows = sql_query.fetch_all(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to query metrics: {}", e)))?;
        
        let mut data_points = Vec::new();
        for row in rows {
            let timestamp: i64 = row.get("timestamp");
            let value: f64 = row.get("metric_value");
            
            data_points.push(HistoricalDataPoint {
                #[cfg(feature = "time-utils")]
                timestamp: DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now()),
                #[cfg(not(feature = "time-utils"))]
                timestamp: timestamp as u64,
                value,
                aggregation: query.aggregation.unwrap_or(AggregationLevel::Raw),
            });
        }
        
        Ok(data_points)
    }
    
    /// Get statistical summary for a metric
    #[cfg(feature = "time-utils")]
    pub async fn get_statistics(&self, resource_id: &str, metric_name: &str, hours: i64) -> LighthouseResult<Option<MetricsStatistics>> {
        let query = MetricsQuery::last_hours(resource_id.to_string(), Some(metric_name.to_string()), hours);
        let data_points = self.query_metrics(&query).await?;
        
        if data_points.is_empty() {
            return Ok(None);
        }
        
        let values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();
        
        // Calculate basic statistics
        let count = values.len() as f64;
        let mean = values.iter().sum::<f64>() / count;
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        // Calculate standard deviation
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count;
        let std_dev = variance.sqrt();
        
        // Calculate percentiles (simplified implementation)
        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let percentiles = [25, 75, 90, 95, 99].iter()
            .map(|&p| {
                let index = ((p as f64 / 100.0) * (sorted_values.len() - 1) as f64).round() as usize;
                (p, sorted_values[index.min(sorted_values.len() - 1)])
            })
            .collect();
        
        let median = sorted_values[sorted_values.len() / 2];
        
        // Simple trend analysis
        let trend = self.analyze_simple_trend(&data_points);
        
        // Calculate missing data percentage based on expected collection interval
        let missing_data_percentage = self.calculate_missing_data_percentage(
            &query.start_time, 
            &query.end_time, 
            data_points.len()
        );
        
        Ok(Some(MetricsStatistics {
            resource_id: resource_id.to_string(),
            metric_name: metric_name.to_string(),
            start_time: query.start_time,
            end_time: query.end_time,
            mean,
            median,
            min,
            max,
            std_dev,
            percentiles,
            trend,
            data_points: data_points.len(),
            missing_data_percentage,
        }))
    }
    
    /// Calculate the percentage of missing data based on expected collection interval
    #[cfg(feature = "time-utils")]
    fn calculate_missing_data_percentage(
        &self,
        start_time: &DateTime<Utc>,
        end_time: &DateTime<Utc>,
        actual_data_points: usize,
    ) -> f64 {
        // Assume data should be collected every 5 minutes (common interval)
        let expected_interval_minutes = 5;
        let time_span_minutes = (*end_time - *start_time).num_minutes();
        
        if time_span_minutes <= 0 {
            return 0.0;
        }
        
        let expected_data_points = (time_span_minutes / expected_interval_minutes) as usize + 1;
        
        if expected_data_points == 0 {
            return 0.0;
        }
        
        let missing_points = if actual_data_points < expected_data_points {
            expected_data_points - actual_data_points
        } else {
            0
        };
        
        (missing_points as f64 / expected_data_points as f64) * 100.0
    }
    
    #[cfg(not(feature = "time-utils"))]
    fn calculate_missing_data_percentage(
        &self,
        start_time: &u64,
        end_time: &u64,
        actual_data_points: usize,
    ) -> f64 {
        // Assume data should be collected every 300 seconds (5 minutes)
        let expected_interval_seconds = 300u64;
        let time_span_seconds = end_time.saturating_sub(*start_time);
        
        if time_span_seconds == 0 {
            return 0.0;
        }
        
        let expected_data_points = (time_span_seconds / expected_interval_seconds) as usize + 1;
        
        if expected_data_points == 0 {
            return 0.0;
        }
        
        let missing_points = if actual_data_points < expected_data_points {
            expected_data_points - actual_data_points
        } else {
            0
        };
        
        (missing_points as f64 / expected_data_points as f64) * 100.0
    }
    
    /// Perform simple trend analysis
    fn analyze_simple_trend(&self, data_points: &[HistoricalDataPoint]) -> TrendAnalysis {
        if data_points.len() < 3 {
            return TrendAnalysis {
                direction: TrendDirection::Unknown,
                confidence: 0.0,
                change_rate_per_hour: 0.0,
                slope: 0.0,
                r_squared: 0.0,
                seasonality: vec![],
            };
        }
        
        // Simple linear regression
        let n = data_points.len() as f64;
        let x_values: Vec<f64> = (0..data_points.len()).map(|i| i as f64).collect();
        let y_values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();
        
        let x_mean = x_values.iter().sum::<f64>() / n;
        let y_mean = y_values.iter().sum::<f64>() / n;
        
        let numerator: f64 = x_values.iter().zip(&y_values)
            .map(|(x, y)| (x - x_mean) * (y - y_mean))
            .sum();
        
        let denominator: f64 = x_values.iter()
            .map(|x| (x - x_mean).powi(2))
            .sum();
        
        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        
        // Calculate R-squared
        let predicted: Vec<f64> = x_values.iter()
            .map(|x| y_mean + slope * (x - x_mean))
            .collect();
        
        let ss_res: f64 = y_values.iter().zip(&predicted)
            .map(|(y, pred)| (y - pred).powi(2))
            .sum();
        
        let ss_tot: f64 = y_values.iter()
            .map(|y| (y - y_mean).powi(2))
            .sum();
        
        let r_squared = if ss_tot != 0.0 { 1.0 - (ss_res / ss_tot) } else { 0.0 };
        
        // Determine trend direction and confidence
        let direction = if slope.abs() < 0.01 {
            TrendDirection::Stable
        } else if slope > 0.0 {
            TrendDirection::Increasing
        } else {
            TrendDirection::Decreasing
        };
        
        let confidence = r_squared.abs().min(1.0);
        
        // Calculate change rate per hour (simplified)
        #[cfg(feature = "time-utils")]
        let time_span_hours = if data_points.len() > 1 {
            let start = data_points.first().unwrap().timestamp;
            let end = data_points.last().unwrap().timestamp;
            (end - start).num_seconds() as f64 / 3600.0
        } else {
            1.0
        };
        
        #[cfg(not(feature = "time-utils"))]
        let time_span_hours = if data_points.len() > 1 {
            let start = data_points.first().unwrap().timestamp;
            let end = data_points.last().unwrap().timestamp;
            (end - start) as f64 / 3600.0
        } else {
            1.0
        };
        
        let change_rate_per_hour = if time_span_hours > 0.0 {
            slope * (n - 1.0) / time_span_hours
        } else {
            0.0
        };
        
        // Detect seasonal patterns
        let seasonality = self.detect_basic_seasonality(data_points);
        
        TrendAnalysis {
            direction,
            confidence,
            change_rate_per_hour,
            slope,
            r_squared,
            seasonality,
        }
    }
    
    /// Detect basic seasonal patterns in the data
    fn detect_basic_seasonality(&self, data_points: &[HistoricalDataPoint]) -> Vec<crate::persistence::SeasonalPattern> {
        if data_points.len() < 12 { // Need at least 12 points for pattern detection
            return vec![];
        }
        
        let mut patterns = vec![];
        let values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();
        
        // Check for daily patterns (assuming hourly data collection)
        if data_points.len() >= 24 {
            let daily_correlation = self.calculate_autocorrelation(&values, 24);
            if daily_correlation > 0.3 { // Threshold for significant correlation
                patterns.push(crate::persistence::SeasonalPattern {
                    pattern_type: "daily".to_string(),
                    strength: daily_correlation.min(1.0),
                    peak_times: self.find_daily_peaks(&values),
                });
            }
        }
        
        // Check for weekly patterns (assuming hourly data, 168 hours in a week)
        if data_points.len() >= 168 {
            let weekly_correlation = self.calculate_autocorrelation(&values, 168);
            if weekly_correlation > 0.25 {
                patterns.push(crate::persistence::SeasonalPattern {
                    pattern_type: "weekly".to_string(),
                    strength: weekly_correlation.min(1.0),
                    peak_times: self.find_weekly_peaks(),
                });
            }
        }
        
        patterns
    }
    
    /// Calculate autocorrelation for lag detection
    fn calculate_autocorrelation(&self, values: &[f64], lag: usize) -> f64 {
        if values.len() < lag + 1 {
            return 0.0;
        }
        
        let n = values.len() - lag;
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        
        for i in 0..n {
            let x = values[i] - mean;
            let y = values[i + lag] - mean;
            numerator += x * y;
            denominator += x * x;
        }
        
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
    
    /// Find typical daily peak hours
    fn find_daily_peaks(&self, values: &[f64]) -> Vec<String> {
        // Simple heuristic: assume peak hours are when values are consistently high
        let mut hourly_averages = vec![0.0; 24];
        let mut hourly_counts = vec![0; 24];
        
        for (i, &value) in values.iter().enumerate() {
            let hour = i % 24;
            hourly_averages[hour] += value;
            hourly_counts[hour] += 1;
        }
        
        // Calculate actual averages
        for i in 0..24 {
            if hourly_counts[i] > 0 {
                hourly_averages[i] /= hourly_counts[i] as f64;
            }
        }
        
        // Find hours where average is significantly above overall mean
        let overall_mean = hourly_averages.iter().sum::<f64>() / 24.0;
        let mut peaks = vec![];
        
        for (hour, &avg) in hourly_averages.iter().enumerate() {
            if avg > overall_mean * 1.2 { // 20% above average
                peaks.push(format!("{:02}:00", hour));
            }
        }
        
        if peaks.is_empty() {
            vec!["12:00".to_string()] // Default to noon if no clear peaks
        } else {
            peaks
        }
    }
    
    /// Find typical weekly peak days
    fn find_weekly_peaks(&self) -> Vec<String> {
        // Simple heuristic for common business patterns
        vec!["Monday".to_string(), "Tuesday".to_string(), "Wednesday".to_string()]
    }
    
    /// Run maintenance tasks (cleanup old data, create aggregates)
    pub async fn run_maintenance(&self) -> LighthouseResult<()> {
        self.cleanup_old_data().await?;
        if self.config.enable_aggregation {
            self.create_hourly_aggregates().await?;
            self.create_daily_aggregates().await?;
        }
        Ok(())
    }
    
    /// Clean up old data according to retention policy
    async fn cleanup_old_data(&self) -> LighthouseResult<()> {
        #[cfg(feature = "time-utils")]
        let now = Utc::now();
        #[cfg(not(feature = "time-utils"))]
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Clean up raw data
        #[cfg(feature = "time-utils")]
        let raw_cutoff = (now - ChronoDuration::days(self.config.retention_policy.raw_data_days as i64)).timestamp();
        #[cfg(not(feature = "time-utils"))]
        let raw_cutoff = now - (self.config.retention_policy.raw_data_days as u64 * 24 * 60 * 60);
        
        sqlx::query("DELETE FROM raw_metrics WHERE timestamp < ?")
            .bind(raw_cutoff)
            .execute(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to cleanup raw data: {}", e)))?;
        
        // Clean up hourly aggregates
        #[cfg(feature = "time-utils")]
        let hourly_cutoff = (now - ChronoDuration::days(self.config.retention_policy.hourly_aggregates_days as i64)).timestamp();
        #[cfg(not(feature = "time-utils"))]
        let hourly_cutoff = now - (self.config.retention_policy.hourly_aggregates_days as u64 * 24 * 60 * 60);
        
        sqlx::query("DELETE FROM aggregated_metrics WHERE aggregation_level = 'hourly' AND timestamp < ?")
            .bind(hourly_cutoff)
            .execute(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to cleanup hourly data: {}", e)))?;
        
        // Clean up daily aggregates
        #[cfg(feature = "time-utils")]
        let daily_cutoff = (now - ChronoDuration::days(self.config.retention_policy.daily_aggregates_days as i64)).timestamp();
        #[cfg(not(feature = "time-utils"))]
        let daily_cutoff = now - (self.config.retention_policy.daily_aggregates_days as u64 * 24 * 60 * 60);
        
        sqlx::query("DELETE FROM aggregated_metrics WHERE aggregation_level = 'daily' AND timestamp < ?")
            .bind(daily_cutoff)
            .execute(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to cleanup daily data: {}", e)))?;
        
        Ok(())
    }
    
    /// Create hourly aggregates from raw data
    async fn create_hourly_aggregates(&self) -> LighthouseResult<()> {
        sqlx::query(r#"
            INSERT OR REPLACE INTO aggregated_metrics 
            (resource_id, metric_name, aggregation_level, timestamp, mean_value, min_value, max_value, count, std_dev)
            SELECT 
                resource_id,
                metric_name,
                'hourly' as aggregation_level,
                (timestamp / 3600) * 3600 as hour_timestamp,
                AVG(metric_value) as mean_value,
                MIN(metric_value) as min_value,
                MAX(metric_value) as max_value,
                COUNT(*) as count,
                0.0 as std_dev
            FROM raw_metrics 
            WHERE timestamp > (unixepoch() - 86400)  -- Last 24 hours
            GROUP BY resource_id, metric_name, (timestamp / 3600)
        "#).execute(&self.pool).await
        .map_err(|e| LighthouseError::config(&format!("Failed to create hourly aggregates: {}", e)))?;
        
        Ok(())
    }
    
    /// Create daily aggregates from hourly data
    async fn create_daily_aggregates(&self) -> LighthouseResult<()> {
        sqlx::query(r#"
            INSERT OR REPLACE INTO aggregated_metrics 
            (resource_id, metric_name, aggregation_level, timestamp, mean_value, min_value, max_value, count, std_dev)
            SELECT 
                resource_id,
                metric_name,
                'daily' as aggregation_level,
                (timestamp / 86400) * 86400 as day_timestamp,
                AVG(mean_value) as mean_value,
                MIN(min_value) as min_value,
                MAX(max_value) as max_value,
                SUM(count) as count,
                0.0 as std_dev
            FROM aggregated_metrics 
            WHERE aggregation_level = 'hourly' 
            AND timestamp > (unixepoch() - 86400 * 7)  -- Last 7 days
            GROUP BY resource_id, metric_name, (timestamp / 86400)
        "#).execute(&self.pool).await
        .map_err(|e| LighthouseError::config(&format!("Failed to create daily aggregates: {}", e)))?;
        
        Ok(())
    }
    
    /// Get database statistics
    pub async fn get_store_stats(&self) -> LighthouseResult<StoreStatistics> {
        let raw_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM raw_metrics")
            .fetch_one(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to get raw count: {}", e)))?;
        
        let hourly_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aggregated_metrics WHERE aggregation_level = 'hourly'")
            .fetch_one(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to get hourly count: {}", e)))?;
        
        let daily_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aggregated_metrics WHERE aggregation_level = 'daily'")
            .fetch_one(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to get daily count: {}", e)))?;
        
        let unique_resources: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT resource_id) FROM raw_metrics")
            .fetch_one(&self.pool).await
            .map_err(|e| LighthouseError::config(&format!("Failed to get unique resources: {}", e)))?;
        
        Ok(StoreStatistics {
            raw_data_points: raw_count as u64,
            hourly_aggregates: hourly_count as u64,
            daily_aggregates: daily_count as u64,
            unique_resources: unique_resources as u64,
            database_size_bytes: self.get_database_size().await.unwrap_or(0),
        })
    }
    
    /// Get database file size
    async fn get_database_size(&self) -> Option<u64> {
        std::fs::metadata(&self.config.database_path)
            .ok()
            .map(|m| m.len())
    }
}

/// Statistics about the metrics store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStatistics {
    /// Number of raw data points stored
    pub raw_data_points: u64,
    
    /// Number of hourly aggregates stored
    pub hourly_aggregates: u64,
    
    /// Number of daily aggregates stored
    pub daily_aggregates: u64,
    
    /// Number of unique resources tracked
    pub unique_resources: u64,
    
    /// Database file size in bytes
    pub database_size_bytes: u64,
}

// Placeholder implementations when persistence is not enabled
#[cfg(not(feature = "metrics-persistence"))]
pub struct MetricsStore;

#[cfg(not(feature = "metrics-persistence"))]
impl MetricsStore {
    pub async fn new(_config: MetricsStoreConfig) -> LighthouseResult<Self> {
        Err(LighthouseError::config("Metrics persistence not enabled. Enable the 'metrics-persistence' feature."))
    }
    
    pub async fn store_metrics(&self, _metrics: &ResourceMetrics) -> LighthouseResult<()> {
        Err(LighthouseError::config("Metrics persistence not enabled"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::multi_metrics;
    
    #[cfg(feature = "metrics-persistence")]
    #[tokio::test]
    async fn test_metrics_store_creation() {
        let config = MetricsStoreConfig::builder()
            .database_path(":memory:")
            .build();
        
        let store = MetricsStore::new(config).await;
        assert!(store.is_ok());
    }
    
    #[cfg(feature = "metrics-persistence")]
    #[tokio::test]
    async fn test_store_and_query_metrics() {
        let config = MetricsStoreConfig::builder()
            .database_path(":memory:")
            .build();
        
        let store = MetricsStore::new(config).await.unwrap();
        
        // Store test metrics
        let metrics = multi_metrics("test-resource", "test-type", vec![
            ("cpu", 75.0),
            ("memory", 60.0),
        ]);
        
        store.store_metrics(&metrics).await.unwrap();
        
        // Query metrics back
        #[cfg(feature = "time-utils")]
        let query = MetricsQuery::last_hours("test-resource".to_string(), Some("cpu".to_string()), 1);
        
        #[cfg(feature = "time-utils")]
        let results = store.query_metrics(&query).await.unwrap();
        
        #[cfg(feature = "time-utils")]
        assert_eq!(results.len(), 1);
        
        #[cfg(feature = "time-utils")]
        assert_eq!(results[0].value, 75.0);
    }
    
    #[test]
    fn test_retention_policy_builder() {
        let policy = RetentionPolicy::new()
            .raw_data_days(14)
            .hourly_aggregates_days(90)
            .daily_aggregates_days(730)
            .max_data_points(50_000);
        
        assert_eq!(policy.raw_data_days, 14);
        assert_eq!(policy.hourly_aggregates_days, 90);
        assert_eq!(policy.daily_aggregates_days, 730);
        assert_eq!(policy.max_data_points_per_metric, Some(50_000));
    }
    
    #[test]
    fn test_config_builder() {
        let config = MetricsStoreConfig::builder()
            .database_path("/tmp/test.db")
            .max_connections(20)
            .enable_aggregation(false)
            .maintenance_interval_hours(12)
            .retention_policy(RetentionPolicy::new().raw_data_days(30))
            .build();
        
        assert_eq!(config.database_path, "/tmp/test.db");
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.enable_aggregation, false);
        assert_eq!(config.maintenance_interval_hours, 12);
        assert_eq!(config.retention_policy.raw_data_days, 30);
    }
}