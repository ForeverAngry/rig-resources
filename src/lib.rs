//! Reusable resources for [rig-compose](https://crates.io/crates/rig-compose) agents.
//!
//! `rig-compose` owns the kernel traits and runtime composition surfaces.
//! This crate owns reusable implementations: skills, tools, pattern
//! registries, baseline stores, and optional graph resources.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::panic_in_result_fn,
    )
)]

pub mod baseline;
pub mod memory;
pub mod patterns;
pub mod projection;
pub mod skills;
pub mod trace;

#[cfg(feature = "graph")]
pub mod graph;

#[cfg(feature = "security")]
pub mod security;

pub use baseline::{
    BaselineCompareTool, BaselineError, BaselineStore, EntityBaseline, InMemoryBaselineStore,
    OnlineStats, baseline_compare_trace_envelope,
};
pub use memory::{
    MemoryLookupError, MemoryLookupHit, MemoryLookupStore, MemoryLookupTool,
    memory_lookup_trace_envelope,
};
pub use patterns::{
    BehaviorPattern, BehaviorPatternSkill, BehaviorRegistry, PatternId, PatternRule,
};
#[cfg(feature = "graph")]
pub use projection::subgraph_to_context_item;
pub use projection::{
    IntoContextItem, evidence_to_context_item, evidence_to_context_items,
    memory_hit_to_context_item, memory_hits_to_context_items, pack_resource_context,
};
pub use skills::{BaselineCompareSkill, MemoryPivotSkill};
pub use trace::ResourceTraceEnvelope;

#[cfg(feature = "graph")]
pub use graph::{
    GraphEdge, GraphError, GraphExpansionConfig, GraphExpansionSkill, GraphStore, GraphTool,
    InMemoryGraph, Subgraph,
};

#[cfg(feature = "security")]
pub use security::{
    FindingSeverity, SecurityFinding, security_finding_to_context_item,
    security_finding_trace_envelope, security_findings_to_context_items,
};
