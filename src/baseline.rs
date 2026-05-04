//! Environmental baselines and the `baseline.compare` tool.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use rig_compose::{KernelError, Tool, ToolSchema};

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("baseline `{entity}/{metric}` not found")]
    NotFound { entity: String, metric: String },
}

/// Statistical envelope for one (entity, metric) pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBaseline {
    pub entity: String,
    pub metric: String,
    pub mean: f64,
    pub std_dev: f64,
    pub samples: u64,
}

impl EntityBaseline {
    pub fn within(&self, value: f64, k: f64) -> bool {
        let bound = (k * self.std_dev).max(f64::EPSILON);
        (value - self.mean).abs() <= bound
    }
}

#[async_trait]
pub trait BaselineStore: Send + Sync {
    async fn put(&self, baseline: EntityBaseline) -> Result<(), BaselineError>;
    async fn get(&self, entity: &str, metric: &str) -> Result<EntityBaseline, BaselineError>;
    async fn contains(&self, entity: &str, metric: &str) -> bool;
}

#[derive(Clone, Default)]
pub struct InMemoryBaselineStore {
    inner: Arc<RwLock<HashMap<(String, String), EntityBaseline>>>,
}

impl InMemoryBaselineStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }
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
    pub const NAME: &'static str = "baseline.compare";

    pub fn new(store: Arc<dyn BaselineStore>) -> Self {
        Self { store }
    }

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
}
