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

use crate::trace::ResourceTraceEnvelope;

const TRACE_RESOURCE: &str = "memory";
const TRACE_OPERATION: &str = "lookup";
const TRACE_KIND: &str = "memory_lookup";

/// Reason code emitted on the [`ResourceTraceEnvelope`] when a lookup
/// returned zero hits.
pub const TRACE_REASON_NO_HITS: &str = "no_hits";
/// Reason code emitted when the backing [`MemoryLookupStore`] failed.
pub const TRACE_REASON_BACKEND_ERROR: &str = "backend_error";

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

/// Build a [`ResourceTraceEnvelope`] describing a single `memory.lookup`
/// invocation.
///
/// This complements [`crate::memory_hit_to_context_item`] (the prompt-side
/// projection) by giving observability and audit consumers a trace-side
/// record of the query, scope, hit count, and top match. The envelope is
/// shaped to mirror [`crate::security_finding_trace_envelope`] so the same
/// downstream pipelines can route both kinds without bespoke shapes.
///
/// `principal` and `scope` are optional caller-provided context (typically
/// the calling agent's tenant or workspace, which the store may or may not
/// have echoed back on each hit). When `hits` is empty the envelope carries
/// the [`TRACE_REASON_NO_HITS`] reason code.
///
/// ```no_run
/// use rig_resources::{MemoryLookupHit, memory_lookup_trace_envelope};
///
/// let hits = vec![MemoryLookupHit::new(0.82, "matched episode").with_key("ep-7")];
/// let envelope = memory_lookup_trace_envelope("beacon", 3, &hits, Some("alice"), None);
/// assert_eq!(envelope.resource, "memory");
/// assert_eq!(envelope.output_summary["hit_count"], 1);
/// ```
#[must_use]
pub fn memory_lookup_trace_envelope(
    query: &str,
    k: usize,
    hits: &[MemoryLookupHit],
    principal: Option<&str>,
    scope: Option<&str>,
) -> ResourceTraceEnvelope {
    let mut input = json!({
        "query": query,
        "k": k,
    });
    if let Some(map) = input.as_object_mut() {
        if let Some(principal) = principal {
            map.insert("principal".into(), Value::String(principal.to_string()));
        }
        if let Some(scope) = scope {
            map.insert("scope".into(), Value::String(scope.to_string()));
        }
    }

    let mut output = json!({
        "hit_count": hits.len(),
    });
    if let (Some(top), Some(map)) = (hits.first(), output.as_object_mut()) {
        if let Some(score) = serde_json::Number::from_f64(top.score as f64) {
            map.insert("top_score".into(), Value::Number(score));
        }
        if let Some(key) = &top.key {
            map.insert("top_key".into(), Value::String(key.clone()));
        }
    }

    let mut envelope = ResourceTraceEnvelope::new(TRACE_RESOURCE, TRACE_OPERATION, TRACE_KIND)
        .with_input_summary(input)
        .with_output_summary(output);

    if hits.is_empty() {
        envelope = envelope.with_reason(TRACE_REASON_NO_HITS);
    }

    if let Some(top) = hits.first() {
        let mut metadata = serde_json::Map::new();
        if let Some(source_uri) = &top.source_uri {
            metadata.insert("source_uri".into(), Value::String(source_uri.clone()));
        }
        if let Some(recorded_at_millis) = top.recorded_at_millis {
            metadata.insert(
                "recorded_at_millis".into(),
                Value::Number(serde_json::Number::from(recorded_at_millis)),
            );
        }
        if let Some(top_principal) = &top.principal
            && principal.is_none_or(|p| p != top_principal)
        {
            metadata.insert("top_principal".into(), Value::String(top_principal.clone()));
        }
        if let Some(top_scope) = &top.scope
            && scope.is_none_or(|s| s != top_scope)
        {
            metadata.insert("top_scope".into(), Value::String(top_scope.clone()));
        }
        if !metadata.is_empty() {
            envelope = envelope.with_metadata(Value::Object(metadata));
        }
    }

    envelope
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

    #[test]
    fn trace_envelope_summarises_hits_and_metadata() {
        let hits = vec![
            MemoryLookupHit::new(0.91, "top hit")
                .with_key("ep-1")
                .with_source_uri("memory://ep/1")
                .with_recorded_at_millis(1_700_000_000_000)
                .with_principal("alice")
                .with_scope("workspace"),
            MemoryLookupHit::new(0.42, "runner up").with_key("ep-2"),
        ];

        let envelope =
            memory_lookup_trace_envelope("beacon", 3, &hits, Some("alice"), Some("workspace"));

        assert_eq!(envelope.version, ResourceTraceEnvelope::VERSION);
        assert_eq!(envelope.resource, "memory");
        assert_eq!(envelope.operation, "lookup");
        assert_eq!(envelope.trace_kind, "memory_lookup");
        assert_eq!(envelope.input_summary["query"], "beacon");
        assert_eq!(envelope.input_summary["k"], 3);
        assert_eq!(envelope.input_summary["principal"], "alice");
        assert_eq!(envelope.input_summary["scope"], "workspace");
        assert_eq!(envelope.output_summary["hit_count"], 2);
        let top_score = envelope.output_summary["top_score"].as_f64().unwrap();
        assert!((top_score - 0.91).abs() < 1e-6);
        assert_eq!(envelope.output_summary["top_key"], "ep-1");
        assert!(envelope.reason.is_none());
        assert_eq!(envelope.metadata["source_uri"], "memory://ep/1");
        assert_eq!(
            envelope.metadata["recorded_at_millis"],
            1_700_000_000_000_i64
        );
        // Caller-supplied principal/scope matches the top hit, so they are
        // not echoed into metadata.
        assert!(envelope.metadata.get("top_principal").is_none());
        assert!(envelope.metadata.get("top_scope").is_none());
    }

    #[test]
    fn trace_envelope_emits_no_hits_reason_when_empty() {
        let envelope = memory_lookup_trace_envelope("nothing", 5, &[], None, None);
        assert_eq!(envelope.output_summary["hit_count"], 0);
        assert!(envelope.output_summary.get("top_score").is_none());
        assert_eq!(envelope.reason.as_deref(), Some(TRACE_REASON_NO_HITS));
        assert!(envelope.metadata.is_null());
        assert!(envelope.input_summary.get("principal").is_none());
    }

    #[test]
    fn trace_envelope_records_mismatched_top_principal_scope() {
        let hits = vec![
            MemoryLookupHit::new(0.5, "cross-tenant")
                .with_key("ep-9")
                .with_principal("bob")
                .with_scope("other"),
        ];

        let envelope =
            memory_lookup_trace_envelope("q", 1, &hits, Some("alice"), Some("workspace"));

        assert_eq!(envelope.metadata["top_principal"], "bob");
        assert_eq!(envelope.metadata["top_scope"], "other");
    }
}
