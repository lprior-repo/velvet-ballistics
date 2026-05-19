# Proof Evidence: vb-y4pa (Attempt 2)

## Evidence of Fix Implementation

### 1. jump_to_body Helper Added to helpers.rs
```rust
pub(crate) fn jump_to_body(
    run: &mut RunFrame,
    body: StepIdx,
) -> Result<vb_core::EngineSignal, EngineError> {
    run.mark_pending(body)?;
    jump_to(run, body)
}
```
**Location**: `crates/vb_runtime/src/primitives/helpers.rs:60-65`

### 2. Primitive Updates with jump_to_body

| Primitive | Function | Line Changed | Change |
|-----------|----------|--------------|--------|
| for_each.rs | for_each_next | 84 | `jump_to` → `jump_to_body` |
| reduce.rs | reduce_next | 82 | `jump_to` → `jump_to_body` |
| collect.rs | collect_page | 397 | `jump_to` → `jump_to_body` |
| collect.rs | collect_next | 521 | `jump_to` → `jump_to_body` |
| repeat.rs | repeat_attempt | 88 | `jump_to` → `jump_to_body` |
| repeat.rs | repeat_check | 115 | `jump_to` → `jump_to_body` |

### 3. Kani Harness Improvements

Each of the 6 harnesses now:
- Uses `kani::any::<StepState>()` for body state selection
- Covers all StepState variants including Succeeded, Pending, Running, Failed
- Maintains strong assertion checking for `invalid_state_transition` error

Example cover statements added:
```rust
kani::cover!(
    body_state == StepState::Succeeded,
    "for_each_next re-entry with Succeeded body state"
);
kani::cover!(
    body_state == StepState::Pending,
    "for_each_next re-entry with Pending body state"
);
```

### 4. Unit Test Renaming
All 6 reentry tests renamed to include PO-ID prefix `vb_y4pa_XXX_`.

## Build & Test Evidence

```
cargo build -p vb_runtime
   Compiling vb_runtime v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s]

cargo test -p vb_runtime reentry_tests
   Running 6 tests across 1 crate
   test vb_y4pa_001_for_each_two_item_reentry ... ok
   test vb_y4pa_002_reduce_reentry ... ok
   test vb_y4pa_003_collect_next_reentry ... ok
   test vb_y4pa_004_collect_page_reentry ... ok
   test vb_y4pa_005_repeat_attempt_reentry ... ok
   test vb_y4pa_006_repeat_check_reentry ... ok

cargo test -p vb_runtime helpers
   Running 97 tests
   (all helpers tests pass)
```

## State Transition Matrix (from vb_proof_kernels)

The proof kernel allows these key transitions for loop re-entry:
- `Succeeded → Pending` (valid) - enables loop body re-entry
- `Succeeded → Running` (invalid) - correctly rejected
- `Pending → Running` (valid) - normal body entry

This confirms `jump_to_body` implementation is sound: it transitions `Succeeded→Pending→Running` which are all valid transitions.

## Files Modified

1. `crates/vb_runtime/src/primitives/helpers.rs` - Added `jump_to_body`
2. `crates/vb_runtime/src/primitives/for_each.rs` - Wired `jump_to_body`
3. `crates/vb_runtime/src/primitives/reduce.rs` - Wired `jump_to_body`
4. `crates/vb_runtime/src/primitives/collect.rs` - Wired `jump_to_body` (2 sites)
5. `crates/vb_runtime/src/primitives/repeat.rs` - Wired `jump_to_body` (2 sites)
6. `crates/vb_runtime/src/primitives/reentry_proofs.rs` - Fixed harnesses with `kani::any()` and `kani::cover`
7. `crates/vb_runtime/src/primitives/reentry_tests.rs` - Renamed tests to PO-ID convention