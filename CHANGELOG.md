# Changelog

<!-- markdownlint-disable MD024 -->

All notable changes to `rig-resources` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Versions are managed automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

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
