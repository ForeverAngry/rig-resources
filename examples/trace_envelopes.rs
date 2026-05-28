//! Emit the four `ResourceTraceEnvelope` shapes from the canonical
//! rig-resources tool surfaces side-by-side.
//!
//! Run with: `cargo run --example trace_envelopes --features full`
//!
//! This example exists so consumers can see the wire shape of every
//! envelope kind in one place:
//!
//! * `memory.lookup` via [`MemoryLookupTool`] + [`memory_lookup_trace_envelope`]
//! * `baseline.compare` via [`BaselineCompareTool`] + [`baseline_compare_trace_envelope`]
//! * `security.finding` via [`security_finding_trace_envelope`]
//! * `graph.expand` is exercised by the in-tree graph evidence path;
//!   see `src/graph/skills.rs` tests for that envelope.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use rig_resources::{
    BaselineCompareTool, BaselineStore, EntityBaseline, InMemoryBaselineStore, MemoryLookupError,
    MemoryLookupHit, MemoryLookupStore, MemoryLookupTool, baseline_compare_trace_envelope,
    memory_lookup_trace_envelope,
};

#[cfg(feature = "security")]
use rig_resources::{FindingSeverity, SecurityFinding, security_finding_trace_envelope};

use rig_compose::Tool;

/// Stub memory store with a single canned hit.
struct StubMemory;

#[async_trait]
impl MemoryLookupStore for StubMemory {
    async fn lookup(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<MemoryLookupHit>, MemoryLookupError> {
        let hit = MemoryLookupHit::new(0.87, format!("matched `{query}`"))
            .with_key("ep-42")
            .with_source_uri("memory://episodes/42")
            .with_principal("alice")
            .with_scope("workspace")
            .with_recorded_at_millis(1_700_000_000_000);
        Ok(vec![hit].into_iter().take(k).collect())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("== memory.lookup ==");
    memory_lookup_demo().await?;

    println!("\n== baseline.compare (within bounds) ==");
    baseline_demo_within().await?;

    println!("\n== baseline.compare (not found) ==");
    baseline_demo_not_found().await?;

    #[cfg(feature = "security")]
    {
        println!("\n== security.finding ==");
        security_demo();
    }

    Ok(())
}

async fn memory_lookup_demo() -> Result<(), Box<dyn std::error::Error>> {
    let store: Arc<dyn MemoryLookupStore> = Arc::new(StubMemory);
    let tool = MemoryLookupTool::new(store);

    let args = json!({"query": "beacon", "k": 3});
    let result = tool.invoke(args).await?;
    let hits_value = result
        .get("hits")
        .cloned()
        .ok_or("memory.lookup result missing `hits`")?;
    let hits: Vec<MemoryLookupHit> = serde_json::from_value(hits_value)?;

    let envelope =
        memory_lookup_trace_envelope("beacon", 3, &hits, Some("alice"), Some("workspace"));
    print_envelope(&envelope.to_value())?;
    Ok(())
}

async fn baseline_demo_within() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryBaselineStore::new());
    let baseline = EntityBaseline {
        entity: "host-1".into(),
        metric: "fanout".into(),
        mean: 10.0,
        std_dev: 2.0,
        samples: 144,
    };
    store.put(baseline.clone()).await?;

    let tool = BaselineCompareTool::new(store.clone());
    let _result = tool
        .invoke(json!({"entity": "host-1", "metric": "fanout", "value": 11.0, "k": 2.0}))
        .await?;

    let envelope = baseline_compare_trace_envelope("host-1", "fanout", 11.0, 2.0, Some(&baseline));
    print_envelope(&envelope.to_value())?;
    Ok(())
}

async fn baseline_demo_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let envelope = baseline_compare_trace_envelope("ghost", "fanout", 7.0, 2.0, None);
    print_envelope(&envelope.to_value())?;
    Ok(())
}

#[cfg(feature = "security")]
fn security_demo() {
    let finding =
        SecurityFinding::new("lateral.auth_spawn_connect", FindingSeverity::High, "chain")
            .with_principal("host-9")
            .with_signals(["auth.success", "process.spawn", "net.connect"])
            .with_source_skill("lateral.auth_spawn_connect")
            .with_technique_id("T1021")
            .with_tactic("lateral-movement");
    let envelope = security_finding_trace_envelope(&finding);
    if let Err(err) = print_envelope(&envelope.to_value()) {
        eprintln!("failed to print security envelope: {err}");
    }
}

fn print_envelope(value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
