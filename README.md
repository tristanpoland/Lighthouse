# 🚨 Lighthouse

**Intelligent Autoscaling Library for Rust**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/tristanpoland/lighthouse#license)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

Lighthouse is a flexible, generic autoscaling library that can work with any infrastructure or resource type. It provides type-safe callbacks and streaming metrics processing to make intelligent scaling decisions for your applications.

## 🌟 Why Lighthouse?

Modern applications need to scale dynamically based on real-time metrics, but most autoscaling solutions are tightly coupled to specific platforms. Lighthouse breaks this limitation by providing a **generic, type-safe framework** that works with any infrastructure while maintaining the safety and performance you expect from Rust.

Whether you're running Kubernetes, AWS, bare metal servers, or even custom infrastructure, Lighthouse adapts to your environment through simple trait implementations.

## ✨ Features

**Generic & Platform Agnostic** - Works with Kubernetes, AWS, Docker, bare metal, or any custom infrastructure through trait-based callbacks.

**Type-Safe Configuration** - Compile-time guarantees for your scaling logic with rich configuration options and validation.

**Live Configuration Updates** - Change scaling policies, thresholds, and parameters without restarting your application.

**Smart Cooldown Handling** - Prevents scaling flapping with configurable cooldown periods and safety checks.

**Comprehensive Observability** - Built-in hooks for logging, monitoring, and alerting on scaling events.

**Fully Async** - Built on Tokio with non-blocking operations and efficient resource usage.

## 🚀 Quick Start

Add Lighthouse to your `Cargo.toml`:

```toml
[dependencies]
lighthouse = { git = "https://github.com/tristanpoland/Lighthouse" tag = "0.1.0" }
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
```

Create your autoscaler in just a few lines:

```rust
use lighthouse::{
    LighthouseEngine, LighthouseConfig, LighthouseCallbacks,
    MetricsProvider, ScalingExecutor, policies
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Implement how to get metrics from your system
    let metrics_provider = Arc::new(MyMetricsProvider::new());
    
    // 2. Implement how to scale your infrastructure  
    let scaling_executor = Arc::new(MyScalingExecutor::new());
    
    // 3. Configure scaling policies
    let config = LighthouseConfig::builder()
        .evaluation_interval(30)
        .add_resource_config("web-servers", ResourceConfig {
            resource_type: "web-servers".to_string(),
            policies: vec![
                policies::cpu_scaling_policy(80.0, 20.0, 1.5, 300)
            ],
            // ... other settings
        })
        .build();
    
    // 4. Start the engine
    let callbacks = LighthouseCallbacks::new(metrics_provider, scaling_executor);
    let engine = LighthouseEngine::new(config, callbacks);
    let handle = engine.handle();
    
    tokio::spawn(async move { engine.start().await });
    
    // 5. Send metrics and get automatic scaling
    handle.update_metrics(my_metrics).await?;
    
    Ok(())
}
```

## 🏗️ Architecture

Lighthouse uses a callback-based architecture that separates concerns and maximizes flexibility:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Your Metrics   │──▶│    Lighthouse    │───▶│   Your Scaling  │
│     System      │    │      Engine      │    │  Infrastructure │
│                 │    │                  │    │                 │
│ • Prometheus    │    │ • Policy Engine  │    │ • Kubernetes    │
│ • Custom APIs   │    │ • Cooldowns      │    │ • AWS ASG       │
│ • Databases     │    │ • Safety Checks  │    │ • Custom APIs   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

You implement three simple traits to integrate with your infrastructure:

**`MetricsProvider`** - Fetch metrics from your monitoring system  
**`ScalingExecutor`** - Execute scaling actions on your infrastructure  
**`ScalingObserver`** - Receive notifications about scaling events (optional)

## 💡 Real-World Example

Here's how you might use Lighthouse to autoscale a Kubernetes deployment based on CPU and memory metrics:

```rust
use lighthouse::*;
use async_trait::async_trait;

struct KubernetesMetrics {
    client: kube::Client,
}

#[async_trait]
impl MetricsProvider for KubernetesMetrics {
    async fn get_metrics(&self, resource_id: &str, _ctx: &CallbackContext) 
        -> LighthouseResult<Option<ResourceMetrics>> 
    {
        // Fetch CPU/memory from Kubernetes metrics API
        let cpu = self.get_cpu_usage(resource_id).await?;
        let memory = self.get_memory_usage(resource_id).await?;
        
        Ok(Some(utils::multi_metrics(
            resource_id, "k8s-deployment",
            vec![("cpu_percent", cpu), ("memory_percent", memory)]
        )))
    }
}

struct KubernetesScaler {
    client: kube::Client,
}

#[async_trait]
impl ScalingExecutor for KubernetesScaler {
    async fn execute_scale_action(&self, action: &ScaleAction, _ctx: &CallbackContext) 
        -> LighthouseResult<bool> 
    {
        match action.direction {
            ScaleDirection::Up => {
                self.scale_deployment(&action.resource_id, action.scale_factor).await?;
                println!("✅ Scaled up {}: {}", action.resource_id, action.reason);
            },
            ScaleDirection::Down => {
                self.scale_deployment(&action.resource_id, action.scale_factor).await?;
                println!("📉 Scaled down {}: {}", action.resource_id, action.reason);
            },
            ScaleDirection::Maintain => {
                println!("➡️ Maintaining {}", action.resource_id);
            }
        }
        Ok(true)
    }
    
    async fn is_safe_to_scale(&self, action: &ScaleAction, _ctx: &CallbackContext) 
        -> LighthouseResult<bool> 
    {
        // Implement your safety checks
        // - Don't scale during maintenance windows
        // - Respect minimum/maximum limits
        // - Check cluster resource availability
        Ok(self.check_cluster_capacity().await? && !self.is_maintenance_window())
    }
}
```

## 🎛️ Configuration

Lighthouse supports rich configuration options that can be updated live:

```rust
let config = LighthouseConfig::builder()
    .evaluation_interval(30) // Check every 30 seconds
    .add_resource_config("web-tier", ResourceConfig {
        resource_type: "kubernetes-deployment".to_string(),
        policies: vec![
            // Multi-metric policy
            policies::multi_metric_policy(
                "web-scaling",
                (75.0, 25.0), // CPU thresholds  
                (80.0, 30.0), // Memory thresholds
                1.5,          // Scale factor
                300           // 5 minute cooldown
            ),
            // Custom policy for request rate
            ScalingPolicy {
                name: "request-rate".to_string(),
                thresholds: vec![ScalingThreshold {
                    metric_name: "requests_per_second".to_string(),
                    scale_up_threshold: 1000.0,
                    scale_down_threshold: 200.0,
                    scale_factor: 2.0,
                    cooldown_seconds: 180,
                }],
                min_capacity: Some(2),
                max_capacity: Some(50),
                enabled: true,
            }
        ],
        default_policy: Some("web-scaling".to_string()),
        settings: [
            ("cluster_name".to_string(), "prod-us-west".to_string()),
            ("namespace".to_string(), "web-services".to_string()),
        ].into(),
    })
    .global_setting("environment", "production")
    .enable_logging(true)
    .build();

// Update configuration live
handle.update_config(new_config).await?;
```

## 📊 Observability

Monitor your scaling decisions with built-in observability:

```rust
struct ScalingLogger;

#[async_trait]
impl ScalingObserver for ScalingLogger {
    async fn on_scaling_decision(&self, action: &ScaleAction, _ctx: &CallbackContext) 
        -> LighthouseResult<()> 
    {
        info!(
            resource = action.resource_id,
            direction = ?action.direction,
            confidence = action.confidence,
            reason = action.reason,
            "Scaling decision made"
        );
        
        // Send to your monitoring system
        metrics::counter!("lighthouse_scaling_decisions").increment(1);
        Ok(())
    }
    
    async fn on_scaling_executed(&self, action: &ScaleAction, success: bool, _ctx: &CallbackContext) 
        -> LighthouseResult<()> 
    {
        if success {
            info!("✅ Successfully scaled {}", action.resource_id);
        } else {
            warn!("⚠️ Scaling was skipped for {}", action.resource_id);
        }
        Ok(())
    }
}

let callbacks = LighthouseCallbacks::new(metrics_provider, scaling_executor)
    .add_observer(Arc::new(ScalingLogger));
```

## 🔧 Platform Integrations

Lighthouse includes optional integrations for common platforms:

```toml
[dependencies]
lighthouse = { git = "https://github.com/tristanpoland/Lighthouse" tag = "0.1.0", features = ["prometheus-metrics"] }
```

**Kubernetes** - Ready-to-use implementations for Kubernetes deployments, StatefulSets, and HPA integration.

**AWS** - Support for Auto Scaling Groups, ECS services, and Lambda concurrency scaling.

**Prometheus** - Built-in metrics collection and alerting integration.

**Webhooks** - HTTP callback support for custom integrations and notifications.

## 🚦 Examples

The repository includes comprehensive examples:

- **`basic_usage.rs`** - Core concepts and simple setup
- **`kubernetes.rs`** - Complete Kubernetes integration
- **`aws_ec2.rs`** - AWS Auto Scaling Groups
- **`prometheus_metrics.rs`** - Metrics collection and alerting
- **`webhook_notifications.rs`** - Custom HTTP integrations

Run any example:

```bash
cargo run --example basic_usage
cargo run --example kubernetes
```

## 📈 Performance

Lighthouse is designed for production use with minimal overhead:

- **Memory efficient** - Streaming metrics processing with configurable retention
- **CPU optimized** - Efficient policy evaluation and async processing  
- **Network friendly** - Batched operations and connection pooling
- **Scalable** - Handles thousands of resources with consistent performance

## 🤝 Contributing

We welcome contributions! Lighthouse thrives on community input and diverse use cases.

- **Issues** - Report bugs, request features, or ask questions
- **Pull Requests** - Code improvements, new integrations, documentation
- **Examples** - Share how you're using Lighthouse in production

Please read our [Contributing Guide](CONTRIBUTING.md) for details on our development process.

## 📄 License

Lighthouse is dual-licensed under MIT OR Apache-2.0, the same as Rust itself.

---

**Ready to get started?** dive into the [examples](examples/) to see Lighthouse in action.

*Built with ❤️ in Rust for the modern infrastructure era.*
