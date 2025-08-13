#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::*;
	use crate::policies::*;
	use crate::utils::*;
	use crate::engine::*;
	use crate::callbacks::*;
	use crate::error::*;
	use std::collections::HashMap;
	use std::sync::Arc;
	use async_trait::async_trait;

	#[test]
	fn test_resource_metrics_construction() {
		let mut metrics = HashMap::new();
		metrics.insert("cpu_percent".to_string(), 75.0);
		let rm = ResourceMetrics {
			resource_id: "web-servers".to_string(),
			resource_type: "kubernetes-deployment".to_string(),
			timestamp: 1234567890,
			metrics,
		};
		assert_eq!(rm.resource_id, "web-servers");
		assert_eq!(rm.metrics["cpu_percent"], 75.0);
	}

	#[test]
	fn test_scale_action_construction() {
		let action = ScaleAction {
			resource_id: "db".to_string(),
			resource_type: "database".to_string(),
			direction: ScaleDirection::Up,
			target_capacity: Some(10),
			scale_factor: Some(2.0),
			reason: "High load".to_string(),
			confidence: 0.95,
			timestamp: 1234567890,
			metadata: HashMap::new(),
		};
		assert_eq!(action.direction, ScaleDirection::Up);
		assert_eq!(action.target_capacity, Some(10));
	}

	#[test]
	fn test_scaling_policy_builders() {
		let cpu = cpu_scaling_policy(80.0, 20.0, 1.5, 300);
		assert_eq!(cpu.name, "cpu-scaling");
		let mem = memory_scaling_policy(75.0, 30.0, 1.2, 120);
		assert_eq!(mem.name, "memory-scaling");
		let req = request_rate_scaling_policy(1000.0, 200.0, 2.0, 180);
		assert_eq!(req.name, "request-rate-scaling");
		let multi = multi_metric_policy("multi", (80.0, 20.0), (75.0, 30.0), 1.5, 300);
		assert_eq!(multi.thresholds.len(), 2);
	}

	#[test]
	fn test_utils_functions() {
		let rm = single_metric("id", "type", "cpu", 99.0);
		assert_eq!(rm.metrics["cpu"], 99.0);
		let rm2 = multi_metrics("id2", "type2", vec![("cpu", 88.0), ("mem", 77.0)]);
		assert_eq!(rm2.metrics["mem"], 77.0);
		let ts = current_timestamp();
		assert!(ts > 0);
	}

	struct DummyMetricsProvider;
	#[async_trait]
	impl MetricsProvider for DummyMetricsProvider {
		async fn get_metrics(&self, _resource_id: &str, _ctx: &CallbackContext) -> LighthouseResult<Option<ResourceMetrics>> {
			Ok(Some(single_metric("id", "type", "cpu", 50.0)))
		}
	}

	struct DummyScalingExecutor;
	#[async_trait]
	impl ScalingExecutor for DummyScalingExecutor {
		async fn execute_scale_action(&self, _action: &ScaleAction, _ctx: &CallbackContext) -> LighthouseResult<bool> {
			Ok(true)
		}
	}

	struct DummyObserver;
	#[async_trait]
	impl ScalingObserver for DummyObserver {
		async fn on_scaling_decision(&self, _action: &ScaleAction, _ctx: &CallbackContext) -> LighthouseResult<()> {
			Ok(())
		}
		async fn on_scaling_executed(&self, _action: &ScaleAction, _success: bool, _ctx: &CallbackContext) -> LighthouseResult<()> {
			Ok(())
		}
	}

	#[tokio::test]
	async fn test_engine_lifecycle() {
		let config = LighthouseConfig::builder()
			.evaluation_interval(1)
			.add_resource_config("test", ResourceConfig {
				resource_type: "test-type".to_string(),
				policies: vec![cpu_scaling_policy(80.0, 20.0, 1.5, 1)],
				default_policy: Some("cpu-scaling".to_string()),
				settings: HashMap::new(),
			})
			.build();
		let callbacks = LighthouseCallbacks::new(
			Arc::new(DummyMetricsProvider),
			Arc::new(DummyScalingExecutor),
		).add_observer(Arc::new(DummyObserver));
		let engine = LighthouseEngine::new(config.clone(), callbacks);
		let handle = engine.handle();
		let engine_task = tokio::spawn(async move {
			engine.start().await.unwrap();
		});
		let metrics = single_metric("test", "test-type", "cpu_percent", 85.0);
		handle.update_metrics(metrics).await.unwrap();
		let rec = handle.get_recommendation("test".to_string()).await.unwrap();
		assert!(rec.is_some());
		let status = handle.get_status().await.unwrap();
		assert!(status.resources_tracked >= 1);
		handle.shutdown().await.unwrap();
		engine_task.abort();
	}

	#[test]
	fn test_error_variants() {
	let err = LighthouseError::config("bad config");
	assert!(matches!(err, LighthouseError::Config { .. }));
	let err2 = LighthouseError::resource_not_found("id");
	assert!(matches!(err2, LighthouseError::ResourceNotFound { .. }));
	let err3 = LighthouseError::invalid_metric("bad metric");
	assert!(matches!(err3, LighthouseError::InvalidMetric { .. }));
	}
}
