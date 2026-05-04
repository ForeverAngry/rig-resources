//! Kernel-facing graph tool.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use super::store::{GraphEdge, GraphStore};
use rig_compose::{KernelError, Tool, ToolSchema};

pub struct GraphTool {
    store: Arc<dyn GraphStore>,
}

impl GraphTool {
    pub const NAME: &'static str = "graph.entity";

    pub fn new(store: Arc<dyn GraphStore>) -> Self {
        Self { store }
    }

    pub fn arc(store: Arc<dyn GraphStore>) -> Arc<dyn Tool> {
        Arc::new(Self::new(store))
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum GraphOp {
    Upsert {
        src: String,
        dst: String,
        kind: String,
    },
    Expand {
        entity: String,
        #[serde(default = "default_depth")]
        depth: usize,
    },
    Centrality {
        entity: String,
    },
}

fn default_depth() -> usize {
    2
}

#[async_trait]
impl Tool for GraphTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: Self::NAME.into(),
            description:
                "Selective entity-graph operations: upsert an edge, expand a neighbourhood, or query degree centrality."
                    .into(),
            args_schema: json!({
                "type": "object",
                "required": ["op"],
                "properties": {
                    "op": {"type": "string", "enum": ["upsert", "expand", "centrality"]},
                    "src": {"type": "string"},
                    "dst": {"type": "string"},
                    "kind": {"type": "string"},
                    "entity": {"type": "string"},
                    "depth": {"type": "integer", "minimum": 0}
                }
            }),
            result_schema: json!({"type": "object"}),
        }
    }

    fn name(&self) -> rig_compose::tool::ToolName {
        Self::NAME.to_string()
    }

    async fn invoke(&self, args: Value) -> Result<Value, KernelError> {
        let op: GraphOp = serde_json::from_value(args)?;
        match op {
            GraphOp::Upsert { src, dst, kind } => {
                self.store
                    .upsert_edge(GraphEdge::new(src, dst, kind))
                    .await
                    .map_err(|err| KernelError::ToolFailed(err.to_string()))?;
                Ok(json!({"ok": true}))
            }
            GraphOp::Expand { entity, depth } => {
                let subgraph = self
                    .store
                    .expand(&entity, depth)
                    .await
                    .map_err(|err| KernelError::ToolFailed(err.to_string()))?;
                Ok(serde_json::to_value(subgraph)?)
            }
            GraphOp::Centrality { entity } => {
                let centrality = self.store.centrality(&entity).await;
                Ok(json!({"entity": entity, "centrality": centrality}))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::InMemoryGraph;

    #[tokio::test]
    async fn tool_upsert_then_expand() {
        let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraph::new());
        let tool = GraphTool::new(graph);
        tool.invoke(json!({"op": "upsert", "src": "a", "dst": "b", "kind": "auth"}))
            .await
            .unwrap();
        let out = tool
            .invoke(json!({"op": "expand", "entity": "a", "depth": 1}))
            .await
            .unwrap();
        assert!(
            out["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node == "b")
        );
    }
}
