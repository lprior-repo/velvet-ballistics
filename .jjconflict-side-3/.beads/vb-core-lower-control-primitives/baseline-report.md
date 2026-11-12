# Baseline Report - vb-core-lower-control-primitives

bead_id: vb-core-lower-control-primitives
captured_at: 2026-05-15T00:00:00Z
source_checkout: /home/lewis/src/velvet-ballistics

## Baseline Git Log (last 5 commits)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Touched Crates/Files (from bd show)

- Primary crate: `vb_compile` (crates/vb_compile/src/lower.rs, crates/vb_compile/src/api_build2.rs)
- Related crate: `vb_core` (crates/vb_core/src/nodes.rs)
- Issue labels: compiler, core-priority, engine, ir, no-codegen, yaml

## Known Constraints

1. Excludes generated Rust - must not modify generated Rust files
2. Must preserve dense numeric IR indexes
3. Must maintain runtime-compatible compiled output
4. No synthetic id-plus-one body assumptions
5. Each control primitive requires: positive lowering tests, invalid-shape diagnostics, dense numeric IR indexes, runtime-compatible compiled output
6. Zero unwrap/expect/panic/todo/unimplemented discipline required
7. Safe Rust only (`#![forbid(unsafe_code)]`)

## Acceptance Criteria Summary

- Each control primitive has positive lowering tests
- Invalid-shape diagnostics work correctly
- Dense numeric IR indexes maintained
- Runtime-compatible compiled output
- No synthetic id-plus-one body assumptions remain
