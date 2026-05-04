# rig-resources

[![CI](https://github.com/ForeverAngry/rig-resources/actions/workflows/ci.yml/badge.svg)](https://github.com/ForeverAngry/rig-resources/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rig-resources.svg)](https://crates.io/crates/rig-resources)
[![docs.rs](https://img.shields.io/docsrs/rig-resources)](https://docs.rs/rig-resources)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg)](#rust-version)

Reusable resource primitives for [`rig-compose`](https://crates.io/crates/rig-compose)
agents: skills, tools, behavior-pattern registries, baseline stores, and
optional graph resources.

`rig-compose` stays the small kernel. `rig-resources` is where reusable
implementations live so downstream agents do not have to reimplement them.

## Install

```toml
[dependencies]
rig-compose   = "0.1"
rig-resources = "0.1"
```

## Features

| Feature    | Default | Pulls in   | Enables                                                            |
| ---------- | :-----: | ---------- | ------------------------------------------------------------------ |
| `security` |    ✗    | —          | `security::*` namespace (credential, exfil, lateral, recon).       |
| `graph`    |    ✗    | `petgraph` | `GraphStore`, `InMemoryGraph`, `GraphTool`, `GraphExpansionSkill`. |
| `full`     |    ✗    | both above | Convenience umbrella enabling `security` + `graph`.                |

## What you get

- **Baselines.** `BaselineStore` + `InMemoryBaselineStore`, `EntityBaseline`,
  and the `BaselineCompareTool` / `BaselineCompareSkill` pair for "is this
  value within k·σ of the rolling mean?" decisions.
- **Behavior patterns.** `BehaviorPattern`, `BehaviorPatternSkill`,
  `BehaviorRegistry`, `PatternId`, `PatternRule` for declarative pattern
  routing on top of `rig-compose`'s skill surface.
- **Pivots.** `MemoryPivotSkill` for cross-store pivots reusable across
  agents.
- **Graph (optional).** `GraphStore`, `InMemoryGraph`, `GraphEdge`,
  `Subgraph`, `GraphTool`, `GraphExpansionSkill`, `GraphExpansionConfig`,
  `GraphError` — a transport-agnostic graph resource backed by
  `petgraph` in-process.
- **Security (optional).** Namespaced security primitives gated behind the
  `security` feature.

## Rust version

The crate targets Rust **1.88** (edition 2024). MSRV bumps follow the
[Rig contributing policy](https://github.com/0xPlaygrounds/rig/blob/main/CONTRIBUTING.md)
and ship as a `feat!:` change.

## License

Dual-licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual-licensed as above, without any additional terms or conditions.
