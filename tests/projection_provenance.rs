#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use rig_compose::{ContextProjectionState, ContextSourceKind, Evidence, InvestigationContext};
use rig_resources::{
    BehaviorPattern, EntityBaseline, IntoContextItem, MemoryLookupHit, PatternRule,
    evidence_to_context_item, evidence_to_context_items, memory_hit_to_context_item,
};
use serde_json::json;

fn assert_shared_candidate_keys(item: &rig_compose::ContextItem) {
    assert!(item.metadata.get("resource").is_some());
    let prov = item.context_provenance().unwrap();
    assert!(prov.source_uri.is_some());
    assert_eq!(
        prov.projection_state.unwrap(),
        ContextProjectionState::Candidate
    );
    assert!(prov.confidence.is_some());
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
    assert_shared_candidate_keys(&pattern_item);
    assert_eq!(pattern_item.metadata["resource"], "behavior_pattern");
    assert_eq!(
        pattern_item
            .context_provenance()
            .unwrap()
            .version_key
            .as_deref()
            .unwrap(),
        "spray"
    );

    let baseline = EntityBaseline {
        entity: "host-1".into(),
        metric: "dns_qps".into(),
        mean: 10.0,
        std_dev: 2.0,
        samples: 20,
    };
    let baseline_item = baseline.to_context_item();
    assert_eq!(baseline_item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&baseline_item);
    assert_eq!(
        baseline_item
            .context_provenance()
            .unwrap()
            .principal
            .as_deref()
            .unwrap(),
        "host-1"
    );

    let memory = MemoryLookupHit::new(0.8, "prior incident")
        .with_key("frame-1")
        .with_source_uri("memory://frame-1")
        .with_principal("host-1")
        .with_scope("tenant-a")
        .with_recorded_at_millis(1_700_000_000_000);
    let memory_item = memory_hit_to_context_item(&memory, 3);
    assert_eq!(memory_item.source, ContextSourceKind::Memory);
    assert_shared_candidate_keys(&memory_item);
    let memory_prov = memory_item.context_provenance().unwrap();
    assert_eq!(memory_prov.scope.as_deref().unwrap(), "tenant-a");
    assert_eq!(
        memory_prov
            .source_frame_id
            .as_ref()
            .unwrap()
            .as_str()
            .unwrap(),
        "frame-1"
    );

    let evidence = Evidence::new("detector", "finding").with_detail(json!({
        "summary": "high fan-out",
        "score": 0.7,
    }));
    let evidence_item = evidence_to_context_item(&evidence, 1);
    assert_eq!(evidence_item.source, ContextSourceKind::Resource);
    assert_shared_candidate_keys(&evidence_item);
    assert_eq!(evidence_item.metadata["source_skill"], "detector");

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
    assert_eq!(item.metadata["resource"], "graph.subgraph");
    let prov = item.context_provenance().unwrap();
    assert_eq!(prov.source_uri.as_deref().unwrap(), "graph://host-1");
    assert_eq!(prov.principal.as_deref().unwrap(), "host-1");
    assert_eq!(
        prov.projection_state.unwrap(),
        ContextProjectionState::Expanded
    );
    assert_eq!(prov.reason.as_deref().unwrap(), "graph_expansion");
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
    assert_shared_candidate_keys(&item);
    assert_eq!(item.metadata["resource"], "security.finding");

    let prov = item.context_provenance().unwrap();
    assert_eq!(prov.principal.as_deref().unwrap(), "host-1");
    assert_eq!(prov.scope.as_deref().unwrap(), "tenant-a");

    assert_eq!(item.metadata["finding_id"], "credential.password_spray");
    assert_eq!(item.metadata["severity"], "high");
    assert_eq!(item.metadata["technique_id"], "T1110.003");
    assert_eq!(item.metadata["signals"][0], "auth.failure.burst");
}
