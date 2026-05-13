//! Projection helpers for `rig-compose` context packing.

use rig_compose::{
    ContextItem, ContextPack, ContextPackConfig, ContextSourceKind, Evidence, InvestigationContext,
};
use serde_json::{Value, json};

use crate::{BehaviorPattern, EntityBaseline, MemoryLookupHit};

/// Convert resource-native records into [`ContextItem`] values.
pub trait IntoContextItem {
    /// Project this resource record into a prompt-ready context item.
    fn to_context_item(&self) -> ContextItem;
}

impl IntoContextItem for BehaviorPattern {
    fn to_context_item(&self) -> ContextItem {
        let source_id = format!("behavior_pattern/{}@v{}", self.id, self.version);
        let text = if self.description.is_empty() {
            format!("behavior pattern {} version {}", self.id, self.version)
        } else {
            self.description.clone()
        };
        ContextItem::new(ContextSourceKind::Resource, source_id, text)
            .with_score(f64::from(self.confidence_delta))
            .with_provenance(json!({
                "resource": "behavior_pattern",
                "id": self.id,
                "version": self.version,
                "required": self.rule.required,
                "forbidden": self.rule.forbidden,
                "confidence_delta": self.confidence_delta,
                "conclude": self.conclude,
            }))
    }
}

impl IntoContextItem for EntityBaseline {
    fn to_context_item(&self) -> ContextItem {
        ContextItem::new(
            ContextSourceKind::Resource,
            format!("baseline/{}/{}", self.entity, self.metric),
            format!(
                "baseline for {} {}: mean {}, std_dev {}, samples {}",
                self.entity, self.metric, self.mean, self.std_dev, self.samples
            ),
        )
        .with_score(self.samples as f64)
        .with_provenance(json!({
            "resource": "baseline",
            "entity": self.entity,
            "metric": self.metric,
            "mean": self.mean,
            "std_dev": self.std_dev,
            "samples": self.samples,
        }))
    }
}

impl IntoContextItem for MemoryLookupHit {
    fn to_context_item(&self) -> ContextItem {
        memory_hit_to_context_item(self, 0)
    }
}

/// Project a memory lookup hit into a ranked memory context item.
#[must_use]
pub fn memory_hit_to_context_item(hit: &MemoryLookupHit, rank: usize) -> ContextItem {
    let source_id = hit
        .key
        .clone()
        .unwrap_or_else(|| format!("memory.hit/{rank}"));
    ContextItem::new(ContextSourceKind::Memory, source_id, hit.summary.clone())
        .with_rank(rank)
        .with_score(f64::from(hit.score))
        .with_provenance(json!({
            "resource": "memory.lookup",
            "key": hit.key,
            "score": hit.score,
            "metadata": hit.metadata,
        }))
}

/// Project memory lookup hits into ranked memory context items.
#[must_use]
pub fn memory_hits_to_context_items(hits: &[MemoryLookupHit]) -> Vec<ContextItem> {
    hits.iter()
        .enumerate()
        .map(|(rank, hit)| memory_hit_to_context_item(hit, rank))
        .collect()
}

/// Project all accumulated investigation evidence into resource or memory
/// context items.
#[must_use]
pub fn evidence_to_context_items(ctx: &InvestigationContext) -> Vec<ContextItem> {
    ctx.evidence
        .iter()
        .enumerate()
        .map(|(rank, evidence)| evidence_to_context_item(evidence, rank))
        .collect()
}

/// Project one evidence record into a context item.
#[must_use]
pub fn evidence_to_context_item(evidence: &Evidence, rank: usize) -> ContextItem {
    let source = if evidence.source_skill == "general.memory_pivot"
        || evidence.label.starts_with("memory.")
    {
        ContextSourceKind::Memory
    } else {
        ContextSourceKind::Resource
    };
    let source_id = format!("{}/{}", evidence.source_skill, evidence.label);
    ContextItem::new(source, source_id, evidence_text(evidence))
        .with_rank(rank)
        .with_score(evidence_score(&evidence.detail))
        .with_provenance(json!({
            "source_skill": evidence.source_skill,
            "label": evidence.label,
            "detail": evidence.detail,
        }))
}

/// Pack resource-projected context items with the shared kernel packer.
#[must_use]
pub fn pack_resource_context(items: Vec<ContextItem>, config: ContextPackConfig) -> ContextPack {
    ContextPack::pack(items, config)
}

fn evidence_text(evidence: &Evidence) -> String {
    evidence
        .detail
        .get("summary")
        .and_then(Value::as_str)
        .or_else(|| evidence.detail.get("description").and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(|| evidence.label.clone())
}

fn evidence_score(detail: &Value) -> f64 {
    detail
        .get("score")
        .and_then(Value::as_f64)
        .or_else(|| detail.get("delta").and_then(Value::as_f64))
        .or_else(|| detail.get("confidence_delta").and_then(Value::as_f64))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PatternRule;
    use rig_compose::ContextOmissionReason;

    #[test]
    fn behavior_pattern_projects_to_resource_context() {
        let pattern = BehaviorPattern::new(
            "spray",
            2,
            PatternRule {
                required: vec!["auth.failure.burst".into()],
                forbidden: vec!["baseline.within".into()],
            },
            0.25,
        )
        .with_description("password spray around one host");

        let item = pattern.to_context_item();

        assert_eq!(item.source, ContextSourceKind::Resource);
        assert_eq!(item.source_id, "behavior_pattern/spray@v2");
        assert_eq!(item.text, "password spray around one host");
        assert!((item.score - 0.25).abs() < 1e-9);
        assert_eq!(item.provenance["resource"], "behavior_pattern");
        assert_eq!(item.provenance["required"][0], "auth.failure.burst");
    }

    #[test]
    fn memory_hits_project_with_stable_ranks() {
        let hits = vec![
            MemoryLookupHit::new(0.9, "first").with_key("episode-1"),
            MemoryLookupHit::new(0.5, "second"),
        ];

        let items = memory_hits_to_context_items(&hits);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].source, ContextSourceKind::Memory);
        assert_eq!(items[0].source_id, "episode-1");
        assert_eq!(items[0].rank, 0);
        assert_eq!(items[1].source_id, "memory.hit/1");
        assert_eq!(items[1].rank, 1);
    }

    #[test]
    fn evidence_projection_packs_and_omits_by_kernel_rules() {
        let mut ctx = InvestigationContext::new("host", "partition");
        ctx.evidence.push(
            Evidence::new("general.memory_pivot", "memory.hit")
                .with_detail(json!({"summary": "matching episode", "score": 0.8})),
        );
        ctx.evidence.push(
            Evidence::new("knowledge.behavior_pattern", "pattern:spray")
                .with_detail(json!({"description": "spray pattern", "delta": 0.2})),
        );

        let items = evidence_to_context_items(&ctx);
        let pack = pack_resource_context(items, ContextPackConfig::new(1_000).with_max_items(1));

        assert_eq!(pack.selected.len(), 1);
        assert_eq!(pack.omitted.len(), 1);
        assert_eq!(pack.omitted[0].reason, ContextOmissionReason::MaxItems);
        assert_eq!(pack.selected[0].source, ContextSourceKind::Memory);
        assert_eq!(pack.selected[0].text, "matching episode");
    }
}
