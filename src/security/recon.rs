//! Reconnaissance skills.

use async_trait::async_trait;
use serde_json::json;

use rig_compose::{Evidence, InvestigationContext, KernelError, Skill, SkillOutcome, ToolRegistry};

/// Fires when a `fanout.high` signal is present.
pub struct HighFanoutSkill {
    pub lift: f32,
}

impl Default for HighFanoutSkill {
    fn default() -> Self {
        Self { lift: 0.2 }
    }
}

#[async_trait]
impl Skill for HighFanoutSkill {
    fn id(&self) -> &str {
        "recon.high_fanout"
    }
    fn description(&self) -> &str {
        "Lifts confidence when the entity exhibits high host fan-out."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.has_signal("fanout.high")
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence
            .push(Evidence::new(self.id(), "recon.fanout").with_detail(json!({"lift": self.lift})));
        Ok(SkillOutcome::default().with_delta(self.lift))
    }
}

/// Fires when grammar entropy looks anomalous (`entropy.anomalous` signal).
pub struct EntropyCheckSkill {
    pub lift: f32,
}

impl Default for EntropyCheckSkill {
    fn default() -> Self {
        Self { lift: 0.1 }
    }
}

#[async_trait]
impl Skill for EntropyCheckSkill {
    fn id(&self) -> &str {
        "recon.entropy_check"
    }
    fn description(&self) -> &str {
        "Lifts confidence when grammar entropy is flagged anomalous."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.has_signal("entropy.anomalous")
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence.push(Evidence::new(self.id(), "recon.entropy"));
        Ok(SkillOutcome::default().with_delta(self.lift))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn high_fanout_lifts_when_signal_present() {
        let skill = HighFanoutSkill::default();
        let registry = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("a", "p").with_signal("fanout.high");
        assert!(skill.applies(&ctx));
        let outcome = skill.execute(&mut ctx, &registry).await.unwrap();
        assert!(outcome.confidence_delta > 0.0);
        assert_eq!(ctx.evidence.len(), 1);
    }
}
