#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use rig_compose::{ContextSourceKind, Evidence, InvestigationContext};
use rig_resources::{
    BehaviorPattern, EntityBaseline, IntoContextItem, MemoryLookupHit, PatternRule,
    evidence_to_context_item, evidence_to_context_items, memory_hit_to_context_item,
};
use serde_json::json;

fn assert_shared_candidate_keys(provenance: &serde_json::Value) {
    assert!(provenance.get("resource").is_some());
    assert!(provenance.get("source_uri").is_some());
    assert_eq!(provenance["projection_state"], "candidate");
    assert!(provenance.get("confidence").is_some());
}

#[test]
fn default_resource_projections_carry_shared_provenance_keys() {
    let pattern = BehaviorPattern::new(
        "spray",
        1,
        PatternRule {
            required: vec!["auth.failure.burst".into()],
            forbidden: vec!["baseline.within".into()],
        },
        0.25,
    )
    .with_description("password spray");
    let pattern_item = pattern.to_context_item();
    assert_eq!(pattern_item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&pattern_item.provenance);
    assert_eq!(pattern_item.provenance["resource"], "behavior_pattern");
    assert_eq!(pattern_item.provenance["version_key"], "spray");

    let baseline = EntityBaseline {
        entity: "host-1".into(),
        metric: "dns_qps".into(),
        mean: 10.0,
        std_dev: 2.0,
        samples: 20,
    };
    let baseline_item = baseline.to_context_item();
    assert_eq!(baseline_item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&baseline_item.provenance);
    assert_eq!(baseline_item.provenance["principal"], "host-1");

    let memory = MemoryLookupHit::new(0.8, "prior incident")
        .with_key("frame-1")
        .with_source_uri("memory://frame-1")
        .with_principal("host-1")
        .with_scope("tenant-a")
        .with_recorded_at_millis(1_700_000_000_000);
    let memory_item = memory_hit_to_context_item(&memory, 3);
    assert_eq!(memory_item.source, ContextSourceKind::Memory);
    assert_shared_candidate_keys(&memory_item.provenance);
    assert_eq!(memory_item.provenance["scope"], "tenant-a");
    assert_eq!(memory_item.provenance["source_frame_id"], "frame-1");

    let evidence = Evidence::new("detector", "finding").with_detail(json!({
        "summary": "high fan-out",
        "score": 0.7,
    }));
    let evidence_item = evidence_to_context_item(&evidence, 1);
    assert_eq!(evidence_item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&evidence_item.provenance);
    assert_eq!(evidence_item.provenance["source_skill"], "detector");

    let mut ctx = InvestigationContext::new("host-1", "tenant-a");
    ctx.evidence.push(evidence);
    assert_eq!(evidence_to_context_items(&ctx).len(), 1);
}

#[cfg(feature = "graph")]
#[test]
fn graph_projection_uses_expanded_state_and_reason() {
    use rig_resources::{GraphEdge, Subgraph, subgraph_to_context_item};

    let subgraph = Subgraph {
        seed: "host-1".into(),
        nodes: vec!["host-1".into(), "host-2".into()],
        edges: vec![GraphEdge::new("host-1", "host-2", "auth")],
    };
    let item = subgraph_to_context_item(&subgraph, 0);

    assert_eq!(item.source, ContextSourceKind::Resource);
    assert_eq!(item.provenance["resource"], "graph.subgraph");
    assert_eq!(item.provenance["source_uri"], "graph://host-1");
    assert_eq!(item.provenance["principal"], "host-1");
    assert_eq!(item.provenance["projection_state"], "expanded");
    assert_eq!(item.provenance["reason"], "graph_expansion");
}

#[cfg(feature = "security")]
#[test]
fn security_projection_carries_shared_and_security_specific_keys() {
    use rig_resources::{FindingSeverity, SecurityFinding, security_finding_to_context_item};

    let finding = SecurityFinding::new(
        "credential.password_spray",
        FindingSeverity::High,
        "failed logins across accounts",
    )
    .with_principal("host-1")
    .with_scope("tenant-a")
    .with_source_uri("ecs://event/1")
    .with_recorded_at_millis(1_700_000_000_000)
    .with_technique_id("T1110.003")
    .with_tactic("credential-access")
    .with_source_skill("credential.password_spray")
    .with_signals(["auth.failure.burst"]);

    let item = security_finding_to_context_item(&finding, 0);

    assert_eq!(item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&item.provenance);
    assert_eq!(item.provenance["resource"], "security.finding");
    assert_eq!(item.provenance["principal"], "host-1");
    assert_eq!(item.provenance["scope"], "tenant-a");
    assert_eq!(item.provenance["finding_id"], "credential.password_spray");
    assert_eq!(item.provenance["severity"], "high");
    assert_eq!(item.provenance["technique_id"], "T1110.003");
    assert_eq!(item.provenance["signals"][0], "auth.failure.burst");
}
