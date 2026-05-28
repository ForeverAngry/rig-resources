//! Fixture-backed graph examples for `rig-resources`.
//!
//! Run with: `cargo run --example graph_fixtures --features graph`
//!
//! The example exercises four graph paths over one small fixture:
//!
//! * direct `graph.entity` expansion
//! * degree-style centrality
//! * sparse-context handling through `GraphExpansionSkill`
//! * multi-hop expansion projected into a context-item summary

#[cfg(feature = "graph")]
use std::sync::Arc;

#[cfg(feature = "graph")]
use rig_compose::{Evidence, InvestigationContext, Skill, Tool, ToolRegistry};
#[cfg(feature = "graph")]
use rig_resources::{
    GraphExpansionSkill, GraphStore, GraphTool, InMemoryGraph, Subgraph, subgraph_to_context_item,
};
#[cfg(feature = "graph")]
use serde_json::{Value, json};

#[cfg(feature = "graph")]
const FIXTURE_EDGES: &[(&str, &str, &str)] = &[
    ("host-a", "host-b", "auth.success"),
    ("host-a", "host-c", "process.spawn"),
    ("host-a", "host-d", "net.connect"),
    ("host-b", "host-e", "auth.success"),
    ("host-c", "svc-payments", "dns.query"),
    ("svc-payments", "db-ledger", "sql.connect"),
];

#[cfg(feature = "graph")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraph::new());
    let tool = Arc::new(GraphTool::new(store.clone()));
    load_fixture(tool.as_ref()).await?;

    println!("== expand depth 1 ==");
    let expand = tool
        .invoke(json!({"op": "expand", "entity": "host-a", "depth": 1}))
        .await?;
    print_json(&expand)?;

    println!("\n== centrality ==");
    let centrality = tool
        .invoke(json!({"op": "centrality", "entity": "host-a"}))
        .await?;
    print_json(&centrality)?;

    println!("\n== sparse context through GraphExpansionSkill ==");
    let sparse = sparse_context(tool.clone()).await?;
    print_json(&sparse)?;

    println!("\n== multi-hop context summary ==");
    let multi_hop = tool
        .invoke(json!({"op": "expand", "entity": "host-a", "depth": 2}))
        .await?;
    let subgraph: Subgraph = serde_json::from_value(multi_hop)?;
    let item = subgraph_to_context_item(&subgraph, 0);
    print_json(&json!({
        "source_id": item.source_id,
        "text": item.text,
        "score": item.score,
        "provenance": item.provenance,
    }))?;

    Ok(())
}

#[cfg(not(feature = "graph"))]
fn main() {
    eprintln!("enable the graph feature: cargo run --example graph_fixtures --features graph");
}

#[cfg(feature = "graph")]
async fn load_fixture(tool: &GraphTool) -> Result<(), Box<dyn std::error::Error>> {
    for (src, dst, kind) in FIXTURE_EDGES {
        tool.invoke(json!({"op": "upsert", "src": src, "dst": dst, "kind": kind}))
            .await?;
    }
    Ok(())
}

#[cfg(feature = "graph")]
async fn sparse_context(tool: Arc<GraphTool>) -> Result<Value, Box<dyn std::error::Error>> {
    let registry = ToolRegistry::new();
    let graph_tool: Arc<dyn Tool> = tool;
    registry.register(graph_tool);

    let skill = GraphExpansionSkill::with_defaults();
    let mut ctx = InvestigationContext::new("unknown-host", "demo");
    ctx.confidence = 0.8;

    let outcome = skill.execute(&mut ctx, &registry).await?;
    Ok(json!({
        "confidence_delta": outcome.confidence_delta,
        "evidence": evidence_labels(&ctx.evidence),
    }))
}

#[cfg(feature = "graph")]
fn evidence_labels(evidence: &[Evidence]) -> Vec<String> {
    evidence.iter().map(|entry| entry.label.clone()).collect()
}

#[cfg(feature = "graph")]
fn print_json(value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
