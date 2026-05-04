//! Graph expansion skill.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::tool::GraphTool;
use rig_compose::{
    Evidence, InvestigationContext, KernelError, NextAction, Skill, SkillOutcome, ToolRegistry,
};

#[derive(Debug, Clone)]
pub struct GraphExpansionConfig {
    pub min_confidence: f32,
    pub depth: usize,
    pub fanout_threshold: usize,
    pub confidence_lift: f32,
}

impl Default for GraphExpansionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.4,
            depth: 2,
            fanout_threshold: 3,
            confidence_lift: 0.15,
        }
    }
}

pub struct GraphExpansionSkill {
    cfg: GraphExpansionConfig,
}

impl GraphExpansionSkill {
    pub const ID: &'static str = "graph.expansion";

    pub fn new(cfg: GraphExpansionConfig) -> Self {
        Self { cfg }
    }

    pub fn with_defaults() -> Self {
        Self::new(GraphExpansionConfig::default())
    }
}

#[async_trait]
impl Skill for GraphExpansionSkill {
    fn id(&self) -> &str {
        Self::ID
    }

    fn description(&self) -> &str {
        "Pivot on graph fan-out around the entity once baseline confidence is non-trivial."
    }

    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.confidence >= self.cfg.min_confidence && !ctx.entity_id.is_empty()
    }

    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        let tool = tools.get(GraphTool::NAME)?;
        let value = tool
            .invoke(json!({
                "op": "expand",
                "entity": ctx.entity_id,
                "depth": self.cfg.depth,
            }))
            .await?;
        let neighbours = distinct_neighbour_count(&value, &ctx.entity_id);
        if neighbours < self.cfg.fanout_threshold {
            return Ok(SkillOutcome::noop());
        }
        ctx.evidence
            .push(Evidence::new(Self::ID, "graph.fanout").with_detail(json!({
                "entity": ctx.entity_id,
                "depth": self.cfg.depth,
                "distinct_neighbours": neighbours,
                "threshold": self.cfg.fanout_threshold,
            })));
        Ok(SkillOutcome::default()
            .with_delta(self.cfg.confidence_lift)
            .with_next(NextAction::RunSkill("general.memory_pivot".into())))
    }
}

fn distinct_neighbour_count(value: &Value, seed: &str) -> usize {
    let Some(nodes) = value.get("nodes").and_then(|nodes| nodes.as_array()) else {
        return 0;
    };
    nodes
        .iter()
        .filter_map(|node| node.as_str())
        .filter(|node| *node != seed)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::graph::{GraphEdge, GraphStore, GraphTool, InMemoryGraph};
    use rig_compose::Tool;

    fn registry_with(store: Arc<dyn GraphStore>) -> ToolRegistry {
        let registry = ToolRegistry::new();
        let tool: Arc<dyn Tool> = Arc::new(GraphTool::new(store));
        registry.register(tool);
        registry
    }

    #[tokio::test]
    async fn lifts_confidence_on_multi_host_fanout() {
        let store = Arc::new(InMemoryGraph::new());
        for target in ["h1", "h2", "h3", "h4"] {
            store
                .upsert_edge(GraphEdge::new("attacker", target, "auth"))
                .await
                .unwrap();
        }
        let registry = registry_with(store);
        let skill = GraphExpansionSkill::with_defaults();
        let mut ctx = InvestigationContext::new("attacker", "p");
        ctx.confidence = 0.5;
        let outcome = skill.execute(&mut ctx, &registry).await.unwrap();
        assert!(outcome.confidence_delta > 0.0);
        assert_eq!(ctx.evidence.len(), 1);
    }
}
