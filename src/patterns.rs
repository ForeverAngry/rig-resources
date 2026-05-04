//! Domain-neutral behavioural pattern primitives.

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;

use rig_compose::{
    Evidence, InvestigationContext, KernelError, NextAction, Skill, SkillOutcome, ToolRegistry,
};

pub type PatternId = String;

/// One rule clause: every signal in `required` must be present, and none
/// of the signals in `forbidden` may be present.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternRule {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

impl PatternRule {
    pub fn matches(&self, ctx: &InvestigationContext) -> bool {
        self.required.iter().all(|s| ctx.has_signal(s))
            && self.forbidden.iter().all(|s| !ctx.has_signal(s))
    }
}

/// One immutable behaviour pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub id: PatternId,
    pub version: u32,
    pub description: String,
    pub rule: PatternRule,
    pub confidence_delta: f32,
    #[serde(default)]
    pub conclude: bool,
}

impl BehaviorPattern {
    pub fn new(id: impl Into<String>, version: u32, rule: PatternRule, delta: f32) -> Self {
        Self {
            id: id.into(),
            version,
            description: String::new(),
            rule,
            confidence_delta: delta,
            conclude: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn concluding(mut self) -> Self {
        self.conclude = true;
        self
    }
}

/// Versioned, append-only registry of behaviour patterns. Cheap to clone
/// (Arc-wrapped). `register` keeps the highest-version pattern per id.
#[derive(Clone, Default)]
pub struct BehaviorRegistry {
    inner: Arc<RwLock<Vec<BehaviorPattern>>>,
}

impl BehaviorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, pattern: BehaviorPattern) {
        let mut guard = self.inner.write();
        if let Some(existing) = guard.iter_mut().find(|p| p.id == pattern.id) {
            if pattern.version >= existing.version {
                *existing = pattern;
            }
        } else {
            guard.push(pattern);
        }
    }

    pub fn extend<I: IntoIterator<Item = BehaviorPattern>>(&self, patterns: I) {
        for pattern in patterns {
            self.register(pattern);
        }
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    pub fn snapshot(&self) -> Vec<BehaviorPattern> {
        self.inner.read().clone()
    }
}

/// Stateless skill that evaluates every registered pattern against the
/// context.
pub struct BehaviorPatternSkill {
    registry: BehaviorRegistry,
}

impl BehaviorPatternSkill {
    pub const ID: &'static str = "knowledge.behavior_pattern";

    pub fn new(registry: BehaviorRegistry) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Skill for BehaviorPatternSkill {
    fn id(&self) -> &str {
        Self::ID
    }
    fn description(&self) -> &str {
        "Evaluates a behavioural-pattern registry against the investigation context."
    }
    fn applies(&self, _ctx: &InvestigationContext) -> bool {
        !self.registry.is_empty()
    }
    async fn execute(
        &self,
        ctx: &mut InvestigationContext,
        _tools: &ToolRegistry,
    ) -> Result<SkillOutcome, KernelError> {
        let _span = tracing::debug_span!(
            "rig_resources.patterns.behavior_eval",
            patterns = self.registry.len(),
        )
        .entered();
        let matched: Vec<BehaviorPattern> = self
            .registry
            .snapshot()
            .into_iter()
            .filter(|pattern| pattern.rule.matches(ctx))
            .collect();
        let mut total = 0.0f32;
        let mut conclude = false;
        for pattern in matched {
            total += pattern.confidence_delta;
            conclude |= pattern.conclude;
            ctx.evidence.push(
                Evidence::new(Self::ID, format!("pattern:{}", pattern.id)).with_detail(json!({
                    "version": pattern.version,
                    "delta": pattern.confidence_delta,
                })),
            );
        }
        let mut outcome = SkillOutcome::default().with_delta(total);
        if conclude {
            outcome = outcome.with_next(NextAction::Conclude);
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(required: &[&str]) -> PatternRule {
        PatternRule {
            required: required.iter().map(|s| s.to_string()).collect(),
            forbidden: vec![],
        }
    }

    #[tokio::test]
    async fn matching_pattern_lifts_and_records_evidence() {
        let reg = BehaviorRegistry::new();
        reg.register(
            BehaviorPattern::new("brute", 1, rule(&["auth.failure.burst"]), 0.25)
                .with_description("password spray"),
        );
        let skill = BehaviorPatternSkill::new(reg);
        let mut ctx = InvestigationContext::new("e", "p").with_signal("auth.failure.burst");
        let tools = ToolRegistry::new();
        let outcome = skill.execute(&mut ctx, &tools).await.unwrap();
        assert!((outcome.confidence_delta - 0.25).abs() < 1e-6);
        assert_eq!(ctx.evidence.len(), 1);
    }

    #[tokio::test]
    async fn nonmatching_pattern_is_inert() {
        let reg = BehaviorRegistry::new();
        reg.register(BehaviorPattern::new("x", 1, rule(&["never"]), 0.5));
        let skill = BehaviorPatternSkill::new(reg);
        let mut ctx = InvestigationContext::new("e", "p");
        let tools = ToolRegistry::new();
        let outcome = skill.execute(&mut ctx, &tools).await.unwrap();
        assert_eq!(outcome.confidence_delta, 0.0);
        assert!(ctx.evidence.is_empty());
    }

    #[test]
    fn registry_keeps_highest_version() {
        let registry = BehaviorRegistry::new();
        registry.register(BehaviorPattern::new("p", 1, PatternRule::default(), 0.1));
        registry.register(BehaviorPattern::new("p", 2, PatternRule::default(), 0.2));
        registry.register(BehaviorPattern::new("p", 1, PatternRule::default(), 0.9));
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].version, 2);
        assert!((snapshot[0].confidence_delta - 0.2).abs() < 1e-6);
    }

    #[test]
    fn forbidden_signal_blocks_match() {
        let rule = PatternRule {
            required: vec!["a".into()],
            forbidden: vec!["b".into()],
        };
        let ctx_ok = InvestigationContext::new("e", "p").with_signal("a");
        let ctx_block = InvestigationContext::new("e", "p")
            .with_signal("a")
            .with_signal("b");
        assert!(rule.matches(&ctx_ok));
        assert!(!rule.matches(&ctx_block));
    }
}
