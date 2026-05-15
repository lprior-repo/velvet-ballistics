# Architecture drift refactor

STATUS: REFACTORED

Bead-owned naming scan scope was split to satisfy the 300-line physical file limit without changing behavior.

- Replaced the monolithic `crates/velvet_ballastics/src/naming_scan.rs` with cohesive modules under `crates/velvet_ballastics/src/naming_scan/`:
  - `types.rs`, `config.rs`, `classify.rs`, `allowlist.rs`, `legacy.rs`, `line_scan.rs`, `discovery.rs`, `repository.rs`, `report.rs`, `ordering.rs`, `mod.rs`.
- Split `tests/vb_37lc_canonical_spelling_red.rs` into a small harness plus focused test/helper modules under `tests/vb_37lc_canonical_spelling_red/`.
- Kept public API exports unchanged through `naming_scan/mod.rs`.
- Reduced focused function bodies to a maximum of 25 lines.

Validation:

- `rtk cargo test --test vb_37lc_canonical_spelling_red` — 76 passed.
- `rtk cargo clippy --test vb_37lc_canonical_spelling_red -- -D warnings` — no issues.
- Focused file line count — all files <= 300 lines.
- Focused function length — max 25 lines.
- Forbidden construct scan — no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` in focused naming scan scope.
