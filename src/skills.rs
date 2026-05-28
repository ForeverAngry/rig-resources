//! Prebuilt domain-neutral skills.

use async_trait::async_trait;
use serde_json::json;

use rig_compose::{Evidence, InvestigationContext, KernelError, Skill, SkillOutcome, ToolRegistry};

use crate::memory::{MemoryLookupHit, memory_lookup_trace_envelope};

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

        // Decode typed hits when the tool conforms to MemoryLookupTool's
        // schema. Stores that emit a different shape get the legacy
        // raw-JSON evidence path without the trace envelope; this keeps
        // the skill backward-compatible with non-canonical memory tools.
        let hits_array = v.get("hits").and_then(|h| h.as_array()).cloned();
        let typed_hits: Vec<MemoryLookupHit> = hits_array
            .as_ref()
            .and_then(|arr| serde_json::from_value(json!(arr)).ok())
            .unwrap_or_default();

        if let Some(arr) = hits_array.as_ref()
            && let Some(hit) = arr.first()
        {
            ctx.evidence
                .push(Evidence::new(self.id(), "memory.hit").with_detail(hit.clone()));
        }

        if let Some(arr) = hits_array.as_ref()
            && (typed_hits.len() == arr.len())
        {
            let envelope =
                memory_lookup_trace_envelope(&ctx.entity_id, self.k, &typed_hits, None, None);
            ctx.evidence
                .push(Evidence::new(self.id(), "memory.trace").with_detail(envelope.to_value()));
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
            Ok(json!({"hits": [{"score": 0.9, "summary": "match", "key": "k"}]}))
        }));
        reg.register(stub);
        let mut ctx = InvestigationContext::new("e", "p");
        ctx.confidence = 0.6;
        skill.execute(&mut ctx, &reg).await.unwrap();
        // memory.hit (raw top JSON) + memory.trace (trace envelope)
        assert_eq!(ctx.evidence.len(), 2);
        assert_eq!(ctx.evidence[0].label, "memory.hit");
        assert_eq!(ctx.evidence[1].label, "memory.trace");
        let trace = &ctx.evidence[1].detail;
        assert_eq!(trace["resource"], "memory");
        assert_eq!(trace["operation"], "lookup");
        assert_eq!(trace["output_summary"]["hit_count"], 1);
        assert_eq!(trace["output_summary"]["top_key"], "k");
    }

    #[tokio::test]
    async fn memory_pivot_emits_no_hits_trace_when_empty() {
        let skill = MemoryPivotSkill::default();
        let reg = ToolRegistry::new();
        let schema = ToolSchema {
            name: "memory.lookup".into(),
            description: "stub".into(),
            args_schema: json!({}),
            result_schema: json!({}),
        };
        let stub: Arc<dyn Tool> = Arc::new(LocalTool::new(schema, |_v| async {
            Ok(json!({"hits": []}))
        }));
        reg.register(stub);
        let mut ctx = InvestigationContext::new("nothing", "p");
        ctx.confidence = 0.6;
        skill.execute(&mut ctx, &reg).await.unwrap();
        // Only memory.trace — no memory.hit when the array is empty.
        assert_eq!(ctx.evidence.len(), 1);
        assert_eq!(ctx.evidence[0].label, "memory.trace");
        let trace = &ctx.evidence[0].detail;
        assert_eq!(trace["output_summary"]["hit_count"], 0);
        assert_eq!(trace["reason"], "no_hits");
    }
}
