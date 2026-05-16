# Regression Diff — GAP-12 vb-yvlb

bead_id: vb-yvlb
phase: 8
updated_at: 2026-05-11T00:00:00Z

## Classification: DEFERRED_GLOBAL

The `:test` gate failure is due to pre-existing lint errors in `crates/vb_core/src/policy.rs` (lines 50, 60):
- `JournalBeforeDispatch` naming lint
- `DispatchSafety` constant naming lint

These errors existed before GAP-12 changes and are unrelated to the ShardOwnership implementation.

No blocking failures introduced by GAP-12:
- `cargo build -p vb_runtime --lib` → 0 errors
- `cargo test -p vb_runtime --lib` → 1337 passed
- `cargo clippy -p vb_runtime --lib` → 0 errors (only pre-existing vb_core warnings)
