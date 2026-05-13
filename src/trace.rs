//! Local resource trace envelopes for evidence metadata.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Machine-readable trace envelope for resource-side decisions.
///
/// This is intentionally local to `rig-resources`. It proves a stable shape
/// for graph, security, baseline, and memory-resource metadata before any
/// trace API is promoted into the `rig-compose` kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceTraceEnvelope {
    /// Trace shape version.
    pub version: u32,
    /// Category such as `graph`, `security`, `baseline`, or `memory`.
    pub resource: String,
    /// Specific operation performed by the resource.
    pub operation: String,
    /// Machine-readable trace kind such as `graph_expansion`.
    pub trace_kind: String,
    /// Compact, non-secret input summary.
    pub input_summary: Value,
    /// Compact output summary.
    pub output_summary: Value,
    /// Optional reason code for skip, suppress, deny, or not-applicable paths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional resource-specific metadata.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

impl ResourceTraceEnvelope {
    /// Current envelope version.
    pub const VERSION: u32 = 1;

    /// Create a trace envelope with empty summaries.
    #[must_use]
    pub fn new(
        resource: impl Into<String>,
        operation: impl Into<String>,
        trace_kind: impl Into<String>,
    ) -> Self {
        Self {
            version: Self::VERSION,
            resource: resource.into(),
            operation: operation.into(),
            trace_kind: trace_kind.into(),
            input_summary: Value::Null,
            output_summary: Value::Null,
            reason: None,
            metadata: Value::Null,
        }
    }

    /// Attach the input summary.
    #[must_use]
    pub fn with_input_summary(mut self, input_summary: Value) -> Self {
        self.input_summary = input_summary;
        self
    }

    /// Attach the output summary.
    #[must_use]
    pub fn with_output_summary(mut self, output_summary: Value) -> Self {
        self.output_summary = output_summary;
        self
    }

    /// Attach a machine-readable reason code.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Attach resource-specific metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Convert the trace envelope into JSON metadata.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_envelope_round_trips_as_json() {
        let trace = ResourceTraceEnvelope::new("graph", "expand", "graph_expansion")
            .with_input_summary(json!({"entity": "host-1"}))
            .with_output_summary(json!({"distinct_neighbours": 4}))
            .with_reason("threshold_exceeded");

        let value = trace.to_value();
        let decoded: ResourceTraceEnvelope = serde_json::from_value(value).unwrap();

        assert_eq!(decoded.version, ResourceTraceEnvelope::VERSION);
        assert_eq!(decoded.resource, "graph");
        assert_eq!(decoded.reason.as_deref(), Some("threshold_exceeded"));
    }
}
