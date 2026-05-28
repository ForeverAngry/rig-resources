//! Memory lookup tool contract.
//!
//! [`MemoryPivotSkill`](crate::MemoryPivotSkill) calls a tool named
//! `memory.lookup`. This module supplies the canonical tool and a small
//! backend trait so stores such as `rig-memvid`, test fakes, or
//! application-specific episode stores can expose the same lookup shape
//! without depending on each other.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use rig_compose::{KernelError, Tool, ToolSchema};

/// Error returned by a [`MemoryLookupStore`].
#[derive(Debug, Error)]
pub enum MemoryLookupError {
    /// The backing memory store failed.
    #[error("memory lookup backend error: {0}")]
    Backend(String),
}

/// One hit returned by a [`MemoryLookupStore`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLookupHit {
    /// Retrieval score in `[0, 1]`; higher is more similar.
    pub score: f32,
    /// Short text summary suitable for evidence display.
    pub summary: String,
    /// Optional stable store key, frame id, or episode id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Optional URI or locator for the backing memory source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// Optional principal, actor, tenant, or subject associated with the hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// Optional caller-defined lookup scope such as tenant, workspace, or
    /// profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Optional milliseconds since the Unix epoch when the source was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at_millis: Option<i64>,
    /// Optional store-specific metadata.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

impl MemoryLookupHit {
    /// Create a hit with no key or metadata.
    pub fn new(score: f32, summary: impl Into<String>) -> Self {
        Self {
            score,
            summary: summary.into(),
            key: None,
            source_uri: None,
            principal: None,
            scope: None,
            recorded_at_millis: None,
            metadata: Value::Null,
        }
    }

    /// Attach a stable storage key.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Attach a source URI or locator.
    pub fn with_source_uri(mut self, source_uri: impl Into<String>) -> Self {
        self.source_uri = Some(source_uri.into());
        self
    }

    /// Attach the principal, actor, tenant, or subject associated with the hit.
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Attach the caller-defined lookup scope.
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Attach the source record timestamp in milliseconds since the Unix epoch.
    pub fn with_recorded_at_millis(mut self, recorded_at_millis: i64) -> Self {
        self.recorded_at_millis = Some(recorded_at_millis);
        self
    }

    /// Attach store-specific metadata.
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Backend contract for the canonical `memory.lookup` tool.
#[async_trait]
pub trait MemoryLookupStore: Send + Sync {
    /// Return up to `k` hits most relevant to `query`.
    async fn lookup(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<MemoryLookupHit>, MemoryLookupError>;
}

/// `memory.lookup` — reusable kernel tool for semantic or lexical memory pivots.
pub struct MemoryLookupTool {
    store: Arc<dyn MemoryLookupStore>,
}

impl MemoryLookupTool {
    /// Stable tool name consumed by [`crate::MemoryPivotSkill`].
    pub const NAME: &'static str = "memory.lookup";

    /// Create a lookup tool backed by `store`.
    pub fn new(store: Arc<dyn MemoryLookupStore>) -> Self {
        Self { store }
    }

    /// Create the tool behind an [`Arc`] for registration in a `ToolRegistry`.
    pub fn arc(store: Arc<dyn MemoryLookupStore>) -> Arc<dyn Tool> {
        Arc::new(Self::new(store))
    }
}

#[derive(Deserialize)]
struct LookupArgs {
    query: String,
    #[serde(default = "default_k")]
    k: usize,
}

fn default_k() -> usize {
    3
}

#[async_trait]
impl Tool for MemoryLookupTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: Self::NAME.into(),
            description: "Retrieve up to k similar memory episodes for a query.".into(),
            args_schema: json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {"type": "string"},
                    "k": {"type": "integer", "minimum": 1, "default": 3}
                }
            }),
            result_schema: json!({
                "type": "object",
                "properties": {
                    "hits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "score": {"type": "number"},
                                "summary": {"type": "string"},
                                "key": {"type": "string"},
                                "source_uri": {"type": "string"},
                                "principal": {"type": "string"},
                                "scope": {"type": "string"},
                                "recorded_at_millis": {"type": "integer"},
                                "metadata": {"type": "object"}
                            }
                        }
                    }
                }
            }),
        }
    }

    fn name(&self) -> rig_compose::tool::ToolName {
        Self::NAME.to_string()
    }

    async fn invoke(&self, args: Value) -> Result<Value, KernelError> {
        let parsed: LookupArgs = serde_json::from_value(args)?;
        if parsed.k == 0 {
            return Err(KernelError::InvalidArgument(
                "memory.lookup requires k >= 1".into(),
            ));
        }
        let hits = self
            .store
            .lookup(&parsed.query, parsed.k)
            .await
            .map_err(|err| KernelError::ToolFailed(err.to_string()))?;
        Ok(json!({ "hits": hits }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubMemory;

    #[async_trait]
    impl MemoryLookupStore for StubMemory {
        async fn lookup(
            &self,
            query: &str,
            k: usize,
        ) -> Result<Vec<MemoryLookupHit>, MemoryLookupError> {
            Ok(vec![
                MemoryLookupHit::new(0.9, format!("matched {query}"))
                    .with_key("ep-1")
                    .with_metadata(json!({"rank": 1})),
            ]
            .into_iter()
            .take(k)
            .collect())
        }
    }

    #[tokio::test]
    async fn lookup_tool_returns_hits() {
        let tool = MemoryLookupTool::new(Arc::new(StubMemory));
        let out = tool
            .invoke(json!({"query": "beacon", "k": 1}))
            .await
            .unwrap();
        let score = out["hits"][0]["score"].as_f64().unwrap();
        assert!((score - 0.9).abs() < 1e-6);
        assert_eq!(out["hits"][0]["key"], "ep-1");
    }

    #[test]
    fn lookup_hit_serializes_shared_metadata() {
        let hit = MemoryLookupHit::new(0.75, "matched episode")
            .with_key("ep-7")
            .with_source_uri("memory://episode/7")
            .with_principal("alice")
            .with_scope("workspace")
            .with_recorded_at_millis(1_700_000_000_000);

        let json = serde_json::to_value(hit).unwrap();

        assert_eq!(json["key"], "ep-7");
        assert_eq!(json["source_uri"], "memory://episode/7");
        assert_eq!(json["principal"], "alice");
        assert_eq!(json["scope"], "workspace");
        assert_eq!(json["recorded_at_millis"], 1_700_000_000_000_i64);
    }

    #[tokio::test]
    async fn lookup_tool_rejects_zero_k() {
        let tool = MemoryLookupTool::new(Arc::new(StubMemory));
        let err = tool
            .invoke(json!({"query": "beacon", "k": 0}))
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::InvalidArgument(_)));
    }
}
