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

## Prototype Grade

- Resource lookup outputs are useful but not yet projected into the `rig-compose` `ContextItem` / `ContextPack` vocabulary.
- Graph resources cover in-memory graph expansion, but not a stable backend-neutral read API for richer graph evals.
- Security primitives are reusable skills/helpers, not a full policy engine with approvals, sandboxing, secrets, or risk workflows.
- Baseline and memory lookup traces are local tool outputs, not shared observability records.

## Next Work

1. Add context projections for memory lookup hits, graph expansions, baseline findings, and security findings into the `rig-compose` context vocabulary.
2. Tighten `memory.lookup` metadata: source URI, principal, timestamp, confidence, scope, and omission/rejection reasons.
3. Extend graph resources with fixture-backed examples for expand, centrality, sparse context, and multi-hop summaries.
4. Add security/resource trace envelopes with machine-readable reasons for skipped, suppressed, expanded, or escalated findings.
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
