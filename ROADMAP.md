# rig-resources Roadmap

This roadmap is the crate-local operating plan for `rig-resources`. The cross-crate coordination summary lives in [`rig-ecosystem/docs/roadmap.md`](../rig-ecosystem/docs/roadmap.md).

## Role

`rig-resources` is the reusable implementation layer for `rig-compose` agents. It supplies concrete skills, tools, baseline stores, memory lookup contracts, behavior-pattern registries, optional graph resources, and optional security primitives without bloating the `rig-compose` kernel.

## Landed

- Baseline storage, online baseline accumulation, baseline comparison tool, and baseline comparison skill.
- Behavior-pattern registry and behavior-pattern skill.
- Canonical `memory.lookup` tool contract with `MemoryLookupStore`, `MemoryLookupHit`, and `MemoryLookupTool`.
- `MemoryPivotSkill` that calls a registered `memory.lookup` tool after confidence crosses a threshold.
- Optional `graph` feature with `GraphStore`, `InMemoryGraph`, `GraphTool`, and `GraphExpansionSkill`.
- Fixture-backed graph example for expand, centrality, sparse context,
  and multi-hop context summaries ([examples/graph_fixtures.rs](examples/graph_fixtures.rs)).
- Optional `security` feature with credential, ECS signal, exfiltration, lateral-movement, reconnaissance, and related security helpers.
- `full` feature covering graph and security together.
- Caller-side context projection helpers for behavior patterns, memory
  lookup hits, baselines, and accumulated investigation evidence
  ([src/projection.rs](src/projection.rs)).
- Shared context-provenance keys across behavior patterns, baselines, memory
  lookup hits, graph expansions, and accumulated evidence, including source
  URI, principal, scope, recorded-at time, confidence, source frame id,
  projection state, and machine-readable reasons where available.
- Structured security-finding projection: `SecurityFinding` +
  `FindingSeverity` + `security_finding_to_context_item` /
  `security_findings_to_context_items` (feature `security`) project
  detector output into `rig_compose::ContextItem` with the shared
  provenance vocabulary plus security-specific keys (`finding_id`,
  `severity`, `technique_id`, `tactic`, `source_skill`, `signals`,
  `detail`).
- `ResourceTraceEnvelope` trace metadata shape
  ([src/trace.rs](src/trace.rs)), attached today to graph expansion evidence
  and to security findings via `security_finding_trace_envelope`.

## Prototype Grade

- Resource lookup outputs project into `rig-compose` `ContextItem` /
  `ContextPack` helpers with stable provenance keys. Security findings now
  share that surface via `security_finding_to_context_item`; broader
  trace-envelope coverage for memory and baseline paths is still
  incomplete.
- Graph resources cover in-memory graph expansion, but not a stable backend-neutral read API for richer graph evals.
- Security primitives are reusable skills/helpers plus a structured
  `SecurityFinding` projection, not a full policy engine with approvals,
  sandboxing, secrets, or risk workflows.
- `ResourceTraceEnvelope` is wired into graph evidence, security
  findings, and `memory.lookup` + `baseline.compare` evaluations via
  `memory_lookup_trace_envelope` / `baseline_compare_trace_envelope`.
  `MemoryPivotSkill` attaches a `memory.trace` evidence entry on every
  canonical `memory.lookup` invocation, and
  `examples/trace_envelopes.rs` emits all four envelope shapes
  side-by-side.

## Next Work

1. Add graph-specific eval fixtures that consume `Subgraph` / graph-projected
  context items and compare sparse vs. multi-hop retrieval quality.
2. Promote the trace-envelope evidence to a first-class
   `InvestigationContext` channel (out-of-band trace stream vs.
   inline evidence) once a downstream consumer needs to filter by
   envelope shape rather than re-decode from `Evidence::detail`.
3. Keep graph and security feature gates clean under the four-feature CI matrix.

## Maturity Bar

- A resource result can become prompt context without custom glue or lossy metadata.
- Missing/sparse resources return typed no-op or not-applicable outcomes rather than stringly errors.
- Graph and security features remain optional and do not leak dependencies into default builds.
- Tests cover default, `security`, `graph`, and `full` feature combinations.
- The crate intentionally tracks `rig-compose`'s MSRV floor (`1.88` today);
  bump this only when the kernel crate does.

## Non-Goals

- Do not define new kernel traits that belong in `rig-compose`.
- Do not own concrete memory archives; persistent memory belongs in `rig-memvid` or host stores.
- Do not become the product policy/governance layer.
