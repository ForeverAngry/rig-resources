//! Entity graph storage trait and data types.

use std::time::SystemTime;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors returned by graph stores.
#[derive(Debug, Error)]
pub enum GraphError {
    /// The requested entity does not exist in the graph.
    #[error("graph entity `{0}` not found")]
    NotFound(String),
}

/// Directed edge `src --kind--> dst` observed at `ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source entity id.
    pub src: String,
    /// Destination entity id.
    pub dst: String,
    /// Edge relation kind.
    pub kind: String,
    /// Observation timestamp.
    #[serde(with = "ts_serde")]
    pub ts: SystemTime,
}

impl GraphEdge {
    /// Build a timestamped directed edge observed now.
    pub fn new(src: impl Into<String>, dst: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            dst: dst.into(),
            kind: kind.into(),
            ts: SystemTime::now(),
        }
    }
}

/// Bounded graph expansion result around a seed entity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Subgraph {
    /// Seed entity used for expansion.
    pub seed: String,
    /// Entity ids included in the expansion.
    pub nodes: Vec<String>,
    /// Directed edges included in the expansion.
    pub edges: Vec<GraphEdge>,
}

/// Storage contract for selective entity graph operations.
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Insert or replace a directed edge.
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphError>;
    /// Expand from `entity` up to `depth` hops.
    async fn expand(&self, entity: &str, depth: usize) -> Result<Subgraph, GraphError>;
    /// Return normalized out-degree centrality for `entity`.
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
