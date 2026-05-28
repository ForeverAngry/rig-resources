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

## Next Work

1. Extend graph resources with fixture-backed examples for expand, centrality, sparse context, and multi-hop summaries.
2. Wire the memory and baseline trace envelopes into the
   `MemoryPivotSkill` / `BaselineCompareSkill` execution paths (or a
   sample skill that does so) and add an end-to-end example that emits
   them alongside graph and security evidence.
3. Keep graph and security feature gates clean under the four-feature CI matrix.

## Maturity Bar

- A resource result can become prompt context without custom glue or lossy metadata.
- Missing/sparse resources return typed no-op or not-applicable outcomes rather than stringly errors.
- Graph and security features remain optional and do not leak dependencies into default builds.
- Tests cover default, `security`, `graph`, and `full` feature combinations.

## Non-Goals

- Do not define new kernel traits that belong in `rig-compose`.
- Do not own concrete memory archives; persistent memory belongs in `rig-memvid` or host stores.
- Do not become the product policy/governance layer.
