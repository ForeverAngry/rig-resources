# Changelog

All notable changes to `rig-resources` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Versions are managed automatically by [release-plz](https://release-plz.dev/)
from [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

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
