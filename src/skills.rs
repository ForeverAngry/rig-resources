//! Prebuilt domain-neutral skills.

use async_trait::async_trait;
use serde_json::json;

use rig_compose::{Evidence, InvestigationContext, KernelError, Skill, SkillOutcome, ToolRegistry};

/// `general.baseline_compare` — suppresses confidence when behaviour falls
/// inside the entity's known baseline. Conservative by design: if no
/// `baseline.available` signal is present the skill is a no-op.
#[derive(Default)]
pub struct BaselineCompareSkill;

#[async_trait]
impl Skill for BaselineCompareSkill {
    fn id(&self) -> &str {
        "general.baseline_compare"
    }
    fn description(&self) -> &str {
        "Suppresses confidence when observed behaviour is within the entity's known baseline."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.has_signal("baseline.available") && ctx.has_signal("baseline.within")
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence
            .push(Evidence::new(self.id(), "baseline.suppress"));
        Ok(SkillOutcome::default().with_delta(-0.2))
    }
}

/// `general.memory_pivot` — calls `memory.lookup` once confidence has
/// crossed `min_confidence`. Records the top hit as evidence; never
/// adjusts confidence on its own (memory is context, not a verdict).
pub struct MemoryPivotSkill {
    pub min_confidence: f32,
    pub k: usize,
}

impl Default for MemoryPivotSkill {
    fn default() -> Self {
        Self {
            min_confidence: 0.4,
            k: 3,
        }
    }
}

#[async_trait]
impl Skill for MemoryPivotSkill {
    fn id(&self) -> &str {
        "general.memory_pivot"
    }
    fn description(&self) -> &str {
        "Retrieves similar episodes from memory once confidence is non-trivial."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.confidence >= self.min_confidence && !ctx.entity_id.is_empty()
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        let Ok(tool) = tools.get("memory.lookup") else {
            return Ok(SkillOutcome::noop());
        };
        let v = tool
            .invoke(json!({"query": ctx.entity_id, "k": self.k}))
            .await?;
        let top = v
            .get("hits")
            .and_then(|h| h.as_array())
            .and_then(|a| a.first())
            .cloned();
        if let Some(hit) = top {
            ctx.evidence
                .push(Evidence::new(self.id(), "memory.hit").with_detail(hit));
        }
        Ok(SkillOutcome::noop())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use rig_compose::{LocalTool, Tool, ToolSchema};

    #[tokio::test]
    async fn baseline_compare_suppresses_when_within() {
        let skill = BaselineCompareSkill;
        let reg = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("a", "p")
            .with_signal("baseline.available")
            .with_signal("baseline.within");
        ctx.confidence = 0.5;
        let outcome = skill.execute(&mut ctx, &reg).await.unwrap();
        assert!(outcome.confidence_delta < 0.0);
    }

    #[tokio::test]
    async fn memory_pivot_skipped_without_tool_authorisation() {
        let skill = MemoryPivotSkill::default();
        let reg = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("e", "p");
        ctx.confidence = 0.6;
        let outcome = skill.execute(&mut ctx, &reg).await.unwrap();
        assert_eq!(outcome.confidence_delta, 0.0);
        assert!(ctx.evidence.is_empty());
    }

    #[tokio::test]
    async fn memory_pivot_records_top_hit() {
        let skill = MemoryPivotSkill::default();
        let reg = ToolRegistry::new();
        let schema = ToolSchema {
            name: "memory.lookup".into(),
            description: "stub".into(),
            args_schema: json!({}),
            result_schema: json!({}),
        };
        let stub: Arc<dyn Tool> = Arc::new(LocalTool::new(schema, |_v| async {
            Ok(json!({"hits": [{"score": 0.9, "summary": "match", "episode_key": "k"}]}))
        }));
        reg.register(stub);
        let mut ctx = InvestigationContext::new("e", "p");
        ctx.confidence = 0.6;
        skill.execute(&mut ctx, &reg).await.unwrap();
        assert_eq!(ctx.evidence.len(), 1);
        assert_eq!(ctx.evidence[0].label, "memory.hit");
    }
}
