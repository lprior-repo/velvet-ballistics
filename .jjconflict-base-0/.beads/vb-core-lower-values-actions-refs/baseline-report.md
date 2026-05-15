# Baseline Report — vb-core-lower-values-actions-refs

## Git Log (HEAD~5)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Baseline Diff (HEAD vs HEAD~5 — 61 files changed)

Touched crates/files summary (most relevant):

| Crate / Path | Nature of Change |
|---|---|
| `crates/vb_core/` | Core engine types, taint, capability, replay, validation, workflow, budget, kani harnesses |
| `crates/vb_compile/` | Compiler lowering (primary target for this bead) |
| `crates/velvet_ballastics/` | CLI postcard, exit codes, main |
| `crates/vb_storage/` | Recovery subsystem |
| `crates/vb_runtime/` | Runtime recovery |
| `crates/vb_ui_model/` | Postcard tests, binary decode fuzz |
| `fuzz/` | New fuzz targets for postcard decode |
| `verification/verus/` | Verus diagnostic envelope updates |
| `.beads/` | Bead tracking state |

## Known Constraints

1. **No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`**, or `dbg` in runtime core
2. **Generated Rust mode mandatory** for maxperf execution paths
3. **No YAML/JSON/HTTP in runtime core** — YAML is parsing/input only; core is data-model only
4. **Holzman Rust governance**: pinned nightly, feature whitelist enforced via `scripts/check-nightly-features.sh`
5. **Source lint is zero tolerance**; tests must compile and run
6. **`bd` / Dolt workflow**: commits must not include `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, or runtime database state
7. The bead targets `crates/vb_compile/src/lower.rs` (and related lowering infrastructure in `vb_compile`)

## Isolated Workspace

- Path: `/tmp/vb-ws/vb-core-lower-values-actions-refs`
- Not equal to source checkout: **YES**
- Not nested under source: **YES**
- Workspace path proof: `pwd -P` yields `/home/lewis/src/velvet-ballistics` (source checkout); isolated workspace is under `/tmp/` — completely separate filesystem tree
