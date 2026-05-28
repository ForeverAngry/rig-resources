//! Projection helpers for `rig-compose` context packing.

use rig_compose::{
    ContextItem, ContextPack, ContextPackConfig, ContextSourceKind, Evidence, InvestigationContext,
};
use serde_json::{Value, json};

use crate::{BehaviorPattern, EntityBaseline, MemoryLookupHit};

#[cfg(feature = "graph")]
use crate::Subgraph;

const STATE_CANDIDATE: &str = "candidate";
#[cfg(feature = "graph")]
const STATE_EXPANDED: &str = "expanded";

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
                "source_uri": format!("behavior-pattern://{}@v{}", self.id, self.version),
                "confidence": self.confidence_delta,
                "version_key": self.id,
                "projection_state": STATE_CANDIDATE,
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
            "source_uri": format!("baseline://{}/{}", self.entity, self.metric),
            "principal": self.entity,
            "confidence": self.samples,
            "projection_state": STATE_CANDIDATE,
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

#[cfg(feature = "graph")]
impl IntoContextItem for Subgraph {
    fn to_context_item(&self) -> ContextItem {
        subgraph_to_context_item(self, 0)
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
            "source_uri": hit.source_uri,
            "principal": hit.principal,
            "scope": hit.scope,
            "recorded_at_millis": hit.recorded_at_millis,
            "confidence": hit.score,
            "source_frame_id": hit.key,
            "projection_state": STATE_CANDIDATE,
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

/// Project a graph expansion into a resource context item.
#[cfg(feature = "graph")]
#[must_use]
pub fn subgraph_to_context_item(subgraph: &Subgraph, rank: usize) -> ContextItem {
    let node_count = subgraph.nodes.len();
    let edge_count = subgraph.edges.len();
    ContextItem::new(
        ContextSourceKind::Resource,
        format!("graph/{}", subgraph.seed),
        format!(
            "graph expansion for {}: {} nodes, {} edges",
            subgraph.seed, node_count, edge_count
        ),
    )
    .with_rank(rank)
    .with_score(node_count.saturating_add(edge_count) as f64)
    .with_provenance(json!({
        "resource": "graph.subgraph",
        "source_uri": format!("graph://{}", subgraph.seed),
        "principal": subgraph.seed,
        "projection_state": STATE_EXPANDED,
        "reason": "graph_expansion",
        "seed": subgraph.seed,
        "nodes": subgraph.nodes,
        "edges": subgraph.edges,
    }))
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
            "resource": "investigation.evidence",
            "source_uri": format!("evidence://{}/{}", evidence.source_skill, evidence.label),
            "confidence": evidence_score(&evidence.detail),
            "projection_state": STATE_CANDIDATE,
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
        assert_eq!(item.provenance["source_uri"], "behavior-pattern://spray@v2");
        assert_eq!(item.provenance["projection_state"], "candidate");
        assert_eq!(item.provenance["required"][0], "auth.failure.burst");
    }

    #[test]
    fn memory_hits_project_with_stable_ranks() {
        let hits = vec![
            MemoryLookupHit::new(0.9, "first")
                .with_key("episode-1")
                .with_source_uri("memory://episode/1")
                .with_principal("alice")
                .with_scope("workspace")
                .with_recorded_at_millis(1_700_000_000_000),
            MemoryLookupHit::new(0.5, "second"),
        ];

        let items = memory_hits_to_context_items(&hits);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].source, ContextSourceKind::Memory);
        assert_eq!(items[0].source_id, "episode-1");
        assert_eq!(items[0].rank, 0);
        assert_eq!(items[0].provenance["source_uri"], "memory://episode/1");
        assert_eq!(items[0].provenance["principal"], "alice");
        assert_eq!(items[0].provenance["scope"], "workspace");
        assert_eq!(
            items[0].provenance["recorded_at_millis"],
            1_700_000_000_000_i64
        );
        let confidence = items[0].provenance["confidence"]
            .as_f64()
            .expect("confidence should serialize as a number");
        assert!((confidence - 0.9).abs() < 1e-6);
        assert_eq!(items[0].provenance["source_frame_id"], "episode-1");
        assert_eq!(items[0].provenance["projection_state"], "candidate");
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

    #[cfg(feature = "graph")]
    #[test]
    fn subgraph_projects_to_resource_context() {
        use crate::GraphEdge;

        let subgraph = Subgraph {
            seed: "host-1".into(),
            nodes: vec!["host-1".into(), "host-2".into()],
            edges: vec![GraphEdge::new("host-1", "host-2", "connects")],
        };

        let item = subgraph_to_context_item(&subgraph, 3);

        assert_eq!(item.source, ContextSourceKind::Resource);
        assert_eq!(item.source_id, "graph/host-1");
        assert_eq!(item.rank, 3);
        assert_eq!(item.score, 3.0);
        assert_eq!(item.provenance["resource"], "graph.subgraph");
        assert_eq!(item.provenance["source_uri"], "graph://host-1");
        assert_eq!(item.provenance["projection_state"], "expanded");
        assert_eq!(item.provenance["reason"], "graph_expansion");
        assert_eq!(item.provenance["seed"], "host-1");
    }
}
