//! Exfiltration / beaconing skills.

use async_trait::async_trait;

use rig_compose::{Evidence, InvestigationContext, KernelError, Skill, SkillOutcome, ToolRegistry};

/// Fires on `beacon.regular` (low-and-slow regular outbound cadence).
#[derive(Default)]
pub struct SlowBeaconSkill;

#[async_trait]
impl Skill for SlowBeaconSkill {
    fn id(&self) -> &str {
        "exfil.slow_beacon"
    }
    fn description(&self) -> &str {
        "Lifts confidence on regular outbound beaconing cadence."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.has_signal("beacon.regular")
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence.push(Evidence::new(self.id(), "exfil.beacon"));
        Ok(SkillOutcome::default().with_delta(0.2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lifts_on_regular_beacon() {
        let skill = SlowBeaconSkill;
        let registry = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("a", "p").with_signal("beacon.regular");
        let outcome = skill.execute(&mut ctx, &registry).await.unwrap();
        assert!(outcome.confidence_delta > 0.0);
    }
}
