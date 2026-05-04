//! In-memory [`GraphStore`] backed by `petgraph::DiGraph`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use super::store::{GraphEdge, GraphError, GraphStore, Subgraph};

#[derive(Default)]
struct GraphState {
    graph: DiGraph<String, GraphEdge>,
    index: HashMap<String, NodeIndex>,
}

impl GraphState {
    fn ensure_node(&mut self, name: &str) -> NodeIndex {
        if let Some(&node) = self.index.get(name) {
            return node;
        }
        let node = self.graph.add_node(name.to_string());
        self.index.insert(name.to_string(), node);
        node
    }

    fn upsert(&mut self, edge: GraphEdge) {
        let src = self.ensure_node(&edge.src);
        let dst = self.ensure_node(&edge.dst);
        let existing = self
            .graph
            .edges_connecting(src, dst)
            .find(|candidate| candidate.weight().kind == edge.kind)
            .map(|candidate| candidate.id());
        if let Some(id) = existing {
            if let Some(weight) = self.graph.edge_weight_mut(id) {
                *weight = edge;
            }
        } else {
            self.graph.add_edge(src, dst, edge);
        }
    }

    fn out_degree(&self, name: &str) -> usize {
        self.index
            .get(name)
            .map(|&node| self.graph.edges_directed(node, Direction::Outgoing).count())
            .unwrap_or(0)
    }

    fn max_out_degree(&self) -> usize {
        self.graph
            .node_indices()
            .map(|node| self.graph.edges_directed(node, Direction::Outgoing).count())
            .max()
            .unwrap_or(0)
    }

    fn expand(&self, seed: &str, depth: usize) -> Subgraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let Some(&start) = self.index.get(seed) else {
            return Subgraph {
                seed: seed.to_string(),
                ..Default::default()
            };
        };
        let mut visited = std::collections::HashSet::new();
        let mut frontier = vec![(start, 0usize)];
        while let Some((node, distance)) = frontier.pop() {
            if !visited.insert(node) {
                continue;
            }
            nodes.push(self.graph[node].clone());
            if distance >= depth {
                continue;
            }
            for edge in self.graph.edges_directed(node, Direction::Outgoing) {
                edges.push(edge.weight().clone());
                frontier.push((edge.target(), distance + 1));
            }
        }
        Subgraph {
            seed: seed.to_string(),
            nodes,
            edges,
        }
    }
}

#[derive(Clone, Default)]
pub struct InMemoryGraph {
    state: Arc<RwLock<GraphState>>,
}

impl InMemoryGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arc() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn node_count(&self) -> usize {
        self.state.read().graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.state.read().graph.edge_count()
    }
}

#[async_trait]
impl GraphStore for InMemoryGraph {
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphError> {
        let _span = tracing::debug_span!(
            "rig_resources.graph.upsert",
            src = %edge.src,
            dst = %edge.dst,
            kind = %edge.kind,
        )
        .entered();
        self.state.write().upsert(edge);
        Ok(())
    }

    async fn expand(&self, entity: &str, depth: usize) -> Result<Subgraph, GraphError> {
        let _span =
            tracing::debug_span!("rig_resources.graph.expand", entity = %entity, depth).entered();
        Ok(self.state.read().expand(entity, depth))
    }

    async fn centrality(&self, entity: &str) -> f64 {
        let _span =
            tracing::debug_span!("rig_resources.graph.centrality", entity = %entity).entered();
        let state = self.state.read();
        let max = state.max_out_degree();
        if max == 0 {
            return 0.0;
        }
        state.out_degree(entity) as f64 / max as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn upsert_is_idempotent_per_kind() {
        let graph = InMemoryGraph::new();
        graph
            .upsert_edge(GraphEdge::new("a", "b", "auth"))
            .await
            .unwrap();
        graph
            .upsert_edge(GraphEdge::new("a", "b", "auth"))
            .await
            .unwrap();
        graph
            .upsert_edge(GraphEdge::new("a", "b", "spawn"))
            .await
            .unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 2);
    }
}
