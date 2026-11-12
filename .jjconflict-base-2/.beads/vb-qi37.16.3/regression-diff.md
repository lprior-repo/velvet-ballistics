# Regression Diff — vb-qi37.16.3 (State 8)

**Bead:** vb-qi37.16.3
**Phase:** State 8 regression classification
**Date:** 2026-05-11

---

## STATUS: PASS_WITH_DEFERRED_GLOBAL

## Delivery Scope Check

vb-qi37.16.3 scope is durable retry transition in `vb_runtime` retry/action-failure lifecycle behavior, plus repaired TLA files under `specs/RetryFSM.*` and `specs/RetryJournal.*`.

## Gate Outcomes

| Gate | Outcome | Classification |
|---|---|---|
| `rtk cargo test -p vb_runtime --test durable_retry_red_phase` | `9 passed` | PASS |
| `rtk cargo test -p vb_runtime --lib` | `1337 passed` | PASS |
| `moon run :quick` | `Tasks: 1 completed` | PASS |
| `moon run :test` | `9860 passed, 0 skipped` | PASS |
| `rtk cargo fmt -- --check` | formatting diffs in unrelated global files | DEFERRED_GLOBAL |

## Deferred Global Evidence

The format failure touched unrelated proof/Kani/Miri/storage/fuzz/xtask files, not the vb-qi37.16.3 retry lifecycle scope:

- `crates/vb_core/src/engine/expr_eval/kani_stack.rs`
- `crates/vb_core/src/ids/kani_id_bounds.rs`
- `crates/vb_core/src/kani_expr_bound.rs`
- `crates/vb_expr/src/lexer/miri_tests.rs`
- `crates/vb_expr/src/parser/miri_tests.rs`
- `crates/vb_proof_kernels/src/envelope_header.rs`
- `crates/vb_storage/src/codec_miri_tests.rs`
- `fuzz/fuzz_targets/decode_record.rs`
- `xtask/src/main.rs`
- `xtask/src/proof.rs`

## Retry Class

`DEFERRED_GLOBAL` follow-up required. No `BLOCK_LOCAL`, `BLOCK_REGRESSION`, `BLOCK_RELEASE`, or `REQUIRED_OBLIGATION_FAIL` found for vb-qi37.16.3 in this State 8 rerun.
