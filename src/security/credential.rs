//! Credential-attack skills.

use async_trait::async_trait;

use rig_compose::{Evidence, InvestigationContext, KernelError, Skill, SkillOutcome, ToolRegistry};

/// Fires when an `auth.failure.burst` signal is present.
#[derive(Default)]
pub struct PasswordSpraySkill;

#[async_trait]
impl Skill for PasswordSpraySkill {
    fn id(&self) -> &str {
        "credential.password_spray"
    }
    fn description(&self) -> &str {
        "Lifts confidence on bursts of authentication failures across distinct accounts."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        ctx.has_signal("auth.failure.burst")
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence
            .push(Evidence::new(self.id(), "credential.spray"));
        Ok(SkillOutcome::default().with_delta(0.25))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lifts_on_burst_signal() {
        let skill = PasswordSpraySkill;
        let registry = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("a", "p").with_signal("auth.failure.burst");
        let outcome = skill.execute(&mut ctx, &registry).await.unwrap();
        assert!(outcome.confidence_delta > 0.0);
        assert_eq!(ctx.evidence.len(), 1);
    }
}
