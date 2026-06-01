//! Environmental baselines and the `baseline.compare` tool.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use rig_compose::{KernelError, Tool, ToolSchema};

use crate::trace::ResourceTraceEnvelope;

const TRACE_RESOURCE: &str = "baseline";
const TRACE_OPERATION: &str = "compare";
const TRACE_KIND: &str = "baseline_compare";

/// Reason emitted when no baseline existed for the requested
/// `(entity, metric)` pair.
pub const TRACE_REASON_NOT_FOUND: &str = "baseline_not_found";
/// Reason emitted when the observation fell inside the `mean ± k·σ` bound.
pub const TRACE_REASON_WITHIN_BOUNDS: &str = "within_bounds";
/// Reason emitted when the observation fell outside the `mean ± k·σ` bound.
pub const TRACE_REASON_EXCEEDS_BOUNDS: &str = "exceeds_bounds";

/// Errors returned by baseline stores.
#[derive(Debug, Error)]
pub enum BaselineError {
    /// No baseline exists for an entity/metric pair.
    #[error("baseline `{entity}/{metric}` not found")]
    NotFound {
        /// Entity identifier used for lookup.
        entity: String,
        /// Metric identifier used for lookup.
        metric: String,
    },
}

/// Statistical envelope for one (entity, metric) pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBaseline {
    /// Entity identifier (host, user, service, etc.).
    pub entity: String,
    /// Metric name represented by this baseline.
    pub metric: String,
    /// Observed mean.
    pub mean: f64,
    /// Sample standard deviation.
    pub std_dev: f64,
    /// Number of observations used to build the baseline.
    pub samples: u64,
}

impl EntityBaseline {
    /// Build a baseline envelope from online statistics.
    pub fn from_stats(
        entity: impl Into<String>,
        metric: impl Into<String>,
        stats: &OnlineStats,
    ) -> Self {
        Self {
            entity: entity.into(),
            metric: metric.into(),
            mean: stats.mean(),
            std_dev: stats.std_dev(),
            samples: stats.count(),
        }
    }

    /// Return `true` when `value` falls within `mean ± k * std_dev`.
    pub fn within(&self, value: f64, k: f64) -> bool {
        let bound = (k * self.std_dev).max(f64::EPSILON);
        (value - self.mean).abs() <= bound
    }
}

/// Online mean/variance accumulator for building [`EntityBaseline`] values.
///
/// Uses Welford's algorithm, so callers can update an environmental baseline
/// one observation at a time without storing raw samples.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OnlineStats {
    count: u64,
    mean: f64,
    m2: f64,
}

impl OnlineStats {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one sample to the accumulator.
    pub fn push(&mut self, value: f64) {
        self.count = self.count.saturating_add(1);
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    /// Number of samples observed.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Whether no samples have been observed.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Current mean, or `0.0` before the first sample.
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Sample variance. Returns `0.0` until at least two samples exist.
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    /// Sample standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Convert the accumulated stats into an [`EntityBaseline`].
    pub fn to_baseline(
        &self,
        entity: impl Into<String>,
        metric: impl Into<String>,
    ) -> EntityBaseline {
        EntityBaseline::from_stats(entity, metric, self)
    }
}

/// Storage contract for entity/metric baselines.
#[async_trait]
pub trait BaselineStore: Send + Sync {
    /// Insert or replace a baseline.
    async fn put(&self, baseline: EntityBaseline) -> Result<(), BaselineError>;
    /// Fetch one baseline by entity and metric.
    async fn get(&self, entity: &str, metric: &str) -> Result<EntityBaseline, BaselineError>;
    /// Return `true` when a baseline exists for entity and metric.
    async fn contains(&self, entity: &str, metric: &str) -> bool;
}

/// In-memory baseline store for tests, examples, and single-process agents.
#[derive(Clone, Default)]
pub struct InMemoryBaselineStore {
    inner: Arc<RwLock<HashMap<(String, String), EntityBaseline>>>,
}

impl InMemoryBaselineStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }
    /// Create an empty store wrapped in [`Arc`].
    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
    /// Number of baselines stored.
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }
    /// Whether the store contains no baselines.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

#[async_trait]
impl BaselineStore for InMemoryBaselineStore {
    async fn put(&self, baseline: EntityBaseline) -> Result<(), BaselineError> {
        self.inner
            .write()
            .insert((baseline.entity.clone(), baseline.metric.clone()), baseline);
        Ok(())
    }
    async fn get(&self, entity: &str, metric: &str) -> Result<EntityBaseline, BaselineError> {
        self.inner
            .read()
            .get(&(entity.to_string(), metric.to_string()))
            .cloned()
            .ok_or_else(|| BaselineError::NotFound {
                entity: entity.to_string(),
                metric: metric.to_string(),
            })
    }
    async fn contains(&self, entity: &str, metric: &str) -> bool {
        self.inner
            .read()
            .contains_key(&(entity.to_string(), metric.to_string()))
    }
}

/// `baseline.compare` — kernel tool.
pub struct BaselineCompareTool {
    store: Arc<dyn BaselineStore>,
}

impl BaselineCompareTool {
    /// Canonical tool name registered with `rig-compose`.
    pub const NAME: &'static str = "baseline.compare";

    /// Build a tool backed by `store`.
    pub fn new(store: Arc<dyn BaselineStore>) -> Self {
        Self { store }
    }

    /// Build a trait-object handle suitable for direct registry insertion.
    pub fn arc(store: Arc<dyn BaselineStore>) -> Arc<dyn Tool> {
        Arc::new(Self::new(store))
    }
}

#[async_trait]
impl Tool for BaselineCompareTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: Self::NAME.into(),
            description:
                "Compare an observed value to the entity's baseline (mean +/- k*sigma). Returns availability and within-bound flags."
                    .into(),
            args_schema: json!({
                "type": "object",
                "required": ["entity", "metric", "value"],
                "properties": {
                    "entity": {"type": "string"},
                    "metric": {"type": "string"},
                    "value": {"type": "number"},
                    "k": {"type": "number", "default": 2.0}
                }
            }),
            result_schema: json!({"type": "object"}),
        }
    }

    fn name(&self) -> rig_compose::tool::ToolName {
        Self::NAME.to_string()
    }

    async fn invoke(&self, args: Value) -> Result<Value, KernelError> {
        #[derive(serde::Deserialize)]
        struct Args {
            entity: String,
            metric: String,
            value: f64,
            #[serde(default = "default_k")]
            k: f64,
        }
        fn default_k() -> f64 {
            2.0
        }
        let parsed: Args = serde_json::from_value(args)?;
        match self.store.get(&parsed.entity, &parsed.metric).await {
            Ok(baseline) => Ok(json!({
                "available": true,
                "within": baseline.within(parsed.value, parsed.k),
                "mean": baseline.mean,
                "std_dev": baseline.std_dev,
                "k": parsed.k,
            })),
            Err(_) => Ok(json!({
                "available": false,
                "within": false,
                "k": parsed.k,
            })),
        }
    }
}

/// Build a [`ResourceTraceEnvelope`] describing a single `baseline.compare`
/// evaluation.
///
/// Pass `baseline` as `Some(&EntityBaseline)` when the store had a record
/// for the `(entity, metric)` pair, or `None` to record a not-available
/// comparison. The envelope mirrors the structure of
/// [`crate::security_finding_trace_envelope`] and
/// [`crate::memory_lookup_trace_envelope`] so audit and observability
/// pipelines can route all three with one shape.
///
/// Reason codes:
/// * `None` → [`TRACE_REASON_NOT_FOUND`]
/// * `Some(_)` and inside `mean ± k·σ` → [`TRACE_REASON_WITHIN_BOUNDS`]
/// * `Some(_)` and outside the bound → [`TRACE_REASON_EXCEEDS_BOUNDS`]
///
/// ```no_run
/// use rig_resources::{EntityBaseline, baseline_compare_trace_envelope};
///
/// let baseline = EntityBaseline {
///     entity: "host-1".into(),
///     metric: "fanout".into(),
///     mean: 10.0,
///     std_dev: 2.0,
///     samples: 100,
/// };
/// let envelope =
///     baseline_compare_trace_envelope("host-1", "fanout", 11.0, 2.0, Some(&baseline));
/// assert_eq!(envelope.resource, "baseline");
/// assert_eq!(envelope.output_summary["within"], true);
/// ```
#[must_use]
pub fn baseline_compare_trace_envelope(
    entity: &str,
    metric: &str,
    observed: f64,
    k: f64,
    baseline: Option<&EntityBaseline>,
) -> ResourceTraceEnvelope {
    let input = json!({
        "entity": entity,
        "metric": metric,
        "observed_value": observed,
        "k": k,
    });

    let mut envelope = ResourceTraceEnvelope::new(TRACE_RESOURCE, TRACE_OPERATION, TRACE_KIND)
        .with_input_summary(input);

    match baseline {
        None => {
            envelope = envelope
                .with_output_summary(json!({
                    "available": false,
                    "within": false,
                }))
                .with_reason(TRACE_REASON_NOT_FOUND);
        }
        Some(baseline) => {
            let within = baseline.within(observed, k);
            let bound = (k * baseline.std_dev).max(f64::EPSILON);
            let deviation = (observed - baseline.mean).abs();
            envelope = envelope
                .with_output_summary(json!({
                    "available": true,
                    "within": within,
                    "mean": baseline.mean,
                    "std_dev": baseline.std_dev,
                    "bound": bound,
                    "deviation": deviation,
                }))
                .with_reason(if within {
                    TRACE_REASON_WITHIN_BOUNDS
                } else {
                    TRACE_REASON_EXCEEDS_BOUNDS
                });

            let mut metadata = json!({
                "samples": baseline.samples,
            });
            if baseline.std_dev > f64::EPSILON
                && let Some(map) = metadata.as_object_mut()
                && let Some(z) =
                    serde_json::Number::from_f64((observed - baseline.mean) / baseline.std_dev)
            {
                map.insert("z_score".into(), Value::Number(z));
            }
            envelope = envelope.with_metadata(metadata);
        }
    }

    envelope
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(entity: &str, metric: &str, mean: f64, sd: f64) -> EntityBaseline {
        EntityBaseline {
            entity: entity.into(),
            metric: metric.into(),
            mean,
            std_dev: sd,
            samples: 100,
        }
    }

    #[tokio::test]
    async fn within_bounds_check() {
        let b = baseline("e", "fanout", 10.0, 2.0);
        assert!(b.within(11.0, 2.0));
        assert!(!b.within(20.0, 2.0));
    }

    #[test]
    fn online_stats_builds_entity_baseline() {
        let mut stats = OnlineStats::new();
        for value in [2.0_f64, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0] {
            stats.push(value);
        }
        let baseline = stats.to_baseline("host", "bytes");
        assert_eq!(baseline.samples, 8);
        assert!((baseline.mean - 5.0).abs() < 1e-12);
        assert!((baseline.std_dev - 4.571_428_571_428_f64.sqrt()).abs() < 1e-12);
    }

    #[tokio::test]
    async fn store_put_then_get() {
        let store = InMemoryBaselineStore::new();
        store.put(baseline("e", "m", 5.0, 1.0)).await.unwrap();
        let got = store.get("e", "m").await.unwrap();
        assert_eq!(got.samples, 100);
        assert!(store.contains("e", "m").await);
    }

    #[tokio::test]
    async fn tool_reports_available_and_within() {
        let store: Arc<dyn BaselineStore> = Arc::new(InMemoryBaselineStore::new());
        store.put(baseline("e", "m", 100.0, 5.0)).await.unwrap();
        let tool = BaselineCompareTool::new(store);
        let out = tool
            .invoke(json!({"entity": "e", "metric": "m", "value": 102.0, "k": 2.0}))
            .await
            .unwrap();
        assert_eq!(out["available"], true);
        assert_eq!(out["within"], true);
    }

    #[test]
    fn trace_envelope_within_bounds_includes_metadata() {
        let b = baseline("host-1", "fanout", 10.0, 2.0);
        let envelope = baseline_compare_trace_envelope("host-1", "fanout", 11.0, 2.0, Some(&b));

        assert_eq!(envelope.version, ResourceTraceEnvelope::VERSION);
        assert_eq!(envelope.resource, "baseline");
        assert_eq!(envelope.operation, "compare");
        assert_eq!(envelope.trace_kind, "baseline_compare");
        assert_eq!(envelope.input_summary["entity"], "host-1");
        assert_eq!(envelope.input_summary["metric"], "fanout");
        let observed = envelope.input_summary["observed_value"].as_f64().unwrap();
        assert!((observed - 11.0).abs() < 1e-9);
        assert_eq!(envelope.output_summary["available"], true);
        assert_eq!(envelope.output_summary["within"], true);
        let mean = envelope.output_summary["mean"].as_f64().unwrap();
        assert!((mean - 10.0).abs() < 1e-9);
        let bound = envelope.output_summary["bound"].as_f64().unwrap();
        assert!((bound - 4.0).abs() < 1e-9);
        assert_eq!(envelope.reason.as_deref(), Some(TRACE_REASON_WITHIN_BOUNDS));
        assert_eq!(envelope.metadata["samples"], 100);
        let z = envelope.metadata["z_score"].as_f64().unwrap();
        assert!((z - 0.5).abs() < 1e-9);
    }

    #[test]
    fn trace_envelope_exceeds_bounds_sets_reason() {
        let b = baseline("host-1", "fanout", 10.0, 2.0);
        let envelope = baseline_compare_trace_envelope("host-1", "fanout", 20.0, 2.0, Some(&b));
        assert_eq!(envelope.output_summary["within"], false);
        assert_eq!(
            envelope.reason.as_deref(),
            Some(TRACE_REASON_EXCEEDS_BOUNDS)
        );
        let deviation = envelope.output_summary["deviation"].as_f64().unwrap();
        assert!((deviation - 10.0).abs() < 1e-9);
    }

    #[test]
    fn trace_envelope_not_found_omits_baseline_fields() {
        let envelope = baseline_compare_trace_envelope("ghost", "metric", 7.0, 2.0, None);
        assert_eq!(envelope.output_summary["available"], false);
        assert_eq!(envelope.output_summary["within"], false);
        assert!(envelope.output_summary.get("mean").is_none());
        assert_eq!(envelope.reason.as_deref(), Some(TRACE_REASON_NOT_FOUND));
        assert!(envelope.metadata.is_null());
    }
}
