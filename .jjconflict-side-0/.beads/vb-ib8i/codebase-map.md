bead_id: vb-ib8i
phase: 2
updated_at: 2026-05-17T22:06:00Z
attempt: 1-of-7

Mapped scope:
- `crates/vb_expr/src/eval.rs`: unused arity-check bindings blocking compile with `-D warnings`.
- `fuzz/src/lib.rs`: clippy `expect_used` and collapsible-if blocker.
- `crates/workspace_tests/benches/*.rs`: stale benchmark API drift and rustfmt drift.
- `crates/workspace_tests/Cargo.toml` and `Cargo.lock`: add existing workspace dependencies required by restored bench targets (`crossbeam-queue`, `rtrb`, `serde`).
- Formatting-only drift touched multiple Rust files after `rtk cargo fmt --all`.

No excluded bead metadata was edited.
