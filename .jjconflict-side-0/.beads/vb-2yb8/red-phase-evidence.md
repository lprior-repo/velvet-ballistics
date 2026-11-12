# Red Phase Evidence — vb-2yb8

## Date: 2026-05-09

### Test Compilation
- `cargo test -p vb_runtime --test durability_matrix_integration` compiles successfully
- Integration tests: 9 tests total

### Test Results (Red Phase)
- 8 tests PASSED
- 1 test FAILED (intentionally — missing "repeat" primitive)

### Failing Test
```
gate_fails_when_primitive_row_is_missing
  Expected: Matrix should be complete
  Actual: Err(MissingPrimitiveRow { primitive: "repeat" })
```

### Tests Created
1. `submit_handler_persists_before_ack`
2. `action_completed_persists_before_ack`
3. `action_failed_persists_before_ack`
4. `ask_answered_persists_before_ack`
5. `cancel_persists_before_ack`
6. `timer_fired_persists_before_ack`
7. `gate_fails_when_primitive_row_is_missing` — FAILING (red)
8. `gate_fails_when_row_omits_replay_evidence`
9. `gate_fails_when_row_claims_ack_before_persist`

### Files Modified
- `crates/vb_runtime/src/durability_matrix.rs` — new module
- `crates/vb_runtime/src/lib.rs` — module declaration
- `crates/vb_runtime/tests/durability_matrix_integration.rs` — new integration tests

STATUS: RED (tests compile, one intentionally failing)
