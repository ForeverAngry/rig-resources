//! Entity graph storage trait and data types.

use std::time::SystemTime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("graph entity `{0}` not found")]
    NotFound(String),
}

/// Directed edge `src --kind--> dst` observed at `ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub src: String,
    pub dst: String,
    pub kind: String,
    #[serde(with = "ts_serde")]
    pub ts: SystemTime,
}

impl GraphEdge {
    pub fn new(src: impl Into<String>, dst: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            dst: dst.into(),
            kind: kind.into(),
            ts: SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subgraph {
    pub seed: String,
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphError>;
    async fn expand(&self, entity: &str, depth: usize) -> Result<Subgraph, GraphError>;
    async fn centrality(&self, entity: &str) -> f64;
}

mod ts_serde {
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
        let secs = time
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0);
        serializer.serialize_f64(secs)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        let secs = f64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs_f64(secs.max(0.0)))
    }
}
