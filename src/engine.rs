// src/engine.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::callbacks::{CallbackContext, LighthouseCallbacks};
use crate::error::{LighthouseError, LighthouseResult};
use crate::types::{
    LighthouseConfig, ResourceId, ResourceMetrics, ScaleAction, ScaleDirection, ScalingPolicy,
};

/// Commands that can be sent to the lighthouse engine
#[derive(Debug)]
pub enum EngineCommand {
    /// Add or update metrics for a resource
    UpdateMetrics(ResourceMetrics),
    /// Update the configuration
    UpdateConfig(LighthouseConfig),
    /// Request scaling recommendation for a resource
    GetRecommendation {
        resource_id: ResourceId,
        response: tokio::sync::oneshot::Sender<LighthouseResult<Option<ScaleAction>>>,
    },
    /// Get current engine status
    GetStatus {
        response: tokio::sync::oneshot::Sender<EngineStatus>,
    },
    /// Shutdown the engine
    Shutdown,
}

/// Status information about the lighthouse engine
#[derive(Debug, Clone)]
pub struct EngineStatus {
    pub is_running: bool,
    pub resources_tracked: usize,
    pub total_recommendations: u64,
    pub last_evaluation: Option<u64>,
}

/// Tracks cooldown periods for resources to prevent flapping
#[derive(Debug)]
struct CooldownTracker {
    last_scale_time: HashMap<ResourceId, u64>,
}

impl CooldownTracker {
    fn new() -> Self {
        Self {
            last_scale_time: HashMap::new(),
        }
    }

    fn is_cooled_down(&self, resource_id: &ResourceId, cooldown_seconds: u64) -> bool {
        let now = current_timestamp();
        match self.last_scale_time.get(resource_id) {
            Some(last_time) => now >= last_time + cooldown_seconds,
            None => true, // Never scaled before
        }
    }

    fn record_scale_action(&mut self, resource_id: &ResourceId) {
        self.last_scale_time.insert(resource_id.clone(), current_timestamp());
    }
}

/// The main lighthouse autoscaling engine
pub struct LighthouseEngine {
    config: Arc<RwLock<LighthouseConfig>>,
    callbacks: LighthouseCallbacks,
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    command_rx: Option<mpsc::UnboundedReceiver<EngineCommand>>,
    status: Arc<RwLock<EngineStatus>>,
    metrics_cache: Arc<RwLock<HashMap<ResourceId, ResourceMetrics>>>,
    cooldown_tracker: Arc<RwLock<CooldownTracker>>,
}

impl LighthouseEngine {
    /// Create a new lighthouse engine
    pub fn new(config: LighthouseConfig, callbacks: LighthouseCallbacks) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        Self {
            config: Arc::new(RwLock::new(config)),
            callbacks,
            command_tx,
            command_rx: Some(command_rx),
            status: Arc::new(RwLock::new(EngineStatus {
                is_running: false,
                resources_tracked: 0,
                total_recommendations: 0,
                last_evaluation: None,
            })),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            cooldown_tracker: Arc::new(RwLock::new(CooldownTracker::new())),
        }
    }

    /// Get a handle to send commands to the engine
    pub fn handle(&self) -> LighthouseHandle {
        LighthouseHandle {
            command_tx: self.command_tx.clone(),
        }
    }

    /// Start the lighthouse engine (consumes self)
    pub async fn start(mut self) -> LighthouseResult<()> {
        let mut command_rx = self.command_rx.take()
            .ok_or_else(|| LighthouseError::engine_not_running("Engine already started"))?;

        // Update status
        {
            let mut status = self.status.write().await;
            status.is_running = true;
        }

        info!("Lighthouse engine starting...");

        // Create evaluation timer
        let config = self.config.read().await;
        let interval_duration = Duration::from_secs(config.evaluation_interval_seconds);
        drop(config);

        let mut evaluation_timer = interval(interval_duration);

        // Main engine loop
        loop {
            tokio::select! {
                // Handle commands
                command = command_rx.recv() => {
                    match command {
                        Some(cmd) => {
                            if let Err(e) = self.handle_command(cmd).await {
                                error!("Error handling command: {}", e);
                            }
                        }
                        None => {
                            info!("Command channel closed, shutting down engine");
                            break;
                        }
                    }
                }

                // Periodic evaluation
                _ = evaluation_timer.tick() => {
                    if let Err(e) = self.evaluate_all_resources().await {
                        error!("Error during evaluation: {}", e);
                    }
                }
            }
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.is_running = false;
        }

        info!("Lighthouse engine stopped");
        Ok(())
    }

    /// Handle incoming commands
    async fn handle_command(&self, command: EngineCommand) -> LighthouseResult<()> {
        match command {
            EngineCommand::UpdateMetrics(metrics) => {
                self.update_metrics(metrics).await?;
            }
            EngineCommand::UpdateConfig(new_config) => {
                self.update_config(new_config).await?;
            }
            EngineCommand::GetRecommendation { resource_id, response } => {
                let recommendation = self.get_recommendation(&resource_id).await;
                let _ = response.send(recommendation); // Ignore send errors
            }
            EngineCommand::GetStatus { response } => {
                let status = self.status.read().await.clone();
                let _ = response.send(status); // Ignore send errors
            }
            EngineCommand::Shutdown => {
                info!("Shutdown command received");
                return Err(LighthouseError::engine_not_running("Shutdown requested"));
            }
        }
        Ok(())
    }

    /// Update metrics for a resource
    async fn update_metrics(&self, metrics: ResourceMetrics) -> LighthouseResult<()> {
        // Validate metrics using callback
        let context = CallbackContext {
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        };

        let validated_metrics = self.callbacks.metrics_provider
            .validate_metrics(&metrics, &context).await?;

        if let Some(validated) = validated_metrics {
            let mut cache = self.metrics_cache.write().await;
            cache.insert(validated.resource_id.clone(), validated);

            // Update status
            let mut status = self.status.write().await;
            status.resources_tracked = cache.len();
        }

        Ok(())
    }

    /// Update the engine configuration
    async fn update_config(&self, new_config: LighthouseConfig) -> LighthouseResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Configuration updated");
        Ok(())
    }

    /// Get scaling recommendation for a specific resource
    async fn get_recommendation(&self, resource_id: &ResourceId) -> LighthouseResult<Option<ScaleAction>> {
        let metrics_cache = self.metrics_cache.read().await;
        let config = self.config.read().await;

        let metrics = match metrics_cache.get(resource_id) {
            Some(m) => m,
            None => return Ok(None), // No metrics available
        };

        // Find appropriate scaling policy
        let resource_config = config.resource_configs.get(&metrics.resource_type);
        let resource_config = match resource_config {
            Some(rc) => rc,
            None => return Ok(None), // No configuration for this resource type
        };

        // Evaluate all policies for this resource
        for policy in &resource_config.policies {
            if !policy.enabled {
                continue;
            }

            if let Some(action) = self.evaluate_policy(metrics, policy).await? {
                // Check cooldown
                let cooldown_tracker = self.cooldown_tracker.read().await;
                let cooldown_seconds = policy.thresholds.iter()
                    .map(|t| t.cooldown_seconds)
                    .max()
                    .unwrap_or(0);

                if !cooldown_tracker.is_cooled_down(resource_id, cooldown_seconds) {
                    debug!("Scaling action skipped due to cooldown: {}", resource_id);
                    continue;
                }

                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    /// Evaluate a scaling policy against metrics
    async fn evaluate_policy(
        &self,
        metrics: &ResourceMetrics,
        policy: &ScalingPolicy,
    ) -> LighthouseResult<Option<ScaleAction>> {
        for threshold in &policy.thresholds {
            let metric_value = match metrics.metrics.get(&threshold.metric_name) {
                Some(v) => *v,
                None => continue, // Metric not available
            };

            // Check for scale up
            if metric_value > threshold.scale_up_threshold {
                return Ok(Some(ScaleAction {
                    resource_id: metrics.resource_id.clone(),
                    resource_type: metrics.resource_type.clone(),
                    direction: ScaleDirection::Up,
                    target_capacity: None,
                    scale_factor: Some(threshold.scale_factor),
                    reason: format!(
                        "{} ({:.2}) exceeded scale-up threshold ({:.2})",
                        threshold.metric_name, metric_value, threshold.scale_up_threshold
                    ),
                    confidence: 0.8, // TODO: Make this configurable
                    timestamp: current_timestamp(),
                    metadata: HashMap::new(),
                }));
            }

            // Check for scale down
            if metric_value < threshold.scale_down_threshold {
                return Ok(Some(ScaleAction {
                    resource_id: metrics.resource_id.clone(),
                    resource_type: metrics.resource_type.clone(),
                    direction: ScaleDirection::Down,
                    target_capacity: None,
                    scale_factor: Some(1.0 / threshold.scale_factor),
                    reason: format!(
                        "{} ({:.2}) below scale-down threshold ({:.2})",
                        threshold.metric_name, metric_value, threshold.scale_down_threshold
                    ),
                    confidence: 0.8, // TODO: Make this configurable
                    timestamp: current_timestamp(),
                    metadata: HashMap::new(),
                }));
            }
        }

        Ok(None)
    }

    /// Evaluate all resources and execute scaling actions
    async fn evaluate_all_resources(&self) -> LighthouseResult<()> {
        let metrics_cache = self.metrics_cache.read().await;
        let resource_ids: Vec<ResourceId> = metrics_cache.keys().cloned().collect();
        drop(metrics_cache);

        let mut status = self.status.write().await;
        status.last_evaluation = Some(current_timestamp());
        drop(status);

        for resource_id in resource_ids {
            if let Some(action) = self.get_recommendation(&resource_id).await? {
                if let Err(e) = self.execute_scaling_action(action).await {
                    error!("Failed to execute scaling action for {}: {}", resource_id, e);
                }
            }
        }

        Ok(())
    }

    /// Execute a scaling action
    async fn execute_scaling_action(&self, action: ScaleAction) -> LighthouseResult<()> {
        let context = CallbackContext {
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        };

        // Notify observers of decision
        for observer in &self.callbacks.observers {
            if let Err(e) = observer.on_scaling_decision(&action, &context).await {
                warn!("Observer error on scaling decision: {}", e);
            }
        }

        // Check safety
        let is_safe = self.callbacks.scaling_executor
            .is_safe_to_scale(&action, &context).await?;

        if !is_safe {
            for observer in &self.callbacks.observers {
                if let Err(e) = observer.on_scaling_skipped(&action, "Safety check failed", &context).await {
                    warn!("Observer error on scaling skipped: {}", e);
                }
            }
            return Ok(());
        }

        // Execute the action
        match self.callbacks.scaling_executor.execute_scale_action(&action, &context).await {
            Ok(executed) => {
                if executed {
                    // Record successful scaling
                    let mut cooldown_tracker = self.cooldown_tracker.write().await;
                    cooldown_tracker.record_scale_action(&action.resource_id);

                    let mut status = self.status.write().await;
                    status.total_recommendations += 1;

                    info!("Scaling action executed: {} - {}", action.resource_id, action.reason);
                }

                // Notify observers
                for observer in &self.callbacks.observers {
                    if let Err(e) = observer.on_scaling_executed(&action, executed, &context).await {
                        warn!("Observer error on scaling executed: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to execute scaling action: {}", e);
                
                // Notify observers of error
                for observer in &self.callbacks.observers {
                    if let Err(err) = observer.on_scaling_error(&action, &e, &context).await {
                        warn!("Observer error on scaling error: {}", err);
                    }
                }

                return Err(e);
            }
        }

        Ok(())
    }
}

/// Handle for interacting with a running lighthouse engine
#[derive(Clone)]
pub struct LighthouseHandle {
    command_tx: mpsc::UnboundedSender<EngineCommand>,
}

impl LighthouseHandle {
    /// Send metrics to the engine
    pub async fn update_metrics(&self, metrics: ResourceMetrics) -> LighthouseResult<()> {
        self.command_tx.send(EngineCommand::UpdateMetrics(metrics))?;
        Ok(())
    }

    /// Update the engine configuration
    pub async fn update_config(&self, config: LighthouseConfig) -> LighthouseResult<()> {
        self.command_tx.send(EngineCommand::UpdateConfig(config))?;
        Ok(())
    }

    /// Get a scaling recommendation for a resource
    pub async fn get_recommendation(&self, resource_id: ResourceId) -> LighthouseResult<Option<ScaleAction>> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.command_tx.send(EngineCommand::GetRecommendation {
            resource_id,
            response: response_tx,
        })?;
        response_rx.await?
    }

    /// Get current engine status
    pub async fn get_status(&self) -> LighthouseResult<EngineStatus> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.command_tx.send(EngineCommand::GetStatus {
            response: response_tx,
        })?;
        Ok(response_rx.await?)
    }

    /// Shutdown the engine
    pub async fn shutdown(&self) -> LighthouseResult<()> {
        self.command_tx.send(EngineCommand::Shutdown)?;
        Ok(())
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}