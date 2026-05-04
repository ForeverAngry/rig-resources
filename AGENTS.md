# AGENTS.md

Guidance for AI coding agents working in `rig-resources`.

## Project

Reusable resource primitives for [`rig-compose`](https://crates.io/crates/rig-compose):
baseline stores, behavior-pattern registries, memory pivots, optional graph
resources, and optional security primitives.

## Rules

- Rust 2024, MSRV 1.88. Library is runtime-agnostic; no `tokio` in
  `[dependencies]`.
- Errors: `thiserror` enums; return `Result<_, _>`.
- Never `.await` while holding a `parking_lot` guard. Scope-drop first.
- No `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`/indexing
  in library code (clippy deny/forbid). Allowed in `#[cfg(test)]`.
- Use `tracing` for logs.
- Document new `pub` items with `///` rustdoc.

## Features

Default = none. Optional: `security`, `graph` (pulls `petgraph`), `full`
(both). CI matrix runs all four. Gate optional code with
`#[cfg(feature = "...")]`.

## Validation

```sh
just check
# fmt + clippy (× 4 feature combos) + test (× 4) + rustdoc strict
```

## Scope

Do not duplicate kernel surfaces from `rig-compose`; depend on its public
re-exports. Update [README.md](README.md) and [CHANGELOG.md](CHANGELOG.md)
for user-visible changes.
