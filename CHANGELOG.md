# Changelog

<!-- markdownlint-disable MD024 -->

All notable changes to `rig-resources` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Versions are managed automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

## [0.2.2](https://github.com/ForeverAngry/rig-resources/compare/v0.2.1...v0.2.2) - 2026-06-01

### Documentation

- Document resource API surface ([#32](https://github.com/ForeverAngry/rig-resources/pull/32))

## [0.2.1](https://github.com/ForeverAngry/rig-resources/compare/v0.2.0...v0.2.1) - 2026-05-28

### Added

- *(graph)* Add fixture-backed graph examples ([#28](https://github.com/ForeverAngry/rig-resources/pull/28))
- *(skills)* Wire memory_lookup_trace_envelope into MemoryPivotSkill ([#27](https://github.com/ForeverAngry/rig-resources/pull/27))
- *(trace)* Memory.lookup and baseline.compare trace envelopes ([#26](https://github.com/ForeverAngry/rig-resources/pull/26))
- *(security)* Project SecurityFinding into rig-compose context ([#25](https://github.com/ForeverAngry/rig-resources/pull/25))

### Documentation

- Refresh README status to reflect shipped 0.2.0 ([#23](https://github.com/ForeverAngry/rig-resources/pull/23))

### Added

- Structured security-finding projection under the `security` feature.
  New `SecurityFinding` + `FindingSeverity` types and
  `security_finding_to_context_item` /
  `security_findings_to_context_items` helpers project findings into
  `rig_compose::ContextItem` with the shared provenance vocabulary
  (`source_uri`, `principal`, `scope`, `recorded_at_millis`,
  `confidence`, `projection_state`) plus security-specific keys
  (`finding_id`, `severity`, `technique_id`, `tactic`, `source_skill`,
  `signals`, `detail`). `SecurityFinding` also implements
  `IntoContextItem`. Validates the typed context vocabulary in
  `rig-compose` with a structurally distinct second producer alongside
  memory-shaped projections.
- `security_finding_trace_envelope` produces a `ResourceTraceEnvelope`
  describing the finding (input signals, output severity/confidence,
  optional MITRE technique/tactic metadata) so trace consumers see the
  same shape used by graph expansion evidence.
- `memory_lookup_trace_envelope` produces a `ResourceTraceEnvelope` for
  a single `memory.lookup` invocation. Captures the query, requested
  `k`, optional caller `principal` / `scope`, hit count, top score and
  key, top-hit `source_uri` / `recorded_at_millis`, and emits the
  `no_hits` reason code when the store returned nothing. Mismatches
  between the caller's principal/scope and the top hit's are surfaced
  in `metadata`.
- `baseline_compare_trace_envelope` produces a `ResourceTraceEnvelope`
  for a single `baseline.compare` evaluation. Captures `entity`,
  `metric`, observed value, and `k`, plus the mean / std-dev / bound /
  deviation when a baseline exists, and emits one of three reason
  codes (`baseline_not_found`, `within_bounds`, `exceeds_bounds`).
  Metadata carries `samples` and the signed `z_score` when std-dev is
  non-zero. Closes the trace-envelope half of ROADMAP Next #2; memory
  and baseline paths now share the same envelope shape as security
  findings and graph expansion evidence.
- `MemoryPivotSkill` now attaches a `memory.trace` evidence entry
  carrying a `memory_lookup_trace_envelope` alongside the existing
  `memory.hit` raw-JSON evidence. Stores whose `memory.lookup`
  response deserialises into `Vec<MemoryLookupHit>` (the canonical
  `MemoryLookupTool` schema) get the typed envelope; non-canonical
  stores still receive the legacy raw-JSON evidence and no trace.
  Empty result arrays emit a trace with `reason = no_hits` and no
  `memory.hit` evidence.
- `examples/trace_envelopes.rs` demonstrates emitting the
  `memory.lookup`, `baseline.compare` (within-bounds and not-found),
  and `security.finding` envelopes side-by-side as pretty JSON. Run
  with `cargo run --example trace_envelopes --features full`.
- `examples/graph_fixtures.rs` provides a fixture-backed graph walkthrough
  covering `graph.entity` expansion, centrality, sparse-context handling
  through `GraphExpansionSkill`, and a multi-hop expansion projected into
  a context-item summary. Run with
  `cargo run --example graph_fixtures --features graph`.

## [0.2.0](https://github.com/ForeverAngry/rig-resources/compare/v0.1.6...v0.2.0) - 2026-05-28

### Added

- Add shared provenance fields to resource context projections. Behavior
  patterns, baselines, memory lookup hits, graph expansions, and accumulated
  evidence now project stable JSON keys such as `source_uri`, `principal`,
  `scope`, `recorded_at_millis`, `confidence`, `source_frame_id`,
  `projection_state`, and `reason` into `ContextItem::provenance`, matching the
  typed context vocabulary in `rig-compose` without requiring a path dependency.
- Extend `MemoryLookupHit` with optional source URI, principal, scope, and
  recorded-at metadata plus builder methods so memory stores can provide enough
  provenance for prompt-context replay and eval fixtures.

## [0.1.6](https://github.com/ForeverAngry/rig-resources/compare/v0.1.5...v0.1.6) - 2026-05-27

### Added

- Project graph resources into context

### Added

- Extend resource context projection to graph expansions: `Subgraph` now
  implements `IntoContextItem` behind the `graph` feature, and
  `subgraph_to_context_item` projects node/edge counts plus graph provenance
  into `rig_compose::ContextItem`.

### Changed

- Align the sibling `rig-compose` dependency with the local `0.4` crate so
  `just check` resolves the path dependency successfully.

## [0.1.5](https://github.com/ForeverAngry/rig-resources/compare/v0.1.4...v0.1.5) - 2026-05-12

### Documentation

- Add resource roadmap

### Added

- Add crate-local `ROADMAP.md` documenting maturity status, next work, and
  non-goals for reusable resource primitives.
- Add caller-side context projection helpers for behavior patterns, memory
  lookup hits, baselines, and accumulated investigation evidence.
- Add `ResourceTraceEnvelope`, a crate-local trace metadata shape for resource
  evidence, and attach it to graph expansion evidence.

## [0.1.4](https://github.com/ForeverAngry/rig-resources/compare/v0.1.3...v0.1.4) - 2026-05-07

### Documentation

- Remove retired repo references

## [0.1.3](https://github.com/ForeverAngry/rig-resources/compare/v0.1.2...v0.1.3) - 2026-05-06

### Added

- Add memory lookup and ECS resources

### Added

- `memory.lookup` resource contract: `MemoryLookupStore`, `MemoryLookupHit`,
  `MemoryLookupError`, and `MemoryLookupTool`. This gives
  `MemoryPivotSkill` a canonical tool implementation while leaving storage in
  downstream crates such as `rig-memvid`.
- `OnlineStats` Welford accumulator plus `EntityBaseline::from_stats` /
  `OnlineStats::to_baseline` helpers for building baselines from streaming
  observations.
- `security::ecs` helpers for converting ECS-shaped JSON rows into stable
  security signals such as `auth.failure`, `process.spawn`, `net.connect`, and
  `lateral.move`.

## [0.1.2](https://github.com/ForeverAngry/rig-resources/compare/v0.1.1...v0.1.2) - 2026-05-06

### Fixed

- Depend on released rig-compose
- Surface missing graph entities as inapplicable

### Fixed

- `InMemoryGraph::expand` now returns `GraphError::NotFound` for unknown
  seed entities instead of an empty subgraph, so callers can distinguish a
  missing graph node from a known isolated node. `GraphExpansionSkill`
  treats that sparse-context case as a no-op while direct `GraphTool`
  expansion calls remain fallible.
- `GraphTool` now maps `GraphError::NotFound` to
  `KernelError::ToolNotApplicable`, and `GraphExpansionSkill` matches on
  that typed variant instead of inspecting error message text.

## [0.1.1](https://github.com/ForeverAngry/rig-resources/compare/v0.1.0...v0.1.1) - 2026-05-04

### Fixed

- Correct author metadata
- Depend on rig-compose from crates.io (drop sibling path)

## [0.1.0] - Unreleased

### Added

- Initial release of reusable resource primitives for `rig-compose` agents.
- `BaselineStore` / `InMemoryBaselineStore`, `EntityBaseline`, and the
  `BaselineCompareTool` / `BaselineCompareSkill` pair.
- `BehaviorPattern`, `BehaviorPatternSkill`, `BehaviorRegistry`, `PatternId`,
  and `PatternRule` for declarative pattern routing.
- `MemoryPivotSkill` for reusable cross-store pivots.
- Optional `graph` feature: `GraphStore`, `InMemoryGraph`, `GraphEdge`,
  `Subgraph`, `GraphTool`, `GraphExpansionSkill`, `GraphExpansionConfig`,
  `GraphError`.
- Optional `security` feature: namespaced security primitives.
- `full` umbrella feature enabling both `graph` and `security`.
