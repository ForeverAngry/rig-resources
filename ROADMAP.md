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
- `ResourceTraceEnvelope` trace metadata shape
  ([src/trace.rs](src/trace.rs)), attached today to graph expansion evidence.

## Prototype Grade

- Resource lookup outputs project into caller-side helpers today; folding
  them into the `rig-compose` `ContextItem` / `ContextPack` vocabulary
  end-to-end is the next step.
- Graph resources cover in-memory graph expansion, but not a stable backend-neutral read API for richer graph evals.
- Security primitives are reusable skills/helpers, not a full policy engine with approvals, sandboxing, secrets, or risk workflows.
- `ResourceTraceEnvelope` is wired into graph evidence; memory, baseline,
  and security paths still emit local tool outputs without the shared
  envelope.

## Next Work

1. Fold the caller-side projection helpers into the `rig-compose`
   `ContextItem` / `ContextPack` vocabulary so memory lookups, graph
   expansions, baseline findings, and security findings reach prompt
   context without per-host glue.
2. Tighten `memory.lookup` metadata: source URI, principal, timestamp, confidence, scope, and omission/rejection reasons.
3. Extend graph resources with fixture-backed examples for expand, centrality, sparse context, and multi-hop summaries.
4. Extend `ResourceTraceEnvelope` coverage to memory, baseline, and
   security findings with machine-readable reasons for skipped,
   suppressed, expanded, or escalated outcomes.
5. Keep graph and security feature gates clean under the four-feature CI matrix.

## Maturity Bar

- A resource result can become prompt context without custom glue or lossy metadata.
- Missing/sparse resources return typed no-op or not-applicable outcomes rather than stringly errors.
- Graph and security features remain optional and do not leak dependencies into default builds.
- Tests cover default, `security`, `graph`, and `full` feature combinations.

## Non-Goals

- Do not define new kernel traits that belong in `rig-compose`.
- Do not own concrete memory archives; persistent memory belongs in `rig-memvid` or host stores.
- Do not become the product policy/governance layer.
