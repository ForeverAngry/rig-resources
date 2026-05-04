//! Lateral-movement skills.

use async_trait::async_trait;
use serde_json::json;

use rig_compose::{
    Evidence, InvestigationContext, KernelError, NextAction, Skill, SkillOutcome, ToolRegistry,
};

const REQUIRED: &[&str] = &["auth.success", "process.spawn", "net.connect"];

/// Detects a successful auth followed by a spawned process that initiates
/// outbound connections.
#[derive(Default)]
pub struct AuthSpawnConnectSkill;

#[async_trait]
impl Skill for AuthSpawnConnectSkill {
    fn id(&self) -> &str {
        "lateral.auth_spawn_connect"
    }
    fn description(&self) -> &str {
        "Detects auth->spawn->connect chains characteristic of lateral movement."
    }
    fn applies(&self, ctx: &InvestigationContext) -> bool {
        REQUIRED.iter().all(|signal| ctx.has_signal(signal))
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        ctx.evidence.push(
            Evidence::new(self.id(), "lateral.chain").with_detail(json!({"signals": REQUIRED})),
        );
        Ok(SkillOutcome::default()
            .with_delta(0.3)
            .with_next(NextAction::Conclude))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fires_when_full_chain_present() {
        let skill = AuthSpawnConnectSkill;
        let registry = ToolRegistry::new();
        let mut ctx = InvestigationContext::new("a", "p")
            .with_signal("auth.success")
            .with_signal("process.spawn")
            .with_signal("net.connect");
        assert!(skill.applies(&ctx));
        let outcome = skill.execute(&mut ctx, &registry).await.unwrap();
        assert!((outcome.confidence_delta - 0.3).abs() < 1e-6);
        assert!(matches!(outcome.next_actions[0], NextAction::Conclude));
    }
}
