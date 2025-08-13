//! Advanced scaling policy builders and composite policy types

use crate::types::{ScalingPolicy, ScalingThreshold, CompositePolicy, CompositeLogic};

#[cfg(feature = "time-utils")]
use crate::types::{TimeBasedPolicy, TimeSchedule};

#[cfg(feature = "time-utils")]
use chrono::{DateTime, Utc, Weekday, NaiveTime};

/// Create a simple CPU-based scaling policy
pub fn cpu_scaling_policy(
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    scale_factor: f64,
    cooldown_seconds: u64,
) -> ScalingPolicy {
    ScalingPolicy {
        name: "cpu-scaling".to_string(),
        thresholds: vec![ScalingThreshold {
            metric_name: "cpu_percent".to_string(),
            scale_up_threshold,
            scale_down_threshold,
            scale_factor,
            cooldown_seconds,
            confidence: None, // Use default confidence
        }],
        min_capacity: None,
        max_capacity: None,
        enabled: true,
    }
}

/// Create a memory-based scaling policy
pub fn memory_scaling_policy(
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    scale_factor: f64,
    cooldown_seconds: u64,
) -> ScalingPolicy {
    ScalingPolicy {
        name: "memory-scaling".to_string(),
        thresholds: vec![ScalingThreshold {
            metric_name: "memory_percent".to_string(),
            scale_up_threshold,
            scale_down_threshold,
            scale_factor,
            cooldown_seconds,
            confidence: None, // Use default confidence
        }],
        min_capacity: None,
        max_capacity: None,
        enabled: true,
    }
}

/// Create a request-rate based scaling policy
pub fn request_rate_scaling_policy(
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    scale_factor: f64,
    cooldown_seconds: u64,
) -> ScalingPolicy {
    ScalingPolicy {
        name: "request-rate-scaling".to_string(),
        thresholds: vec![ScalingThreshold {
            metric_name: "requests_per_second".to_string(),
            scale_up_threshold,
            scale_down_threshold,
            scale_factor,
            cooldown_seconds,
            confidence: None, // Use default confidence
        }],
        min_capacity: None,
        max_capacity: None,
        enabled: true,
    }
}

/// Create a multi-metric scaling policy
pub fn multi_metric_policy(
    name: &str,
    cpu_threshold: (f64, f64),
    memory_threshold: (f64, f64),
    scale_factor: f64,
    cooldown_seconds: u64,
) -> ScalingPolicy {
    ScalingPolicy {
        name: name.to_string(),
        thresholds: vec![
            ScalingThreshold {
                metric_name: "cpu_percent".to_string(),
                scale_up_threshold: cpu_threshold.0,
                scale_down_threshold: cpu_threshold.1,
                scale_factor,
                cooldown_seconds,
                confidence: None, // Use default confidence
            },
            ScalingThreshold {
                metric_name: "memory_percent".to_string(),
                scale_up_threshold: memory_threshold.0,
                scale_down_threshold: memory_threshold.1,
                scale_factor,
                cooldown_seconds,
                confidence: None, // Use default confidence
            },
        ],
        min_capacity: None,
        max_capacity: None,
        enabled: true,
    }
}

// ============================================================================
// Advanced Policy Types
// ============================================================================

/// Create a composite policy that combines multiple policies with AND logic
/// All policies must agree to trigger scaling
pub fn composite_and_policy(name: &str, policies: Vec<ScalingPolicy>) -> CompositePolicy {
    CompositePolicy {
        name: name.to_string(),
        policies,
        logic: CompositeLogic::AllMustTrigger,
        #[cfg(feature = "time-utils")]
        schedule: None,
        enabled: true,
    }
}

/// Create a composite policy that combines multiple policies with OR logic
/// Any policy can trigger scaling
pub fn composite_or_policy(name: &str, policies: Vec<ScalingPolicy>) -> CompositePolicy {
    CompositePolicy {
        name: name.to_string(),
        policies,
        logic: CompositeLogic::AnyCanTrigger,
        #[cfg(feature = "time-utils")]
        schedule: None,
        enabled: true,
    }
}

/// Create a composite policy with majority voting logic
/// More than half of the policies must agree to trigger scaling
pub fn composite_majority_policy(name: &str, policies: Vec<ScalingPolicy>) -> CompositePolicy {
    CompositePolicy {
        name: name.to_string(),
        policies,
        logic: CompositeLogic::Majority,
        #[cfg(feature = "time-utils")]
        schedule: None,
        enabled: true,
    }
}

/// Create a composite policy with weighted voting
/// Each policy has a weight, and scaling triggers when total weight exceeds threshold
pub fn composite_weighted_policy(
    name: &str, 
    policies: Vec<ScalingPolicy>, 
    weights: Vec<f64>
) -> CompositePolicy {
    assert_eq!(policies.len(), weights.len(), "Number of policies must match number of weights");
    
    CompositePolicy {
        name: name.to_string(),
        policies,
        logic: CompositeLogic::Weighted(weights),
        #[cfg(feature = "time-utils")]
        schedule: None,
        enabled: true,
    }
}

/// Create a time-based schedule
#[cfg(feature = "time-utils")]
pub fn time_schedule(
    start_time: &str,
    end_time: &str,
    days_of_week: Vec<u8>,
    timezone: Option<&str>,
) -> TimeSchedule {
    TimeSchedule {
        start_time: start_time.to_string(),
        end_time: end_time.to_string(),
        days_of_week,
        timezone: timezone.map(|tz| tz.to_string()),
    }
}

/// Create a business hours schedule (9 AM - 5 PM, Monday-Friday)
#[cfg(feature = "time-utils")]
pub fn business_hours_schedule(timezone: Option<&str>) -> TimeSchedule {
    time_schedule("09:00", "17:00", vec![1, 2, 3, 4, 5], timezone) // Monday-Friday
}

/// Create a weekend schedule (Saturday-Sunday)
#[cfg(feature = "time-utils")]
pub fn weekend_schedule(timezone: Option<&str>) -> TimeSchedule {
    time_schedule("00:00", "23:59", vec![0, 6], timezone) // Saturday-Sunday
}

/// Create a 24/7 schedule (always active)
#[cfg(feature = "time-utils")]
pub fn always_active_schedule(timezone: Option<&str>) -> TimeSchedule {
    time_schedule("00:00", "23:59", vec![0, 1, 2, 3, 4, 5, 6], timezone) // All days
}

/// Create a time-based policy with different scaling rules for different periods
#[cfg(feature = "time-utils")]
pub fn time_based_policy(
    name: &str,
    scheduled_policies: Vec<(TimeSchedule, ScalingPolicy)>,
    default_policy: Option<ScalingPolicy>,
) -> TimeBasedPolicy {
    TimeBasedPolicy {
        name: name.to_string(),
        scheduled_policies,
        default_policy,
        enabled: true,
    }
}

/// Create a business hours scaling policy with different rules for day/night
#[cfg(feature = "time-utils")]
pub fn business_hours_scaling_policy(
    name: &str,
    business_hours_policy: ScalingPolicy,
    after_hours_policy: ScalingPolicy,
    timezone: Option<&str>,
) -> TimeBasedPolicy {
    let business_schedule = business_hours_schedule(timezone);
    let after_hours_schedule = time_schedule("17:01", "08:59", vec![1, 2, 3, 4, 5], timezone);
    
    time_based_policy(
        name,
        vec![
            (business_schedule, business_hours_policy),
            (after_hours_schedule, after_hours_policy),
        ],
        None,
    )
}

/// Create an advanced multi-tier scaling policy
/// Combines CPU, memory, and request rate with different sensitivity levels
pub fn advanced_multi_tier_policy(
    name: &str,
    cpu_thresholds: (f64, f64),     // (scale_up, scale_down)
    memory_thresholds: (f64, f64),   // (scale_up, scale_down)
    request_thresholds: (f64, f64),  // (scale_up, scale_down)
    cooldown_seconds: u64,
) -> CompositePolicy {
    let cpu_policy = cpu_scaling_policy(
        cpu_thresholds.0, 
        cpu_thresholds.1, 
        1.5, 
        cooldown_seconds
    );
    
    let memory_policy = memory_scaling_policy(
        memory_thresholds.0, 
        memory_thresholds.1, 
        1.4, 
        cooldown_seconds
    );
    
    let request_policy = request_rate_scaling_policy(
        request_thresholds.0, 
        request_thresholds.1, 
        1.6, 
        cooldown_seconds / 2 // Faster response to request spikes
    );
    
    // Use weighted voting: CPU and memory are more important than request rate
    composite_weighted_policy(
        name,
        vec![cpu_policy, memory_policy, request_policy],
        vec![0.4, 0.4, 0.2], // 40% CPU, 40% memory, 20% requests
    )
}

/// Utility functions for policy evaluation
pub mod evaluation {
    #[allow(unused_imports)]
    use crate::types::{CompositePolicy, CompositeLogic, ResourceMetrics, ScaleAction};
    #[allow(unused_imports)]
    use crate::callbacks::CallbackContext;
    
    #[cfg(feature = "time-utils")]
    use crate::types::{TimeBasedPolicy, TimeSchedule};
    #[cfg(feature = "time-utils")]
    use chrono::{DateTime, Utc, NaiveTime, Weekday, Datelike};
    
    /// Check if a time schedule is currently active
    #[cfg(feature = "time-utils")]
    pub fn is_schedule_active(schedule: &TimeSchedule) -> bool {
        let now = chrono::Utc::now();
        
        // Check if current day of week is in the schedule
        let weekday = now.weekday().num_days_from_sunday() as u8;
        if !schedule.days_of_week.contains(&weekday) {
            return false;
        }
        
        // Parse time strings and check if current time is within range
        let start_time = NaiveTime::parse_from_str(&schedule.start_time, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let end_time = NaiveTime::parse_from_str(&schedule.end_time, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        
        let current_time = now.time();
        
        // Handle cases where end time is on next day (e.g., 22:00 - 06:00)
        if start_time <= end_time {
            current_time >= start_time && current_time <= end_time
        } else {
            current_time >= start_time || current_time <= end_time
        }
    }
}
